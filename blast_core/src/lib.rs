// Standard libraries
use std::process::{Command, Child};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use std::thread;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::io::BufReader;
use std::io;

// Extra dependencies
use bitcoincore_rpc::Auth;
use bitcoincore_rpc::RpcApi;
use anyhow::Error;
use bitcoincore_rpc::Client;
use tokio::task::JoinSet;
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};
use flate2::Compression;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use tar::Archive;

// Blast libraries
mod blast_model_manager;
use crate::blast_model_manager::*;
mod blast_event_manager;
use crate::blast_event_manager::*;
mod blast_simln_manager;
use crate::blast_simln_manager::*;

/// The RPC address for the bitcoind instance that is running
pub const BITCOIND_RPC: &str = "http://127.0.0.1:18443/";

/// The bitcoind RPC user
pub const BITCOIND_USER: &str = "user";

/// The bitcoind RPC password
pub const BITCOIND_PASS: &str = "pass";

/// The directory to save simulations in
pub const BLAST_SIM_DIR: &str = "/home/blast_sims";

/// The Blast struct is the main public interface that can be used to run a simulation.
pub struct Blast {
    blast_model_manager: BlastModelManager,
    blast_event_manager: BlastEventManager,
    blast_simln_manager: BlastSimLnManager,
    network: Option<BlastNetwork>,
    bitcoin_rpc: Option<Client>,
}

/// The BlastNetwork describes how many nodes are run for each model.
#[derive(Serialize, Deserialize, Clone)]
pub struct BlastNetwork {
    name: String,
    model_map: HashMap<String, i32>
}

impl Blast {
    /// Create a new Blast object with a new BlastModelManager.
    pub fn new() -> Self {
        // Create the blast object
        let blast = Blast {
            blast_model_manager: BlastModelManager::new(),
            blast_event_manager: BlastEventManager::new(),
            blast_simln_manager: BlastSimLnManager::new(),
            network: None,
            bitcoin_rpc: match Client::new(BITCOIND_RPC, Auth::UserPass(String::from(BITCOIND_USER), String::from(BITCOIND_PASS))) {
                Ok(c) => Some(c),
                Err(_) => None
            }
        };

        blast
    }

    /// Create a new network from scratch
    pub async fn create_network(&mut self, name: &str, model_map: HashMap<String, i32>, running: Arc<AtomicBool>) -> Result<Vec<Child>, String> {
        log::info!("Creating BLAST Network");

        // Start bitcoind
        let mut command = Command::new("bash");
        let mut script_file = env!("CARGO_MANIFEST_DIR").to_owned();
        script_file.push_str("/start_bitcoind.sh");
        command.arg(&script_file);
        match command.output() {
            Ok(_) => {},
            Err(e) => return Err(format!("Could not start bitcoind: {}", e)),
        };

        // Create the blast network
        self.network = Some(BlastNetwork{name: String::from(name), model_map: model_map});
        let net = match &self.network {
            Some(n) => n,
            None => return Err(format!("No network found")),
        };

        // Start the models and nodes
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

    /// Shutdown the simulation network -- this will shutdown the models and nodes
    pub async fn stop_network(&mut self) -> Result<(), String> {
        log::info!("Stopping BLAST Network");

        // Get the blast network
        let net = match &self.network {
            Some(n) => n,
            None => return Err(format!("No network found")),
        };

        // Stop the models (each model should shutdown all its nodes)
        for (key, _) in net.model_map.clone().into_iter() {
            self.stop_model(key).await?
        }

        // Stop bitcoind
        let mut command = Command::new("bash");
        let mut script_file = env!("CARGO_MANIFEST_DIR").to_owned();
        script_file.push_str("/stop_bitcoind.sh");
        command.arg(&script_file);
        match command.output() {
            Ok(_) => {},
            Err(e) => return Err(format!("Could not stop bitcoind: {}", e)),
        };

        Ok(())
    }

    /// Gets the simulation ready to run by creating a sim-ln simulation
    pub async fn finalize_simulation(&mut self) -> Result<(), String> {
        log::info!("Finalizing BLAST Simulation");

        match self.blast_simln_manager.setup_simln().await {
            Ok(_) => {},
            Err(e) => return Err(format!("Failed to setup simln: {:?}", e)),
        };

        Ok(())
    }

    /// Start the simulation -- this will start the simulation events and the simln transaction generation
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

        // Start the simln thread
        sim_tasks.spawn(async move {
            simln_man.start().await
        });

        // Start the event thread
        sim_tasks.spawn(async move {
            event_man.start(sender).await
        });

        // Start the model manager thread
        sim_tasks.spawn(async move {
            model_man.process_events(receiver).await
        });

        Ok(sim_tasks)
    }

