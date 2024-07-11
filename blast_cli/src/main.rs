// https://github.com/ratatui-org/ratatui
// https://github.com/ratatui-org/ratatui/blob/main/examples
// https://www.kammerl.de/ascii/AsciiSignature.php

use std::{error::Error, io};

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
        Text::from(format!("{}           <{}>", self.name, self.num_nodes))
    }
}

struct NewTab {
    models: StatefulList<Model>
}

struct LoadTab {
    sims: StatefulList<String>
}

struct ConfigureTab {
    input: String,
    character_index: usize,
    messages: Vec<String>,
}

impl ConfigureTab {
    fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            character_index: 0,
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

struct RunTab {
    message: String
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
            run: RunTab{message: "Simulation Running".to_string()},
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
    loop {
        // Draw the frame
        terminal.draw(|f| ui(f, &mut blast_cli))?;

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
}

fn process_new_event(cli: &mut BlastCli, key: KeyEvent) {
    match key.code {
        // Go back to the tab menu
        KeyCode::Esc => {
            cli.mode = Mode::Menu;
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
                let command = cli.config.input.clone();
                if command == "stop" {
                    // If the stop command is entered, stop the network and go back to the New page
                    // TODO: stop the network
                    cli.config.messages.clear();
                    cli.current_tab = Tab::New;
                } else if command == "start" {
                    // If the start command is entered, start the simulation and go to the Run page
                    // TODO: start the simulation
                    cli.config.messages.clear();
                    cli.current_tab = Tab::Run;
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
                cli.config.enter_char(to_insert);
            }
            KeyCode::Backspace => {
                cli.config.delete_char();
            }
            KeyCode::Left => {
                cli.config.move_cursor_left();
            }
            KeyCode::Right => {
                cli.config.move_cursor_right();
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
        Tab::Run => draw_run_tab(frame, &cli.run, chunks[1]),
    };
}

fn draw_new_tab(frame: &mut Frame, tab: &mut NewTab, area: Rect) {
    let layout1 = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(15), Constraint::Percentage(85)],
    )
    .split(area);

    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(25), Constraint::Percentage(5), Constraint::Percentage(70)],
    )
    .split(layout1[0]);

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
    let layout1 = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(15), Constraint::Percentage(85)],
    )
    .split(area);

    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(25), Constraint::Percentage(5), Constraint::Percentage(70)],
    )
    .split(layout1[0]);

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
    let layout1 = Layout::new(
        Direction::Horizontal,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    )
    .split(area);

    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(33)],
    ).split(layout1[1]);

    // TODO: these are placeholders, show the actual simulation data here
    frame.render_widget(
        Paragraph::new("OpenChannel         blast_lnd0000           blast_lnd0001           10s         2000msat").block(Block::bordered().title("Events")),
        layout[0],
    );
    frame.render_widget(
        Paragraph::new("0           blast_lnd0003           blast_cln0000           5000msat").block(Block::bordered().title("Channels")),
        layout[1],
    );
    frame.render_widget(
        Paragraph::new("blast_ldk0000           blast_lnd0004           2000msat            5s").block(Block::bordered().title("Activity")),
        layout[2],
    );

    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Min(1),
    ]);
    let [help_area, input_area, messages_area] = vertical.areas(layout1[0]);

    let msg = vec![
        "Use ".into(),
        "stop".bold(),
        " command to stop network, ".into(),
        "start".bold(),
        " command to start simulation.".into(),
    ];
    let text = Text::from(Line::from(msg)).patch_style(Style::default());
    let help_message = Paragraph::new(text);
    frame.render_widget(help_message, help_area);

    let input = Paragraph::new(cli.config.input.as_str())
        .style(Style::default().fg(Color::LightYellow))
        .block(Block::bordered().title("Command"));
        frame.render_widget(input, input_area);
    #[allow(clippy::cast_possible_truncation)]
    frame.set_cursor(
        input_area.x + cli.config.character_index as u16 + 1,
        input_area.y + 1,
    );

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

fn draw_run_tab(frame: &mut Frame, tab: &RunTab, area: Rect) {
    let layout = Layout::new(
        Direction::Vertical,
        [Constraint::Percentage(5), Constraint::Percentage(95)],
    )
    .split(area);

    let msg = vec![
        "Press ".into(),
        "q".bold(),
        " to exit, ".into(),
        "s".bold(),
        " to stop sim".into()
    ];
    let text = Text::from(Line::from(msg)).patch_style(Style::default());
    let help_message = Paragraph::new(text);
    frame.render_widget(help_message, layout[0]);

    // TODO: this is a placeholder, show the simulation status as it runs here
    frame.render_widget(
        Paragraph::new(tab.message.to_string()).block(Block::bordered().title("Status")),
        layout[1],
    );
}

