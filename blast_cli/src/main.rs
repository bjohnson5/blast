// https://github.com/ratatui-org/ratatui
// https://github.com/ratatui-org/ratatui/blob/main/examples
// https://www.kammerl.de/ascii/AsciiSignature.php

use std::{error::Error, io, time::Instant, time::Duration};

use ratatui::{
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    prelude::*,
    widgets::*,
};

const BANNER: &str = r"
   ____  _                _____ _______ 
  |  _ \| |        /\    / ____|__   __|
  | |_) | |       /  \  | (___    | |   
  |  _ <| |      / /\ \  \___ \   | |   
  | |_) | |____ / ____ \ ____) |  | |   
  |____/|______/_/    \_\_____/   |_|   ";

const TAB_TITLES: [&'static str; 4] = ["New", "Load", "Configure", "Run"];

pub struct StatefulList<T> {
    pub state: ListState,
    pub items: Vec<T>,
}

impl<T> StatefulList<T> {
    pub fn with_items(items: Vec<T>) -> Self {
        Self {
            state: ListState::default(),
            items,
        }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn clear(&mut self) {
        self.state.select(None);
    }
}

#[derive(PartialEq,Clone)]
enum Mode {
    Menu,
    Page
}

#[derive(PartialEq,Clone)]
enum Tab {
    New,
    Load,
    Configure,
    Run
}

impl Into<usize> for Tab {
    fn into(self) -> usize {
        match self {
            Tab::New => { 0 },
            Tab::Load => { 1 },
            Tab::Configure => { 2 },
            Tab::Run => { 3 },
        }
    }
}

#[derive(Clone)]
struct Model {
    name: String,
    num_nodes: u32
}

impl<'a> Into<Text<'a>> for Model {
    fn into(self) -> Text<'a> {
        Text::from(format!("{}           < {} >", self.name, self.num_nodes))
    }
}

struct NewTab {
    models: StatefulList<Model>
}

struct LoadTab {
    sims: StatefulList<String>
}

#[derive(PartialEq,Clone)]
enum ConfigureSection {
    Command,
    Events,
    Channels,
    Activity
}

struct ConfigureTab {
    input: String,
    character_index: usize,
    messages: Vec<String>,
    events: StatefulList<String>,
    channels: StatefulList<String>,
    activity: StatefulList<String>,
    current_section: ConfigureSection
}

impl ConfigureTab {
    fn new() -> Self {
        // TODO: this is a placeholder, initialize with the actual saved simulations
        let mut events_list: Vec<String> = Vec::new();
        let mut channel_list: Vec<String> = Vec::new();
        let mut activity_list: Vec<String> = Vec::new();
        events_list.push(String::from("10s OpenChannel (blast_lnd0000 --> blast_lnd0001: 2000msat)"));
        channel_list.push(String::from("0: blast_lnd0000 --> blast_lnd0001: 2000msat"));
        activity_list.push(String::from("blast_ldk0000 --> blast_lnd0004: 2000msat, 5s"));

        Self {
            input: String::new(),
            messages: Vec::new(),
            character_index: 0,
            events: StatefulList::with_items(events_list),
            channels: StatefulList::with_items(channel_list),
            activity: StatefulList::with_items(activity_list),
            current_section: ConfigureSection::Command
        }
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn byte_index(&mut self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }
}

enum RunSection {
    Events,
    Activity,
    Stats
}

struct RunTab {
    events: StatefulList<String>,
    activity: StatefulList<String>,
    stats: StatefulList<String>,
    current_section: RunSection,
    progress: f64,
    window: [f64; 2],
    success_rate_data: [(f64, f64); 21]
}

impl RunTab {
    fn new() -> Self {
        // TODO: this is a placeholder, initialize with the actual saved simulations
        let mut events_list: Vec<String> = Vec::new();
        let mut activity_list: Vec<String> = Vec::new();
        let mut stats_list: Vec<String> = Vec::new();
        events_list.push(String::from("10s OpenChannel (blast_lnd0000 --> blast_lnd0001: 2000msat)"));
        events_list.push(String::from("20s CloseChannel (0)"));
        activity_list.push(String::from("blast_ldk0000 --> blast_lnd0004: 2000msat, 5s"));
        activity_list.push(String::from("blast_ldk0001 --> blast_lnd0005: 1000msat, 15s"));
        activity_list.push(String::from("blast_ldk0002 --> blast_lnd0006: 8000msat, 10s"));
        activity_list.push(String::from("blast_ldk0003 --> blast_lnd0007: 5000msat, 25s"));
        stats_list.push(String::from("Number of Nodes:          15"));
        stats_list.push(String::from("Total Payment Attempts:   76"));
        stats_list.push(String::from("Payment Success Rate:     100%"));

        Self {
            events: StatefulList::with_items(events_list),
            activity: StatefulList::with_items(activity_list),
            stats: StatefulList::with_items(stats_list),
            current_section: RunSection::Events,
            progress: 0.0,
            window: [0.0, 20.0],
            success_rate_data: [(0.0, 0.0); 21]
        }
    }
}

struct BlastCli {
    new: NewTab,
    load: LoadTab,
    config: ConfigureTab,
    run: RunTab,
    current_tab: Tab,
    mode: Mode
}

impl BlastCli {
    fn new() -> Self {
        // TODO: this is a placeholder, initialize with the actual available models
        let mut model_list: Vec<Model> = Vec::new();
        model_list.push(Model{name: String::from("blast_lnd"), num_nodes: 0});
        model_list.push(Model{name: String::from("blast_ldk"), num_nodes: 0});
        model_list.push(Model{name: String::from("blast_cln"), num_nodes: 0});

        // TODO: this is a placeholder, initialize with the actual saved simulations
        let mut sim_list: Vec<String> = Vec::new();
        sim_list.push(String::from("Test Simulation 1"));
        sim_list.push(String::from("Another Test Simulation"));
        sim_list.push(String::from("Simulation3"));

        Self {
            new: NewTab{models: StatefulList::with_items(model_list)},
            load: LoadTab{sims: StatefulList::with_items(sim_list)},
            config: ConfigureTab::new(),
            run: RunTab::new(),
            current_tab: Tab::New,
            mode: Mode::Menu
        }
    }
}

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
    let mut add = true;

    loop {
        // Draw the frame
        terminal.draw(|f| ui(f, &mut blast_cli))?;

        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            // Get the next key event
            if let Event::Key(key) = event::read()? {
                // If Menu mode, allow the tabs to be selected
                if blast_cli.mode == Mode::Menu {
                    match key.code {
                        // Quit the BlastCli
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        // Select the current tab, so switch to Page mode
                        KeyCode::Enter => {
                            blast_cli.mode = Mode::Page;
                            match blast_cli.current_tab {
                                Tab::New => {
                                    blast_cli.new.models.next();
                                }
                                Tab::Load => {
                                    blast_cli.load.sims.next();
                                }
                                Tab::Configure => {}
                                Tab::Run => {}
                            }
                        }
                        // Choose a different tab
                        KeyCode::Left => {
                            if blast_cli.current_tab == Tab::Load {
                                blast_cli.current_tab = Tab::New
                            }
                        }
                        // Choose a different tab
                        KeyCode::Right => {
                            if blast_cli.current_tab == Tab::New {
                                blast_cli.current_tab = Tab::Load
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
                        // Pass the key event to the correct page
                        _ => {
                            match blast_cli.current_tab {
                                Tab::New => {
                                    process_new_event(&mut blast_cli, key);
                                }
                                Tab::Load => {
                                    process_load_event(&mut blast_cli, key);
                                }
                                Tab::Configure => {
                                    process_configure_event(&mut blast_cli, key);
                                }
                                Tab::Run => {
                                    process_run_event(&mut blast_cli, key);
                                }
                            }
                        }
                    }
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            // TODO: update data from blast core
            if blast_cli.current_tab == Tab::Run {
                blast_cli.run.progress = blast_cli.run.progress + 1.0;
                blast_cli.run.window[0] += 1.0;
                blast_cli.run.window[1] += 1.0;
                for i in 0..20 {
                    blast_cli.run.success_rate_data[i] = blast_cli.run.success_rate_data[i + 1];
                }

                if add {
                    blast_cli.run.success_rate_data[20] = (blast_cli.run.window[1], blast_cli.run.success_rate_data[19].1 + 5.0);
                } else {
                    blast_cli.run.success_rate_data[20] = (blast_cli.run.window[1], blast_cli.run.success_rate_data[19].1 - 5.0);
                }

                if blast_cli.run.window[1] % 10.0 == 0.0 {
                    add = !add;
                }
            }
            last_tick = Instant::now();
        }
    }
}

fn process_new_event(cli: &mut BlastCli, key: KeyEvent) {
    match key.code {
        // Go back to the tab menu
        KeyCode::Esc => {
            cli.mode = Mode::Menu;
            cli.new.models.clear();
        }
        // Scroll the list of models
        KeyCode::Down => {
            cli.new.models.next();
        }
        // Scroll the list of models
        KeyCode::Up => {
            cli.new.models.previous();
        }
        // Increase the selected model's node number
        KeyCode::Right => {
            if let Some(i) = cli.new.models.state.selected() {
                cli.new.models.items[i].num_nodes = cli.new.models.items[i].num_nodes + 1
            }
        }
        // Decrease the selected model's node number
        KeyCode::Left => {
            if let Some(i) = cli.new.models.state.selected() {
                let current_num = cli.new.models.items[i].num_nodes;
                if current_num > 0 {
                    cli.new.models.items[i].num_nodes = current_num - 1
                }
            }
        }
        // If Enter is pressed while on the New page, start the network and go to the Configure page
        KeyCode::Enter => {
            // TODO: start the network
            cli.current_tab = Tab::Configure;
        }
        _ => {}
    }
}

fn process_load_event(cli: &mut BlastCli, key: KeyEvent) {
    match key.code {
        // Go back to the tab menu
        KeyCode::Esc => {
            cli.mode = Mode::Menu;
            cli.load.sims.clear();
        }
        // Scroll the list of simulations
        KeyCode::Down => {
            cli.load.sims.next();
        }
        // Scroll the list of simulations
        KeyCode::Up => {
            cli.load.sims.previous();
        }
        // If Enter is pressed while on the Load page, load and start the network and go to the Configure page
        KeyCode::Enter => {
            // TODO: start the network
            cli.current_tab = Tab::Configure;
        }
        _ => {}
    }
}

fn process_configure_event(cli: &mut BlastCli, key: KeyEvent) {
    if key.kind == KeyEventKind::Press {
        match key.code {
            // If Enter is pressed while on the Config page, execute the current command
            KeyCode::Enter => {
                if cli.config.current_section != ConfigureSection::Command {
                    return;
                }
                let command = cli.config.input.clone();
                if command == "stop" {
                    // If the stop command is entered, stop the network and go back to the New page
                    // TODO: stop the network
                    cli.config.messages.clear();
                    cli.current_tab = Tab::New;
                    cli.mode = Mode::Menu;
                    cli.new.models.clear();
                    cli.load.sims.clear();
                } else if command == "start" {
                    // If the start command is entered, start the simulation and go to the Run page
                    // TODO: start the simulation
                    cli.config.messages.clear();
                    cli.current_tab = Tab::Run;
                    cli.run.events.next();
                } else {
                    // Otherwise, run the command and show the output
                    // TODO: execute the command and get the output
                    cli.config.messages.clear();
                    cli.config.messages.push(command.clone());
                }
                cli.config.input.clear();
                cli.config.reset_cursor();
            }
            KeyCode::Char(to_insert) => {
                if cli.config.current_section != ConfigureSection::Command {
                    return;
                }
                cli.config.enter_char(to_insert);
            }
            KeyCode::Backspace => {
                if cli.config.current_section != ConfigureSection::Command {
                    return;
                }
                cli.config.delete_char();
            }
            KeyCode::Left => {
                if cli.config.current_section != ConfigureSection::Command {
                    return;
                }
                cli.config.move_cursor_left();
            }
            KeyCode::Right => {
                if cli.config.current_section != ConfigureSection::Command {
                    return;
                }
                cli.config.move_cursor_right();
            }
            KeyCode::Tab => {
                match cli.config.current_section {
                    ConfigureSection::Command => {
                        cli.config.current_section = ConfigureSection::Events;
                        cli.config.input.clear();
                        cli.config.reset_cursor();
                        cli.config.events.next();
                    }
                    ConfigureSection::Events => {
                        cli.config.current_section = ConfigureSection::Channels;
                        cli.config.events.clear();
                        cli.config.channels.next();
                    }
                    ConfigureSection::Channels => {
                        cli.config.current_section = ConfigureSection::Activity;
                        cli.config.channels.clear();
                        cli.config.activity.next();
                    }
                    ConfigureSection::Activity => {
                        cli.config.current_section = ConfigureSection::Command;
                        cli.config.activity.clear();
                    }
                }
            }
            KeyCode::Up => {
                match cli.config.current_section {
                    ConfigureSection::Command => {}
                    ConfigureSection::Events => {
                        cli.config.events.previous();
                    }
                    ConfigureSection::Channels => {
                        cli.config.channels.previous();
                    }
                    ConfigureSection::Activity => {
                        cli.config.activity.previous();
                    }
                }
            }
            KeyCode::Down => {
                match cli.config.current_section {
                    ConfigureSection::Command => {}
                    ConfigureSection::Events => {
                        cli.config.events.next();
                    }
                    ConfigureSection::Channels => {
                        cli.config.channels.next();
                    }
                    ConfigureSection::Activity => {
                        cli.config.activity.next();
                    }
                }
            }
            _ => {}
        }
    }
}

fn process_run_event(cli: &mut BlastCli, key: KeyEvent) {
    match key.code {
        // The Run page is mainly readonly and will show the status of the running simulation, use `s` to stop the simulation and go back to the Configure page
        KeyCode::Char('s') => {
            // TODO: stop the simulation
            cli.current_tab = Tab::Configure;
            cli.run.events.clear();
            cli.run.activity.clear();
            cli.run.stats.clear();
            cli.run.events.next();
            cli.run.current_section = RunSection::Events;
            cli.run.progress = 0.0;
            cli.run.window[0] = 0.0;
            cli.run.window[1] = 20.0;
            cli.run.success_rate_data = [(0.0, 0.0); 21];
        }
        KeyCode::Tab => {
            match cli.run.current_section {
                RunSection::Events => {
                    cli.run.current_section = RunSection::Activity;
                    cli.run.events.clear();
                    cli.run.activity.next();
                }
                RunSection::Activity => {
                    cli.run.current_section = RunSection::Stats;
                    cli.run.activity.clear();
                    cli.run.stats.next();
                }
                RunSection::Stats => {
                    cli.run.current_section = RunSection::Events;
                    cli.run.stats.clear();
                    cli.run.events.next();
                }
            }
        }
        KeyCode::Up => {
            match cli.run.current_section {
                RunSection::Events => {
                    cli.run.events.previous();
                }
                RunSection::Activity => {
                    cli.run.activity.previous();
                }
                RunSection::Stats => {
                    cli.run.stats.previous();
                }
            }
        }
        KeyCode::Down => {
            match cli.run.current_section {
                RunSection::Events => {
                    cli.run.events.next();
                }
                RunSection::Activity => {
                    cli.run.activity.next();
                }
                RunSection::Stats => {
                    cli.run.stats.next();
                }
            }
        }
        _ => {}
    }
}

fn ui(frame: &mut Frame, cli: &mut BlastCli) {
    let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(frame.size());

    // Draw the tab menu
    let tabs = TAB_TITLES
        .iter()
        .map(|t| text::Line::from(Span::styled(*t, Style::default().fg(Color::LightBlue))))
        .collect::<Tabs>()
        .block(Block::bordered().title("BLAST"))
        .highlight_style(Style::default().fg(Color::LightYellow))
        .select(cli.current_tab.clone().into());
    frame.render_widget(tabs, chunks[0]);
    
    // Draw the current page
    match cli.current_tab {
        Tab::New => draw_new_tab(frame, &mut cli.new, chunks[1]),
        Tab::Load => draw_load_tab(frame, &mut cli.load, chunks[1]),
        Tab::Configure => draw_configure_tab(frame, cli, chunks[1]),
        Tab::Run => draw_run_tab(frame, cli, chunks[1]),
    };
}

fn draw_new_tab(frame: &mut Frame, tab: &mut NewTab, area: Rect) {
    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(25), Constraint::Percentage(5), Constraint::Percentage(70)],
    )
    .split(area);

    let msg = vec![
        "Press ".into(),
        "q".bold(),
        " to exit, ".into(),
        "Enter".bold(),
        " to start network, ".into(),
        "Esc".bold(),
        " to change tabs".into()
    ];
    let text = Text::from(Line::from(msg)).patch_style(Style::default());
    let help_message = Paragraph::new(text);
    frame.render_widget(help_message, layout[1]);

    frame.render_widget(
        Paragraph::new(BANNER), layout[0]
    );

    let tasks: Vec<ListItem> = tab
    .models
    .items
    .clone()
    .iter()
    .map(|i| ListItem::new(i.clone()))
    .collect();
    
    let tasks = List::new(tasks)
    .block(Block::bordered().title("Models"))
    .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
    .highlight_symbol("> ");
    frame.render_stateful_widget(tasks, layout[2], &mut tab.models.state);
}

fn draw_load_tab(frame: &mut Frame, tab: &mut LoadTab, area: Rect) {
    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(25), Constraint::Percentage(5), Constraint::Percentage(70)],
    )
    .split(area);

    let msg = vec![
        "Press ".into(),
        "q".bold(),
        " to exit, ".into(),
        "Enter".bold(),
        " to load simulation, ".into(),
        "Esc".bold(),
        " to change tabs".into()
    ];
    let text = Text::from(Line::from(msg)).patch_style(Style::default());
    let help_message = Paragraph::new(text);
    frame.render_widget(help_message, layout[1]);

    frame.render_widget(
        Paragraph::new(BANNER), layout[0]
    );

    let tasks: Vec<ListItem> = tab
    .sims
    .items
    .clone()
    .iter()
    .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))]))
    .collect();
    
    let tasks = List::new(tasks)
    .block(Block::bordered().title("Simulations"))
    .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
    .highlight_symbol("> ");
    frame.render_stateful_widget(tasks, layout[2], &mut tab.sims.state);
}