    /// Stop the simulation -- this will stop the simulation events and the simln transaction generation
    pub fn stop_simulation(&mut self) {
        log::info!("Stopping BLAST Simulation");

        // Stop the event thread
        self.blast_event_manager.stop();

        // Stop the simln thread
        self.blast_simln_manager.stop();
    }

    /// Load a simulation -- this will load a saved simulation network (nodes, channels, balance) and load events/activity
    pub async fn load(&mut self, sim_name: &str, running: Arc<AtomicBool>) -> Result<Vec<Child>, String> {
        log::info!("Loading BLAST Simulation");

        // Get events
        let mut events_path: String = BLAST_SIM_DIR.to_owned();
        events_path.push_str("/");
        events_path.push_str(sim_name);
        events_path.push_str("/");
        events_path.push_str("events.json");
        self.blast_event_manager.set_event_json(&events_path)?;
  
        // Get simln
        let mut simln_path: String = BLAST_SIM_DIR.to_owned();
        simln_path.push_str("/");
        simln_path.push_str(sim_name);
        simln_path.push_str("/");
        simln_path.push_str("simln.json");
        self.blast_simln_manager.set_simln_json(&simln_path)?;

        // Load bitcoind
        let mut path: String = BLAST_SIM_DIR.to_owned();
        path.push_str("/");
        path.push_str(sim_name);
        path.push_str("/");
        path.push_str("bitcoin.tar.gz");
        let tar_gz = match File::open(path) {
            Ok(t) => t,
            Err(e) => return Err(format!("Error reading bitcoin data: {}", e)),
        };

        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);
        match archive.unpack("/root/.bitcoin") {
            Ok(_) => {},
            Err(e) => return Err(format!("Error reading simln data: {}", e)),
        }

        let mut command = Command::new("bash");
        let mut script_file = env!("CARGO_MANIFEST_DIR").to_owned();
        script_file.push_str("/load_bitcoind.sh");
        command.arg(&script_file);
        match command.output() {
            Ok(_) => {},
            Err(e) => return Err(format!("Could not load bitcoind: {}", e)),
        };
        
        // Get network from models.json file
        let mut models_path: String = BLAST_SIM_DIR.to_owned();
        models_path.push_str("/");
        models_path.push_str(sim_name);
        models_path.push_str("/");
        models_path.push_str("models.json");
        let file = match File::open(models_path) {
            Ok(f) => f,
            Err(e) => return Err(format!("Error opening models file: {}", e)),
        };
        let reader = BufReader::new(file);
        let net: BlastNetwork = match serde_json::from_reader(reader) {
            Ok(n) => n,
            Err(e) => return Err(format!("Error reading simln data: {}", e)),
        };
        self.network = Some(net.clone());

        // Start the models and load the network
        let mut child_list: Vec<Child> = Vec::new();
        for (key, _) in net.model_map.clone().into_iter() {
            let child = self.start_model(key.clone(), running.clone()).await?;
            child_list.push(child);
            self.blast_model_manager.load_model(key, sim_name.to_owned()).await?;
        }

