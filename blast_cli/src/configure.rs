use ratatui::{
    crossterm::event::{KeyCode, KeyEvent, KeyEventKind},
    prelude::*,
    widgets::*,
};

use crate::shared::*;

#[derive(PartialEq,Clone)]
pub enum ConfigureSection {
    Command,
    Events,
    Channels,
    Activity
}

pub struct ConfigureTab {
    pub input: String,
    pub history: Vec<String>,
    pub history_index: usize,
    pub character_index: usize,
    pub messages: Vec<String>,
    pub events: StatefulList<String>,
    pub channels: StatefulList<String>,
    pub activity: StatefulList<String>,
    pub current_section: ConfigureSection
}

impl ConfigureTab {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            history: Vec::new(),
            history_index: 0,
            messages: Vec::new(),
            character_index: 0,
            events: StatefulList::with_items(Vec::new()),
            channels: StatefulList::with_items(Vec::new()),
            activity: StatefulList::with_items(Vec::new()),
            current_section: ConfigureSection::Command
        }
    }

    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    pub fn byte_index(&mut self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    pub fn delete_char(&mut self) {
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

    pub fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    pub fn reset_cursor(&mut self) {
        self.character_index = 0;
    }
}

impl BlastTab for ConfigureTab {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
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
    
        let e: Vec<ListItem> = self.events.items.clone().iter()
        .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
        let etasks = List::new(e)
        .block(Block::bordered().title("Events"))
        .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
        frame.render_stateful_widget(etasks, layout[0], &mut self.events.state);
    
        let c: Vec<ListItem> = self.channels.items.clone().iter()
        .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
        let ctasks = List::new(c)
        .block(Block::bordered().title("Channels"))
        .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
        frame.render_stateful_widget(ctasks, layout[1], &mut self.channels.state);
    
        let a: Vec<ListItem> = self.activity.items.clone().iter()
        .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
        let atasks = List::new(a)
        .block(Block::bordered().title("Activity"))
        .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
        frame.render_stateful_widget(atasks, layout[2], &mut self.activity.state);
    
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
            "help".bold(),
            " command to show commands,".into(),
            "Tab".bold(),
            " to change sections".into()
        ];
        let text = Text::from(Line::from(msg)).patch_style(Style::default());
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, l[0]);
    
        let input = Paragraph::new(self.input.as_str())
            .style(match self.current_section {
                ConfigureSection::Command => Style::default().fg(Color::LightYellow),
                _ => Style::default(),
            })
            .block(Block::bordered().title("Command"));
        frame.render_widget(input, input_area);
    
        match self.current_section {
            ConfigureSection::Command => {
                #[allow(clippy::cast_possible_truncation)]
                frame.set_cursor(
                    input_area.x + self.character_index as u16 + 1,
                    input_area.y + 1,
                );
            }
            _ => {}
        }
    
        let messages: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(_, m)| {
                let content = Line::from(Span::raw(format!("{m}")));
                ListItem::new(content)
            })
            .collect();
        let messages = List::new(messages).block(Block::bordered().title("Output"));
        frame.render_widget(messages, messages_area);
    }

    fn init(&mut self) {
        self.current_section = ConfigureSection::Command;
    }

    fn close(&mut self) {
        self.messages.clear();
        self.input.clear();
        self.events.clear();
        self.channels.clear();
        self.activity.clear();
        self.reset_cursor();
        self.history_index = 0;
    }

    fn process(&mut self, key: KeyEvent) -> ProcessResult {
        if key.kind == KeyEventKind::Press {
            match key.code {
                // If Enter is pressed while on the Config page, execute the current command
                KeyCode::Enter => {
                    if self.current_section != ConfigureSection::Command {
                        return ProcessResult::NoOp;
                    }
                    let command = self.input.clone();
                    if command == "stop" {
                        self.close();
                        return ProcessResult::StopNetwork;
                    } else if command == "start" {
                        self.close();
                        return ProcessResult::StartSim;
                    } else {
                        // Otherwise, run the command and show the output
                        self.history.push(command.clone());
                        self.history_index = self.history.len();
                        self.messages.clear();
                        self.input.clear();
                        self.reset_cursor();
                        return ProcessResult::Command(command.clone());
                    }
                }
                KeyCode::Char(to_insert) => {
                    if self.current_section != ConfigureSection::Command {
                        return ProcessResult::NoOp;
                    }
                    self.enter_char(to_insert);
                }
                KeyCode::Backspace => {
                    if self.current_section != ConfigureSection::Command {
                        return ProcessResult::NoOp;
                    }
                    self.delete_char();
                }
                KeyCode::Left => {
                    if self.current_section != ConfigureSection::Command {
                        return ProcessResult::NoOp;
                    }
                    self.move_cursor_left();
                }
                KeyCode::Right => {
                    if self.current_section != ConfigureSection::Command {
                        return ProcessResult::NoOp;
                    }
                    self.move_cursor_right();
                }
                KeyCode::Tab => {
                    match self.current_section {
                        ConfigureSection::Command => {
                            self.current_section = ConfigureSection::Events;
                            self.input.clear();
                            self.reset_cursor();
                            self.events.next();
                        }
                        ConfigureSection::Events => {
                            self.current_section = ConfigureSection::Channels;
                            self.events.clear();
                            self.channels.next();
                        }
                        ConfigureSection::Channels => {
                            self.current_section = ConfigureSection::Activity;
                            self.channels.clear();
                            self.activity.next();
                        }
                        ConfigureSection::Activity => {
                            self.current_section = ConfigureSection::Command;
                            self.activity.clear();
                        }
                    }
                }
                KeyCode::Up => {
                    match self.current_section {
                        ConfigureSection::Command => {
                            if self.history_index != 0 {
                                self.history_index -= 1;
                            }
                            self.input = self.history.get(self.history_index).unwrap_or(&String::from("")).to_string();
                            self.character_index = self.input.len();
                        }
                        ConfigureSection::Events => {
                            self.events.previous();
                        }
                        ConfigureSection::Channels => {
                            self.channels.previous();
                        }
                        ConfigureSection::Activity => {
                            self.activity.previous();
                        }
                    }
                }
                KeyCode::Down => {
                    match self.current_section {
                        ConfigureSection::Command => {
                            if self.history_index <= self.history.len() - 1 {
                                self.history_index += 1;
                                self.input = self.history.get(self.history_index).unwrap_or(&String::from("")).to_string();
                                self.character_index = self.input.len();
                            }
                        }
                        ConfigureSection::Events => {
                            self.events.next();
                        }
                        ConfigureSection::Channels => {
                            self.channels.next();
                        }
                        ConfigureSection::Activity => {
                            self.activity.next();
                        }
                    }
                }
                _ => {}
            }
        }

        return ProcessResult::NoOp;
    }

    fn get_index(&self) -> usize {
        2
    }

    fn update_runtime_data(&mut self, _: Option<Vec<String>>, _: Option<Vec<String>>, _: Option<Vec<String>>, _: u64, _: u64, _: f64) {
        return;
    }

    fn update_config_data(&mut self, data1: Option<Vec<String>>, data2: Option<Vec<String>>, data3: Option<Vec<String>>, data4: Option<Vec<String>>) {
        self.messages = data1.unwrap_or(Vec::new());
        self.events.items = data2.unwrap_or(Vec::new());
        self.channels.items = data3.unwrap_or(Vec::new());
        self.activity.items = data4.unwrap_or(Vec::new());
    }

    fn esc_operation(&mut self) -> ProcessResult {
        ProcessResult::NoOp
    }

    fn quit_operation(&mut self) -> ProcessResult {
        ProcessResult::NoOp
    }
}
