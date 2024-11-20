// Standard libraries
use std::sync::Arc;
use std::fs::File;
use std::path::PathBuf;
use std::env;
use std::time::Duration;
use std::thread;
use std::process::Command;
use std::net::TcpListener;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::io::BufReader;

// Extra dependencies
use tonic::{transport::Server, Request, Response, Status};
use tonic::Code;
use tokio::sync::Mutex;
use tonic::transport::channel::ClientTlsConfig;
use tokio::sync::oneshot;
use tonic::transport::Certificate;
use node_client::NodeClient;
use tonic::transport::{Channel, Uri};
use simplelog::WriteLogger;
use simplelog::Config as LogConfig;
use log::LevelFilter;
use serde::Serialize;
use serde::Deserialize;
use amount_or_all::Value;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;
use flate2::read::GzDecoder;
use tar::Archive;
use cln::*;
pub mod cln {
    tonic::include_proto!("cln");
}

// Blast libraries
use blast_rpc_server::BlastRpcServer;
use blast_rpc_server::BlastRpc;
use blast_proto::*;
pub mod blast_proto {
    tonic::include_proto!("blast_proto");
}

// The name of this model (should match the name in model.json)
pub const MODEL_NAME: &str = "blast_cln";

// The directory to save simulations
pub const SIM_DIR: &str = ".blast/blast_sims";

// The temporary directory to save runtime cln data
pub const DATA_DIR: &str = ".blast/blast_data/blast_cln";

// The data that is stored in the sim-ln sim.json file
#[derive(Serialize, Deserialize, Debug)]
struct SimLnNode {
	id: String,
	address: String,
	ca_cert: String,
	client_cert: String,
	client_key: String
}

// The sim.json file for a sim-ln simulation
#[derive(Serialize, Deserialize, Debug)]
struct SimJsonFile {
	nodes: Vec<SimLnNode>
}

// The data that the CLN model will store about an open channel
#[derive(Serialize, Deserialize, Debug)]
struct ClnChannel {
	source: String,
	dest_pk: String,
	chan_id: String
}

// The main data structure for the CLN model
struct BlastCln {
	nodes: HashMap<String, NodeClient<Channel>>,
	simln_data: String,
	addresses: HashMap<String, String>,
	open_channels: HashMap<i64, ClnChannel>,
    shutdown_sender: Option<oneshot::Sender<()>>
}

// Constructor for the CLN model
impl BlastCln {
    fn new() -> Self {
        Self {
			nodes: HashMap::new(),
			simln_data: String::from(""),
			addresses: HashMap::new(),
			open_channels: HashMap::new(),
            shutdown_sender: None
        }
    }
}

// The RPC server that implements the BLAST model interface
struct BlastClnServer {
    blast_cln: Arc<Mutex<BlastCln>>,
}

// Helper functions for the RPC server
impl BlastClnServer {
	// Get an ldk-node "Node" object from an id
	async fn get_node(&self, id: String) -> Result<NodeClient<Channel>, Status> {
		let bcln = self.blast_cln.lock().await;
		let node = match bcln.nodes.get(&id) {
			Some(n) => n,
			None => {
				return Err(Status::new(Code::NotFound, "Node not found."))
			}
		};

		Ok(node.clone())
	}

	// Get an available port that can be used for listening
	fn get_available_port(&self, start: u16, end: u16) -> Option<u16> {
		(start..end).find(|port| self.port_is_available(*port))
	}

	// Check if a port is available
	fn port_is_available(&self, port: u16) -> bool {
		match TcpListener::bind(("127.0.0.1", port)) {
			Ok(_) => true,
			Err(_) => false,
		}
	}
}

