// TUI libraries
use ratatui::{
    crossterm::event::KeyEvent,
    prelude::*,
    widgets::*,
};

// BLAST constants
pub const BANNER: &str = r"
   ____  _                _____ _______
  |  _ \| |        /\    / ____|__   __|
  | |_) | |       /  \  | (___    | |
  |  _ <| |      / /\ \  \___ \   | |
  | |_) | |____ / ____ \ ____) |  | |
  |____/|______/_/    \_\_____/   |_|   ";

pub const TAB_TITLES: [&'static str; 4] = ["New", "Load", "Configure", "Run"];

// A list that stores a selected state
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

// The return values for a tab when a key event is processed
pub enum ProcessResult {
    StartNetwork(Vec<Model>),
    LoadNetwork(String),
    StartSim,
    StopNetwork,
    StopSim,
    Command(String),
    ExitPage,
    Quit,
    NoOp,
}

// The TUI mode
// Menu: selecting the top level tabs
// Page: working on a particular tab
// Error: displaying an error
#[derive(PartialEq,Clone)]
pub enum Mode {
    Menu,
    Page,
    Error
}

// An available model within BLAST
#[derive(Clone)]
pub struct Model {
    pub name: String,
    pub num_nodes: i32
}

impl<'a> Into<Text<'a>> for Model {
    fn into(self) -> Text<'a> {
        Text::from(format!("{}           < {} >", self.name, self.num_nodes))
    }
}

// The trait that all tabs on the TUI must implement
pub trait BlastTab {
    fn draw(&mut self, frame: &mut Frame, area: Rect);
    fn init(&mut self);
    fn close(&mut self);
    fn process(&mut self, key: KeyEvent) -> ProcessResult;
    fn get_index(&self) -> usize;
    fn update_runtime_data(&mut self, events: Option<Vec<String>>, activity: Option<Vec<String>>, stats: Option<Vec<String>>, frame: u64, num_frames: u64, succes_rate: f64);
    fn update_config_data(&mut self, data1: Option<Vec<String>>, data2: Option<Vec<String>>, data3: Option<Vec<String>>, data4: Option<Vec<String>>);
    fn esc_operation(&mut self) -> ProcessResult;
    fn quit_operation(&mut self) -> ProcessResult;
}
