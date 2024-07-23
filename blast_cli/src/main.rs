// https://github.com/ratatui-org/ratatui
// https://github.com/ratatui-org/ratatui/blob/main/examples
// https://www.kammerl.de/ascii/AsciiSignature.php

use std::{error::Error, io, time::Instant, time::Duration};

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

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let cli = BlastCli::new();
    let res = run(&mut terminal, cli);

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

fn run<B: Backend>(terminal: &mut Terminal<B>, mut blast_cli: BlastCli) -> io::Result<()> {
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(1000);
    let mut current: &mut dyn BlastTab = &mut blast_cli.new;
    let mut mode: Mode = Mode::Menu;

    loop {
        // Draw the frameclear
        terminal.draw(|f| ui(f, current))?;

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
                                ProcessResult::StartNetwork => {
                                    // TODO: start the network
                                    current.close();
                                    current = &mut blast_cli.config;
                                    current.init();
                                },
                                ProcessResult::StartSim => {
                                    // TODO: start the simulation
                                    current.close();
                                    current = &mut blast_cli.run;
                                    current.init();
                                },
                                ProcessResult::StopNetwork => {
                                    // TODO: stop the network
                                    current.close();
                                    current = &mut blast_cli.new;
                                    mode = Mode::Menu;
                                    current.close();
                                    blast_cli.load.close();
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

fn ui(frame: &mut Frame, tab: &mut dyn BlastTab) {
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
    
    // Draw the current page
    tab.draw(frame, chunks[1]);
}