// The RPC server that the blast framework will connect to
#[tonic::async_trait]
impl BlastRpc for BlastClnServer {
	/// Start a certain number of nodes
	async fn start_nodes(&self, request: Request<BlastStartRequest>) -> Result<Response<BlastStartResponse>,Status> {
		let num_nodes = request.get_ref().num_nodes;
		let mut node_list = SimJsonFile{nodes: Vec::new()};
		let home = env::var("HOME").expect("HOME environment variable not set");
		let data_dir = PathBuf::from(home).join(DATA_DIR).display().to_string();

		// Start the requested number of cln nodes
		for i in 0..num_nodes {
			// Create a node id and alias
			let node_id = format!("{}{:04}", "blast_cln-", i);
			let port = self.get_available_port(8000, 9000).unwrap();
			let rpcport = self.get_available_port(port+1, 9000).unwrap().to_string();
			let cln_dir = format!("{}/{}", data_dir, node_id);
			let addr = format!("{}:{}", "https://localhost", rpcport.to_string());
			let ca_path = format!("{}{}", cln_dir, "/regtest/ca.pem");
			let client_path = format!("{}{}", cln_dir, "/regtest/client.pem");
			let client_key_path = format!("{}{}", cln_dir, "/regtest/client-key.pem");

			// Start the nodes
			let mut command = Command::new("bash");
			let mut script_file = env!("CARGO_MANIFEST_DIR").to_owned();
			script_file.push_str("/start_cln.sh");
			command.arg(&script_file);
			command.arg(&port.to_string());
			command.arg(&rpcport);
			command.arg(&cln_dir);
			command.arg(&node_id);
			match command.spawn() {
				Ok(_) => {},
				Err(_e) => return Err(Status::new(Code::InvalidArgument, "Could not start cln.")),
			};

			// Let the node get started up
			thread::sleep(Duration::from_secs(2));

			// Load the certificates
			let ca_cert = fs::read(ca_path.clone()).unwrap();
			let ca_certificate = Certificate::from_pem(ca_cert);
			let client_cert = fs::read(client_path.clone()).unwrap();
			let client_key_cert = fs::read(client_key_path.clone()).unwrap();
			let id=tonic::transport::Identity::from_pem(client_cert, client_key_cert);
	
			// Configure TLS settings with the CA certificate
			let tls_config = ClientTlsConfig::new()
				.domain_name("localhost")
				.identity(id)
				.ca_certificate(ca_certificate);

			// Create the URI from the generated address
			let uri: Uri = addr.parse().expect("Invalid URI format");
	
			// Connect to the gRPC server using SSL/TLS
			let channel = Channel::builder(uri)
				.tls_config(tls_config).unwrap()
				.connect()
				.await.unwrap();
	
			// Create a new client from the connected channel
			let client = NodeClient::new(channel);

			// Add the node to the model's list of nodes and to the SimLn data list
			let mut bcln = self.blast_cln.lock().await;
			bcln.nodes.insert(node_id.clone(), client);
			let n = SimLnNode{id: node_id.clone(), address: addr.clone(), ca_cert: ca_path, client_cert: client_path, client_key: client_key_path};
			node_list.nodes.push(n);

			bcln.addresses.insert(node_id.clone(), format!("localhost:{}", &port.to_string()));
		}

		// Serialize the SimLn data into a json string
		let mut bcln = self.blast_cln.lock().await;
		bcln.simln_data = match serde_json::to_string(&node_list) {
			Ok(s) => s,
			Err(_) => {
				let start_response = BlastStartResponse { success: false };
				let response = Response::new(start_response);
				return Ok(response);
			}
		};

		// Return the response to start_nodes
		let start_response = BlastStartResponse { success: true };
		let response = Response::new(start_response);
		Ok(response)
	}

	/// Get the sim-ln data for this model
	async fn get_sim_ln(&self, _request: Request<BlastSimlnRequest>) -> Result<Response<BlastSimlnResponse>, Status> {
		let bcln = self.blast_cln.lock().await;
		let simln_response = BlastSimlnResponse { simln_data: bcln.simln_data.clone().into() };
		let response = Response::new(simln_response);
		Ok(response)
	}

