// Standard libraries
use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs::File;
use std::io::BufReader;
use std::fs;

// Extra dependencies
use serde::{Serialize, Deserialize};
use sim_lib::ActivityDefinition;
use sim_lib::Simulation;
use sim_lib::LightningNode;
use sim_lib::SimParams;
use sim_lib::*;
use sim_lib::lnd::*;
use sim_lib::cln::*;
use anyhow::{anyhow, Error};
use bitcoin::secp256k1::PublicKey;
use tokio::sync::Mutex;

/// The expected payment amount for the sim-ln simulation
pub const EXPECTED_PAYMENT_AMOUNT: u64 = 3_800_000;

/// The activity multiplier for the sim-ln simulation
pub const ACTIVITY_MULTIPLIER: f64 = 2.0;

/// The directory to write the sim-ln results to
pub const RESULTS_DIR: &str = "/home/blast_results";

/// The BlastSimLnManager holds the main sim-ln Simulation object and the current node and activity data that sim-ln uses
#[derive(Clone)]
pub struct BlastSimLnManager {
    sim: Option<Simulation>,
    data: BlastSimLnData
}

/// The BlastSimLnData is the live objects that are created from the sim-ln json file
#[derive(Serialize, Deserialize, Clone)]
struct BlastSimLnData {
    nodes: Vec<NodeConnection>,
    activity: Vec<ActivityParser>
}

impl BlastSimLnManager {
    /// Create a new sim-ln manager without any nodes or activity
    pub fn new() -> Self {
        let data = BlastSimLnData {
            activity: Vec::<ActivityParser>::new(),
            nodes: Vec::<NodeConnection>::new()
        };

        let simln = BlastSimLnManager {
            sim: None,
            data: data
        };

        simln
    }

    /// Create payment activity for the simulation
    pub fn add_activity(&mut self, source: &str, destination: &str, start_secs: u16, count: Option<u64>, interval_secs: u16, amount_msat: u64) {
        let a = ActivityParser{source: NodeId::Alias(String::from(source)), destination: NodeId::Alias(String::from(destination)), start_secs: start_secs, count: count, interval_secs: ValueOrRange::Value(interval_secs), amount_msat: ValueOrRange::Value(amount_msat)};
        self.data.activity.push(a);
    }

    /// Add nodes from a json string returned by the model
    pub fn add_nodes(&mut self, s: String) -> Result<(), String> {
        let SimParams { mut nodes, .. } = match serde_json::from_str(&s) {
            Ok(sp) => sp,
            Err(e) => return Err(format!("Error adding nodes: {}", e))
        };

        self.data.nodes.append(&mut nodes);
        Ok(())
    }

    /// Get simln json data
    pub fn get_simln_json(&self) -> Result<String, String> {
        match serde_json::to_string(&self.data) {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting simln data: {}", e))
        }
    }

    /// Set the simln data from a json file
    pub fn set_simln_json(&mut self, path: &str) -> Result<(), String> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) => return Err(format!("Error opening simln file: {}", e)),
        };

        let reader = BufReader::new(file);
        self.data = match serde_json::from_reader(reader) {
            Ok(d) => d,
            Err(e) => return Err(format!("Error reading simln data: {}", e)),
        };

        Ok(())
    }

    /// Create a simln simulation from the json data blast gets from each model in the sim
    pub async fn setup_simln(&mut self) -> Result<(), anyhow::Error> {
        let nodes = self.data.nodes.clone();
        let activity = self.data.activity.clone();

        let mut clients: HashMap<PublicKey, Arc<Mutex<dyn LightningNode>>> = HashMap::new();
        let mut pk_node_map = HashMap::new();
        let mut alias_node_map = HashMap::new();
        for connection in nodes {
            let node: Arc<Mutex<dyn LightningNode>> = match connection {
                NodeConnection::LND(c) => {
                    Arc::new(Mutex::new(LndNode::new(c).await?))
                },
                NodeConnection::CLN(c) => Arc::new(Mutex::new(ClnNode::new(c).await?)),
            };

            let node_info = node.lock().await.get_info().clone();

            if alias_node_map.contains_key(&node_info.alias) {
                anyhow::bail!(LightningError::ValidationError(format!(
                    "duplicated node: {}.",
                    node_info.alias
                )));
            }

            clients.insert(node_info.pubkey, node);
            pk_node_map.insert(node_info.pubkey, node_info.clone());
            alias_node_map.insert(node_info.alias.clone(), node_info);
        }

        let mut validated_activities = vec![];
        for act in activity.into_iter() {
            let source = if let Some(source) = match &act.source {
                NodeId::PublicKey(pk) => pk_node_map.get(pk),
                NodeId::Alias(a) => alias_node_map.get(a),
            } {
                source.clone()
            } else {
                anyhow::bail!(LightningError::ValidationError(format!(
                    "activity source {} not found in nodes.",
                    act.source
                )));
            };

            let destination = match &act.destination {
                NodeId::Alias(a) => {
                    if let Some(info) = alias_node_map.get(a) {
                        info.clone()
                    } else {
                        anyhow::bail!(LightningError::ValidationError(format!(
                            "unknown activity destination: {}.",
                            act.destination
                        )));
                    }
                },
                NodeId::PublicKey(pk) => {
                    if let Some(info) = pk_node_map.get(pk) {
                        info.clone()
                    } else {
                        clients
                            .get(&source.pubkey)
                            .unwrap()
                            .lock()
                            .await
                            .get_node_info(pk)
                            .await
                            .map_err(|_| {
                                LightningError::ValidationError(format!(
                                    "Destination node unknown or invalid: {}.",
                                    pk,
                                ))
                            })?
                    }
                },
            };

            validated_activities.push(ActivityDefinition {
                source,
                destination,
                start_secs: act.start_secs,
                count: act.count,
                interval_secs: act.interval_secs,
                amount_msat: act.amount_msat,
            });
        }

        let sim = Simulation::new(
            clients,
            validated_activities,
            None,
            EXPECTED_PAYMENT_AMOUNT,
            ACTIVITY_MULTIPLIER,
            Some(WriteResults {
                results_dir: PathBuf::from(String::from(RESULTS_DIR)),
                batch_size: 1,
            })
        );
        self.sim = Some(sim);
        Ok(())
    }

    /// Run SimLn simulation
    pub async fn start(&self) -> Result<(), Error> {
        log::info!("BlastSimlnManager starting simulation.");

        // Create the results directory if it does not exist
        match fs::create_dir_all(RESULTS_DIR) {
            Ok(_) => {},
            Err(e) => return Err(anyhow!("Error creating results directory: {}", e))
        };

        // Start the sim-ln simulation
        match &self.sim {
            Some(s) => {
                s.run().await?;
                Ok(())
            },
            None => return Err(anyhow!("Simln not setup. Call set_simln before starting the simulation")),
        }        
    }

    /// Stop SimLn simulation
    pub fn stop(&self) {
        log::info!("BlastSimlnManager stopping simulation.");
        match &self.sim {
            Some(s) => s.shutdown(),
            None => {}
        };
    }

    /// Get all the nodes
    pub fn get_nodes(&self) -> Vec<String> {
        let mut ids = Vec::<String>::new();
        for n in &self.data.nodes {
            let id = match n {
                NodeConnection::LND(c) => c.id.to_string(),
                NodeConnection::CLN(_) => String::from(""),
            };
            ids.push(id);
        }
        ids
    }
}
