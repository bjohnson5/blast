use std::time::Duration;
use std::thread;
use std::sync::Arc;
use std::collections::HashMap;
use std::fs;

use ldk_node::bip39::serde::{Deserialize, Serialize};
use ldk_node::{Builder, LogLevel};
use ldk_node::bitcoin::Network;
use ldk_node::config::Config;

use tonic::{transport::Server, Request, Response, Status};
use tonic::Code;
use tokio::sync::Mutex;
use tokio::sync::oneshot;
use tokio::runtime::Runtime;

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

struct BlastLdk {
    nodes: HashMap<String, Arc<ldk_node::Node>>,
	simln_data: String,
	shutdown_sender: Option<oneshot::Sender<()>>
}

impl BlastLdk {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
			simln_data: String::from(""),
			shutdown_sender: None
        }
    }
}

struct BlastLdkServer {
    blast_ldk: Arc<Mutex<BlastLdk>>,
	runtime: Arc<Runtime>
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
			let config = Config {
				storage_dir_path: format!("{}{}", data_dir, node_id),
				log_dir_path: None,
				network: Network::Regtest,
				listening_addresses: None,
				node_alias: None,
				sending_parameters: None,
				trusted_peers_0conf: Vec::new(),
				probing_liquidity_limit_multiplier: 0,
				log_level: LogLevel::Debug,
				anchor_channels_config: None
			};

			let mut builder = Builder::from_config(config);
			builder.set_chain_source_bitcoind_rpc(String::from("127.0.0.1"), 18443, String::from("user"), String::from("pass"));
			builder.set_gossip_source_p2p();

			let node = Arc::new(builder.build().unwrap());

			node.start_with_runtime(Arc::clone(&self.runtime)).unwrap();
			println!("Node ({:?}) Status: {:?}", node_id, node.status());
			thread::sleep(Duration::from_secs(10));

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
		let bldk = self.blast_ldk.lock().await;
		let node = bldk.nodes.get(node_id).unwrap();
		let pub_key = node.node_id().to_string();

		let key_response = BlastPubKeyResponse { pub_key: pub_key };
		let response = Response::new(key_response);
		Ok(response)
	}

	async fn list_peers(&self, _request: Request<BlastPeersRequest>,) -> Result<Response<BlastPeersResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn wallet_balance(&self, _request: Request<BlastWalletBalanceRequest>) -> Result<Response<BlastWalletBalanceResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn channel_balance(&self, _request: Request<BlastChannelBalanceRequest>) -> Result<Response<BlastChannelBalanceResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn list_channels(&self, _request: Request<BlastListChannelsRequest>) -> Result<Response<BlastListChannelsResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn open_channel(&self, _request: Request<BlastOpenChannelRequest>) -> Result<Response<BlastOpenChannelResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn close_channel(&self, _request: Request<BlastCloseChannelRequest>) -> Result<Response<BlastCloseChannelResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn get_model_channels(&self, _request: Request<BlastGetModelChannelsRequest>) -> Result<Response<BlastGetModelChannelsResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn connect_peer(&self, _request: Request<BlastConnectRequest>) -> Result<Response<BlastConnectResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn disconnect_peer(&self, _request: Request<BlastDisconnectRequest>) -> Result<Response<BlastDisconnectResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn get_btc_address(&self, _request: Request<BlastBtcAddressRequest>) -> Result<Response<BlastBtcAddressResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
	}

	async fn get_listen_address(&self, _request: Request<BlastListenAddressRequest>) -> Result<Response<BlastListenAddressResponse>, Status> {
		Err(Status::new(Code::InvalidArgument, "name is invalid"))
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

    println!("Starting gRPC server at {}", addr);

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

	println!("Shutting down gRPC server at {}", addr);

    Ok(())
}

fn prepend_and_pad(input: &str, num: i32) -> String {
    format!("{}{:04}", input, num)
}