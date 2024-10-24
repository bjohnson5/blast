use std::str::FromStr;
use std::time::Duration;
use std::thread;
use std::sync::Arc;
use std::collections::HashMap;
use std::fs;
use std::fs::File;

use ldk_node::bip39::serde::{Deserialize, Serialize};
use ldk_node::{Builder, LogLevel};
use ldk_node::bitcoin::Network;
use ldk_node::config::Config;
use ldk_node::lightning::ln::msgs::SocketAddress;
use ldk_node::lightning::routing::gossip::NodeAlias;
use ldk_node::UserChannelId;

use secp256k1::PublicKey;
use tonic::{transport::Server, Request, Response, Status};
use tonic::Code;
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use tokio::runtime::Runtime;
use simplelog::WriteLogger;
use simplelog::Config as LogConfig;
use log::LevelFilter;
use std::path::PathBuf;
use std::env;
use std::net::TcpListener;

use blast_rpc_server::BlastRpcServer;
use blast_rpc_server::BlastRpc;
use blast_proto::*;
pub mod blast_proto {
    tonic::include_proto!("blast_proto");
}

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

struct Channel {
	id: UserChannelId,
	pk: PublicKey
}

struct BlastLdk {
    nodes: HashMap<String, Arc<ldk_node::Node>>,
	simln_data: String,
	open_channels: HashMap<i64, Channel>,
	shutdown_sender: Option<oneshot::Sender<()>>
}

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

struct BlastLdkServer {
    blast_ldk: Arc<Mutex<BlastLdk>>,
	runtime: Arc<Runtime>
}

impl BlastLdkServer {
	async fn get_node(&self, id: String) -> Result<Arc<ldk_node::Node>, Status> {
		let bldk = self.blast_ldk.lock().await;
		let node = match bldk.nodes.get(&id) {
			Some(n) => n,
			None => {
				return Err(Status::new(Code::NotFound, "Node not found."))
			}
		};

		Ok(node.clone())
	}

	fn get_available_port(&self) -> Option<u16> {
		(8000..9000)
			.find(|port| self.port_is_available(*port))
	}
	
	fn port_is_available(&self, port: u16) -> bool {
		match TcpListener::bind(("127.0.0.1", port)) {
			Ok(_) => true,
			Err(_) => false,
		}
	}
}

#[tonic::async_trait]
impl BlastRpc for BlastLdkServer {
	async fn start_nodes(&self, request: Request<BlastStartRequest>) -> Result<Response<BlastStartResponse>,Status> {
		let num_nodes = request.get_ref().num_nodes;
		let mut node_list = SimJsonFile{nodes: Vec::new()};
		let mut data_dir = env!("CARGO_MANIFEST_DIR").to_owned();
        data_dir.push_str("/blast_data/");
		for i in 0..num_nodes {
			let node_id = prepend_and_pad("blast_ldk-", i);
			let alias = node_id.as_bytes();
			// Create an array and fill it with values from the slice
			let mut alias_array = [0u8; 32]; // Fill with default value 0
			let len = alias.len().min(alias_array.len()); // Get the minimum length
			alias_array[..len].copy_from_slice(alias); // Copy the slice into the array
			let node_alias = NodeAlias(alias_array);
			let mut listen_addr: Vec<SocketAddress> = Vec::new();
			let port = self.get_available_port().unwrap();
			let a = format!("127.0.0.1:{}", port);
			let addr = match SocketAddress::from_str(&a) {
				Ok(a) => a,
				Err(_) => {
					return Err(Status::new(Code::InvalidArgument, "Could not create listen address."));
				}
			};
			listen_addr.push(addr);
			let config = Config {
				storage_dir_path: format!("{}{}", data_dir, node_id),
				log_dir_path: None,
				network: Network::Regtest,
				listening_addresses: Some(listen_addr),
				node_alias: Some(node_alias),
				sending_parameters: None,
				trusted_peers_0conf: Vec::new(),
				probing_liquidity_limit_multiplier: 0,
				log_level: LogLevel::Debug,
				anchor_channels_config: None
			};

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

			match node.start_with_runtime(Arc::clone(&self.runtime)) {
				Ok(_) => {},
				Err(_) => {
					return Err(Status::new(Code::Unknown, "Could not start the ldk node."));
				}
			}

			thread::sleep(Duration::from_secs(2));

			let mut bldk = self.blast_ldk.lock().await;
			bldk.nodes.insert(node_id.clone(), node.clone());

			let n = SimLnNode{id: node_id.clone(), address: String::from(""), macaroon: String::from(""), cert: String::from("")};
			node_list.nodes.push(n);
		}

		let mut bldk = self.blast_ldk.lock().await;
		bldk.simln_data = match serde_json::to_string(&node_list) {
			Ok(s) => s,
			Err(_) => {
				let start_response = BlastStartResponse { success: false };
				let response = Response::new(start_response);
				return Ok(response);
			}
		};

		let start_response = BlastStartResponse { success: true };
		let response = Response::new(start_response);
		Ok(response)
	}

