// Standard libraries
use std::str::FromStr;
use std::time::Duration;
use std::thread;
use std::sync::Arc;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::env;
use std::net::TcpListener;
use std::path::Path;
use std::io::BufReader;

// LDK Node libraries
use ldk_node::bip39::serde::{Deserialize, Serialize};
use ldk_node::{Builder, LogLevel};
use ldk_node::bitcoin::Network;
use ldk_node::config::Config;
use ldk_node::lightning::ln::msgs::SocketAddress;
use ldk_node::lightning::routing::gossip::NodeAlias;
use ldk_node::UserChannelId;
use ldk_node::Node;

// Extra dependencies
use secp256k1::PublicKey;
use tonic::{transport::Server, Request, Response, Status};
use tonic::Code;
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use tokio::runtime::Runtime;
use simplelog::WriteLogger;
use simplelog::Config as LogConfig;
use log::LevelFilter;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder as TarBuilder;
use serde::Serializer;
use serde::Deserializer;
use serde::ser::SerializeStruct;
use flate2::read::GzDecoder;
use tar::Archive;

// Blast libraries
use blast_rpc_server::BlastRpcServer;
use blast_rpc_server::BlastRpc;
use blast_proto::*;
pub mod blast_proto {
    tonic::include_proto!("blast_proto");
}

// The name of this model (should match the name in model.json)
pub const MODEL_NAME: &str = "blast_ldk";

// The directory to save simulations
pub const SIM_DIR: &str = "/.blast/blast_sims/";

// The temporary directory to save runtime ldk data
pub const DATA_DIR: &str = "/blast_data/";

// The data that is stored in the sim-ln sim.json file
#[derive(Serialize, Deserialize, Debug)]
struct SimLnNode {
	id: String,
	address: String,
	macaroon: String,
	cert: String
}

// The sim.json file for a sim-ln simulation
#[derive(Serialize, Deserialize, Debug)]
struct SimJsonFile {
	nodes: Vec<SimLnNode>
}

// The data that the LDK model will store about an open channel
struct Channel {
	source: String,
	id: UserChannelId,
	pk: PublicKey
}

impl Serialize for Channel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("Channel", 3)?;
		s.serialize_field("source", &self.source)?;
		s.serialize_field("id", &self.id.0)?;
		s.serialize_field("pk", &self.pk)?;
		s.end()
    }
}

impl<'de> Deserialize<'de> for Channel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
		#[derive(Deserialize)]
        struct ChannelHelper {
			source: String,
			id: u128,
			pk: PublicKey
		}

		let helper = ChannelHelper::deserialize(deserializer)?;

		Ok(Channel {
			source: helper.source,
			id: UserChannelId(helper.id),
			pk: helper.pk
		})
    }
}

// The main data structure for the LDK model
struct BlastLdk {
    nodes: HashMap<String, Arc<Node>>,
	simln_data: String,
	open_channels: HashMap<i64, Channel>,
	shutdown_sender: Option<oneshot::Sender<()>>
}

// Constructor for the LDK model data structure
impl BlastLdk {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
			simln_data: String::from(""),
			open_channels: HashMap::new(),
			shutdown_sender: None
        }
    }
}

// The RPC server that implements the BLAST model interface
struct BlastLdkServer {
    blast_ldk: Arc<Mutex<BlastLdk>>,
	runtime: Arc<Runtime>
}

// Helper functions for the RPC server
impl BlastLdkServer {
	// Get an ldk-node "Node" object from an id
	async fn get_node(&self, id: String) -> Result<Arc<Node>, Status> {
		let bldk = self.blast_ldk.lock().await;
		let node = match bldk.nodes.get(&id) {
			Some(n) => n,
			None => {
				return Err(Status::new(Code::NotFound, "Node not found."))
			}
		};

		Ok(node.clone())
	}

