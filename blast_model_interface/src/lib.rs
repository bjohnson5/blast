use blast_proto::example_client::ExampleClient;
use blast_proto::HelloRequest;
use tonic::transport::Channel;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::sleep;

// Import the generated proto-rust file into a module
pub mod blast_proto {
    tonic::include_proto!("blast_proto");
}

pub async fn list_peers(_node_id: String, running: Arc<AtomicBool>) -> Result<(), Box<dyn std::error::Error>> {
    // Setting up a test RPC call to the blast_lnd manager
    // This will eventually need to lookup the node_id to determine which RPC call to use (use the model.json file to define the calls for each model)

    let mut client: ExampleClient<Channel>;
    loop {
        match Channel::from_static("http://localhost:50051").connect().await {
            Ok(c) => {
                println!("Connected to server successfully!");
                client = ExampleClient::new(c);
                break;
            }
            Err(err) => {
                if !running.load(Ordering::SeqCst) {
                    return Ok(());
                }
                eprintln!("Failed to connect to server: {}", err);
                // Add some delay before retrying
                sleep(std::time::Duration::from_secs(1));
            }
        }        
    }
    

    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });
    let response = client.say_hello(request).await?;
    println!("RESPONSE={:?}", response);

    Ok(())
}