	async fn get_sim_ln(&self, _request: Request<BlastSimlnRequest>) -> Result<Response<BlastSimlnResponse>, Status> {
		let bldk = self.blast_ldk.lock().await;
		let simln_response = BlastSimlnResponse { simln_data: bldk.simln_data.clone().into() };
		let response = Response::new(simln_response);
		Ok(response)
	}

	async fn get_pub_key(&self, request: Request<BlastPubKeyRequest>,) -> Result<Response<BlastPubKeyResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;
		let pub_key = node.node_id().to_string();

		let key_response = BlastPubKeyResponse { pub_key: pub_key };
		let response = Response::new(key_response);
		Ok(response)
	}

	async fn list_peers(&self, request: Request<BlastPeersRequest>,) -> Result<Response<BlastPeersResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;
		let peers = format!("{:?}", node.list_peers());

		let peers_response = BlastPeersResponse { peers: peers };
		let response = Response::new(peers_response);
		Ok(response)
	}

	async fn wallet_balance(&self, request: Request<BlastWalletBalanceRequest>) -> Result<Response<BlastWalletBalanceResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;
		let balance = node.list_balances().total_onchain_balance_sats;

		let balance_response = BlastWalletBalanceResponse { balance: balance.to_string() };
		let response = Response::new(balance_response);
		Ok(response)
	}

	async fn channel_balance(&self, request: Request<BlastChannelBalanceRequest>) -> Result<Response<BlastChannelBalanceResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;
		let balance = node.list_balances().total_lightning_balance_sats;

		let balance_response = BlastChannelBalanceResponse { balance: balance.to_string() };
		let response = Response::new(balance_response);
		Ok(response)
	}

	async fn list_channels(&self, request: Request<BlastListChannelsRequest>) -> Result<Response<BlastListChannelsResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;
		let chans = format!("{:?}", node.list_channels());

		let chan_response = BlastListChannelsResponse { channels: chans };
		let response = Response::new(chan_response);
		Ok(response)
	}

	async fn open_channel(&self, request: Request<BlastOpenChannelRequest>) -> Result<Response<BlastOpenChannelResponse>, Status> {
		let req = &request.get_ref();
		let node_id = &req.node;
		let node = self.get_node(node_id.to_string()).await?;
		let peer_pub = match PublicKey::from_slice(hex::decode(&req.peer_pub_key).unwrap().as_slice()) {
			Ok(k) => { k },
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer pub key: {:?}", req.peer_pub_key)));
			}
		};
		let address = match SocketAddress::from_str(&req.peer_address) {
			Ok(a) => a,
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer address: {:?}", &req.peer_address)));
			}
		};
		let amount = req.amount;
		let push = req.push_amout;
		let id = req.channel_id;

		let chan_id = match node.open_announced_channel(peer_pub, address, amount as u64, Some(push as u64), None) {
			Ok(id) => id,
			Err(_) => {
				return Err(Status::new(Code::Unknown, format!("Could not open channel.")));
			}
		};

		let mut bldk = self.blast_ldk.lock().await;
		bldk.open_channels.insert(id, Channel{id: chan_id, pk: peer_pub});

		let chan_response = BlastOpenChannelResponse { success: true };
		let response = Response::new(chan_response);
		Ok(response)
	}

	async fn close_channel(&self, request: Request<BlastCloseChannelRequest>) -> Result<Response<BlastCloseChannelResponse>, Status> {
		let req = &request.get_ref();
		let node_id = &req.node;
		let node = self.get_node(node_id.to_string()).await?;
		let id = req.channel_id;

		let bldk = self.blast_ldk.lock().await;
		let channel = match bldk.open_channels.get(&id) {
			Some(c) => c,
			None => {
				return Err(Status::new(Code::Unknown, format!("Could not close channel.")));
			}
		};

		match node.close_channel(&channel.id, channel.pk) {
			Ok(_) => {},
			Err(_) => {
				return Err(Status::new(Code::Unknown, format!("Could not close channel.")));
			}
		}

		let chan_response = BlastCloseChannelResponse { success: true };
		let response = Response::new(chan_response);
		Ok(response)
	}

	async fn get_model_channels(&self, _request: Request<BlastGetModelChannelsRequest>) -> Result<Response<BlastGetModelChannelsResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn connect_peer(&self, request: Request<BlastConnectRequest>) -> Result<Response<BlastConnectResponse>, Status> {
		let req = &request.get_ref();
		let node_id = &req.node;
		let peer_pub = match PublicKey::from_slice(hex::decode(&req.peer_pub_key).unwrap().as_slice()) {
			Ok(k) => { k },
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer pub key: {:?}", req.peer_pub_key)));
			}
		};
		let addr = req.peer_addr.clone();
		let converted_addr = addr.replace("localhost", "127.0.0.1");
		let peer_addr = match SocketAddress::from_str(&converted_addr) {
			Ok(a) => { a },
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer address: {:?}", &req.peer_addr)));
			}
		};
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

	async fn disconnect_peer(&self, request: Request<BlastDisconnectRequest>) -> Result<Response<BlastDisconnectResponse>, Status> {
		let req = &request.get_ref();
		let node_id = &req.node;
		let peer_pub = match PublicKey::from_slice(hex::decode(&req.peer_pub_key).unwrap().as_slice()) {
			Ok(k) => { k },
			Err(_) => {
				return Err(Status::new(Code::InvalidArgument, format!("Could not parse peer pub key: {:?}", req.peer_pub_key)));
			}
		};
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

	async fn get_btc_address(&self, request: Request<BlastBtcAddressRequest>) -> Result<Response<BlastBtcAddressResponse>, Status> {
		let node_id = &request.get_ref().node;
		let node = self.get_node(node_id.to_string()).await?;
		
		let address = match node.onchain_payment().new_address() {
			Ok(address) => address,
			Err(_) => {
				return Err(Status::new(Code::Unknown, "Could not get bitcoin address."));
			}
		};

		let addr_response = BlastBtcAddressResponse { address: address.to_string() };
		let response = Response::new(addr_response);
		Ok(response)
	}

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

	async fn load(&self, _request: Request<BlastLoadRequest>) -> Result<Response<BlastLoadResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn save(&self, _request: Request<BlastSaveRequest>) -> Result<Response<BlastSaveResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
	let home = env::var("HOME").expect("HOME environment variable not set");
    let folder_path = PathBuf::from(home).join(".blast/blast_ldk.log");
    std::fs::create_dir_all(folder_path.parent().unwrap()).unwrap();
	let _ = WriteLogger::init(
        LevelFilter::Info,
        LogConfig::default(),
        File::create(folder_path).unwrap(),
    );

	let rt = Arc::new(tokio::runtime::Builder::new_multi_thread()
	.enable_all()
	.build()
	.unwrap());

    let addr = "127.0.0.1:5051".parse()?;
	let (shutdown_sender, shutdown_receiver) = oneshot::channel::<()>();
	let mut bldk = BlastLdk::new();
	bldk.shutdown_sender = Some(shutdown_sender);
    let blast_ldk = Arc::new(Mutex::new(bldk));
	let server = BlastLdkServer {
        blast_ldk: Arc::clone(&blast_ldk),
		runtime: Arc::clone(&rt)
    };

    log::info!("Starting gRPC server at {}", addr);

	let server = rt.spawn(async move {
		Server::builder()
        .add_service(BlastRpcServer::new(server))
        .serve_with_shutdown(addr, async {
			// Wait for the shutdown signal
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

fn prepend_and_pad(input: &str, num: i32) -> String {
    format!("{}{:04}", input, num)
}