	/// Blast requests the pub key of a node that is controlled by this model
	async fn get_pub_key(&self, request: Request<BlastPubKeyRequest>,) -> Result<Response<BlastPubKeyResponse>, Status> {
		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;

		let cln_resp = match node.getinfo(GetinfoRequest{}).await {
			Ok(r) => {
				r.into_inner()
			},
			Err(s) => {
				return Err(s);
			}
		};

		let key_response = BlastPubKeyResponse { pub_key: hex::encode(cln_resp.id) };
		let response = Response::new(key_response);
		Ok(response)
	}

	/// Blast requests the list of peers for a node that is controlled by this model
	async fn list_peers(&self, request: Request<BlastPeersRequest>,) -> Result<Response<BlastPeersResponse>, Status> {
		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;

		let cln_resp = match node.list_peers(ListpeersRequest{id: None, level: None}).await {
			Ok(r) => {
				r.into_inner()
			},
			Err(s) => {
				return Err(s);
			}
		};

		let peers = format!("{:?}", cln_resp.peers);

		let peers_response = BlastPeersResponse { peers: peers };
		let response = Response::new(peers_response);
		Ok(response)
	}

	/// Blast requests the wallet balance of a node that is controlled by this model
	async fn wallet_balance(&self, request: Request<BlastWalletBalanceRequest>) -> Result<Response<BlastWalletBalanceResponse>, Status> {
		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;

		let cln_resp = match node.list_funds(ListfundsRequest{spent: None}).await {
			Ok(r) => {
				r.into_inner()
			},
			Err(s) => {
				return Err(s);
			}
		};

		let balance = format!("{:?}", cln_resp.outputs);
		
		let balance_response = BlastWalletBalanceResponse { balance: balance };
		let response = Response::new(balance_response);
		Ok(response)
	}

	/// Blast requests the channel balance of a node that is controlled by this model
	async fn channel_balance(&self, request: Request<BlastChannelBalanceRequest>) -> Result<Response<BlastChannelBalanceResponse>, Status> {
		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;

		let cln_resp = match node.list_funds(ListfundsRequest{spent: None}).await {
			Ok(r) => {
				r.into_inner()
			},
			Err(s) => {
				return Err(s);
			}
		};

		let balance = format!("{:?}", cln_resp.channels);

		let balance_response = BlastChannelBalanceResponse { balance: balance };
		let response = Response::new(balance_response);
		Ok(response)
	}

