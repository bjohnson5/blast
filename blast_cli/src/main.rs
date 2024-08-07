use std::{error::Error, io, time::Instant, time::Duration};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::process::Child;
use std::fs::File;
use std::path::PathBuf;
use std::env;

use simplelog::WriteLogger;
use simplelog::Config;
use log::LevelFilter;

use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::*,
    widgets::*,
};

mod shared;
mod new;
mod load;
mod configure;
mod run;
mod blast_cli;
use crate::shared::*;
use crate::blast_cli::*;

/// The log file for blast
pub const BLAST_LOG_FILE: &str = ".blast/blast.log";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let home = env::var("HOME").expect("HOME environment variable not set");
    let folder_path = PathBuf::from(home).join(BLAST_LOG_FILE);
    std::fs::create_dir_all(folder_path.clone()).unwrap();

    let _ = WriteLogger::init(
        LevelFilter::Info,
        Config::default(),
        File::create(folder_path).unwrap(),
    );

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let cli = BlastCli::new();
    let res = run(&mut terminal, cli).await;

    // restore terminal
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

    loop {
        // Draw the frameclear
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
                            return Ok(());
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
                                    let mut m = HashMap::new();
                                    for model in models {
                                        m.insert(model.name.clone(), model.num_nodes);
                                    }
                                    match blast_cli.blast.create_network("test", m, running.clone()).await {
                                        Ok(mut m) => {
                                            running_models.append(&mut m);
                                            current.close();
                                            current = &mut blast_cli.config;
                                            current.init();
                                        },
                                        Err(e) => {
                                            error = Some(e);
                                            mode = Mode::Error;
                                        }
                                    };
                                },
                                ProcessResult::LoadNetwork(sim) => {
                                    match blast_cli.blast.load(&sim, running.clone()).await {
                                        Ok(mut m) => {
                                            running_models.append(&mut m);
                                            current.close();
                                            current = &mut blast_cli.config;
                                            current.init();
                                        },
                                        Err(e) => {
                                            error = Some(e);
                                            mode = Mode::Error;
                                        }
                                    };                                    
                                }
                                ProcessResult::StartSim => {
                                    // TODO: start the simulation
                                    current.close();
                                    current = &mut blast_cli.run;
                                    current.init();
                                },
                                ProcessResult::StopNetwork => {
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
                                    // TODO: stop the simulation
                                    current.close();
                                    current = &mut blast_cli.config;
                                    current.init();
                                },
                                ProcessResult::Command(c) => {
                                    // use c in blast_cli.blast call for that command
                                    let output = run_command(&mut blast_cli.blast, c).await;
                                    let events_list: Vec<String> = Vec::new();
                                    let channel_list: Vec<String> = Vec::new();
                                    let activity_list: Vec<String> = Vec::new();
                                    current.update_config_data(output, activity_list, channel_list, events_list);
                                }
                                ProcessResult::NoOp => {},
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            current.update_runtime_data();
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
            "add_activity" => output.push(String::from(first_word)),
            "add_event" => output.push(String::from(first_word)),
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
            "open_channel" => output.push(String::from(first_word)),
            "close_channel" => output.push(String::from(first_word)),
            "connect_peer" => output.push(String::from(first_word)),
            "disconnect_peer" => output.push(String::from(first_word)),
            "fund_node" => output.push(String::from(first_word)),
            _ => output.push(String::from("Unknown command")),
        }
    }

    output
}