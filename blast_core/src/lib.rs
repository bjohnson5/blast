use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use std::thread;

use bitcoincore_rpc::Auth;
use bitcoincore_rpc::RpcApi;
use simple_logger::SimpleLogger;
use log::LevelFilter;
use anyhow::Error;
use bitcoincore_rpc::Client;
use tokio::task::JoinSet;
use tokio::sync::mpsc;

mod blast_model_manager;
use crate::blast_model_manager::*;
mod blast_event_manager;
use crate::blast_event_manager::*;
mod blast_simln_manager;
use crate::blast_simln_manager::*;

/// The Blast struct is the main public interface that can be used to run a simulation.
pub struct Blast {
    blast_model_manager: BlastModelManager,
    blast_event_manager: BlastEventManager,
    blast_simln_manager: BlastSimLnManager,
    network: Option<BlastNetwork>,
    bitcoin_rpc: Option<Client>,
}

/// The BlastNetwork describes how many nodes are run for each model.
#[derive(Clone)]
pub struct BlastNetwork {
    name: String,
    model_map: HashMap<String, i32>
}

impl Blast {
    /// Create a new Blast object with a new BlastModelManager.
    pub fn new() -> Self {
        // Set up the logger
        SimpleLogger::new()
        .with_level(LevelFilter::Debug)
        .init()
        .unwrap();

        let blast = Blast {
            blast_model_manager: BlastModelManager::new(),
            blast_event_manager: BlastEventManager::new(),
            blast_simln_manager: BlastSimLnManager::new(),
            network: None,
            bitcoin_rpc: match Client::new("http://127.0.0.1:18443/", Auth::UserPass(String::from("user"), String::from("pass"))) {
                Ok(c) => Some(c),
                Err(_) => None
            }
        };

        blast
    }

    /// Create a new network from scratch.
    pub fn create_network(&mut self, name: &str, model_map: HashMap<String, i32>) {
        log::info!("Creating BLAST Network");
        self.network = Some(BlastNetwork{name: String::from(name), model_map: model_map});
    }

    /// Start the simulation network. This will start the models and nodes, fund them with initial balances, open initial channels, etc.
    pub async fn start_network(&mut self, running: Arc<AtomicBool>) -> Result<Vec<Child>, String> {
        log::info!("Starting BLAST Network");

        let net = match &self.network {
            Some(n) => n,
            None => return Err(format!("No network found")),
        };

        let mut child_list: Vec<Child> = Vec::new();
        for (key, value) in net.model_map.clone().into_iter() {
            let child = self.start_model(key.clone(), running.clone()).await?;
            child_list.push(child);
            match self.start_nodes(key.clone(), value).await {
                Ok(_) => {},
                Err(e) => return Err(format!("Unable to start nodes: {}", e)),
            }
        }

        Ok(child_list)
    }

    /// Shutdown the simulation network. This will shutdown the models and nodes.
    pub async fn stop_network(&mut self) -> Result<(), String> {
        log::info!("Stopping BLAST Network");

        let net = match &self.network {
            Some(n) => n,
            None => return Err(format!("No network found")),
        };

        for (key, _) in net.model_map.clone().into_iter() {
            self.stop_model(key).await?
        }

        Ok(())
    }

    /// Gets the simulation ready to run.
    pub async fn finalize_simulation(&mut self) -> Result<(), String> {
        log::info!("Finalizing BLAST Simulation");

        match self.blast_simln_manager.setup_simln().await {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to setup simln: {:?}", e)),
        };

        Ok(())
    }

    /// Start the simulation. This will start the simulation events and the simln transaction generation.
    pub async fn start_simulation(&mut self) -> Result<JoinSet<Result<(),Error>>, String> {
        let net = match &self.network {
            Some(n) => n,
            None => return Err(format!("No network found")),
        };

        log::info!("Starting BLAST Simulation for {}", net.name);

        // Wait for all node announcements to take place (lnd - trickledelay)
        thread::sleep(Duration::from_secs(10));

        let mut sim_tasks = JoinSet::new();
        let simln_man = self.blast_simln_manager.clone();
        let mut event_man = self.blast_event_manager.clone();
        let mut model_man = self.blast_model_manager.clone();
        let (sender, receiver) = mpsc::channel(1);

        sim_tasks.spawn(async move {
            // Start the simln thread
            simln_man.start().await
        });

        sim_tasks.spawn(async move {
            // Start the event thread
            event_man.start(sender).await
        });

        sim_tasks.spawn(async move {
            // Start the model manager thread
            model_man.process_events(receiver).await
        });

        Ok(sim_tasks)
    }

    /// Stop the simulation. This will stop the simulation events and the simln transaction generation.
    pub fn stop_simulation(&mut self) {
        log::info!("Stopping BLAST Simulation");

        // Stop the event thread
        self.blast_event_manager.stop();

        // Stop the simln thread
        self.blast_simln_manager.stop();
    }

    /// Load a simulation. This will load a saved simulation network (nodes, channels, balance) and load events/activity.
    pub fn load(&mut self) {
        log::info!("Loading BLAST Simulation");
    }

    /// Save a simulation. This will save off the current simulation network (nodes, channels, balances) and save events/activity.
    pub fn save(&mut self) {
        log::info!("Saving BLAST Simulation");
    }

