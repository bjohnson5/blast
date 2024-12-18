// TUI libraries
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    prelude::*,
    widgets::*,
};

// BLAST libraries
use crate::shared::*;

// The New Tab structure
pub struct NewTab {
    pub models: StatefulList<Model>
}

// The New Tab is a window that displays the available models and lets the user select the number of nodes for each
impl BlastTab for NewTab {
    /// Draw the tab
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::new(
            Direction::Vertical,
            [Constraint::Percentage(25), Constraint::Percentage(5), Constraint::Percentage(70)],
        )
        .split(area);

        // layout[0] The top of the window show the BLAST banner
        frame.render_widget(
            Paragraph::new(BANNER), layout[0]
        );

        // layout[1] The help message
        let msg = vec![
            "Press ".into(),
            "q".bold(),
            " to quit, ".into(),
            "enter".bold(),
            " to start network, ".into(),
            "esc".bold(),
            " to change tabs".into()
        ];
        let text = Text::from(Line::from(msg)).patch_style(Style::default());
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, layout[1]);

        // layout[2] The list of available models
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

    /// This is called when the new tab is first displayed
    fn init(&mut self) {
        self.models.next();
    }

    /// This is called when the new tab is closing
    fn close(&mut self) {
        self.models.clear();
    }

    /// This is called when a key is pressed while on the new tab
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
            // Start the network
            KeyCode::Enter => {
                return ProcessResult::StartNetwork(self.models.items.clone());
            }
            // Ignore all other keys
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

    fn quit_operation(&mut self) -> ProcessResult {
        ProcessResult::Quit
    }
}