	/// Blast requests the list of channels for a node that is controlled by this model
	async fn list_channels(&self, request: Request<BlastListChannelsRequest>) -> Result<Response<BlastListChannelsResponse>, Status> {
		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;

		let cln_resp = match node.list_channels(ListchannelsRequest{short_channel_id: None, source: None, destination: None}).await {
			Ok(r) => {
				r.into_inner()
			},
			Err(s) => {
				return Err(s);
			}
		};

		let channels = format!("{:?}", cln_resp.channels);

		let chan_response = BlastListChannelsResponse { channels: channels };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model opens a channel
	async fn open_channel(&self, request: Request<BlastOpenChannelRequest>) -> Result<Response<BlastOpenChannelResponse>, Status> {
		let req = &request.get_ref();
		let node_id = &req.node;
		let peer = &req.peer_pub_key;
		let peer_pub = hex::decode(peer.to_string()).unwrap();
		let id = req.channel_id;
		let amount = req.amount;
		let push = Amount { msat: req.push_amout as u64 };

		let mut node = self.get_node(node_id.to_string()).await?;

		let a = Amount { msat: amount as u64 };
		let v = Value::Amount(a);
		let aora = AmountOrAll { value: Some(v) };

		let cln_resp = match node.fund_channel(FundchannelRequest{
			amount: Some(aora),
			announce: None,
			feerate: None,
			push_msat: Some(push),
			close_to: None,
			request_amt: None,
			compact_lease: None,
			id: peer_pub,
			minconf: None,
			utxos: Vec::new(),
			mindepth: None,
			reserve: None,
			channel_type: Vec::new()
		}).await {
			Ok(r) => {
				r.into_inner()
			},
			Err(s) => {
				return Err(s);
			}
		};

		let chanid = hex::encode(cln_resp.channel_id);
		let mut bcln = self.blast_cln.lock().await;
		bcln.open_channels.insert(id, ClnChannel { source: node_id.to_string(), dest_pk: peer.to_string(), chan_id: chanid });

		// Respond to the open channel request
		let chan_response = BlastOpenChannelResponse { success: true };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model closes a channel
	async fn close_channel(&self, request: Request<BlastCloseChannelRequest>) -> Result<Response<BlastCloseChannelResponse>, Status> {
		let req = &request.get_ref();
		let node_id = &req.node;
		let id = &req.channel_id;
		let mut node = self.get_node(node_id.to_string()).await?;
		let mut bcln = self.blast_cln.lock().await;
		let chanid = match bcln.open_channels.get(id) {
			Some(i) => &i.chan_id,
			None => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not get channel from id: {:?}", id)));
			}
		};

		match node.close(CloseRequest{
			id: chanid.to_string(),
			unilateraltimeout: None,
			destination: None,
			fee_negotiation_step: None,
			wrong_funding: None,
			force_lease_closed: None,
			feerange: Vec::new(),
		}).await {
			Ok(_) => {
				bcln.open_channels.remove(id);
				// Respond to the close channel request
				let chan_response = BlastCloseChannelResponse { success: true };
				let response = Response::new(chan_response);
				Ok(response)
			},
			Err(s) => {
				return Err(s);
			}
		}
	}

	/// Create a comma separated list of open channels that this model has control over
	async fn get_model_channels(&self, _request: Request<BlastGetModelChannelsRequest>) -> Result<Response<BlastGetModelChannelsResponse>, Status> {
		let mut result = String::new();
		let bcln = self.blast_cln.lock().await;
		for (key, value) in &bcln.open_channels {
			result.push_str(&format!("{}: {} -> {},", key, &value.source, value.dest_pk));
		}

		result.pop();

		let chan_response = BlastGetModelChannelsResponse { channels: result };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model connects to a peer
	async fn connect_peer(&self, request: Request<BlastConnectRequest>) -> Result<Response<BlastConnectResponse>, Status> {
		let req = &request.get_ref();

		let peer_pub = &req.peer_pub_key;
		let fulladdr = req.peer_addr.clone();
		let parts: Vec<&str> = fulladdr.split(':').collect();
		let addr = parts[0];
		let port = match parts[1].parse::<u32>() {
			Ok(number) => number,
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer port: {:?}", parts[1])));
			}
		};

		// Attempt to connect to the peer from this node
		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;

		match node.connect_peer(ConnectRequest{id: String::from(peer_pub), host: Some(String::from(addr)), port: Some(port)}).await {
			Ok(_) => {
				let connect_response = BlastConnectResponse { success: true };
				let response = Response::new(connect_response);
				Ok(response)
			},
			Err(s) => {
				Err(s)
			}
		}
	}

	/// Blast requests that a node controlled by this model disconnects from a peer
	async fn disconnect_peer(&self, request: Request<BlastDisconnectRequest>) -> Result<Response<BlastDisconnectResponse>, Status> {
		let req = &request.get_ref();
		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;

		match node.disconnect(DisconnectRequest{id: hex::decode(&req.peer_pub_key).unwrap(), force: None}).await {
			Ok(_) => {
				let connect_response = BlastDisconnectResponse { success: true };
				let response = Response::new(connect_response);
				Ok(response)
			},
			Err(s) => {
				Err(s)
			}
		}
	}

	/// Get a BTC address for a node
	async fn get_btc_address(&self, request: Request<BlastBtcAddressRequest>) -> Result<Response<BlastBtcAddressResponse>, Status> {
		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;

		let cln_resp = match node.new_addr(NewaddrRequest{addresstype: Some(3)}).await {
			Ok(r) => {
				r.into_inner()
			},
			Err(s) => {
				return Err(s);
			}
		};

		let addr_response = BlastBtcAddressResponse { address: cln_resp.p2tr.unwrap() };
		let response = Response::new(addr_response);
		Ok(response)
	}

	/// Get the listen address for a node
	async fn get_listen_address(&self, request: Request<BlastListenAddressRequest>) -> Result<Response<BlastListenAddressResponse>, Status> {
		let node_id = &request.get_ref().node;
		let bcln = self.blast_cln.lock().await;

		let addr = match bcln.addresses.get(node_id) {
			Some(a) => a,
			None => {
				return Err(Status::new(Code::InvalidArgument, format!("No addresses")));
			}
		};
		
		let listen_response = BlastListenAddressResponse { address: addr.clone() };
		let response = Response::new(listen_response);
		Ok(response)
	}

	/// Shutdown the nodes
	async fn stop_model(&self, _request: Request<BlastStopModelRequest>) -> Result<Response<BlastStopModelResponse>, Status> {
		let home = env::var("HOME").expect("HOME environment variable not set");
		let data_dir = PathBuf::from(home).join(DATA_DIR).display().to_string();

        let mut bcln = self.blast_cln.lock().await;
		for (id, node) in &bcln.nodes {
			match node.clone().stop(StopRequest{}).await {
				Ok(_) => {},
				Err(_) => {
					let mut command = Command::new("bash");
					let mut script_file = env!("CARGO_MANIFEST_DIR").to_owned();
					script_file.push_str("/stop_cln.sh");
					command.arg(&script_file);
					command.arg(format!("{}/{}", data_dir, id));
					match command.output() {
						Ok(_) => {},
						Err(_e) => return Err(Status::new(Code::InvalidArgument, "Could not stop cln.")),
					};
				}
			}
		}

        let _ = bcln.shutdown_sender.take().unwrap().send(());
		let _ = fs::remove_dir_all(data_dir);

		let stop_response = BlastStopModelResponse { success: true };
		let response = Response::new(stop_response);
		Ok(response)
	}

	/// Load a previous state of this model
	async fn load(&self, request: Request<BlastLoadRequest>) -> Result<Response<BlastLoadResponse>, Status> {
		let req = &request.get_ref();
		let sim_name = &req.sim;
		let home_dir = env::var("HOME").expect("HOME environment variable not set");
		let sim_dir = String::from(SIM_DIR);
		let sim_model_dir = format!("{}/{}/{}/{}/", home_dir, sim_dir, sim_name, MODEL_NAME);

		// Set paths for the archive and JSON file
		let archive_path = Path::new(&sim_model_dir).join(format!("{}.tar.gz", sim_name));
		let json_path = Path::new(&sim_model_dir).join(format!("{}_channels.json", sim_name));

		// Open the .tar.gz file
		let tar_gz = File::open(archive_path)?;
		let decompressor = GzDecoder::new(tar_gz);
		let mut archive = Archive::new(decompressor);
		// Extract the archive into the specified directory
		let home = env::var("HOME").expect("HOME environment variable not set");
		let data_dir = PathBuf::from(home).join(DATA_DIR).display().to_string();
		let data_path = Path::new(&data_dir);
		fs::create_dir_all(data_path).unwrap();
		archive.unpack(data_path).unwrap();


		// Count the number of nodes to start and remove the old symlink
		let mut count = 0;
        for entry in fs::read_dir(data_path).unwrap() {
            let entry = match entry {
				Ok(e) => e,
				Err(_) => {
					return Err(Status::new(Code::Unknown, "Could not read the data directory"));
				}
			};
            let path = entry.path();

            // Check if the entry is a directory and if its name starts with blast_ldk-
            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    if dir_name.starts_with("blast_cln-") {
                        count += 1;

                        // Construct the path to the file to remove
                        let log = path.join("log");
                        if log.exists() {
                            match fs::remove_file(&log) {
								Ok(_) => {},
								Err(_) => {}
							}
                        }

                        // Construct the path to the file to remove
                        let pid = path.join("lightningd-regtest.pid");
                        if pid.exists() {
                            match fs::remove_file(&pid) {
								Ok(_) => {},
								Err(_) => {}
							}
                        }
                    }
                }
            }
        }
		
