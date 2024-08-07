use ratatui::{
    crossterm::event::KeyEvent,
    prelude::*,
    widgets::*,
};

pub const BANNER: &str = r"
   ____  _                _____ _______ 
  |  _ \| |        /\    / ____|__   __|
  | |_) | |       /  \  | (___    | |   
  |  _ <| |      / /\ \  \___ \   | |   
  | |_) | |____ / ____ \ ____) |  | |   
  |____/|______/_/    \_\_____/   |_|   ";

pub const TAB_TITLES: [&'static str; 4] = ["New", "Load", "Configure", "Run"];

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

pub enum ProcessResult {
    StartNetwork(Vec<Model>),
    LoadNetwork(String),
    StartSim,
    StopNetwork,
    StopSim,
    Command(String),
    ExitPage,
    NoOp,
}

#[derive(PartialEq,Clone)]
pub enum Mode {
    Menu,
    Page,
    Error
}

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

pub trait BlastTab {
    fn draw(&mut self, frame: &mut Frame, area: Rect);
    fn init(&mut self);
    fn close(&mut self);
    fn process(&mut self, key: KeyEvent) -> ProcessResult;
    fn get_index(&self) -> usize;
    fn update_runtime_data(&mut self);
    fn update_config_data(&mut self, data1: Option<Vec<String>>, data2: Option<Vec<String>>, data3: Option<Vec<String>>, data4: Option<Vec<String>>);
    fn esc_operation(&mut self) -> ProcessResult;
}