	// Get an available port that can be used for listening
	fn get_available_port(&self) -> Option<u16> {
		(8000..9000).find(|port| self.port_is_available(*port))
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
impl BlastRpc for BlastLdkServer {
	/// Start a certain number of nodes
	async fn start_nodes(&self, request: Request<BlastStartRequest>) -> Result<Response<BlastStartResponse>,Status> {
		let num_nodes = request.get_ref().num_nodes;
		let mut node_list = SimJsonFile{nodes: Vec::new()};
		let mut data_dir = env!("CARGO_MANIFEST_DIR").to_owned();
        data_dir.push_str(DATA_DIR);

		// Start the requested number of ldk nodes
		for i in 0..num_nodes {
			// Create a node id and alias
			let node_id = format!("{}{:04}", "blast_ldk-", i);
			let alias = node_id.as_bytes();
			let mut alias_array = [0u8; 32];
			let len = alias.len().min(alias_array.len());
			alias_array[..len].copy_from_slice(alias);
			let node_alias = NodeAlias(alias_array);

			// Set up the listening address for this node
			let mut listen_addr_list: Vec<SocketAddress> = Vec::new();
			let port = self.get_available_port().unwrap();
			let addr = format!("127.0.0.1:{}", port);
			let address = match SocketAddress::from_str(&addr) {
				Ok(a) => a,
				Err(_) => {
					return Err(Status::new(Code::InvalidArgument, "Could not create listen address."));
				}
			};
			listen_addr_list.push(address);

			// Create the config for this node
			let config = Config {
				storage_dir_path: format!("{}{}", data_dir, node_id),
				log_dir_path: None,
				network: Network::Regtest,
				listening_addresses: Some(listen_addr_list),
				node_alias: Some(node_alias),
				sending_parameters: None,
				trusted_peers_0conf: Vec::new(),
				probing_liquidity_limit_multiplier: 0,
				log_level: LogLevel::Debug,
				anchor_channels_config: None
			};

			// Build the ldk node
			let mut builder = Builder::from_config(config);
			builder.set_chain_source_bitcoind_rpc(String::from("127.0.0.1"), 18443, String::from("user"), String::from("pass"));
			builder.set_gossip_source_p2p();
			let ldknode = match builder.build() {
				Ok(n) => n,
				Err(_) => {
					return Err(Status::new(Code::Unknown, "Could not create the ldk node."));
				}
			};
			let node = Arc::new(ldknode);

			// Start the node
			match node.start_with_runtime(Arc::clone(&self.runtime)) {
				Ok(_) => {},
				Err(_) => {
					return Err(Status::new(Code::Unknown, "Could not start the ldk node."));
				}
			}

			// Let the node get started up
			thread::sleep(Duration::from_secs(2));

			// Add the node to the model's list of nodes and to the SimLn data list
			let mut bldk = self.blast_ldk.lock().await;
			bldk.nodes.insert(node_id.clone(), node.clone());
			// TODO: Once and RPC is added to LDK-node, fill in the config for that connection here so that SimLn will be able to connect and generate payments
			let n = SimLnNode{id: node_id.clone(), address: String::from(""), macaroon: String::from(""), cert: String::from("")};
			node_list.nodes.push(n);
		}

		// Serialize the SimLn data into a json string
		let mut bldk = self.blast_ldk.lock().await;
		bldk.simln_data = match serde_json::to_string(&node_list) {
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
		let bldk = self.blast_ldk.lock().await;
		let simln_response = BlastSimlnResponse { simln_data: bldk.simln_data.clone().into() };
		let response = Response::new(simln_response);
		Ok(response)
	}

	/// Blast requests the pub key of a node that is controlled by this model
	async fn get_pub_key(&self, request: Request<BlastPubKeyRequest>,) -> Result<Response<BlastPubKeyResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;

		let pub_key = node.node_id().to_string();

		let key_response = BlastPubKeyResponse { pub_key: pub_key };
		let response = Response::new(key_response);
		Ok(response)
	}

	/// Blast requests the list of peers for a node that is controlled by this model
	async fn list_peers(&self, request: Request<BlastPeersRequest>,) -> Result<Response<BlastPeersResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;

		let peers = format!("{:?}", node.list_peers());

		let peers_response = BlastPeersResponse { peers: peers };
		let response = Response::new(peers_response);
		Ok(response)
	}

	/// Blast requests the wallet balance of a node that is controlled by this model
	async fn wallet_balance(&self, request: Request<BlastWalletBalanceRequest>) -> Result<Response<BlastWalletBalanceResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;

		let balance = node.list_balances().total_onchain_balance_sats;

		let balance_response = BlastWalletBalanceResponse { balance: balance.to_string() };
		let response = Response::new(balance_response);
		Ok(response)
	}

