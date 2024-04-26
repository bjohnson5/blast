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
    let mut blast = Blast::new().expect("Could not create Blast");

    // TODO: configure the simulation: add nodes, models, channels, events, etc...

    let mut child = match blast.load_simulation(running.clone()).await {
        Ok(c) => c,
        Err(e) => {
            println!("{}", format!("Unable to load simulation: {}", e));
            return;
        }
    };

    // Example operations that the blast cli will need to do -- will eventually be cleaned up -- testing purposes only right now
    // TODO: add command line interface to let the user make these calls
    // -------------------------------------------------------------------------------------------------------------------------------

    // TODO: Add a call to list all nodes so that the user can choose which node name to use in these calls

    match blast.get_pub_key(String::from("blast-0000")).await {
        Ok(s) => {
            println!("PubKey Node 0000: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get pub key: {}", e));
        }
    }

    match blast.get_pub_key(String::from("blast-0001")).await {
        Ok(s) => {
            println!("PubKey Node 0001: {}", s);
        },
        Err(e) => {
            println!("{}", format!("Unable to get pub key: {}", e));
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

    match blast.fund_node(String::from("blast-0000")).await {
        Ok(_) => {},
        Err(e) => {
            println!("{}", format!("Unable to fund node: {}", e));
        }
    }

    match blast.fund_node(String::from("blast-0001")).await {
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

    match blast.open_channel(String::from("blast-0000"), String::from("blast-0001"), 30000, 0).await {
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

    // TODO: Add test call for close channel rpc
    // TODO: Add a close channel event at a certain time
    // TODO: Add an open channel event at a certain time

    // -------------------------------------------------------------------------------------------------------------------------------

    // Start the simulation
    let mut sim_tasks = JoinSet::new();
    let mut blast2 = blast.clone();
    sim_tasks.spawn(async move {
        match blast2.start_simulation().await {
            Ok(_) => {},
            Err(e) => {
                println!("Failed to start the simulation: {:?}", e);
                return;
            }
        }
    });

    // Wait for Ctrl+C signal to shutdown
    while running.load(Ordering::SeqCst) {
        thread::sleep(std::time::Duration::from_secs(1));
    }

    // Stop the blast simulation
    blast.stop_simulation();

    // Stop the models
    match blast.unload_simulation().await {
        Ok(_) => {},
        Err(e) => {
            println!("Failed to unload the simulation: {:?}", e);       
        }
    }

    // Wait for blast simulation to stop
    while let Some(res) = sim_tasks.join_next().await {
        if let Err(_) = res {
            println!("Error waiting for simulation to stop");
        }
    }

    // Wait for the models to stop
    let exit_status = match child.wait() {
        Ok(s) => Some(s),
        Err(e) => {
            println!("Failed to wait for child process: {:?}", e);
            None
        }
    };

    println!("Models process exited with status: {:?}", exit_status);
    println!("BLAST CLI shutting down...");
}
