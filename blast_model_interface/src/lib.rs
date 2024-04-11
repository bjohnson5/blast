use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::collections::HashMap;
use std::env;
use std::process::{Command, Child};
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::fs;

use tonic::transport::Channel;

use serde::Deserialize;

use blast_proto::blast_rpc_client::BlastRpcClient;
use blast_proto::BlastRpcRequest;

// Import the generated proto-rust file into a module
pub mod blast_proto {
    tonic::include_proto!("blast_proto");
}

#[derive(Deserialize, Debug)]
struct ModelConfig {
    name: String,
    rpc: String,
    start: String,
}

#[derive(Debug)]
pub struct BlastModel {
    rpc_connection: Option<BlastRpcClient<Channel>>,
    config: ModelConfig
}

pub struct BlastModelInterface {
    models: HashMap<String, BlastModel>
}

impl BlastModelInterface {
    pub fn new() -> Self {
        let blast_model_interface = BlastModelInterface {
            models: parse_models(),
        };

        println!("{:?}", blast_model_interface.models);

        blast_model_interface
    }

    pub async fn create_nodes(&mut self, model: String, num_nodes: u64, running: Arc<AtomicBool>) -> Option<Child> {
        let model = match self.models.get_mut(&model) {
            Some(model) => {
                model
            },
            None => {
                println!("Failed to get the model");
                return None;
            }
        };

        let mut current_dir = match env::current_dir() {
            Ok(d) => d,
            Err(_) => {
                println!("Failed to get the current directory");
                return None;
            }
        };
    
        current_dir.push("../blast_models/".to_owned()+&model.config.name+"/"+&model.config.start);
        let model_dir = current_dir.to_string_lossy().into_owned();

        let child = Command::new(model_dir)
            .arg(&num_nodes.to_string())
            .spawn()
            .expect("Failed to execute process");

        let channel: Channel;
        let address = "http://".to_owned()+&model.config.rpc;
        loop {
            match Channel::from_shared(address.clone()).unwrap().connect().await {
                Ok(c) => {
                    println!("Connected to server successfully!");
                    channel = c;
                    break;
                }
                Err(err) => {
                    if !running.load(Ordering::SeqCst) {
                        return None;
                    }
                    println!("Failed to connect to server: {}", err);
                    // Add some delay before retrying
                    sleep(std::time::Duration::from_secs(1));
                }
            }
        }

        model.rpc_connection = Some(BlastRpcClient::new(channel.to_owned()));

        Some(child)
    }

    pub async fn list_peers(&mut self, node_id: String) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: look up the node_id and find which model it belongs too
        let model = match self.models.get_mut(&String::from("blast_lnd")) {
            Some(model) => {
                model
            },
            None => {
                return Ok(());
            }
        };

        let client = match &mut model.rpc_connection {
            Some(c) => {
                c
            },
            None => {
                return Ok(());
            }
        };
        let request = tonic::Request::new(BlastRpcRequest {
            node: node_id,
        });
        let response = client.list_peers(request).await?;
        println!("RESPONSE={:?}", response);
        Ok(())
    }

    pub async fn get_info(&mut self, node_id: String) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: look up the node_id and find which model it belongs too
        let model = match self.models.get_mut(&String::from("blast_lnd")) {
            Some(model) => {
                model
            },
            None => {
                return Ok(());
            }
        };

        let client = match &mut model.rpc_connection {
            Some(c) => {
                c
            },
            None => {
                return Ok(());
            }
        };
        let request = tonic::Request::new(BlastRpcRequest {
            node: node_id,
        });
        
        let response = client.get_info(request).await?;
        println!("RESPONSE={:?}", response);
        Ok(())
    }
}

fn parse_models() -> HashMap<String, BlastModel> {
    let mut model_map = HashMap::new();
    let mut current_dir = match env::current_dir() {
        Ok(d) => d,
        Err(_) => {
            println!("Failed to get the current directory");
            return model_map;
        }
    };

    current_dir.push("../blast_models/");

    check_for_model(&current_dir.as_path(), 0, &mut model_map);
    return model_map;
}

fn check_for_model(dir_path: &Path, current_depth: usize, model_map: &mut HashMap<String, BlastModel>) {
    if current_depth > 1 {
        return;
    }
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    check_for_model(&entry_path, current_depth + 1, model_map);
                }
                if entry_path.file_name() == Some("model.json".as_ref()) {
                    let config = match read_model_from_file(entry_path) {
                        Ok(mc) => {
                            mc
                        },
                        Err(_) => {
                            println!("Error reading model.json file.");
                            continue;
                        }
                    };
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

fn read_model_from_file<P: AsRef<Path>>(path: P) -> Result<ModelConfig, Box<dyn Error>> {
    // Open the file in read-only mode with buffer.
    let file = File::open(path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let u = serde_json::from_reader(reader)?;

    // Return the `User`.
    Ok(u)
}
