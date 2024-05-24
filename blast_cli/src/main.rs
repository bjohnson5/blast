use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use ctrlc;

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

    // Control Flow: 
    // create_network -- creates the BlastNetwork which contains the models needed and the number of nodes per model
    // start_network -- starts models and nodes
    // ** user can now add activity, events, interact with nodes, connect outside nodes, etc...
    // finalize_simulation -- gets the simulation ready to be run
    // start_simulation -- runs events/activity
    // stop_simulation -- stops events/activity
    // ** user can now add activity, events, interact with nodes, connect outside nodes, etc...
    // finalize_simulation -- gets the simulation ready to be run
    // start_simulation -- runs events/activity
    // stop_simulation -- stops events/activity
    // stop_network -- stops models and nodes
    // exit

    // Create the network
    let mut m = HashMap::new();
    m.insert(String::from("blast_lnd"), 2);
    blast.create_network("test", m);
    // OR blast.load()

    // Start the network
    let models = match blast.start_network(running.clone()).await {
        Ok(m) => m,
        Err(e) => {
            println!("{}", format!("Failed to start network: {}", e));
            return;
        }
    };

    // Example operations that the blast cli will need to do -- will eventually be cleaned up -- testing purposes only right now
    // Add command line interface to let the user make these calls
    // -------------------------------------------------------------------------------------------------------------------------------
    
    for node_id in blast.get_nodes() {
        match blast.get_pub_key(node_id.clone()).await {
            Ok(s) => {
                println!("PubKey Node {}: {}", node_id, s);
            },
            Err(e) => {
                println!("{}", format!("Unable to get pub key: {}", e));
            }
        }
    }

    match blast.list_peers(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Peers Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list peers: {}", e));
        }
    }

    match blast.list_peers(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Peers Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list peers: {}", e));
        }
    }

    match blast.wallet_balance(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Wallet Balance Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
        }
    }

    match blast.wallet_balance(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Wallet Balance Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
        }
    }

    match blast.channel_balance(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Channel Balance Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
        }
    }

    match blast.channel_balance(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Channel Balance Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
        }
    }

    match blast.list_channels(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Channels Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list peers: {}", e));
        }
    }

    match blast.list_channels(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Channels Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list peers: {}", e));
        }
    }

    println!("----------------------------------------------- FUND / CONNECT NODES -----------------------------------------------");

    match blast.fund_node(String::from("blast-0000"), true).await {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Unable to fund node: {}", e));
        }
    }

    match blast.fund_node(String::from("blast-0001"), true).await {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Unable to fund node: {}", e));
        }
    }

    match blast.connect_peer(String::from("blast-0000"), String::from("blast-0001")).await {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Unable to connect peers: {}", e));
        }
    }

    match blast.list_peers(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Peers Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list peers: {}", e));
        }
    }

    match blast.list_peers(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Peers Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list peers: {}", e));
        }
    }

    match blast.wallet_balance(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Wallet Balance Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
        }
    }

    match blast.wallet_balance(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Wallet Balance Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
        }
    }

    match blast.channel_balance(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Channel Balance Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
        }
    }

    match blast.channel_balance(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Channel Balance Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
        }
    }

    println!("----------------------------------------------- OPEN CHANNEL -----------------------------------------------");

    match blast.open_channel(String::from("blast-0000"), String::from("blast-0001"), 30000, 0, 0, true).await {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Unable to open channel: {}", e));
        }
    }

    match blast.list_channels(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Channels Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list channels: {}", e));
        }
    }

    match blast.list_channels(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Channels Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list channels: {}", e));
        }
    }

    match blast.channel_balance(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Channel Balance Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
        }
    }

    match blast.channel_balance(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Channel Balance Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
        }
    }

    blast.add_activity("blast-0000", "blast-0001", 0, None, 1, 2000);
    
    let mut good_start = Vec::new();
    good_start.push(String::from("node1"));
    let mut bad_start = Vec::new();
    bad_start.push(String::from("node1"));
    bad_start.push(String::from("node2"));

    match blast.add_event(15, "StartNode", Some(bad_start.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(15, "StartNode", None) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(5, "StartNode", Some(good_start.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(15, "StopNode", Some(bad_start.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(15, "StopNode", None) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(5, "StopNode", Some(good_start.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }

    let mut bad_open = Vec::new();
    bad_open.push(String::from("node1"));
    //let mut good_open = Vec::new();
    //good_open.push(String::from("node1"));
    //good_open.push(String::from("node2"));
    //good_open.push(String::from("5000"));
    //good_open.push(String::from("0"));
    let mut bad_open1 = Vec::new();
    bad_open1.push(String::from("node1"));
    bad_open1.push(String::from("node2"));
    bad_open1.push(String::from("5000"));
    bad_open1.push(String::from("dfadfae"));

    match blast.add_event(20, "OpenChannel", Some(bad_open.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(20, "OpenChannel", Some(bad_open1.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(20, "OpenChannel", None) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    //match blast.add_event(20, "OpenChannel", Some(good_open.clone())) {
    //    Ok(_) => {},
    //    Err(e) => {
    //        println!("{}", format!("Error adding event: {}", e));
    //    }
    //}

    let mut bad_close = Vec::new();
    bad_close.push(String::from("node1"));
    let mut good_close = Vec::new();
    good_close.push(String::from("blast-0000"));
    good_close.push(String::from("0"));
    let mut bad_close1 = Vec::new();
    bad_close1.push(String::from("node1"));
    bad_close1.push(String::from("node2"));

    match blast.add_event(20, "CloseChannel", Some(bad_close.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(20, "CloseChannel", Some(bad_close1.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(20, "CloseChannel", None) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(10, "CloseChannel", Some(good_close.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }

    //blast.save();

    // -------------------------------------------------------------------------------------------------------------------------------

    // Finalize the simulation and make it ready to run
    match blast.finalize_simulation().await {
        Ok(_) => {},
        Err(e) => {
            println!("Failed to finalize the simulation: {:?}", e);
            return;
        }        
    }

    // Start the simulation
    let mut sim_tasks = match blast.start_simulation().await {
        Ok(j) => j,
        Err(e) => {
            println!("Failed to start the simulation: {:?}", e);
            return;
        }
    };

    // Wait for Ctrl+C signal to shutdown
    while running.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_secs(1));
    }

    // Stop the blast simulation
    blast.stop_simulation();

    // --------------------------------------- Perform more queries of the network, reconfigure network, events, payment activity and then could run again ---------------------------------------------
    // TODO: make this work. currently lnd shutsdown on Ctrlc so the RPC calls fail here. need to keep lnd alive until we call stop_network
    match blast.list_channels(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Channels Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list channels: {}", e));
        }
    }

    match blast.list_peers(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Peers Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list peers: {}", e));
        }
    }
    // --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

    // Stop the models
    match blast.stop_network().await {
        Ok(_) => {},
        Err(e) => {
            println!("Failed to stop the network: {:?}", e);       
        }
    }

    // Wait for blast simulation to stop
    while let Some(res) = sim_tasks.join_next().await {
        if let Err(_) = res {
            println!("Error waiting for simulation to stop");
        }
    }

    // Wait for the models to stop
    for mut child in models {
        let exit_status = match child.wait() {
            Ok(s) => Some(s),
            Err(e) => {
                println!("Failed to wait for child process: {:?}", e);
                None
            }
        };
        println!("Model process exited with status: {:?}", exit_status);
    }

    println!("BLAST CLI shutting down...");
}
