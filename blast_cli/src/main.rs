// Standard libraries
use std::{error::Error, io, time::Instant, time::Duration};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::process::Child;
use std::fs::File;
use std::path::PathBuf;
use std::env;

// Extra Dependencies
use simplelog::WriteLogger;
use simplelog::Config;
use log::LevelFilter;
use tokio::task::JoinSet;
use anyhow::Error as AnyError;

// TUI libraries
use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::*,
    widgets::*,
};

// BLAST libraries
mod shared;
mod new;
mod load;
mod configure;
mod run;
mod wait;
mod blast_cli;
use crate::shared::*;
use crate::blast_cli::*;
use crate::wait::WaitTab;

/// The log file for blast
pub const BLAST_LOG_FILE: &str = ".blast/blast.log";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let home = env::var("HOME").expect("HOME environment variable not set");
    let folder_path = PathBuf::from(home).join(BLAST_LOG_FILE);
    std::fs::create_dir_all(folder_path.parent().unwrap()).unwrap();

    let _ = WriteLogger::init(
        LevelFilter::Info,
        Config::default(),
        File::create(folder_path).unwrap(),
    );

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let cli = BlastCli::new();
    let res = run(&mut terminal, cli).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

async fn run<B: Backend>(terminal: &mut Terminal<B>, mut blast_cli: BlastCli) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(1000);
    let mut current: &mut dyn BlastTab = &mut blast_cli.new;
    let mut mode: Mode = Mode::Menu;
    let mut error: Option<String> = None;
    let running = Arc::new(AtomicBool::new(true));
    let mut running_models: Vec<Child> = Vec::new();
    let mut sim_tasks: Option<JoinSet<Result<(), AnyError>>> = None;

    loop {
        // Draw the frame
        terminal.draw(|f| ui(f, current, error.clone()))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            // Get the next key event
            if let Event::Key(key) = event::read()? {
                // If Menu mode, allow the tabs to be selected
                if mode == Mode::Menu {
                    match key.code {
                        // Quit the BlastCli
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        // Select the current tab, so switch to Page mode
                        KeyCode::Enter => {
                            mode = Mode::Page;
                            current.init();
                        }
                        // Choose a different tab
                        KeyCode::Left => {
                            if current.get_index() == 1 {
                                current = &mut blast_cli.new;
                            }
                        }
                        // Choose a different tab
                        KeyCode::Right => {
                            if current.get_index() == 0 {
                                current = &mut blast_cli.load;
                                let mut sim_list: Vec<String> = Vec::new();
                                match blast_cli.blast.get_available_sims() {
                                    Ok(sims) => {
                                        for name in sims {
                                            sim_list.push(name);
                                        }
                                    },
                                    Err(_) => {}
                                }
                                current.update_config_data(Some(sim_list), None, None, None);
                            }
                        }
                        _ => {}
                    }
                } else if mode == Mode::Error {
                    match key.code {
                        KeyCode::Enter => {
                            mode = Mode::Page;
                            error = None;
                            current.init();
                        }
                        _ => {}
                    }
                } else {
                    // In Page mode, let the individual pages process the key event
                    match key.code {
                        // Quit the BlastCli
                        KeyCode::Char('q') => {
                            match current.quit_operation() {
                                ProcessResult::Quit => {
                                    return Ok(());
                                },
                                _ => {}
                            }
                        }
                        KeyCode::Esc => {
                            match current.esc_operation() {
                                ProcessResult::ExitPage => {
                                    mode = Mode::Menu;
                                    current.close();
                                },
                                _ => {}
                            }
                        }
                        // Pass the key event to the correct page
                        _ => {
                            match current.process(key) {
                                ProcessResult::StartNetwork(models) => {
                                    let mut w = WaitTab{message: String::from("Starting BLAST Network...")};
                                    terminal.draw(|f| ui(f, &mut w, None))?;

                                    running.store(true, Ordering::SeqCst);
                                    let mut m = HashMap::new();
                                    for model in models {
                                        if model.num_nodes > 0 {
                                            m.insert(model.name.clone(), model.num_nodes);
                                        }
                                    }
                                    match blast_cli.blast.create_network("test", m, running.clone()).await {
                                        Ok(mut m) => {
                                            running_models.append(&mut m);
                                            current.close();
                                            current = &mut blast_cli.config;
                                            let events_list: Vec<String> = blast_cli.blast.get_events();
                                            let channel_list: Vec<String> = blast_cli.blast.get_channels().await;
                                            let activity_list: Vec<String> = blast_cli.blast.get_activity();
                                            current.update_config_data(None, Some(events_list), Some(channel_list), Some(activity_list));
                                            current.init();
                                        },
                                        Err(e) => {
                                            error = Some(e);
                                            mode = Mode::Error;
                                        }
                                    };
                                },
                                ProcessResult::LoadNetwork(sim) => {
                                    let mut w = WaitTab{message: String::from("Loading BLAST Network...")};
                                    terminal.draw(|f| ui(f, &mut w, None))?;

                                    running.store(true, Ordering::SeqCst);
                                    match blast_cli.blast.load(&sim, running.clone()).await {
                                        Ok(mut m) => {
                                            running_models.append(&mut m);
                                            current.close();
                                            current = &mut blast_cli.config;
                                            let events_list: Vec<String> = blast_cli.blast.get_events();
                                            let channel_list: Vec<String> = blast_cli.blast.get_channels().await;
                                            let activity_list: Vec<String> = blast_cli.blast.get_activity();
                                            current.update_config_data(None, Some(events_list), Some(channel_list), Some(activity_list));
                                            current.init();
                                        },
                                        Err(e) => {
                                            error = Some(e);
                                            mode = Mode::Error;
                                        }
                                    };
                                }
                                ProcessResult::StartSim => {
                                    let mut w = WaitTab{message: String::from("Starting BLAST Simulation...")};
                                    terminal.draw(|f| ui(f, &mut w, None))?;

                                    let mut failed = false;
                                    let mut error_str = String::from("");
                                    // Finalize the simulation and make it ready to run
                                    match blast_cli.blast.finalize_simulation().await {
                                        Ok(_) => {},
                                        Err(e) => {
                                            failed = true;
                                            error_str = e;
                                        }        
                                    }

                                    // Start the simulation
                                    sim_tasks = match blast_cli.blast.start_simulation().await {
                                        Ok(j) => Some(j),
                                        Err(e) => {
                                            failed = true;
                                            error_str = e;
                                            None
                                        }
                                    };

                                    if failed {
                                        error = Some(format!("Failed to start the simulation: {:?}", error_str));
                                        mode = Mode::Error;
                                    } else {
                                        current.close();
                                        current = &mut blast_cli.run;
                                        current.init();
                                    }
                                },
                                ProcessResult::StopNetwork => {
                                    let mut w = WaitTab{message: String::from("Stopping BLAST Network...")};
                                    terminal.draw(|f| ui(f, &mut w, None))?;

                                    // Stop the models
                                    match blast_cli.blast.stop_network().await {
                                        Ok(_) => {},
                                        Err(e) => {
                                            error = Some(e);
                                            mode = Mode::Error;
                                            continue;
                                        }
                                    }

                                    // Wait for the models to stop
                                    let mut child_err = false;
                                    for child in &mut running_models {
                                        match child.wait() {
                                            Ok(_) => {},
                                            Err(_) => {
                                                child_err = true;
                                            }
                                        };
                                    }

                                    if child_err {
                                        error = Some(String::from("Failed to cleanly shutdown all models"));
                                        mode = Mode::Error;
                                    } else {
                                        running.store(false, Ordering::SeqCst);
                                        current.close();
                                        current = &mut blast_cli.new;
                                        mode = Mode::Menu;
                                        current.close();
                                        blast_cli.load.close();
                                    }
                                },
                                ProcessResult::StopSim => {
                                    let mut w = WaitTab{message: String::from("Stopping BLAST Simulation...")};
                                    terminal.draw(|f| ui(f, &mut w, None))?;

                                    // Stop the blast simulation
                                    blast_cli.blast.stop_simulation();

                                    let mut failed = false;
                                    let mut error_str = String::from("");

                                    match &mut sim_tasks {
                                        Some(t) => {
                                            // Wait for blast simulation to stop
                                            while let Some(res) = t.join_next().await {
                                                if let Err(_) = res {
                                                    failed = true;
                                                    error_str = String::from("Error waiting for simulation to stop");
                                                }
                                            }
                                        },
                                        None => {
                                            failed = true;
                                            error_str = String::from("No simulation tasks to stop");
                                        }
                                    }

                                    if failed {
                                        error = Some(format!("Failed to stop the simulation: {:?}", error_str));
                                        mode = Mode::Error;
                                    } else {
                                        current.close();
                                        current = &mut blast_cli.config;
                                        let events_list: Vec<String> = blast_cli.blast.get_events();
                                        let channel_list: Vec<String> = blast_cli.blast.get_channels().await;
                                        let activity_list: Vec<String> = blast_cli.blast.get_activity();
                                        current.update_config_data(None, Some(events_list), Some(channel_list), Some(activity_list));
                                        current.init();
                                    }
                                },
                                ProcessResult::Command(c) => {
                                    let output = run_command(&mut blast_cli.blast, c).await;
                                    let events_list: Vec<String> = blast_cli.blast.get_events();
                                    let channel_list: Vec<String> = blast_cli.blast.get_channels().await;
                                    let activity_list: Vec<String> = blast_cli.blast.get_activity();
                                    current.update_config_data(Some(output), Some(events_list), Some(channel_list), Some(activity_list));
                                },
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            let blast_stats = blast_cli.blast.get_stats().await;
            match blast_stats {
                Some(s) => {
                    let events_list: Vec<String> = blast_cli.blast.get_events();
                    let activity_list: Vec<String> = blast_cli.blast.get_activity();
                    current.update_runtime_data(Some(events_list), Some(activity_list), Some(s.stats), s.frame, s.total_frames, s.success_rate);
                },
                None => {}
            }
            last_tick = Instant::now();
        }
    }
}

fn ui(frame: &mut Frame, tab: &mut dyn BlastTab, error: Option<String>) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(frame.size());

    // Draw the tab menu
    let tabs = TAB_TITLES
        .iter()
        .map(|t| text::Line::from(Span::styled(*t, Style::default().fg(Color::LightBlue))))
        .collect::<Tabs>()
        .block(Block::bordered().title("BLAST"))
        .highlight_style(Style::default().fg(Color::LightYellow))
        .select(tab.get_index());
    frame.render_widget(tabs, chunks[0]);

    match error {
        Some(e) => {
            let error_msg = Paragraph::new(e.as_str())
            .block(Block::bordered().title("Error"));
            let area = centered_rect(60, 20, chunks[1]);
            frame.render_widget(Clear, area);
            frame.render_widget(error_msg, area);            
        },
        None => {
            // Draw the current page
            tab.draw(frame, chunks[1]);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

async fn run_command(blast: &mut blast_core::Blast, cmd: String) -> Vec<String> {
    let mut output: Vec<String> = Vec::new();
    let mut words = cmd.split_whitespace();

    if let Some(first_word) = words.next() {
        match first_word {
            "help" => {
                output.push(String::from("Command           Parameters "));
                output.push(String::from("-----------------------------"));
                output.push(String::from("save              simulation_name"));
                output.push(String::from("add_activity      source_node dest_node start_secs count interval amount"));
                output.push(String::from("add_event         frame event_type event_args ..."));
                output.push(String::from("get_nodes"));
                output.push(String::from("get_pub_key       node_name"));
                output.push(String::from("list_peers        node_name"));
                output.push(String::from("wallet_balance    node_name"));
                output.push(String::from("channel_balance   node_name"));
                output.push(String::from("list_channels     node_name"));
                output.push(String::from("open_channel      source_node dest_node amount push_amount channel_id"));
                output.push(String::from("close_channel     source_node channel_id"));
                output.push(String::from("connect_peer      source_node dest_node"));
                output.push(String::from("disconnect_peer   source_node dest_node"));
                output.push(String::from("fund_node         source_node amount_btc"));
            }
            "save" => {
                match blast.save(words.next().unwrap_or("simulation1")).await {
                    Ok(()) => {
                        output.push(String::from("Successfully saved simulation."));
                    },
                    Err(e) => {
                        output.push(e);
                    }
                }
            },
            "add_activity" => {
                let source = String::from(words.next().unwrap_or(""));
                let dest = String::from(words.next().unwrap_or(""));
                let start_secs = match words.next().unwrap_or("").parse::<u16>() {
                    Ok(value) => { Some(value) },
                    Err(_) => { None }
                };
                let count = match words.next().unwrap_or("").parse::<u64>() {
                    Ok(value) => { Some(value) },
                    Err(_) => { None }
                };

                let interval = match words.next().unwrap_or("").parse::<u16>() {
                    Ok(value) => { value },
                    Err(_) => { 10 } // TODO set a default and log it to the output
                };

                let amount = match words.next().unwrap_or("").parse::<u64>() {
                    Ok(value) => { value },
                    Err(_) => { 50000 } // TODO set a default and log it to the output
                };
                blast.add_activity(&source, &dest, start_secs, count, interval, amount);
                output.push(String::from("Successfully added activity."));
            },
            "add_event" => {
                let frame = match words.next().unwrap_or("").parse::<u64>() {
                    Ok(value) => { value },
                    Err(_) => { 10 } // TODO set a default and log it to the output
                };

                let event = String::from(words.next().unwrap_or(""));

                let mut event_args = Vec::new();
                while let Some(w) = words.next() {
                    event_args.push(String::from(w));
                }

                // TODO: easier use event command parameters
                let args = if event_args.len() == 0 { None } else { Some(event_args) };
                match blast.add_event(frame, &event, args) {
                    Ok(()) => {
                        output.push(String::from("Successfully added event."));
                    },
                    Err(e) => {
                        output.push(e);
                    }
                }
            },
            "get_nodes" => {
                output = blast.get_nodes();
            },
            "get_pub_key" => {
                match blast.get_pub_key(String::from(words.next().unwrap_or(""))).await {
                    Ok(s) => {
                        output.push(s);
                    },
                    Err(e) => {
                        output.push(e);
                    }
                }
            },
            "list_peers" => {
                match blast.list_peers(String::from(words.next().unwrap_or(""))).await {
                    Ok(s) => {
                        output.push(s);
                    },
                    Err(e) => {
                        output.push(e);
                    }
                }
            },
            "wallet_balance" => {
                match blast.wallet_balance(String::from(words.next().unwrap_or(""))).await {
                    Ok(s) => {
                        output.push(s);
                    },
                    Err(e) => {
                        output.push(e);
                    }
                }
            },
            "channel_balance" =>  {
                match blast.channel_balance(String::from(words.next().unwrap_or(""))).await {
                    Ok(s) => {
                        output.push(s);
                    },
                    Err(e) => {
                        output.push(e);
                    }
                }
            },
            "list_channels" =>  {
                match blast.list_channels(String::from(words.next().unwrap_or(""))).await {
                    Ok(s) => {
                        output.push(s);
                    },
                    Err(e) => {
                        output.push(e);
                    }
                }
            },
            "open_channel" => {
                let source = String::from(words.next().unwrap_or(""));
                let dest = String::from(words.next().unwrap_or(""));
                let amount = match words.next().unwrap_or("").parse::<i64>() {
                    Ok(value) => { value },
                    Err(_) => { 30000 } // TODO set a default and log it to the output
                };
                let push = match words.next().unwrap_or("").parse::<i64>() {
                    Ok(value) => { value },
                    Err(_) => { 0 } // TODO set a default and log it to the output
                };
                let chan_id = match words.next().unwrap_or("").parse::<i64>() {
                    Ok(value) => { value },
                    Err(_) => { 0 } // TODO set a default and log it to the output
                };

                match blast.open_channel(source, dest, amount, push, chan_id, true).await {
                    Ok(_) => {},
                    Err(e) => {
                        let msg = format!("Unable to open channel: {}", e);
                        output.push(msg);
                    }
                }
            },
            "close_channel" => {
                let source = String::from(words.next().unwrap_or(""));
                let chan_id = match words.next().unwrap_or("").parse::<i64>() {
                    Ok(value) => { value },
                    Err(_) => { 0 } // TODO set a default and log it to the output
                };

                match blast.close_channel(source, chan_id).await {
                    Ok(_) => {},
                    Err(e) => {
                        let msg = format!("Unable to open channel: {}", e);
                        output.push(msg);
                    }
                }                
            },
            "connect_peer" => {
                let source = String::from(words.next().unwrap_or(""));
                let dest = String::from(words.next().unwrap_or(""));
                match blast.connect_peer(source, dest).await {
                    Ok(_) => {},
                    Err(e) => {
                        let msg = format!("Unable to connect peers: {}", e);
                        output.push(msg);
                    }
                }
            },
            "disconnect_peer" => {
                let source = String::from(words.next().unwrap_or(""));
                let dest = String::from(words.next().unwrap_or(""));
                match blast.disconnect_peer(source, dest).await {
                    Ok(_) => {},
                    Err(e) => {
                        let msg = format!("Unable to disconnect peers: {}", e);
                        output.push(msg);
                    }
                }
            },
            "fund_node" => {
                let source = String::from(words.next().unwrap_or(""));
                let amount = match words.next().unwrap_or("1.0").parse::<f64>() {
                    Ok(value) => { value },
                    Err(_) => { 1.0 }
                };
                match blast.fund_node(source, amount, true).await {
                    Ok(_) => {},
                    Err(e) => {
                        let msg = format!("Unable to fund node: {}", e);
                        output.push(msg);
                    }
                }
            },
            _ => output.push(String::from("Unknown command")),
        }
    }

    output
}
