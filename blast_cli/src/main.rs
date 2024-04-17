use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use ctrlc;

use blast_core::Blast;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("BLAST CLI starting up...");

    // Set up a Ctrl+C signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Create the blast core object
    let mut blast = Blast::new();

    // Example operations that the blast cli will need to do -- will eventually be cleaned up -- testing purposes only right now
    // -------------------------------------------------------------------------------------------------------------------------------

    // Create and start blast_lnd nodes (2) -- use blast_core library
    let mut child = blast.start_model(String::from("blast_lnd"), running.clone()).await.unwrap();

    // Call start_nodes on blast_lnd
    blast.start_nodes(String::from("blast_lnd"), 2).await;

    // Call get_pub_key on blast_lnd node 0
    blast.get_pub_key(String::from("blast-0000")).await;

    // Call list_peers on blast_lnd node 0
    blast.list_peers(String::from("blast-0000")).await;

    // Start the simulation
    blast.start_simulation();

    // -------------------------------------------------------------------------------------------------------------------------------

    // Wait for Ctrl+C signal to shutdown
    while running.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_secs(1));
    }

    // Stop the blast simulation
    blast.stop_simulation();

    // Wait for the models to exit... the models are responsible for handling ctrlc themselves
    let exit_status = child.wait().expect("failed to wait on child process");
    println!("Child process exited with status: {}", exit_status);

    println!("BLAST CLI shutting down...");
    Ok(())
}
