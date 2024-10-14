use ldk_node::Builder;
use ldk_node::bitcoin::Network;
use std::time::Duration;
use std::thread;

fn main() {
	let mut builder = Builder::new();
	builder.set_network(Network::Regtest);
	builder.set_esplora_server("https://blockstream.info/testnet/api".to_string());
	//builder.set_gossip_source_rgs("https://rapidsync.lightningdevkit.org/testnet/snapshot".to_string());
    builder.set_gossip_source_p2p();

	let node = builder.build().unwrap();
	
	node.start().unwrap();
    thread::sleep(Duration::from_secs(10));
	node.stop().unwrap();
}
