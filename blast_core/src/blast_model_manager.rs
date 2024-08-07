// Standard libraries
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::collections::HashMap;
use std::env;
use std::process::{Command, Child};
use std::error::Error as stdError;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::fs;
use std::thread;
use std::time::Duration;
use std::process::Stdio;
use std::fs::OpenOptions;
use std::path::PathBuf;

// Extra dependencies
use anyhow::Error;
use tonic::transport::Channel;
use serde::Deserialize;
use tokio::sync::mpsc::Receiver;

// Blast libraries
use crate::blast_event_manager::BlastEvent;
use blast_proto::blast_rpc_client::BlastRpcClient;
use crate::blast_proto::*;
pub mod blast_proto {
    tonic::include_proto!("blast_proto");
}

pub const BLAST_MODEL_LOG_DIR: &str = ".blast/";

/// The ModelConfig struct defines a blast model and it contains the information from the model.json file
#[derive(Deserialize, Debug, Clone)]
struct ModelConfig {
    name: String,
    rpc: String,
    start: String,
}

/// The BlastModel struct holds the model config and the RPC connection to the model
#[derive(Debug, Clone)]
struct BlastModel {
    rpc_connection: Option<BlastRpcClient<Channel>>,
    config: ModelConfig
}

/// The BlastModelManager struct is the public interface that allows models to be controlled
#[derive(Clone)]
pub struct BlastModelManager {
    models: HashMap<String, BlastModel>
}

impl BlastModelManager {
    /// Create a new BlastModelManager by searching the models directory and parsing all model.json files that are found
    pub fn new() -> Self {
        let blast_model_manager = BlastModelManager {
            models: parse_models(),
        };

        blast_model_manager
    }

    /// Process events sent from the event thread and control the nodes on the network
    pub async fn process_events(&mut self, mut receiver: Receiver<BlastEvent>) -> Result<(), Error> {
        loop {
            let simulation_event = receiver.recv().await;
            if let Some(event) = simulation_event {
                match event {
                    BlastEvent::StartNodeEvent(_) => {
                        // TODO: implement start node event
                        log::info!("BlastModelManager running event StartNode");
                    },
                    BlastEvent::StopNodeEvent(_) => {
                        // TODO: implement stop node event
                        log::info!("BlastModelManager running event StopNode");
                    },
                    BlastEvent::OpenChannelEvent(source, dest, amount, push, chan_id) => {
                        log::info!("BlastModelManager running event OpenChannel");
                        match self.open_channel(source, dest, amount, push, chan_id).await {
                            Ok(_) => {},
                            Err(e) => {
                                log::error!("BlastModelManager error opening a channel: {}", e);
                            }
                        }
                    },
                    BlastEvent::CloseChannelEvent(source, chan_id) => {
                        log::info!("BlastModelManager running event CloseChannel");
                        match self.close_channel(source, chan_id).await {
                            Ok(_) => {},
                            Err(e) => {
                                log::error!("BlastModelManager error closing a channel: {}", e);
                            }
                        }
                    },
                    BlastEvent::OnChainTransaction(_, _, _) => {
                        // TODO: implement on chain event
                        log::info!("BlastModelManager running event OnChainTransaction");
                    },
                }
            } else {
                return Ok(())
            }
        }
    }

    /// Start a model by name and wait for the RPC connection to be made
    pub async fn start_model(&mut self, model: String, running: Arc<AtomicBool>) -> Result<Child, String> {
        // Get the model from the manager's map
        let model = match self.models.get_mut(&model) {
            Some(model) => model,
            None => {
                return Err(String::from("Failed to get the model"));
            }
        };

        // Get the current working directory
        let mut current_dir = match env::current_dir() {
            Ok(d) => d,
            Err(_) => {
                return Err(String::from("Failed to get the current directory"));
            }
        };
    
        // Get the full path to the model executable
        current_dir.push("../blast_models/".to_owned()+&model.config.name+"/"+&model.config.start);
        let model_exe = current_dir.to_string_lossy().into_owned();

        let home = env::var("HOME").expect("HOME environment variable not set");
        let folder_path = PathBuf::from(home).join(BLAST_MODEL_LOG_DIR);

        // Run the model executable
        let file_stderr = OpenOptions::new()
            .append(true)
            .create(true)
            .open(folder_path.display().to_string()+&model.config.name+".log")
            .unwrap();
        let file_stdout = OpenOptions::new()
            .append(true)
            .create(true)
            .open(folder_path.display().to_string()+&model.config.name+".log")
            .unwrap();
        let child = match Command::new(model_exe).stderr(Stdio::from(file_stderr)).stdout(Stdio::from(file_stdout))
        .spawn() {
            Ok(c) => c,
            Err(_) => {
                return Err(String::from("Failed to execute process"));
            }
        };

        // Connect to the model RPC
        let channel: Channel;
        let address = "http://".to_owned()+&model.config.rpc;
        loop {
            // Continue trying to connect until a successful connection is made or the simulation stops
            match Channel::from_shared(address.clone()).unwrap().connect().await {
                Ok(c) => {
                    channel = c;
                    break;
                }
                Err(_) => {
                    if !running.load(Ordering::SeqCst) {
                        return Err(String::from("Could not connect to model"));
                    }
                    thread::sleep(Duration::from_secs(1));
                }
            }
        }

        // Set the RPC client
        model.rpc_connection = Some(BlastRpcClient::new(channel.to_owned()));

        // Return the model process
        Ok(child)
    }

