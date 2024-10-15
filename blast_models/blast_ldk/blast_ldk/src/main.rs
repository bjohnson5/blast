use ldk_node::{Builder, LogLevel};
use ldk_node::bitcoin::Network;
use ldk_node::Config;

use std::time::Duration;
use std::thread;

fn main() {
	let config = Config {
		storage_dir_path: String::from("./blast_data"),
		log_dir_path: None,
		network: Network::Regtest,
		listening_addresses: None,
		default_cltv_expiry_delta: 0,
		onchain_wallet_sync_interval_secs: 2,
		wallet_sync_interval_secs: 2,
		fee_rate_cache_update_interval_secs: 2,
		trusted_peers_0conf: Vec::new(),
		probing_liquidity_limit_multiplier: 0,
		log_level: LogLevel::Debug,
		anchor_channels_config: None
	};

	let mut builder = Builder::from_config(config);
	builder.set_esplora_server("http://localhost:3002".to_string());
	//builder.set_gossip_source_rgs("https://rapidsync.lightningdevkit.org/testnet/snapshot".to_string());
    builder.set_gossip_source_p2p();

	let node = builder.build().unwrap();
	
	node.start().unwrap();
	println!("Node Status: {:?}", node.status());
    thread::sleep(Duration::from_secs(10));
	node.stop().unwrap();
	println!("Node Status: {:?}", node.status());
}
