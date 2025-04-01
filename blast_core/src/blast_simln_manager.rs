// Standard libraries
use std::sync::Arc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs::File;
use std::io::BufReader;
use std::fs;
use std::env;

// Extra dependencies
use serde::{Serialize, Deserialize};
use simln_lib::ActivityDefinition;
use simln_lib::Simulation;
use simln_lib::LightningNode;
use simln_lib::*;
use simln_lib::lnd::*;
use simln_lib::cln::*;
use anyhow::{anyhow, Error};
use bitcoin::secp256k1::PublicKey;
use tokio::sync::Mutex;
use tokio_util::task::TaskTracker;

/// The expected payment amount for the sim-ln simulation
pub const EXPECTED_PAYMENT_AMOUNT: u64 = 3_800_000;

/// The activity multiplier for the sim-ln simulation
pub const ACTIVITY_MULTIPLIER: f64 = 2.0;

/// The directory to write the sim-ln results to
pub const RESULTS_DIR: &str = ".blast/blast_results";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SimParams {
    pub nodes: Vec<NodeConnection>,
    #[serde(default)]
    pub activity: Vec<ActivityParser>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
enum NodeConnection {
    Lnd(lnd::LndConnection),
    Cln(cln::ClnConnection),
    Eclair(eclair::EclairConnection),
}

/// Data structure used to parse information from the simulation file. It allows source and destination to be
/// [NodeId], which enables the use of public keys and aliases in the simulation description.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActivityParser {
    /// The source of the payment.
    #[serde(with = "serializers::serde_node_id")]
    pub source: NodeId,
    /// The destination of the payment.
    #[serde(with = "serializers::serde_node_id")]
    pub destination: NodeId,
    /// The time in the simulation to start the payment.
    pub start_secs: Option<u16>,
    /// The number of payments to send over the course of the simulation.
    #[serde(default)]
    pub count: Option<u64>,
    /// The interval of the event, as in every how many seconds the payment is performed.
    #[serde(with = "serializers::serde_value_or_range")]
    pub interval_secs: Interval,
    /// The amount of m_sat to used in this payment.
    #[serde(with = "serializers::serde_value_or_range")]
    pub amount_msat: Amount,
}

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
    pub fn add_activity(&mut self, source: &str, destination: &str, start_secs: Option<u16>, count: Option<u64>, interval_secs: u16, amount_msat: u64) {
        let a = ActivityParser{source: NodeId::Alias(String::from(source)), destination: NodeId::Alias(String::from(destination)), start_secs: start_secs, count: count, interval_secs: ValueOrRange::Value(interval_secs), amount_msat: ValueOrRange::Value(amount_msat)};
        self.data.activity.push(a);
    }

    /// Get all of the current activity
    pub fn get_activity(&self) -> Vec<String> {
        let mut act: Vec<String> = Vec::new();
        for a in &self.data.activity {
            let start = match a.start_secs {
                Some(i) => { i.to_string() },
                None => { String::from("None") }
            };
            let count = match a.count {
                Some(i) => { i.to_string() },
                None => { String::from("None") }
            };
            act.push(format!("{} {} {} {} {} {}", a.source, a.destination, start, count, a.interval_secs, a.amount_msat));
        }

        act
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
                NodeConnection::Lnd(c) => {
                    if c.address.is_empty() {
                        continue;
                    }
                    Arc::new(Mutex::new(LndNode::new(c).await?))
                },
                NodeConnection::Cln(c) => Arc::new(Mutex::new(ClnNode::new(c).await?)),
                NodeConnection::Eclair(_) => todo!(),
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

        let home = env::var("HOME").expect("HOME environment variable not set");
        let folder_path = PathBuf::from(home).join(RESULTS_DIR);

        let tasks = TaskTracker::new();
        let sim = Simulation::new(
            SimulationCfg::new(
                Some(crate::TOTAL_FRAMES as u32),
                EXPECTED_PAYMENT_AMOUNT,
                ACTIVITY_MULTIPLIER,
                Some(WriteResults {
                    results_dir: folder_path,
                    batch_size: 1,
                }),
                None
            ),
            clients,
            validated_activities,
            tasks
        );
        self.sim = Some(sim);
        Ok(())
    }

    /// Run SimLn simulation
    pub async fn start(&self) -> Result<(), Error> {
        log::info!("BlastSimlnManager starting simulation.");

        let home = env::var("HOME").expect("HOME environment variable not set");
        let folder_path = PathBuf::from(home).join(RESULTS_DIR);

        // Create the results directory if it does not exist
        match fs::create_dir_all(folder_path) {
            Ok(_) => {},
            Err(e) => return Err(anyhow!("Error creating results directory: {}", e))
        };

        // Start the sim-ln simulation
        match &self.sim {
            Some(s) => {
                match s.run().await {
                    Ok(_) => {},
                    Err(e) => {
                        return Err(anyhow!("Error starting simulation: {:?}", e));
                    }
                }
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

    /// Reset the simln manager when the current blast network is shutdown
    pub fn reset(&mut self) {
        log::info!("BlastSimlnManager resetting.");
        self.data.activity.clear();
        self.data.nodes.clear();
    }

    /// Get all the nodes
    pub fn get_nodes(&self) -> Vec<String> {
        let mut ids = Vec::<String>::new();
        for n in &self.data.nodes {
            let id = match n {
                NodeConnection::Lnd(c) => c.id.to_string(),
                NodeConnection::Cln(c) => c.id.to_string(),
                NodeConnection::Eclair(c) => c.id.to_string(),
            };
            ids.push(id);
        }
        ids
    }

    pub async fn get_success_rate(&self) -> f64 {
        match &self.sim {
            Some(s) => {
                s.get_success_rate().await
            },
            None => { 0.0 }
        }
    }

    pub async fn get_attempts(&self) -> u64 {
        match &self.sim {
            Some(s) => {
                s.get_total_payments().await
            },
            None => { 0 }
        }
    }
}