    /// Stop a model -- The model process should stop all of its nodes and exit
    pub async fn stop_model(&mut self, model: String) -> Result<(), String> {
        // Get the RPC client for this model
        let client = self.get_model_client(model)?;

        // Create a stop request
        let request = tonic::Request::new(BlastStopModelRequest {
        });
    
        // Execute the stop RPC
        let response = match client.stop_model(request).await {
            Ok(r) => r,
            Err(_) => {
                return Err(String::from("RPC stop_model failed"));
            }
        };

        // Determine if the RPC was successful
        if response.get_ref().success {
            Ok(())
        } else {
            Err(String::from("Model did not shutdown successfully"))
        }
    }

    /// Tell a model to load a simulation
    pub async fn load_model(&mut self, model: String, sim_name: String) -> Result<(), String> {
        // Get the RPC client for this model
        let client = self.get_model_client(model)?;

        // Create a load request
        let request = tonic::Request::new(BlastLoadRequest {
            sim: sim_name
        });
    
        // Execute the load RPC
        let response = match client.load(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC load failed: {:?}", e));
            }
        };

        // Determine if the RPC was successful
        if response.get_ref().success {
            Ok(())
        } else {
            Err(String::from("Model did not load successfully"))
        }       
    }

    /// Tell a model to save its current state
    pub async fn save_model(&mut self, model: String, sim_name: String) -> Result<(), String> {
        // Get the RPC client for this model
        let client = self.get_model_client(model)?;

        // Create a save request
        let request = tonic::Request::new(BlastSaveRequest {
            sim: sim_name
        });
    
        // Execute the save RPC
        let response = match client.save(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC save failed: {:?}", e));
            }
        };

        // Determine if the RPC was successful
        if response.get_ref().success {
            Ok(())
        } else {
            Err(String::from("Model did not save successfully"))
        }        
    }

    /// Get a list of the currently available models
    pub fn get_models(&self) -> Result<Vec<String>, String> {
        let keys: Vec<String> = self.models.keys().cloned().collect();
        if keys.is_empty() {
            Err(String::from("No models available"))
        } else {
            Ok(keys)
        }       
    }

    /// Start a given number of nodes for the given model name
    pub async fn start_nodes(&mut self, model: String, num_nodes: i32) -> Result<String, String> {
        // Get the RPC client for this model
        let client = self.get_model_client(model)?;

        // Create a start request
        let request = tonic::Request::new(BlastStartRequest {
            num_nodes: num_nodes,
        });
    
        // Execute the start RPC
        let response = match client.start_nodes(request).await {
            Ok(r) => r,
            Err(_) => {
                return Err(String::from("RPC start nodes failed"));
            }
        };

        // Determine if the RPC was successful
        if response.get_ref().success {
            // If the start was successful request the simln data from the model
            let request = tonic::Request::new(BlastSimlnRequest {});
            let response = match client.get_sim_ln(request).await {
                Ok(r) => r,
                Err(_) => {
                    return Err(String::from("RPC get_sim_ln failed"));
                }
            };
            match std::str::from_utf8(&response.get_ref().simln_data) {
                Ok(v) => Ok(String::from(v)),
                Err(_) => Err(String::from("Invalid UTF-8 sequence")),
            }
        } else {
            return Err(String::from("Could not get simln data"));
        }
    }

    /// Get the public key of a node in the simulation
    pub async fn get_pub_key(&mut self, node_id: String) -> Result<String, String> {
        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(node_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create a pub key request
        let request = tonic::Request::new(BlastPubKeyRequest {
            node: node_id,
        });

        // Execute the pub key RPC
        let response = match client.get_pub_key(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC get_pub_key failed: {:?}", e));
            }
        };

        Ok(response.get_ref().pub_key.clone())
    }

    /// Get the peers of a node in the simulation.
    pub async fn list_peers(&mut self, node_id: String) -> Result<String, String> {
        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(node_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create a list peers request
        let request = tonic::Request::new(BlastPeersRequest {
            node: node_id,
        });

        // Execute the list peers RPC
        let response = match client.list_peers(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC list_peers failed: {:?}", e));
            }
        };

        Ok(response.get_ref().peers.clone())
    }

    /// Get the on chain balance of a node
    pub async fn wallet_balance(&mut self, node_id: String) -> Result<String, String> {
        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(node_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create a wallet balance request
        let request = tonic::Request::new(BlastWalletBalanceRequest {
            node: node_id,
        });

        // Execute the wallet balance RPC
        let response = match client.wallet_balance(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC wallet_balance failed: {:?}", e));
            }
        };
        
        Ok(response.get_ref().balance.clone())
    }

    /// Get the off chain balance of a node
    pub async fn channel_balance(&mut self, node_id: String) -> Result<String, String> {
        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(node_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create a channel balance request
        let request = tonic::Request::new(BlastChannelBalanceRequest {
            node: node_id,
        });

        // Execute the channel balance RPC
        let response = match client.channel_balance(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC channel_balance failed: {:?}", e));
            }
        };
        
        Ok(response.get_ref().balance.clone())
    }

    /// List all the open channels for a node
    pub async fn list_channels(&mut self, node_id: String) -> Result<String, String> {
        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(node_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create a list channels request
        let request = tonic::Request::new(BlastListChannelsRequest {
            node: node_id,
        });

        // Execute the list channels RPC
        let response = match client.list_channels(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC list_channels failed: {:?}", e));
            }
        };
        
        Ok(response.get_ref().channels.clone())
    }

    /// Get all channels for all models
    pub fn get_channels(&self) -> Vec<String> {
        // TODO: implement get all channels
        let chans: Vec<String> = Vec::new();
        chans
    }

    /// Open a channel from node with source_id to node with dest_id for the given amount and with the given chan_id
    pub async fn open_channel(&mut self, source_id: String, dest_id: String, amount: i64, push_amount: i64, chan_id: i64) -> Result<(), String> {
        let pub_key = self.get_pub_key(dest_id.clone()).await?;

        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(source_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create an open channel request
        let request = tonic::Request::new(BlastOpenChannelRequest {
            node: source_id,
            peer_pub_key: pub_key,
            amount: amount,
            push_amout: push_amount,
            channel_id: chan_id
        });

        // Execute the open channel RPC
        let response = match client.open_channel(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC open_channel failed: {:?}", e));
            }
        };
        
        // Determine if the RPC was successful
        if response.get_ref().success {
            Ok(())
        } else {
            Err(String::from("Error opening channel"))
        }
    }

    /// Close a channel with chan_id from a node with source_id 
    pub async fn close_channel(&mut self, source_id: String, chan_id: i64) -> Result<(), String> {
        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(source_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create a close channel request
        let request = tonic::Request::new(BlastCloseChannelRequest {
            node: source_id,
            channel_id: chan_id
        });

        // Execute the close channel RPC
        let response = match client.close_channel(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC close_channel failed: {:?}", e));
            }
        };
        
        // Determine if the RPC was successful
        if response.get_ref().success {
            Ok(())
        } else {
            Err(String::from("Error closing channel"))
        }
    }

    /// Connect two nodes
    pub async fn connect_peer(&mut self, node1_id: String, node2_id: String) -> Result<(), String> {
        let pub_key = self.get_pub_key(node2_id.clone()).await?;
        let addr = self.get_listen_address(node2_id.clone()).await?;

        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(node1_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create a connect request
        let request = tonic::Request::new(BlastConnectRequest {
            node: node1_id,
            peer_pub_key: pub_key,
            peer_addr: addr
        });

        // Execute the connect RPC
        let response = match client.connect_peer(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC connect_peer failed: {:?}", e));
            }
        };
        
        // Determine if the RPC was successful
        if response.get_ref().success {
            Ok(())
        } else {
            Err(String::from("Error connecting to peer"))
        }
    }

    /// Disconnect two nodes
    pub async fn disconnect_peer(&mut self, node1_id: String, node2_id: String) -> Result<(), String> {
        let pub_key = self.get_pub_key(node2_id.clone()).await?;

        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(node1_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create a disconnect request
        let request = tonic::Request::new(BlastDisconnectRequest {
            node: node1_id,
            peer_pub_key: pub_key
        });

        // Execute the disconnect RPC
        let response = match client.disconnect_peer(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC disconnect_peer failed: {:?}", e));
            }
        };
        
        // Determine if the RPC was successful
        if response.get_ref().success {
            Ok(())
        } else {
            Err(String::from("Error disconnecting from peer"))
        }
    }

    /// Get an on chain address for a given node
    pub async fn get_btc_address(&mut self, node_id: String) -> Result<String, String> {
        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(node_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create a btc address request
        let request = tonic::Request::new(BlastBtcAddressRequest {
            node: node_id,
        });

        // Execute the btc address RPC
        let response = match client.get_btc_address(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC get_btc_address failed: {:?}", e));
            }
        };
        
        Ok(response.get_ref().address.clone())
    }

    /// Get the listen address for a given node
    async fn get_listen_address(&mut self, node_id: String) -> Result<String, String> {
        // Get the model name from the node_id (example node_id: model_name-0000)
        let model_name: String = get_model_from_node(node_id.clone());

        // Get the RPC client for this model
        let client = self.get_model_client(model_name)?;

        // Create a listen address request
        let request = tonic::Request::new(BlastListenAddressRequest {
            node: node_id,
        });

        // Execute the listen address RPC
        let response = match client.get_listen_address(request).await {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("RPC get_listen_address failed: {:?}", e));
            }
        };
        
        Ok(response.get_ref().address.clone())
    }

    /// Get the RPC connection for a model
    fn get_model_client(&mut self, model: String) -> Result<&mut BlastRpcClient<Channel>, String> {
        // Look up the model in the model map
        let model = match self.models.get_mut(&model) {
            Some(model) => model,
            None => {
                return Err(String::from("Could not get model"));
            }
        };

        // Return the RPC client for this model
        match &mut model.rpc_connection {
            Some(c) => Ok(c),
            None => Err(String::from("Could not get model connection"))
        }
    }
}