        Ok(child_list)
    }

    /// Save a simulation -- this will save off the current simulation network (nodes, channels, balances) and save events/activity
    pub async fn save(&mut self, sim_name: &str) -> Result<(), String> {
        log::info!("Saving BLAST Simulation");

        // Create folder for sim_name in the simulation directory
        let mut path: String = BLAST_SIM_DIR.to_owned();
        path.push_str("/");
        path.push_str(sim_name);
        path.push_str("/");
        match fs::create_dir_all(&path.clone()) {
            Ok(_) => {},
            Err(e) => return Err(format!("Error creating simulation directory: {}", e))
        };

        self.save_models(sim_name, &path.clone()).await?;
        self.save_bitcoin(&path.clone())?;
        self.save_simln(&path.clone())?;
        self.save_events(&path.clone())?;

        Ok(())
    }

    /// Save the modes that are running for this simulation
    async fn save_models(&mut self, sim_name: &str, path: &str) -> Result<(), String> {
        // Get the blast network
        let net = match &self.network {
            Some(n) => n,
            None => return Err(format!("No network found")),
        };

        // Tell all of the models to save their state
        for (key, _) in net.model_map.clone().into_iter() {
            self.blast_model_manager.save_model(key, sim_name.to_owned()).await?
        }

        // Create models.json file that contains the active model names and save it to BLAST_SIM_DIR/sim_name
        let json = match serde_json::to_string(&self.network.clone().unwrap()) {
            Ok(s) => s,
            Err(e) => return Err(format!("Error getting models data: {}", e))
        };

        let mut jsonfile = path.to_owned();
        jsonfile.push_str("models.json");
        let mut file = match File::create(&jsonfile) {
            Ok(f) => f,
            Err(e) => return Err(format!("Error creating models file: {}", e)),
        };

        match file.write_all(json.as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => return Err(format!("Error writing to models file: {}", e))
        }
    }

    /// Save the bitcoind base layer data
    fn save_bitcoin(&self, path: &str) -> Result<(), String> {
        // Zip up the ~/.bitcoin directory and copy it to BLAST_SIM_DIR/sim_name
        let mut tarfile = path.to_owned();
        tarfile.push_str("bitcoin.tar.gz");
        let tar_gz = match File::create(&tarfile) {
            Ok(t) => t,
            Err(e) => return Err(format!("Error saving bitcoin data dir: {}", e)),
        };

        let enc = GzEncoder::new(tar_gz, Compression::default());
        let mut tar = tar::Builder::new(enc);

        match tar.append_dir_all(".", "/root/.bitcoin") {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error saving bitcoin data dir: {}", e))
        }
    }

    /// Create simln.json that contains the full simln data and copy it to BLAST_SIM_DIR/sim_name
    fn save_simln(&self, path: &str) -> Result<(), String> {
        let json = self.blast_simln_manager.get_simln_json()?;

        let mut jsonfile = path.to_owned();
        jsonfile.push_str("simln.json");
        let mut file = match File::create(&jsonfile) {
            Ok(f) => f,
            Err(e) => return Err(format!("Error creating simln file: {}", e)),
        };

        match file.write_all(json.as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error writing to simln file: {}", e))
        }
    }

    // Create events.json that contains the list of events and copyt it to BLAST_SIM_DIR/sim_name
    fn save_events(&self, path: &str) -> Result<(), String> {
        let json = self.blast_event_manager.get_event_json()?;

        let mut jsonfile = path.to_owned();
        jsonfile.push_str("events.json");
        let mut file = match File::create(&jsonfile) {
            Ok(f) => f,
            Err(e) => return Err(format!("Error creating events file: {}", e)),
        };

        match file.write_all(json.as_bytes()) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error writing to events file: {}", e))
        }
    }

    /// Create payment activity for the simulation
    pub fn add_activity(&mut self, source: &str, destination: &str, start_secs: Option<u16>, count: Option<u64>, interval_secs: u16, amount_msat: u64) {
        self.blast_simln_manager.add_activity(source, destination, start_secs, count, interval_secs, amount_msat);
    }

    /// Create an event for the simulation
    pub fn add_event(&mut self, frame_num: u64, event: &str, args: Option<Vec<String>>) -> Result<(), String> {
        self.blast_event_manager.add_event(frame_num, event, args)
    }

    /// Get all the nodes
    pub fn get_nodes(&self) -> Vec<String> {
        self.blast_simln_manager.get_nodes()
    }

    /// Get the public key of a node
    pub async fn get_pub_key(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_manager.get_pub_key(node_id).await {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting pub key: {}", e))
        }
    }

    /// Get the peers of a node
    pub async fn list_peers(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_manager.list_peers(node_id).await {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting peers: {}", e))
        }
    }

    /// Show this nodes on-chain balance
    pub async fn wallet_balance(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_manager.wallet_balance(node_id).await {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting wallet balance: {}", e))
        }
    }

    /// Show this nodes off-chain balance
    pub async fn channel_balance(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_manager.channel_balance(node_id).await {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting channel balance: {}", e))
        }
    }

    /// View open channels on this node
    pub async fn list_channels(&mut self, node_id: String) -> Result<String, String> {
        match self.blast_model_manager.list_channels(node_id).await {
            Ok(s) => Ok(s),
            Err(e) => Err(format!("Error getting channels: {}", e))
        }
    }

    /// Open a channel and optionally mine blocks to confirm the channel
    pub async fn open_channel(&mut self, node1_id: String, node2_id: String, amount: i64, push_amount: i64, chan_id: i64, confirm: bool) -> Result<(), String> {
        match self.blast_model_manager.open_channel(node1_id, node2_id, amount, push_amount, chan_id).await {
            Ok(_) => {
                if confirm {
                    // TODO: how should we handle time (look at all sleep calls across the whole program)
                    thread::sleep(Duration::from_secs(5));
                    mine_blocks(&mut self.bitcoin_rpc, 10)?;
                    thread::sleep(Duration::from_secs(5));
                }
                Ok(())
            },
            Err(e) => Err(format!("Error opening a channel: {}", e))
        }
    }

    /// Close a channel
    pub async fn close_channel(&mut self, source_id: String, chan_id: i64) -> Result<(), String> {
        match self.blast_model_manager.close_channel(source_id, chan_id).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error closing a channel: {}", e))
        }
    }

    /// Add a peer
    pub async fn connect_peer(&mut self, node1_id: String, node2_id: String) -> Result<(), String> {
        match self.blast_model_manager.connect_peer(node1_id, node2_id).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error connecting to peer: {}", e))
        }
    }

    /// Remove a peer
    pub async fn disconnect_peer(&mut self, node1_id: String, node2_id: String) -> Result<(), String> {
        match self.blast_model_manager.disconnect_peer(node1_id, node2_id).await {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Error disconnecting from peer: {}", e))
        }
    }

    /// Send funds to a node on-chain and optionally mines blocks to confirm that payment
    pub async fn fund_node(&mut self, node_id: String, confirm: bool) -> Result<String, String> {
        match self.blast_model_manager.get_btc_address(node_id).await {
            Ok(a) => {
                let address = bitcoincore_rpc::bitcoin::Address::from_str(&a).map_err(|e|e.to_string())?
                .require_network(bitcoincore_rpc::bitcoin::Network::Regtest).map_err(|e|e.to_string())?;
                let txid = self.bitcoin_rpc.as_mut().unwrap().send_to_address(&address, bitcoincore_rpc::bitcoin::Amount::ONE_BTC, None, None, None, None, None, None)
                .map_err(|e| e.to_string())?;

                if confirm {
                    thread::sleep(Duration::from_secs(5));
                    mine_blocks(&mut self.bitcoin_rpc, 10)?;
                    thread::sleep(Duration::from_secs(5));
                }
                Ok(format!("{}", txid))
            },
            Err(e) => Err(format!("Error getting address: {}", e))
        }
    }

    /// Get the available saved simulations that can be loaded
    pub fn get_available_sims(&self) -> io::Result<Vec<String>> {
        let mut subdirs = Vec::new();
        for entry in fs::read_dir(BLAST_SIM_DIR)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    subdirs.push(name.to_string());
                }
            }
        }
        Ok(subdirs)
    }

    /// Get the available models so that the user can choose which ones to use
    pub fn get_available_models(&self) -> Result<Vec<String>, String> {
        self.blast_model_manager.get_models()
    }

    /// Start a model by name and wait for the RPC connection to be made
    async fn start_model(&mut self, model: String, running: Arc<AtomicBool>) -> Result<Child, String> {
        self.blast_model_manager.start_model(model, running).await
    }

    /// Stop a model by name
    async fn stop_model(&mut self, model: String) -> Result<(), String>{
        self.blast_model_manager.stop_model(model).await
    }
    
    /// Start a given number of nodes for the given model name
    async fn start_nodes(&mut self, model: String, num_nodes: i32) -> Result<(), String> {
        match self.blast_model_manager.start_nodes(model, num_nodes).await {
            Ok(s) => self.blast_simln_manager.add_nodes(s),
            Err(e) => Err(format!("Error starting nodes: {}", e))
        }
    }
}

/// Mine new blocks
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
