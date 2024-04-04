use blast_core;
use blast_model_interface;
use sim_lib::ActivityDefinition;
use sim_lib::Simulation;
use sim_lib::LightningNode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use ctrlc;
use bitcoin::secp256k1::PublicKey;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("BLAST CLI starting up...");

    // Set up a Ctrl+C signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Example operations that the blast cli will need to do -- will eventually be cleaned up -- testing purposes only right now
    // -------------------------------------------------------------------------------------------------------------------------------
    // Create and start blast_lnd nodes (10) -- use blast_core library
    let mut child = blast_core::create_blast_lnd_nodes(5).unwrap();

    // Call list_peers on blast_lnd node 5 -- use blast_model_interface
    let _ = blast_model_interface::list_peers(String::from("lnd0005"), running.clone()).await;

    // Run SimLn
    let clients: HashMap<PublicKey, Arc<Mutex<dyn LightningNode>>> = HashMap::new();
    let validated_activities: Vec<ActivityDefinition> = Vec::new();
    let sim = Simulation::new(
        clients,
        validated_activities,
        None,
        1,
        0.1,
        None,
    );
    //sim.run().await?;
    // -------------------------------------------------------------------------------------------------------------------------------

    // Shutdown
    sim.shutdown();

    // Wait for Ctrl+C signal to shutdown
    while running.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_secs(1));
    }

    let exit_status = child.wait().expect("failed to wait on child process");

    println!("Child process exited with status: {}", exit_status);

    println!("BLAST CLI shutting down...");

    Ok(())
}
