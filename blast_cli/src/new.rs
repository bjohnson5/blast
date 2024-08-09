use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    prelude::*,
    widgets::*,
};

use crate::shared::*;

pub struct NewTab {
    pub models: StatefulList<Model>
}

impl BlastTab for NewTab {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
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
    
        let tasks: Vec<ListItem> = self
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
        frame.render_stateful_widget(tasks, layout[2], &mut self.models.state);
    }

    fn init(&mut self) {
        self.models.next();
    }

    fn close(&mut self) {
        self.models.clear();
    }

    fn process(&mut self, key: KeyEvent) -> ProcessResult {
        match key.code {
            // Scroll the list of models
            KeyCode::Down => {
                self.models.next();
            }
            // Scroll the list of models
            KeyCode::Up => {
                self.models.previous();
            }
            // Increase the selected model's node number
            KeyCode::Right => {
                if let Some(i) = self.models.state.selected() {
                    self.models.items[i].num_nodes = self.models.items[i].num_nodes + 1
                }
            }
            // Decrease the selected model's node number
            KeyCode::Left => {
                if let Some(i) = self.models.state.selected() {
                    let current_num = self.models.items[i].num_nodes;
                    if current_num > 0 {
                        self.models.items[i].num_nodes = current_num - 1
                    }
                }
            }
            KeyCode::Enter => {
                return ProcessResult::StartNetwork(self.models.items.clone());
            }
            _ => {}
        }

        return ProcessResult::NoOp;
    }

    fn get_index(&self) -> usize {
        0
    }

    fn update_runtime_data(&mut self, _: Option<Vec<String>>, _: Option<Vec<String>>, _: Option<Vec<String>>, _: u64, _: u64, _: f64) {
        return;
    }

    fn update_config_data(&mut self, _: Option<Vec<String>>, _: Option<Vec<String>>, _: Option<Vec<String>>, _: Option<Vec<String>>) {
        return;
    }

    fn esc_operation(&mut self) -> ProcessResult {
        ProcessResult::ExitPage
    }
}