    /// Create payment activity for the simulation.
    pub fn add_activity(&mut self, source: &str, destination: &str, start_secs: u16, count: Option<u64>, interval_secs: u16, amount_msat: u64) {
        self.blast_simln_manager.add_activity(source, destination, start_secs, count, interval_secs, amount_msat);
    }

    /// Create an event for the simulation.
    pub fn add_event(&mut self, frame_num: u64, event: &str, args: Option<Vec<String>>) -> Result<(), String> {
        self.blast_event_manager.add_event(frame_num, event, args)
    }

    /// Get all the nodes.
    pub fn get_nodes(&self) -> Vec<String> {
        self.blast_simln_manager.get_nodes()
    }

    /// Get the public key of a node.
    pub async fn get_pub_key(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_manager.get_pub_key(node_id).await {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting pub key: {}", e))
        }
    }

    /// Get the peers of a node.
    pub async fn list_peers(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_manager.list_peers(node_id).await {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting peers: {}", e))
        }
    }

    /// Show this nodes on-chain balance.
    pub async fn wallet_balance(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_manager.wallet_balance(node_id).await {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting wallet balance: {}", e))
        }
    }

    /// Show this nodes off-chain balance.
    pub async fn channel_balance(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_manager.channel_balance(node_id).await {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting channel balance: {}", e))
        }
    }

    /// View open channels on this node.
    pub async fn list_channels(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_manager.list_channels(node_id).await {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting channels: {}", e))
        }
    }

    /// Open a channel and optionally mine blocks to confirm the channel.
    pub async fn open_channel(&mut self, node1_id: String, node2_id: String, amount: i64, push_amount: i64, confirm: bool) -> Result<(), String> {
        match self.blast_model_manager.open_channel(node1_id, node2_id, amount, push_amount).await {
            Ok(_) => {
                if confirm {
                    thread::sleep(Duration::from_secs(5));
                    mine_blocks(&mut self.bitcoin_rpc, 100)?;
                }
                Ok(())
            },
            Err(e) => Err(format!("Error opening a channel: {}", e))
        }
    }

    /// Close a channel.
    pub async fn close_channel(&mut self) -> Result<(), String> {
        match self.blast_model_manager.close_channel().await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error closing a channel: {}", e))
        }
    }

    /// Add a peer.
    pub async fn connect_peer(&mut self, node1_id: String, node2_id: String) -> Result<(), String> {
        match self.blast_model_manager.connect_peer(node1_id, node2_id).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error connecting to peer: {}", e))
        }
    }

    /// Remove a peer.
    pub async fn disconnect_peer(&mut self, node1_id: String, node2_id: String) -> Result<(), String> {
        match self.blast_model_manager.disconnect_peer(node1_id, node2_id).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error disconnecting from peer: {}", e))
        }
    }

    /// Send funds to a node on-chain and optionally mines blocks to confirm that payment.
    pub async fn fund_node(&mut self, node_id: String, confirm: bool) -> Result<String, String> {
        match self.blast_model_manager.get_btc_address(node_id).await {
            Ok(a) => {
                let address = bitcoincore_rpc::bitcoin::Address::from_str(&a).map_err(|e|e.to_string())?
                .require_network(bitcoincore_rpc::bitcoin::Network::Regtest).map_err(|e|e.to_string())?;
                let txid = self.bitcoin_rpc.as_mut().unwrap().send_to_address(&address, bitcoincore_rpc::bitcoin::Amount::ONE_BTC, None, None, None, None, None, None)
                .map_err(|e| e.to_string())?;

                if confirm {
                    mine_blocks(&mut self.bitcoin_rpc, 50)?;
                }
                Ok(format!("{}", txid))
            },
            Err(e) => Err(format!("Error getting address: {}", e))
        }
    }

    /// Start a model by name and wait for the RPC connection to be made.
    async fn start_model(&mut self, model: String, running: Arc<AtomicBool>) -> Result<Child, String> {
        self.blast_model_manager.start_model(model, running).await
    }

    /// Stop a model by name.
    async fn stop_model(&mut self, model: String) -> Result<(), String>{
        self.blast_model_manager.stop_model(model).await
    }
    
    /// Start a given number of nodes for the given model name.
    async fn start_nodes(&mut self, model: String, num_nodes: i32) -> Result<(), String> {
        match self.blast_model_manager.start_nodes(model, num_nodes).await {
            Ok(s) => self.blast_simln_manager.add_nodes(s),
            Err(e) => Err(format!("Error starting nodes: {}", e))
        }
    }
}

/// Mine new blocks.
pub fn mine_blocks(rpc: &mut Option<Client>, num_blocks: u64) -> Result<(), String> {
    let client = match rpc {
        Some(c) => c,
        None => return Err(format!("No bitcoind client found, unable to mine blocks."))
    };

    let mine_address = client.get_new_address(None, Some(bitcoincore_rpc::bitcoincore_rpc_json::AddressType::P2shSegwit))
    .unwrap().require_network(bitcoincore_rpc::bitcoin::Network::Regtest)
    .map_err(|e|e.to_string())?;
    let _ = client.generate_to_address(num_blocks, &mine_address).map_err(|e| e.to_string())?;
    Ok(())
}