	/// Blast requests the channel balance of a node that is controlled by this model
	async fn channel_balance(&self, request: Request<BlastChannelBalanceRequest>) -> Result<Response<BlastChannelBalanceResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;

		let balance = node.list_balances().total_lightning_balance_sats;

		let balance_response = BlastChannelBalanceResponse { balance: balance.to_string() };
		let response = Response::new(balance_response);
		Ok(response)
	}

	/// Blast requests the list of channels for a node that is controlled by this model
	async fn list_channels(&self, request: Request<BlastListChannelsRequest>) -> Result<Response<BlastListChannelsResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;

		let chans = format!("{:?}", node.list_channels());

		let chan_response = BlastListChannelsResponse { channels: chans };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model opens a channel
	async fn open_channel(&self, request: Request<BlastOpenChannelRequest>) -> Result<Response<BlastOpenChannelResponse>, Status> {
		let req = &request.get_ref();

		// Get the source node from the id
		let node_id = &req.node;
		let node = self.get_node(node_id.to_string()).await?;

		// Get the peer public key from the request and convert it to a PublicKey object
		let peer_pub = match PublicKey::from_slice(hex::decode(&req.peer_pub_key).unwrap().as_slice()) {
			Ok(k) => k,
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer pub key: {:?}", req.peer_pub_key)));
			}
		};

		// Get the peer address from the request and convert it to a SocketAddress object
		let addr = req.peer_address.clone();
		let converted_addr = addr.replace("localhost", "127.0.0.1");
		let peer_addr = match SocketAddress::from_str(&converted_addr) {
			Ok(a) => a,
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer address: {:?}", &req.peer_address)));
			}
		};

		// Get the other parameters from the request
		let amount = req.amount;
		let push = req.push_amout;
		let id = req.channel_id;

		// Attempt to open a channel from this node
		let chan_id = match node.open_announced_channel(peer_pub, peer_addr, amount as u64, Some(push as u64), None) {
			Ok(id) => id,
			Err(_) => {
				return Err(Status::new(Code::Unknown, format!("Could not open channel.")));
			}
		};

		// Add the channel to the model's list of open channels
		let mut bldk = self.blast_ldk.lock().await;
		bldk.open_channels.insert(id, Channel{source: node_id.to_string(), id: chan_id, pk: peer_pub});

		// Respond to the open channel request
		let chan_response = BlastOpenChannelResponse { success: true };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model closes a channel
	async fn close_channel(&self, request: Request<BlastCloseChannelRequest>) -> Result<Response<BlastCloseChannelResponse>, Status> {
		let req = &request.get_ref();

		// Get the source node from the id
		let node_id = &req.node;
		let node = self.get_node(node_id.to_string()).await?;

		// Get the channel from the model's open channel map
		let id = req.channel_id;
		let mut bldk = self.blast_ldk.lock().await;
		let channel = match bldk.open_channels.get(&id) {
			Some(c) => c,
			None => {
				return Err(Status::new(Code::Unknown, format!("Could not find the channel.")));
			}
		};

		// Attempt to close the channel
		match node.close_channel(&channel.id, channel.pk) {
			Ok(_) => {},
			Err(_) => {
				return Err(Status::new(Code::Unknown, format!("Could not close channel.")));
			}
		}

		// Remove the channel from the model's list of open channels
		bldk.open_channels.remove(&id);

		// Respond to the close channel request
		let chan_response = BlastCloseChannelResponse { success: true };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Create a comma separated list of open channels that this model has control over
	async fn get_model_channels(&self, _request: Request<BlastGetModelChannelsRequest>) -> Result<Response<BlastGetModelChannelsResponse>, Status> {
		let mut result = String::new();
		let bldk = self.blast_ldk.lock().await;
		for (key, value) in &bldk.open_channels {
			result.push_str(&format!("{}: {} -> {},", key, &value.source, value.pk.to_string()));
		}

		result.pop();

		let chan_response = BlastGetModelChannelsResponse { channels: result };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model connects to a peer
	async fn connect_peer(&self, request: Request<BlastConnectRequest>) -> Result<Response<BlastConnectResponse>, Status> {
		let req = &request.get_ref();

		// Get the peer public key from the request and convert it to a PublicKey object
		let peer_pub = match PublicKey::from_slice(hex::decode(&req.peer_pub_key).unwrap().as_slice()) {
			Ok(k) => k,
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer pub key: {:?}", req.peer_pub_key)));
			}
		};

		// Get the peer address from the request and convert it to a SocketAddress object
		let addr = req.peer_addr.clone();
		let converted_addr = addr.replace("localhost", "127.0.0.1");
		let peer_addr = match SocketAddress::from_str(&converted_addr) {
			Ok(a) => a,
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer address: {:?}", &req.peer_addr)));
			}
		};

		// Attempt to connect to the peer from this node
		let node_id = &req.node;
		let node = self.get_node(node_id.to_string()).await?;
		match node.connect(peer_pub, peer_addr, true) {
			Ok(_) => {
				let connect_response = BlastConnectResponse { success: true };
				let response = Response::new(connect_response);
				Ok(response)
			},
			Err(_) => {
				let connect_response = BlastConnectResponse { success: false };
				let response = Response::new(connect_response);
				Ok(response)
			}
		}
	}

	/// Blast requests that a node controlled by this model disconnects from a peer
	async fn disconnect_peer(&self, request: Request<BlastDisconnectRequest>) -> Result<Response<BlastDisconnectResponse>, Status> {
		let req = &request.get_ref();

		// Get the peer public key from the request and convert it to a PublicKey object
		let peer_pub = match PublicKey::from_slice(hex::decode(&req.peer_pub_key).unwrap().as_slice()) {
			Ok(k) => k,
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer pub key: {:?}", req.peer_pub_key)));
			}
		};

		// Attempt to disconnect from the peer
		let node_id = &req.node;
		let node = self.get_node(node_id.to_string()).await?;
		match node.disconnect(peer_pub) {
			Ok(_) => {
				let connect_response = BlastDisconnectResponse { success: true };
				let response = Response::new(connect_response);
				Ok(response)
			},
			Err(_) => {
				let connect_response = BlastDisconnectResponse { success: false };
				let response = Response::new(connect_response);
				Ok(response)
			}
		}
	}

	/// Get a BTC address for a node
	async fn get_btc_address(&self, request: Request<BlastBtcAddressRequest>) -> Result<Response<BlastBtcAddressResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;
		
		let address = match node.onchain_payment().new_address() {
			Ok(a) => a,
			Err(_) => {
				return Err(Status::new(Code::Unknown, "Could not get bitcoin address."));
			}
		};

		let addr_response = BlastBtcAddressResponse { address: address.to_string() };
		let response = Response::new(addr_response);
		Ok(response)
	}

	/// Get the listen address for a node
	async fn get_listen_address(&self, request: Request<BlastListenAddressRequest>) -> Result<Response<BlastListenAddressResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;

		let addr = match node.config().listening_addresses {
			Some(a) => a,
			None => {
				return Err(Status::new(Code::Unknown, "Could not get listening address."));
			}
		};

		let listen_response = BlastListenAddressResponse { address: addr.clone().get(0).unwrap().clone().to_string() };
		let response = Response::new(listen_response);
		Ok(response)
	}

	/// Shutdown the nodes
	async fn stop_model(&self, _request: Request<BlastStopModelRequest>) -> Result<Response<BlastStopModelResponse>, Status> {
		let mut bldk = self.blast_ldk.lock().await;
		for (_, node) in &bldk.nodes {
			node.stop().unwrap();
		}
		let _ = bldk.shutdown_sender.take().unwrap().send(());

		let mut data_dir = env!("CARGO_MANIFEST_DIR").to_owned();
        data_dir.push_str("/blast_data/");
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
		let sim_model_dir = format!("{}{}{}/{}/", home_dir, sim_dir, sim_name, MODEL_NAME);

		// Set paths for the archive and JSON file
		let archive_path = Path::new(&sim_model_dir).join(format!("{}.tar.gz", sim_name));
		let json_path = Path::new(&sim_model_dir).join(format!("{}_channels.json", sim_name));

		// Open the .tar.gz file
		let tar_gz = File::open(archive_path)?;
		let decompressor = GzDecoder::new(tar_gz);
		let mut archive = Archive::new(decompressor);
		// Extract the archive into the specified directory
		let mut data_dir = env!("CARGO_MANIFEST_DIR").to_owned();
        data_dir.push_str(DATA_DIR);
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
                    if dir_name.starts_with("blast_ldk-") {
                        count += 1;

                        // Construct the path to the file to remove
                        let file_path = path.join("logs/ldk_node_latest.log");

                        // Attempt to remove the file if it exists
                        if file_path.exists() {
                            match fs::remove_file(&file_path) {
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
		let chans: HashMap<i64, Channel> = serde_json::from_reader(reader).unwrap();
		let mut bldk = self.blast_ldk.lock().await;
		bldk.open_channels = chans;

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
		let sim_model_dir = format!("{}{}{}/{}/", home_dir, sim_dir, sim_name, MODEL_NAME);

		// Set paths for the archive and JSON file
		let archive_path = Path::new(&sim_model_dir).join(format!("{}.tar.gz", sim_name));
		let json_path = Path::new(&sim_model_dir).join(format!("{}_channels.json", sim_name));

		// Create the .tar.gz archive
		let mut data_dir = env!("CARGO_MANIFEST_DIR").to_owned();
        data_dir.push_str("/blast_data/");
		if let Some(parent) = archive_path.parent() {
			fs::create_dir_all(parent).unwrap();
		}
		let tar_gz = File::create(&archive_path).unwrap();
		let enc = GzEncoder::new(tar_gz, Compression::default());
		let mut tar = TarBuilder::new(enc);
		tar.append_dir_all(".", data_dir).unwrap();

		// Serialize the HashMap to JSON and write to a file
		let bldk = self.blast_ldk.lock().await;
		let json_string = serde_json::to_string_pretty(&bldk.open_channels).unwrap();
		fs::write(&json_path, json_string)?;

		let save_response = BlastSaveResponse { success: true };
		let response = Response::new(save_response);
		Ok(response)
	}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	// Set up the logger for this model
	let home = env::var("HOME").expect("HOME environment variable not set");
    let folder_path = PathBuf::from(home).join(".blast/blast_ldk.log");
    std::fs::create_dir_all(folder_path.parent().unwrap()).unwrap();
	let _ = WriteLogger::init(
        LevelFilter::Info,
        LogConfig::default(),
        File::create(folder_path).unwrap(),
    );

	// Create a multi-thread runtime that the LDK-nodes will run on
	let rt = Arc::new(tokio::runtime::Builder::new_multi_thread()
	.enable_all()
	.build()
	.unwrap());

	// Create the BlastLdkServer object
    let addr = "127.0.0.1:5051".parse()?;
	let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();
	let mut bldk = BlastLdk::new();
	bldk.shutdown_sender = Some(shutdown_sender);
    let blast_ldk = Arc::new(Mutex::new(bldk));
	let server = BlastLdkServer {
        blast_ldk: Arc::clone(&blast_ldk),
		runtime: Arc::clone(&rt)
    };

	// Start the RPC server
    log::info!("Starting gRPC server at {}", addr);
	let server = rt.spawn(async move {
		Server::builder()
        .add_service(BlastRpcServer::new(server))
        .serve_with_shutdown(addr, async {
			shutdown_receiver.await.ok();
		})
		.await
		.unwrap();
	});

	// Wait for the server task to finish
	rt.block_on(async {
		let _ = server.await;
	});

	log::info!("Shutting down gRPC server at {}", addr);

    Ok(())
}
