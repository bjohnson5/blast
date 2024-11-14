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

// Extra dependencies
use secp256k1::PublicKey;
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

// The temporary directory to save runtime cln data
pub const DATA_DIR: &str = "/blast_data/";

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

// The main data structure for the CLN model
struct BlastCln {
	nodes: HashMap<String, NodeClient<Channel>>,
	simln_data: String,
    shutdown_sender: Option<oneshot::Sender<()>>
}

// Constructor for the CLN model
impl BlastCln {
    fn new() -> Self {
        Self {
			nodes: HashMap::new(),
			simln_data: String::from(""),
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
		let mut data_dir = env!("CARGO_MANIFEST_DIR").to_owned();
        data_dir.push_str(DATA_DIR);

		// Start the requested number of cln nodes
		for i in 0..num_nodes {
			// Create a node id and alias
			let node_id = format!("{}{:04}", "blast_cln-", i);
			let port = self.get_available_port(8000, 9000).unwrap();
			let rpcport = self.get_available_port(port+1, 9000).unwrap().to_string();
			let cln_dir = format!("{}{}", data_dir, node_id);
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

		// TODO: Format the pub key
		let pub_key = format!("{:?}", node.getinfo(GetinfoRequest{}).await);

		let key_response = BlastPubKeyResponse { pub_key: pub_key };
		let response = Response::new(key_response);
		Ok(response)
	}

	/// Blast requests the list of peers for a node that is controlled by this model
	async fn list_peers(&self, request: Request<BlastPeersRequest>,) -> Result<Response<BlastPeersResponse>, Status> {
		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;
		
		// TODO: Format the peers list
		let peers = format!("{:?}", node.list_peers(ListpeersRequest{id: None, level: None}).await);

		let peers_response = BlastPeersResponse { peers: peers };
		let response = Response::new(peers_response);
		Ok(response)
	}

	/// Blast requests the wallet balance of a node that is controlled by this model
	async fn wallet_balance(&self, request: Request<BlastWalletBalanceRequest>) -> Result<Response<BlastWalletBalanceResponse>, Status> {
		let _node_id = &request.get_ref().node;

        // TODO: Get the balance

		let balance_response = BlastWalletBalanceResponse { balance: String::from("") };
		let response = Response::new(balance_response);
		Ok(response)
	}

	/// Blast requests the channel balance of a node that is controlled by this model
	async fn channel_balance(&self, request: Request<BlastChannelBalanceRequest>) -> Result<Response<BlastChannelBalanceResponse>, Status> {
		let _node_id = &request.get_ref().node;

        // TODO: Get the balance

		let balance_response = BlastChannelBalanceResponse { balance: String::from("") };
		let response = Response::new(balance_response);
		Ok(response)
	}

	/// Blast requests the list of channels for a node that is controlled by this model
	async fn list_channels(&self, request: Request<BlastListChannelsRequest>) -> Result<Response<BlastListChannelsResponse>, Status> {
		let _node_id = &request.get_ref().node;

        // TODO: Get the channels

		let chan_response = BlastListChannelsResponse { channels: String::from("") };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model opens a channel
	async fn open_channel(&self, request: Request<BlastOpenChannelRequest>) -> Result<Response<BlastOpenChannelResponse>, Status> {
		let req = &request.get_ref();

		// Get the source node from the id
		let _node_id = &req.node;

        // Get the peer public key from the request and convert it to a PublicKey object
		let _peer_pub = match PublicKey::from_slice(hex::decode(&req.peer_pub_key).unwrap().as_slice()) {
			Ok(k) => k,
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer pub key: {:?}", req.peer_pub_key)));
			}
		};

		// Get the peer address from the request and convert it to a SocketAddress object
		let addr = req.peer_address.clone();
		let _converted_addr = addr.replace("localhost", "127.0.0.1");

		// Get the other parameters from the request
		let _amount = req.amount;
		let _push = req.push_amout;
		let _id = req.channel_id;

        // TODO: Open the channel

		// Respond to the open channel request
		let chan_response = BlastOpenChannelResponse { success: true };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model closes a channel
	async fn close_channel(&self, request: Request<BlastCloseChannelRequest>) -> Result<Response<BlastCloseChannelResponse>, Status> {
		let req = &request.get_ref();

		// Get the source node from the id
		let _node_id = &req.node;

		// Get the channel from the model's open channel map
		let _id = req.channel_id;

        // TODO: Close the channel

		// Respond to the close channel request
		let chan_response = BlastCloseChannelResponse { success: true };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Create a comma separated list of open channels that this model has control over
	async fn get_model_channels(&self, _request: Request<BlastGetModelChannelsRequest>) -> Result<Response<BlastGetModelChannelsResponse>, Status> {
        // TODO: Get the channels

		let chan_response = BlastGetModelChannelsResponse { channels: String::from("") };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model connects to a peer
	async fn connect_peer(&self, request: Request<BlastConnectRequest>) -> Result<Response<BlastConnectResponse>, Status> {
		let req = &request.get_ref();

		// Get the peer public key from the request and convert it to a PublicKey object
		let _peer_pub = match PublicKey::from_slice(hex::decode(&req.peer_pub_key).unwrap().as_slice()) {
			Ok(k) => k,
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer pub key: {:?}", req.peer_pub_key)));
			}
		};

		// Get the peer address from the request and convert it to a SocketAddress object
		let addr = req.peer_addr.clone();
		let _converted_addr = addr.replace("localhost", "127.0.0.1");

		// Attempt to connect to the peer from this node
		let _node_id = &req.node;

        // TODO: Connect to the peer

        let connect_response = BlastConnectResponse { success: true };
        let response = Response::new(connect_response);
        Ok(response)
	}

	/// Blast requests that a node controlled by this model disconnects from a peer
	async fn disconnect_peer(&self, request: Request<BlastDisconnectRequest>) -> Result<Response<BlastDisconnectResponse>, Status> {
		let req = &request.get_ref();

		// Get the peer public key from the request and convert it to a PublicKey object
		let _peer_pub = match PublicKey::from_slice(hex::decode(&req.peer_pub_key).unwrap().as_slice()) {
			Ok(k) => k,
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer pub key: {:?}", req.peer_pub_key)));
			}
		};

		// Attempt to disconnect from the peer
		let _node_id = &req.node;

        // TODO: Disconnect from the peer

        let connect_response = BlastDisconnectResponse { success: true };
        let response = Response::new(connect_response);
        Ok(response)
	}

	/// Get a BTC address for a node
	async fn get_btc_address(&self, request: Request<BlastBtcAddressRequest>) -> Result<Response<BlastBtcAddressResponse>, Status> {
		let _node_id = &request.get_ref().node;

        // TODO: Get the BTC address

		let addr_response = BlastBtcAddressResponse { address: String::from("") };
		let response = Response::new(addr_response);
		Ok(response)
	}

	/// Get the listen address for a node
	async fn get_listen_address(&self, request: Request<BlastListenAddressRequest>) -> Result<Response<BlastListenAddressResponse>, Status> {
		let _node_id = &request.get_ref().node;

        // TODO: Get the listen address

		let listen_response = BlastListenAddressResponse { address: String::from("") };
		let response = Response::new(listen_response);
		Ok(response)
	}

	/// Shutdown the nodes
	async fn stop_model(&self, _request: Request<BlastStopModelRequest>) -> Result<Response<BlastStopModelResponse>, Status> {
		let mut data_dir = env!("CARGO_MANIFEST_DIR").to_owned();
        data_dir.push_str(DATA_DIR);

        let mut bcln = self.blast_cln.lock().await;
		for (id, node) in &bcln.nodes {
			match node.clone().stop(StopRequest{}).await {
				Ok(_) => {},
				Err(_) => {
					let mut command = Command::new("bash");
					let mut script_file = env!("CARGO_MANIFEST_DIR").to_owned();
					script_file.push_str("/stop_cln.sh");
					command.arg(&script_file);
					command.arg(format!("{}{}", data_dir, id));
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
		let _sim_name = &req.sim;

        // TODO: Load the sim

		let load_response = BlastLoadResponse { success: true };
		let response = Response::new(load_response);
		Ok(response)
	}

	/// Save this models current state
	async fn save(&self, request: Request<BlastSaveRequest>) -> Result<Response<BlastSaveResponse>, Status> {
		let req = &request.get_ref();
		let _sim_name = &req.sim;

        // TODO: Save the sim

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
