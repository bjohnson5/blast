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

// The address to connect to this model
pub const RPC_ADDR: &str = "127.0.0.1:5052";

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

// The CLN data that will be saved to disk when a simulation is saved
#[derive(Serialize, Deserialize, Debug)]
struct BlastClnData {
	simln_data: SimJsonFile,
	addresses: HashMap<String, String>,
	ports: HashMap<String, (String, String)>,
	open_channels: HashMap<i64, ClnChannel>,
}

// The main data structure for the CLN model
struct BlastCln {
	nodes: HashMap<String, NodeClient<Channel>>,
	cln_data: BlastClnData,
    shutdown_sender: Option<oneshot::Sender<()>>
}

// Constructor for the CLN model
impl BlastCln {
    fn new() -> Self {
        Self {
			nodes: HashMap::new(),
			cln_data: BlastClnData{ simln_data: SimJsonFile{nodes: Vec::new()}, addresses: HashMap::new(), ports: HashMap::new(), open_channels: HashMap::new()},
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
	/// Get a cln node connection from an id
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

	/// Get the HOME environment variable
	fn get_home(&self) -> Result<String, Status> {
		match env::var("HOME") {
			Ok(h) => {
				Ok(h)
			},
			Err(_) => {
				Err(Status::new(Code::NotFound, "HOME environment variable is not set."))
			}
		}
	}

	/// Get an available port that can be used for listening
	fn get_available_port(&self, start: u16, end: u16) -> Result<u16, Status> {
		match (start..end).find(|port| self.port_is_available(*port)) {
			Some(p) => {
				Ok(p)
			},
			None => {
				Err(Status::new(Code::NotFound, "Could not find an available port."))
			}
		}
	}

	/// Check if a port is available
	fn port_is_available(&self, port: u16) -> bool {
		match TcpListener::bind(("127.0.0.1", port)) {
			Ok(_) => true,
			Err(_) => false,
		}
	}

	/// Load the saved nodes using the saved data
	async fn load_nodes(&self, data: BlastClnData) -> Result<Response<BlastLoadResponse>,Status> {
		let mut bcln = self.blast_cln.lock().await;
		let mut nodes: HashMap<String, NodeClient<Channel>> = HashMap::new();
		for n in &data.simln_data.nodes {
			// Create a node id, get available ports and set the cert paths
			let node_id = n.id.clone();
			let port = &data.ports.get(&node_id).unwrap().0.clone();
			let rpcport = &data.ports.get(&node_id).unwrap().1.clone();

			// Create a new client from the connected channel
			let (_,c) = self.start_node(node_id.clone(), port.to_string().clone(), rpcport.clone()).await?;
			nodes.insert(node_id.clone(), c);
		}

		bcln.cln_data = data;
		bcln.nodes = nodes;

		// Return the response to start_nodes
		let start_response = BlastLoadResponse { success: true };
		let response = Response::new(start_response);
		Ok(response)
	}

	/// Start a node with a given id and ports
	async fn start_node(&self, node_id: String, port: String, rpcport: String) -> Result<(SimLnNode,NodeClient<Channel>),Status> {
		// Set the node file paths and address
		let home = self.get_home()?;
		let data_dir = PathBuf::from(home).join(DATA_DIR).display().to_string();
		let cln_dir = format!("{}/{}", data_dir, node_id);
		let addr = format!("{}:{}", "https://localhost", rpcport.to_string());
		let ca_path = format!("{}{}", cln_dir, "/regtest/ca.pem");
		let client_path = format!("{}{}", cln_dir, "/regtest/client.pem");
		let client_key_path = format!("{}{}", cln_dir, "/regtest/client-key.pem");

		// Start a node
		let mut command = Command::new("bash");
		let mut script_file = env!("CARGO_MANIFEST_DIR").to_owned();
		script_file.push_str("/start_cln.sh");
		command.arg(&script_file);
		command.arg(&port);
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
		let ca_cert = match fs::read(ca_path.clone()) {
			Ok(c) => { c }
			Err(_) => return Err(Status::new(Code::Unknown, "Could not read the ca path.")),
		};
		let ca_certificate = Certificate::from_pem(ca_cert);
		let client_cert = match fs::read(client_path.clone()) {
			Ok(c) => { c }
			Err(_) => return Err(Status::new(Code::Unknown, "Could not read the client path.")),
		};
		let client_key_cert = match fs::read(client_key_path.clone()) {
			Ok(c) => { c }
			Err(_) => return Err(Status::new(Code::Unknown, "Could not read the client key.")),
		};
		let id = tonic::transport::Identity::from_pem(client_cert, client_key_cert);

		// Configure TLS settings with the CA certificate
		let tls_config = ClientTlsConfig::new()
			.domain_name("localhost")
			.identity(id)
			.ca_certificate(ca_certificate);

		// Create the URI from the generated address
		let uri: Uri = match addr.parse() {
			Ok(u) => { u }
			Err(_) => return Err(Status::new(Code::Unknown, "Invalid uri.")),
		};

		// Connect to the gRPC server using SSL/TLS
		let channel = match Channel::builder(uri)
			.tls_config(tls_config).unwrap()
			.connect()
			.await {
				Ok(c) => { c }
				Err(_) => return Err(Status::new(Code::Unknown, "Could not connect to server.")),
			};

		// Add the node to the model's list of nodes and to the SimLn data list
		let n = SimLnNode{id: node_id.clone(), address: addr.clone(), ca_cert: ca_path, client_cert: client_path, client_key: client_key_path};

		// Create a new client from the connected channel
		Ok((n, NodeClient::new(channel)))
	}
}

// The RPC server that the blast framework will connect to
#[tonic::async_trait]
impl BlastRpc for BlastClnServer {
	/// Start a certain number of nodes
	async fn start_nodes(&self, request: Request<BlastStartRequest>) -> Result<Response<BlastStartResponse>,Status> {
		log::info!("BlastClnServer: RPC start_nodes");

		let num_nodes = request.get_ref().num_nodes;
		let mut bcln = self.blast_cln.lock().await;

		// Start the requested number of cln nodes
		for i in 0..num_nodes {
			// Create a node id, get available ports and set the cert paths
			let node_id = format!("{}{:04}", "blast_cln-", i);
			let port = self.get_available_port(8000, 9000)?;
			let rpcport = self.get_available_port(port+1, 9000)?.to_string();

			// Create a new client from the connected channel
			let (n,c) = self.start_node(node_id.clone(), port.to_string().clone(), rpcport.clone()).await?;
			bcln.nodes.insert(node_id.clone(), c);
			bcln.cln_data.simln_data.nodes.push(n);
			bcln.cln_data.addresses.insert(node_id.clone(), format!("localhost:{}", &port.to_string()));
			bcln.cln_data.ports.insert(node_id.clone(), (port.to_string().clone(), rpcport.clone()));
		}

		// Return the response to start_nodes
		let start_response = BlastStartResponse { success: true };
		let response = Response::new(start_response);
		Ok(response)
	}

	/// Get the sim-ln data for this model
	async fn get_sim_ln(&self, _request: Request<BlastSimlnRequest>) -> Result<Response<BlastSimlnResponse>, Status> {
		log::info!("BlastClnServer: RPC get_sim_ln");

		// Serialize the SimLn data into a json string
		let bcln = self.blast_cln.lock().await;
		let data = match serde_json::to_string(&bcln.cln_data.simln_data) {
			Ok(s) => s,
			Err(_) => {
				let simln_response = BlastSimlnResponse { simln_data: String::from("").into() };
				let response = Response::new(simln_response);
				return Ok(response);
			}
		};

		let simln_response = BlastSimlnResponse { simln_data: data.clone().into() };
		let response = Response::new(simln_response);
		Ok(response)
	}

	/// Blast requests the pub key of a node that is controlled by this model
	async fn get_pub_key(&self, request: Request<BlastPubKeyRequest>,) -> Result<Response<BlastPubKeyResponse>, Status> {
		log::info!("BlastClnServer: RPC get_pub_key");

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
		log::info!("BlastClnServer: RPC list_peers");

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

		log::info!("Peers: {:?}", cln_resp.peers);

		let mut result = String::new();
		for p in cln_resp.peers {
			result.push_str(&format!("Pubkey: {}, Address: {}", hex::encode(p.id), p.netaddr[0]));
			result.push('\n');
		}

		let peers_response = BlastPeersResponse { peers: result };
		let response = Response::new(peers_response);
		Ok(response)
	}

	/// Blast requests the wallet balance of a node that is controlled by this model
	async fn wallet_balance(&self, request: Request<BlastWalletBalanceRequest>) -> Result<Response<BlastWalletBalanceResponse>, Status> {
		log::info!("BlastClnServer: RPC wallet_balance");

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

		log::info!("Wallet balance: {:?}", cln_resp.outputs);

		let mut balance = 0;
		for o in cln_resp.outputs {
			if let Some(amount) = o.amount_msat {
				balance = balance + amount.msat;
			}
		}

		let balance_response = BlastWalletBalanceResponse { balance: (balance / 1000).to_string() };
		let response = Response::new(balance_response);
		Ok(response)
	}

	/// Blast requests the channel balance of a node that is controlled by this model
	async fn channel_balance(&self, request: Request<BlastChannelBalanceRequest>) -> Result<Response<BlastChannelBalanceResponse>, Status> {
		log::info!("BlastClnServer: RPC channel_balance");

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

		log::info!("Channel balance: {:?}", cln_resp.channels);

		let mut balance = 0;
		for c in cln_resp.channels {
			if let Some(amount) = c.our_amount_msat {
				balance = balance + amount.msat;
			}
		}

		let balance_response = BlastChannelBalanceResponse { balance: (balance / 1000).to_string() };
		let response = Response::new(balance_response);
		Ok(response)
	}

	/// Blast requests the list of channels for a node that is controlled by this model
	async fn list_channels(&self, request: Request<BlastListChannelsRequest>) -> Result<Response<BlastListChannelsResponse>, Status> {
		log::info!("BlastClnServer: RPC list_channels");

		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;
		let cln_resp = match node.list_peer_channels(ListpeerchannelsRequest{id: None}).await {
			Ok(r) => {
				r.into_inner()
			},
			Err(s) => {
				return Err(s);
			}
		};

		log::info!("Channels: {:?}", cln_resp.channels);

		let mut result = String::new();
		for c in cln_resp.channels {
			if let Some(amount) = c.total_msat {
				result.push_str(&format!("Peer: {}, Amount: {}", hex::encode(c.peer_id), amount.msat / 1000));
				result.push('\n');
			}
		}

		let chan_response = BlastListChannelsResponse { channels: result };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model opens a channel
	async fn open_channel(&self, request: Request<BlastOpenChannelRequest>) -> Result<Response<BlastOpenChannelResponse>, Status> {
		log::info!("BlastClnServer: RPC open_channel");

		// Set the channel details
		let req = &request.get_ref();
		let node_id = &req.node;
		let peer = &req.peer_pub_key;
		let peer_pub = match hex::decode(peer.to_string()) {
			Ok(p) => { p }
			Err(_) => return Err(Status::new(Code::Unknown, "Could not decode the peer pub key.")),
		};
		let id = req.channel_id;
		let amount = req.amount * 1000;
		let push = Amount { msat: req.push_amout as u64 * 1000 };
		let a = Amount { msat: amount as u64 };
		let v = Value::Amount(a);
		let aora = AmountOrAll { value: Some(v) };

		// Attempt to open the channel
		log::info!("Opening channel from {} to {} for the amount: {}", node_id, peer.to_string(), amount);
		let mut node = self.get_node(node_id.to_string()).await?;
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

		// Saving channel details
		log::info!("Channel opened, saving details");
		let chanid = hex::encode(cln_resp.channel_id);
		let mut bcln = self.blast_cln.lock().await;
		bcln.cln_data.open_channels.insert(id, ClnChannel { source: node_id.to_string(), dest_pk: peer.to_string(), chan_id: chanid });

		// Respond to the open channel request
		let chan_response = BlastOpenChannelResponse { success: true };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model closes a channel
	async fn close_channel(&self, request: Request<BlastCloseChannelRequest>) -> Result<Response<BlastCloseChannelResponse>, Status> {
		log::info!("BlastClnServer: RPC close_channel");

		// Set the channel details
		let req = &request.get_ref();
		let node_id = &req.node;
		let id = &req.channel_id;
		let mut node = self.get_node(node_id.to_string()).await?;
		let mut bcln = self.blast_cln.lock().await;
		let chanid = match bcln.cln_data.open_channels.get(id) {
			Some(i) => &i.chan_id,
			None => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not get channel from id: {:?}", id)));
			}
		};

		// Attempt to close the channel
		log::info!("Closing channel: {}", chanid.to_string());
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
				bcln.cln_data.open_channels.remove(id);
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
		for (key, value) in &bcln.cln_data.open_channels {
			result.push_str(&format!("{}: {} -> {},", key, &value.source, value.dest_pk));
		}
		result.pop();

		let chan_response = BlastGetModelChannelsResponse { channels: result };
		let response = Response::new(chan_response);
		Ok(response)
	}

	/// Blast requests that a node controlled by this model connects to a peer
	async fn connect_peer(&self, request: Request<BlastConnectRequest>) -> Result<Response<BlastConnectResponse>, Status> {
		log::info!("BlastClnServer: RPC connect_peer");

		// Set the peer details
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
		log::info!("Connecting to peer: {}", peer_pub);
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
		log::info!("BlastClnServer: RPC disconnect_peer");

		// Get the node and the peer id
		let req = &request.get_ref();
		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;
		let id = match hex::decode(&req.peer_pub_key) {
			Ok(i) => { i }
			Err(_) => return Err(Status::new(Code::Unknown, "Could not decode the peer pub key.")),
		};

		// Attempt to disconnect
		log::info!("Disconnecting from peer: {}", &req.peer_pub_key);
		match node.disconnect(DisconnectRequest{id: id, force: None}).await {
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
		log::info!("BlastClnServer: RPC get_btc_address");

		let node_id = &request.get_ref().node;
		let mut node = self.get_node(node_id.to_string()).await?;

		// Get a new on-chain address
		log::info!("Getting new address for node: {}", node_id);
		let cln_resp = match node.new_addr(NewaddrRequest{addresstype: Some(3)}).await {
			Ok(r) => {
				r.into_inner()
			},
			Err(s) => {
				return Err(s);
			}
		};

		let addr = match cln_resp.p2tr {
			Some(a) => { a },
			None => return Err(Status::new(Code::Unknown, "Could not get btc address.")),
		};

		log::info!("Got new btc address: {}", addr);

		// Send the RPC response
		let addr_response = BlastBtcAddressResponse { address: addr };
		let response = Response::new(addr_response);
		Ok(response)
	}

	/// Get the listen address for a node
	async fn get_listen_address(&self, request: Request<BlastListenAddressRequest>) -> Result<Response<BlastListenAddressResponse>, Status> {
		log::info!("BlastClnServer: RPC get_listen_address");

		let node_id = &request.get_ref().node;
		let bcln = self.blast_cln.lock().await;
		let addr = match bcln.cln_data.addresses.get(node_id) {
			Some(a) => a,
			None => {
				return Err(Status::new(Code::InvalidArgument, format!("No addresses")));
			}
		};

		// Send the RPC response
		let listen_response = BlastListenAddressResponse { address: addr.clone() };
		let response = Response::new(listen_response);
		Ok(response)
	}

	/// Shutdown the nodes
	async fn stop_model(&self, _request: Request<BlastStopModelRequest>) -> Result<Response<BlastStopModelResponse>, Status> {
		log::info!("BlastClnServer: RPC stop_model");

		let home = self.get_home()?;
		let data_dir = PathBuf::from(home.clone()).join(DATA_DIR).display().to_string();
		let socket_dir = PathBuf::from(home).join(".blast/clightning/sockets").display().to_string();

		// Loop through the nodes and call stop. If the stop call fails, kills the process
		log::info!("Attempting to stop all nodes");
        let mut bcln = self.blast_cln.lock().await;
		for (id, node) in &mut bcln.nodes {
			match node.stop(StopRequest{}).await {
				Ok(_) => {},
				Err(_) => {
					let mut command = Command::new("bash");
					let mut script_file = env!("CARGO_MANIFEST_DIR").to_owned();
					script_file.push_str("/stop_cln.sh");
					command.arg(&script_file);
					command.arg(format!("{}/{}", data_dir, id));
					match command.output() {
						Ok(_) => {},
						Err(_) => return Err(Status::new(Code::InvalidArgument, "Could not stop cln.")),
					};
				}
			}
		}

		// Cleanup node data
		log::info!("Removing node data");
        let _ = bcln.shutdown_sender.take().unwrap().send(());
		let _ = fs::remove_dir_all(data_dir);
		let _ = fs::remove_dir_all(socket_dir);

		// Send the RPC response
		let stop_response = BlastStopModelResponse { success: true };
		let response = Response::new(stop_response);
		Ok(response)
	}

	/// Load a previous state of this model
	async fn load(&self, request: Request<BlastLoadRequest>) -> Result<Response<BlastLoadResponse>, Status> {
		log::info!("BlastClnServer: RPC load");

		// Set the simulation name and sim directory
		let req = &request.get_ref();
		let sim_name = &req.sim;
		let home_dir = self.get_home()?;
		let sim_dir = String::from(SIM_DIR);
		let sim_model_dir = format!("{}/{}/{}/{}/", home_dir, sim_dir, sim_name, MODEL_NAME);

		// Set paths for the archive and JSON file
		let archive_path = Path::new(&sim_model_dir).join(format!("{}.tar.gz", sim_name));
		let json_path = Path::new(&sim_model_dir).join(format!("{}_data.json", sim_name));

		// Open the .tar.gz file
		log::info!("Opening the tar archive");
		let tar_gz = File::open(archive_path)?;
		let decompressor = GzDecoder::new(tar_gz);
		let mut archive = Archive::new(decompressor);
		let home = self.get_home()?;
		let data_dir = PathBuf::from(home).join(DATA_DIR).display().to_string();
		let data_path = Path::new(&data_dir);
		fs::create_dir_all(data_path).unwrap();
		archive.unpack(data_path).unwrap();

		// Remove old log file and pid file
		log::info!("Clearing out old temporary files");
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

		// Deserialize the json data file
		log::info!("Loading the json data file");
		let file = File::open(json_path).unwrap();
		let reader = BufReader::new(file);
		let data: BlastClnData = serde_json::from_reader(reader).unwrap();

		// Attempt to start the nodes
		log::info!("Loading the cln nodes");
		Ok(self.load_nodes(data).await?)
	}

	/// Save this models current state
	async fn save(&self, request: Request<BlastSaveRequest>) -> Result<Response<BlastSaveResponse>, Status> {
		log::info!("BlastClnServer: RPC save");

		// Set the simulation name and directory
		let req = &request.get_ref();
		let sim_name = &req.sim;
		let home_dir = self.get_home()?;
		let sim_dir = String::from(SIM_DIR);
		let sim_model_dir = format!("{}/{}/{}/{}/", home_dir, sim_dir, sim_name, MODEL_NAME);

		// Set paths for the archive and JSON file
		let archive_path = Path::new(&sim_model_dir).join(format!("{}.tar.gz", sim_name));
		let json_path = Path::new(&sim_model_dir).join(format!("{}_data.json", sim_name));

		// Create the .tar.gz archive
		log::info!("Creating tar archive");
		let home = self.get_home()?;
		let data_dir = PathBuf::from(home).join(DATA_DIR).display().to_string();
		if let Some(parent) = archive_path.parent() {
			fs::create_dir_all(parent).unwrap();
		}
		let tar_gz = File::create(&archive_path).unwrap();
		let enc = GzEncoder::new(tar_gz, Compression::default());
		let mut tar = Builder::new(enc);
		tar.append_dir_all(".", data_dir).unwrap();

		// Serialize the data to JSON and write to a file
		log::info!("Creating json data file");
		let bcln = self.blast_cln.lock().await;
		let json_string = serde_json::to_string_pretty(&bcln.cln_data).unwrap();
		fs::write(&json_path, json_string)?;

		// Send the RPC response
		log::info!("Simulation {} saved successfully", sim_name);
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

	// Create the cln RPC server
	log::info!("Starting the blast_cln model");
    let addr = RPC_ADDR.parse()?;
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
	log::info!("Stopping the blast_cln model");

    Ok(())
}