		// Attempt to start the nodes
		let request = BlastStartRequest { num_nodes: count };
		let start_req = Request::new(request);
		match self.start_nodes(start_req).await {
			Ok(_) => {},
			Err(_) => {
				return Err(Status::new(Code::Unknown, "Could not start nodes."));
			}
		}

		// Open the JSON file
		let file = File::open(json_path).unwrap();
		let reader = BufReader::new(file);

		// Deserialize JSON to Channel map
		let chans: HashMap<i64, ClnChannel> = serde_json::from_reader(reader).unwrap();
		let mut bcln = self.blast_cln.lock().await;
		bcln.open_channels = chans;

		let load_response = BlastLoadResponse { success: true };
		let response = Response::new(load_response);
		Ok(response)
	}

	/// Save this models current state
	async fn save(&self, request: Request<BlastSaveRequest>) -> Result<Response<BlastSaveResponse>, Status> {
		let req = &request.get_ref();
		let sim_name = &req.sim;
		let home_dir = env::var("HOME").expect("HOME environment variable not set");
		let sim_dir = String::from(SIM_DIR);
		let sim_model_dir = format!("{}/{}/{}/{}/", home_dir, sim_dir, sim_name, MODEL_NAME);

		// Set paths for the archive and JSON file
		let archive_path = Path::new(&sim_model_dir).join(format!("{}.tar.gz", sim_name));
		let json_path = Path::new(&sim_model_dir).join(format!("{}_channels.json", sim_name));

		// Create the .tar.gz archive
		let home = env::var("HOME").expect("HOME environment variable not set");
		let data_dir = PathBuf::from(home).join(DATA_DIR).display().to_string();
		if let Some(parent) = archive_path.parent() {
			fs::create_dir_all(parent).unwrap();
		}
		let tar_gz = File::create(&archive_path).unwrap();
		let enc = GzEncoder::new(tar_gz, Compression::default());
		let mut tar = Builder::new(enc);
		tar.append_dir_all(".", data_dir).unwrap();

		// Serialize the HashMap to JSON and write to a file
		let bcln = self.blast_cln.lock().await;
		let json_string = serde_json::to_string_pretty(&bcln.open_channels).unwrap();
		fs::write(&json_path, json_string)?;

		let save_response = BlastSaveResponse { success: true };
		let response = Response::new(save_response);
		Ok(response)
	}
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Set up the logger for this model
	let home = env::var("HOME").expect("HOME environment variable not set");
	let folder_path = PathBuf::from(home).join(".blast/blast_cln.log");
	std::fs::create_dir_all(folder_path.parent().unwrap()).unwrap();
	let _ = WriteLogger::init(
		LevelFilter::Info,
		LogConfig::default(),
		File::create(folder_path).unwrap(),
	);

    let addr = "127.0.0.1:5052".parse()?;
	let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();
	let mut bcln = BlastCln::new();
	bcln.shutdown_sender = Some(shutdown_sender);
    let blast_cln = Arc::new(Mutex::new(bcln));
	let server = BlastClnServer {
        blast_cln: Arc::clone(&blast_cln)
    };

	// Start the RPC server
    log::info!("Starting gRPC server at {}", addr);
	let server = tokio::spawn(async move {
		Server::builder()
        .add_service(BlastRpcServer::new(server))
        .serve_with_shutdown(addr, async {
			shutdown_receiver.await.ok();
		})
		.await
		.unwrap();
	});

	// Wait for the server task to finish
    let _ = server.await;

	log::info!("Shutting down gRPC server at {}", addr);

    Ok(())
}