/// Parse the model name out of a node_id
fn get_model_from_node(node_id: String) -> String {
    // Split the input string on the '-' character
    let parts: Vec<&str> = node_id.split('-').collect();
    
    // Return the first part, or an empty string if the input was empty
    parts.get(0).unwrap_or(&"").to_string()
}

/// Check for model.json files and create BlastModel objects for all known models
fn parse_models() -> HashMap<String, BlastModel> {
    // Create a new map of all the models that are found and then get the models directory.
    let mut model_map = HashMap::new();
    let mut current_dir = match env::current_dir() {
        Ok(d) => d,
        Err(_) => {
            return model_map;
        }
    };
    current_dir.push("../blast_models/");

    // Search for model.json files in the models directory
    check_for_model(&current_dir.as_path(), 0, &mut model_map);

    model_map
}

/// Helper function for parse_models
fn check_for_model(dir_path: &Path, current_depth: usize, model_map: &mut HashMap<String, BlastModel>) {
    if current_depth > 1 {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir_path) {
        // For all the files/folders in this directory
        for entry in entries {
            if let Ok(entry) = entry {
                let entry_path = entry.path();
                // If the entry is a directory then all this function again
                if entry_path.is_dir() {
                    check_for_model(&entry_path, current_depth + 1, model_map);
                }
                // If the entry is a file named model.json, read it and add the model to the map
                if entry_path.file_name() == Some("model.json".as_ref()) {
                    let config = match read_model_from_file(entry_path) {
                        Ok(mc) => {
                            mc
                        },
                        Err(_) => {
                            continue;
                        }
                    };
                    // Create a BlastModel with an empty RPC connection for now
                    let blast_model = BlastModel {
                        rpc_connection: None,
                        config: config
                    };
                    model_map.insert(blast_model.config.name.clone(), blast_model);
                }
            }
        }
    }
}

/// Helper function for parse_models
fn read_model_from_file<P: AsRef<Path>>(path: P) -> Result<ModelConfig, Box<dyn stdError>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let u = serde_json::from_reader(reader)?;
    Ok(u)
}
