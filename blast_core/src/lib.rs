
use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::collections::HashMap;

use simple_logger::SimpleLogger;
use log::LevelFilter;
use anyhow::anyhow;
use bitcoin::secp256k1::PublicKey;
use tokio::sync::Mutex;

use sim_lib::ActivityDefinition;
use sim_lib::Simulation;
use sim_lib::LightningNode;
use sim_lib::SimParams;
use sim_lib::*;
use sim_lib::lnd::*;
use sim_lib::cln::*;

use blast_model_interface::BlastModelInterface;

pub const EXPECTED_PAYMENT_AMOUNT: u64 = 3_800_000;
pub const ACTIVITY_MULTIPLIER: f64 = 2.0;

#[derive(Clone)]
pub struct Blast {
    blast_model_interface: BlastModelInterface,
    simln: Option<Simulation>,
    simln_json: Option<String>
}

impl Blast {
    pub fn new() -> Self {
        let blast = Blast {
            blast_model_interface: BlastModelInterface::new(),
            simln: None,
            simln_json: None
        };

        blast
    }

    pub async fn start_simulation(&mut self) -> anyhow::Result<()> {
        SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();

        log::info!("Setting up BLAST Simulation");

        let SimParams { nodes, activity } = 
        serde_json::from_str(&self.simln_json.as_ref().unwrap())
        .map_err(|e| anyhow!("Could not deserialize node connection data or activity description from simulation file (line {}, col {}).", e.line(), e.column()))?;

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
            None,
        );
        self.simln = Some(sim.clone());

        log::info!("Running BLAST Simulation");
        sim.run().await?;
        Ok(())
    }

    pub fn stop_simulation(&mut self) {
        log::info!("Stopping BLAST Simulation");
        match &self.simln {
            Some(s) => {
                s.shutdown();
            },
            None => {}
        };
    }

    pub async fn start_model(&mut self, model: String, running: Arc<AtomicBool>) -> Result<Child, String> {
        self.blast_model_interface.start_model(model, running).await
    }
    
    pub async fn start_nodes(&mut self, model: String, num_nodes: i32) -> Result<(), String> {
        match self.blast_model_interface.start_nodes(model, num_nodes).await {
            Ok(s) => {
                self.simln_json = Some(s);
                Ok(())
            },
            Err(e) => {
                Err(format!("Error starting nodes: {}", e))
            }
        }
    }

    pub async fn get_pub_key(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_interface.get_pub_key(node_id).await {
            Ok(s) => {
                Ok(s)
            },
            Err(e) => {
                Err(format!("Error getting pub key: {}", e))
            }
        }
    }

    pub async fn list_peers(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_interface.list_peers(node_id).await {
            Ok(s) => {
                Ok(s)
            },
            Err(e) => {
                Err(format!("Error getting peers: {}", e))
            }
        }
    }
}
