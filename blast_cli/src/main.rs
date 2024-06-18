use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::io::{stdin, stdout, Read, Write};
use std::time::Duration;
use std::thread;
use std::env;

use ctrlc;

use blast_core::Blast;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        load_simulation(args[1].clone()).await;
    } else {
        new_simulation().await;
    }
}

async fn new_simulation() {
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
    // create_network -- starts models and nodes OR load network
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
    m.insert(String::from("blast_lnd"), 10);
    let models = match blast.create_network("test", m, running.clone()).await {
        Ok(m) => m,
        Err(e) => {
            println!("{}", format!("Failed to start network: {}", e));
            return;
        }
    };

    // Example operations that the blast cli will need to do -- will eventually be cleaned up -- testing purposes only right now
    // TODO: Add command line interface to let the user make these calls
    // --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
    println!("----------------------------------------------- GET NETWORK INFO -----------------------------------------------");
    
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

    println!("----------------------------------------------- ADD ACTIVITY -----------------------------------------------");

    blast.add_activity("blast-0000", "blast-0001", 0, None, 1, 2000);
    
    println!("----------------------------------------------- ADD EVENTS -----------------------------------------------");

    let mut good_start = Vec::new();
    good_start.push(String::from("node1"));
    let mut bad_start = Vec::new();
    bad_start.push(String::from("node1"));
    bad_start.push(String::from("node2"));

    match blast.add_event(5, "StartNode", Some(bad_start.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(5, "StartNode", None) {
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
    match blast.add_event(5, "StopNode", Some(bad_start.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(5, "StopNode", None) {
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

    let mut bad_close = Vec::new();
    bad_close.push(String::from("node1"));
    let mut good_close = Vec::new();
    good_close.push(String::from("blast-0000"));
    good_close.push(String::from("0"));
    let mut bad_close1 = Vec::new();
    bad_close1.push(String::from("node1"));
    bad_close1.push(String::from("node2"));

    match blast.add_event(10, "CloseChannel", Some(bad_close.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(10, "CloseChannel", Some(bad_close1.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(10, "CloseChannel", None) {
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

    let mut bad_open = Vec::new();
    bad_open.push(String::from("node1"));
    let mut good_open = Vec::new();
    good_open.push(String::from("blast-0000"));
    good_open.push(String::from("blast-0001"));
    good_open.push(String::from("30000"));
    good_open.push(String::from("0"));
    good_open.push(String::from("0"));
    let mut bad_open1 = Vec::new();
    bad_open1.push(String::from("node1"));
    bad_open1.push(String::from("node2"));
    bad_open1.push(String::from("5000"));
    bad_open1.push(String::from("dfadfae"));

    match blast.add_event(23, "OpenChannel", Some(bad_open.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(23, "OpenChannel", Some(bad_open1.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(23, "OpenChannel", None) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }
    match blast.add_event(23, "OpenChannel", Some(good_open.clone())) {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error adding event: {}", e));
        }
    }

    match blast.save("test1", "/home/blast_sims").await {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error saving simulation: {}", e));
        }        
    }

    // --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

    // Finalize and run the simulation

    println!("----------------------------------------------- RUN SIMULATION -----------------------------------------------");

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

    // Pause and let the sim run until ENTER is pressed
    // Pressing ENTER instead of waiting for CtrlC allows the lnd nodes to stay alive
    // The lnd nodes are running as children and will process the INTERRUPT signal and shutdown
    pause();

    // Stop the activity and events

    println!("----------------------------------------------- STOP SIMULATION -----------------------------------------------");

    // Stop the blast simulation
    blast.stop_simulation();

    // Wait for blast simulation to stop
    while let Some(res) = sim_tasks.join_next().await {
        if let Err(_) = res {
            println!("Error waiting for simulation to stop");
        }
    }

    // Make changes to the network
    // TODO: Add command line interface to let the user make these calls
    // --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
    println!("----------------------------------------------- ADD MORE CHANNELS/ACTIVITY -----------------------------------------------");
    thread::sleep(Duration::from_secs(5));
    for i in 2..=6 {
        let param = format!("blast-000{}", i);
        println!("Opening channel from blast-0000 -> {}", param.clone());
        match blast.connect_peer(String::from("blast-0000"), param.clone()).await {
            Ok(_) => {},
            Err(e) => {
                println!("{}", format!("Unable to connect peers: {}", e));
            }
        }
        match blast.open_channel(String::from("blast-0000"), param.clone(), 30000, 0, i, true).await {
            Ok(_) => {},
            Err(e) => {
                println!("{}", format!("Unable to open channel: {}", e));
            }
        }

        blast.add_activity("blast-0000", &param.clone(), 0, None, 1, 2000);
    }

    match blast.list_peers(String::from("blast-0000")).await {
        Ok(s) => {
            println!("Peers Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to list peers: {}", e));
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

    match blast.save("test2", "/home/blast_sims").await {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Error saving simulation: {}", e));
        }        
    }
    // --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------

    // Run the simulation again

    println!("----------------------------------------------- RUN SIMULATION -----------------------------------------------");

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

    pause();

    // Stop the activity and events
    println!("----------------------------------------------- STOP SIMULATION -----------------------------------------------");

    // Stop the blast simulation
    blast.stop_simulation();

    // Wait for blast simulation to stop
    while let Some(res) = sim_tasks.join_next().await {
        if let Err(_) = res {
            println!("Error waiting for simulation to stop");
        }
    }

    for i in 2..=6 {
        let param = format!("blast-000{}", i);
        match blast.list_channels(param.clone()).await {
            Ok(s) => {
                println!("Channels Node {}: {}", param.clone(), s);
            },
            Err(e) => {
                println!("{}", format!("Unable to list channels: {}", e));
            }
        }

        match blast.list_peers(param.clone()).await {
            Ok(s) => {
                println!("Peers Node {}: {}", param.clone(), s);
            },
            Err(e) => {
                println!("{}", format!("Unable to list peers: {}", e));
            }
        }
    }

    // Pause and let the network still run until ENTER is pressed
    // Pressing ENTER instead of waiting for CtrlC allows the lnd nodes to be shutdown by a graceful RPC call and not the os signal
    // The lnd nodes are running as children and will process the INTERRUPT signal and shutdown
    pause();

    // Stop the network
    println!("----------------------------------------------- STOP NETWORK -----------------------------------------------");

    // Stop the models
    match blast.stop_network().await {
        Ok(_) => {},
        Err(e) => {
            println!("Failed to stop the network: {:?}", e);       
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

    running.store(false, Ordering::SeqCst);
    println!("BLAST CLI shutting down...");
}

async fn load_simulation(name: String) {
    println!("BLAST CLI starting up...");

    // Set up a Ctrl+C signal handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");

    // Create the blast core object
    let mut blast = Blast::new();

    let models = match blast.load(&name, "/home/blast_sims", running.clone()).await {
        Ok(m) => m,
        Err(e) => {
            println!("{}", format!("Failed to start network: {}", e));
            match blast.stop_network().await {
                Ok(_) => {},
                Err(e) => {
                    println!("Failed to stop the network: {:?}", e);       
                }
            }
            return
        }
    };

    println!("----------------------------------------------- GET NETWORK INFO -----------------------------------------------");

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

    match blast.wallet_balance(String::from("blast-0001")).await {
        Ok(s) => {
            println!("Wallet Balance Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get wallet balance: {}", e));
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

    println!("----------------------------------------------- RUN SIMULATION -----------------------------------------------");

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

    // Pause and let the sim run until ENTER is pressed
    // Pressing ENTER instead of waiting for CtrlC allows the lnd nodes to stay alive
    // The lnd nodes are running as children and will process the INTERRUPT signal and shutdown
    pause();

    println!("----------------------------------------------- STOP SIMULATION -----------------------------------------------");

    // Stop the blast simulation
    blast.stop_simulation();

    // Wait for blast simulation to stop
    while let Some(res) = sim_tasks.join_next().await {
        if let Err(_) = res {
            println!("Error waiting for simulation to stop");
        }
    }

    // Pause and let the network still run until ENTER is pressed
    // Pressing ENTER instead of waiting for CtrlC allows the lnd nodes to be shutdown by a graceful RPC call and not the os signal
    // The lnd nodes are running as children and will process the INTERRUPT signal and shutdown
    pause();

    // Stop the network
    println!("----------------------------------------------- STOP NETWORK -----------------------------------------------");

    // Stop the models
    match blast.stop_network().await {
        Ok(_) => {},
        Err(e) => {
            println!("Failed to stop the network: {:?}", e);       
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

    running.store(false, Ordering::SeqCst);
    println!("BLAST CLI shutting down...");
}

fn pause() {
    let mut stdout = stdout();
    stdout.write(b"Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    stdin().read(&mut [0]).unwrap();
}