fn draw_configure_tab(frame: &mut Frame, cli: &mut BlastCli, area: Rect) {
    let l = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(5), Constraint::Percentage(95)],
    )
    .split(area);

    let layout1 = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(l[1]);

    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(33)],
    ).split(layout1[1]);

    let e: Vec<ListItem> = cli.config.events.items.clone().iter()
    .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
    let etasks = List::new(e)
    .block(Block::bordered().title("Events"))
    .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
    .highlight_symbol("> ");
    frame.render_stateful_widget(etasks, layout[0], &mut cli.config.events.state);

    let c: Vec<ListItem> = cli.config.channels.items.clone().iter()
    .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
    let ctasks = List::new(c)
    .block(Block::bordered().title("Channels"))
    .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
    .highlight_symbol("> ");
    frame.render_stateful_widget(ctasks, layout[1], &mut cli.config.channels.state);

    let a: Vec<ListItem> = cli.config.activity.items.clone().iter()
    .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
    let atasks = List::new(a)
    .block(Block::bordered().title("Activity"))
    .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
    .highlight_symbol("> ");
    frame.render_stateful_widget(atasks, layout[2], &mut cli.config.activity.state);

    let vertical = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
    ]);
    let [input_area, messages_area] = vertical.areas(layout1[0]);

    let msg = vec![
        "Use ".into(),
        "stop".bold(),
        " command to stop network, ".into(),
        "start".bold(),
        " command to start simulation, ".into(),
        "Tab".bold(),
        " to change sections".into()
    ];
    let text = Text::from(Line::from(msg)).patch_style(Style::default());
    let help_message = Paragraph::new(text);
    frame.render_widget(help_message, l[0]);

    let input = Paragraph::new(cli.config.input.as_str())
        .style(match cli.config.current_section {
            ConfigureSection::Command => Style::default().fg(Color::LightYellow),
            _ => Style::default(),
        })
        .block(Block::bordered().title("Command"));
    frame.render_widget(input, input_area);

    match cli.config.current_section {
        ConfigureSection::Command => {
            #[allow(clippy::cast_possible_truncation)]
            frame.set_cursor(
                input_area.x + cli.config.character_index as u16 + 1,
                input_area.y + 1,
            );
        }
        _ => {}
    }

    let messages: Vec<ListItem> = cli.config
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = Line::from(Span::raw(format!("{i}: {m}")));
            ListItem::new(content)
        })
        .collect();
    let messages = List::new(messages).block(Block::bordered().title("Output"));
    frame.render_widget(messages, messages_area);
}

