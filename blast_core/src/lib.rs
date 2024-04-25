
use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::collections::HashMap;
use std::str::FromStr;
use std::path::PathBuf;
use std::{thread, time};

use bitcoincore_rpc::Auth;
use bitcoincore_rpc::RpcApi;
use simple_logger::SimpleLogger;
use log::LevelFilter;
use anyhow::anyhow;
use bitcoin::secp256k1::PublicKey;
use tokio::sync::Mutex;
use bitcoincore_rpc::Client;

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

/// The Blast struct is the main public interface that can be used to run a simulation.
pub struct Blast {
    blast_model_interface: BlastModelInterface,
    simln: Option<Simulation>,
    simln_json: Option<String>,
    bitcoin_rpc: Option<Client>
}

impl Clone for Blast {
    fn clone(&self) -> Self {
        Self {
            blast_model_interface: self.blast_model_interface.clone(),
            simln: self.simln.clone(),
            simln_json: self.simln_json.clone(),
            bitcoin_rpc: match Client::new("http://127.0.0.1:18443/", Auth::UserPass(String::from("user"), String::from("pass"))) {
                Ok(c) => {
                    Some(c)
                },
                Err(_) => {
                    None
                }
            }
        }
    }
}

impl Blast {
    /// Create a new Blast object with a new BlastModelInterface.
    pub fn new() -> Result<Self, String> {
        // Connect to bitcoind RPC server
        let client = Client::new("http://127.0.0.1:18443/", Auth::UserPass(String::from("user"), String::from("pass"))).map_err(|e| e.to_string())?;

        let blast = Blast {
            blast_model_interface: BlastModelInterface::new(),
            simln: None,
            simln_json: None,
            bitcoin_rpc: Some(client)
        };

        Ok(blast)
    }

    /// Set up Simln
    pub async fn set_simln(&mut self) -> anyhow::Result<()> {
        let sim = self.setup_simln().await?;
        self.simln = Some(sim);
        Ok(())
    }

    /// Start the simulation.
    pub async fn start_simulation(&mut self) -> anyhow::Result<()> {
        // Set up the logger
        SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();

        log::info!("Running BLAST Simulation");

        log::info!("Running BLAST Event Manager");
        // TODO: start the event thread

        log::info!("Running SimLN Simulation");
        
        match &self.simln {
            Some(s) => {
                s.run().await?;
            },
            None => {
                return Err(anyhow!("SimLN not setup. Call set_simln before running the simulation"));
            }
        }

        Ok(())
    }

    /// Stop the simulation.
    pub fn stop_simulation(&mut self) {
        log::info!("Stopping BLAST Simulation");
        match &self.simln {
            Some(s) => {
                log::info!("some");
                s.shutdown();
            },
            None => {
                log::info!("none");
            }
        };
    }

    /// Start a model by name and wait for the RPC connection to be made.
    pub async fn start_model(&mut self, model: String, running: Arc<AtomicBool>) -> Result<Child, String> {
        self.blast_model_interface.start_model(model, running).await
    }
    
    /// Start a given number of nodes for the given model name.
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

    /// Get the public key of a node in the simulation.
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

    /// Get the peers of a node in the simulation.
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

    pub async fn wallet_balance(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_interface.wallet_balance(node_id).await {
            Ok(s) => {
                Ok(s)
            },
            Err(e) => {
                Err(format!("Error getting wallet balance: {}", e))
            }
        }
    }

    pub async fn channel_balance(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_interface.channel_balance(node_id).await {
            Ok(s) => {
                Ok(s)
            },
            Err(e) => {
                Err(format!("Error getting channel balance: {}", e))
            }
        }
    }

    pub async fn list_channels(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_interface.list_channels(node_id).await {
            Ok(s) => {
                Ok(s)
            },
            Err(e) => {
                Err(format!("Error getting channels: {}", e))
            }
        }
    }

    pub async fn open_channel(&mut self, node1_id: String, node2_id: String, amount: i64, push_amount: i64) -> Result<(), String> {
        match self.blast_model_interface.open_channel(node1_id, node2_id, amount, push_amount).await {
            Ok(_) => {
                thread::sleep(time::Duration::from_secs(10));
                let mine_address = bitcoincore_rpc::bitcoin::Address::from_str("bcrt1qwl7p045lawx8tx3ecttu0dmt6pqjlrqdlhz6yt").map_err(|e|e.to_string())?
                .require_network(bitcoincore_rpc::bitcoin::Network::Regtest).map_err(|e|e.to_string())?;
                let _ = self.bitcoin_rpc.as_mut().unwrap().generate_to_address(100, &mine_address).map_err(|e| e.to_string())?;
                Ok(())
            },
            Err(e) => {
                Err(format!("Error opening a channel: {}", e))
            }
        }
    }

    pub async fn close_channel(&mut self) -> Result<(), String> {
        match self.blast_model_interface.close_channel().await {
            Ok(_) => {
                Ok(())
            },
            Err(e) => {
                Err(format!("Error closing a channel: {}", e))
            }
        }
    }

    pub async fn connect_peer(&mut self, node1_id: String, node2_id: String) -> Result<(), String> {
        match self.blast_model_interface.connect_peer(node1_id, node2_id).await {
            Ok(_) => {
                Ok(())
            },
            Err(e) => {
                Err(format!("Error connecting to peer: {}", e))
            }
        }
    }

    pub async fn disconnect_peer(&mut self, node1_id: String, node2_id: String) -> Result<(), String> {
        match self.blast_model_interface.disconnect_peer(node1_id, node2_id).await {
            Ok(_) => {
                Ok(())
            },
            Err(e) => {
                Err(format!("Error disconnecting from peer: {}", e))
            }
        }
    }

    pub async fn fund_node(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_interface.get_btc_address(node_id).await {
            Ok(a) => {
                let address = bitcoincore_rpc::bitcoin::Address::from_str(&a).map_err(|e|e.to_string())?
               .require_network(bitcoincore_rpc::bitcoin::Network::Regtest).map_err(|e|e.to_string())?;
                let txid = self.bitcoin_rpc.as_mut().unwrap().send_to_address(&address, bitcoincore_rpc::bitcoin::Amount::ONE_BTC, None, None, None, None, None, None)
                .map_err(|e| e.to_string())?;

                let mine_address = bitcoincore_rpc::bitcoin::Address::from_str("bcrt1qwl7p045lawx8tx3ecttu0dmt6pqjlrqdlhz6yt").map_err(|e|e.to_string())?
                .require_network(bitcoincore_rpc::bitcoin::Network::Regtest).map_err(|e|e.to_string())?;
                let _ = self.bitcoin_rpc.as_mut().unwrap().generate_to_address(100, &mine_address).map_err(|e| e.to_string())?;
                Ok(format!("{}", txid))
            },
            Err(e) => {
                Err(format!("Error getting address: {}", e))
            }
        }
    }

    async fn setup_simln(&self) -> Result<Simulation, anyhow::Error> {
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
            Some(WriteResults {
                results_dir: PathBuf::from(String::from("/home/simln_results")),
                batch_size: 1,
            })
        );
        Ok(sim)
    }

}
