use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use ctrlc;
use tokio::task::JoinSet;

use blast_core::Blast;

#[tokio::main]
async fn main() {
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
    let mut child = match blast.start_model(String::from("blast_lnd"), running.clone()).await {
        Ok(c) => {
            c
        },
        Err(e) => {
            println!("{}", format!("Unable to start the model: {}", e));
            return;
        }
    };

    // Call start_nodes on blast_lnd
    match blast.start_nodes(String::from("blast_lnd"), 2).await {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Unable to start nodes: {}", e));
            return;
        }
    }

    // Call get_pub_key on blast_lnd node 0
    match blast.get_pub_key(String::from("blast-0000")).await {
        Ok(s) => {
            println!("PubKey: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get pub key: {}", e));
        }
    }

    // Call list_peers on blast_lnd node 0
    match blast.list_peers(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Peers: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list peers: {}", e));
        }
    }

    let mut sim_tasks = JoinSet::new();

    // Start the simulation
    let mut blast2 = blast.clone();
    sim_tasks.spawn(async move {
        match blast2.start_simulation().await {
            Ok(_) => {},
            Err(e) => {
                println!("Failed to start the simulation: {:?}", e);
            }
        }
    });

    // -------------------------------------------------------------------------------------------------------------------------------

    // Wait for Ctrl+C signal to shutdown
    while running.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_secs(1));
    }

    // Stop the blast simulation
    blast.stop_simulation();

    // Wait for blast simulation to exit
    while let Some(res) = sim_tasks.join_next().await {
        if let Err(_) = res {
            println!("Error waiting for simulation to stop");
        }
    }

    // Wait for the models to exit... the models are responsible for handling ctrlc themselves
    let exit_status = child.wait().expect("failed to wait on child process");
    println!("Child process exited with status: {}", exit_status);
    println!("BLAST CLI shutting down...");
}
