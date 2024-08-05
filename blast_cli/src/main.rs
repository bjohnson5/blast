use std::{error::Error, io, time::Instant, time::Duration};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::process::Child;
use std::fs::File;

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
pub const BLAST_LOG_FILE: &str = "/home/blast.log";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = WriteLogger::init(
        LevelFilter::Info,
        Config::default(),
        File::create(BLAST_LOG_FILE).unwrap(),
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
                            if current.is_load() {
                                current = &mut blast_cli.new;
                            }
                        }
                        // Choose a different tab
                        KeyCode::Right => {
                            if current.is_new() {
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
                            if current.is_new() || current.is_load() {
                                mode = Mode::Menu;
                                current.close();
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
                                ProcessResult::NoOp => {}
                            }
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            current.update_data();
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