fn draw_run_tab(frame: &mut Frame, cli: &mut BlastCli, area: Rect) {
    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(5), Constraint::Percentage(10), Constraint::Percentage(85)],
    )
    .split(area);

    let msg = vec![
        "Press ".into(),
        "q".bold(),
        " to exit, ".into(),
        "s".bold(),
        " to stop sim, ".into(),
        "Tab".bold(),
        " to change sections".into()
    ];
    let text = Text::from(Line::from(msg)).patch_style(Style::default());
    let help_message = Paragraph::new(text);
    frame.render_widget(help_message, layout[0]);

    let line_gauge = LineGauge::default()
        .block(Block::new().title("Simulation Progress:"))
        .filled_style(Style::default().fg(Color::LightBlue))
        .line_set(symbols::line::THICK)
        .ratio(cli.run.progress/100.0);
    frame.render_widget(line_gauge, layout[1]);

    let layout2 = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(layout[2]);

    let layout3 = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(33), Constraint::Percentage(50), Constraint::Percentage(33)],
    )
    .split(layout2[0]);

    let e: Vec<ListItem> = cli.run.events.items.clone().iter()
    .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
    let etasks = List::new(e)
    .block(Block::bordered().title("Events"))
    .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
    .highlight_symbol("> ");
    frame.render_stateful_widget(etasks, layout3[0], &mut cli.run.events.state);

    let a: Vec<ListItem> = cli.run.activity.items.clone().iter()
    .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
    let atasks = List::new(a)
    .block(Block::bordered().title("Activity"))
    .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
    .highlight_symbol("> ");
    frame.render_stateful_widget(atasks,  layout3[1], &mut cli.run.activity.state);

    let s: Vec<ListItem> = cli.run.stats.items.clone().iter()
    .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
    let stasks = List::new(s)
    .block(Block::bordered().title("Stats"))
    .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
    .highlight_symbol("> ");
    frame.render_stateful_widget(stasks, layout3[2], &mut cli.run.stats.state);

    let x_labels = vec![
        Span::styled(
            format!("{}", cli.run.window[0]),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(
            "{}",
            (cli.run.window[0] + cli.run.window[1]) / 2.0
        )),
        Span::styled(
            format!("{}", cli.run.window[1]),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];
    let datasets = vec![
        Dataset::default()
            .name("All")
            .marker(symbols::Marker::Dot)
            .style(Style::default().fg(Color::LightYellow))
            .data(&cli.run.success_rate_data),
    ];
    let chart = Chart::new(datasets)
        .block(
            Block::bordered().title(Span::styled(
                "Payment Chart",
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::BOLD),
            )),
        )
        .x_axis(
            Axis::default()
                .title("Sim Time")
                .style(Style::default().fg(Color::Gray))
                .bounds(cli.run.window)
                .labels(x_labels),
        )
        .y_axis(
            Axis::default()
                .title("Success Rate")
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, 100.0])
                .labels(vec![
                    Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw("50"),
                    Span::styled("100", Style::default().add_modifier(Modifier::BOLD)),
                ]),
        );
    frame.render_widget(chart, layout2[1]);
}
