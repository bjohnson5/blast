// TUI libraries
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    prelude::*,
    widgets::*,
};

// BLAST libraries
use crate::shared::*;

// The Load Tab structure
pub struct LoadTab {
    pub sims: StatefulList<String>
}

// The Load Tab is a window that displays the saved simulations and lets the user select one to load
impl BlastTab for LoadTab {
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
            " to exit, ".into(),
            "Enter".bold(),
            " to load simulation, ".into(),
            "Esc".bold(),
            " to change tabs".into()
        ];
        let text = Text::from(Line::from(msg)).patch_style(Style::default());
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, layout[1]);

        // layout[2] The list of simulations
        let tasks: Vec<ListItem> = self
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
        frame.render_stateful_widget(tasks, layout[2], &mut self.sims.state);
    }

    /// This is called when the load tab is first displayed
    fn init(&mut self) {
        self.sims.next();
    }

    /// This is called when the load tab is closing
    fn close(&mut self) {
        self.sims.clear();
    }

    /// This is called when a key is pressed while on the load tab
    fn process(&mut self, key: KeyEvent) -> ProcessResult {
        match key.code {
            // Scroll the list of simulations
            KeyCode::Down => {
                self.sims.next();
            }
            // Scroll the list of simulations
            KeyCode::Up => {
                self.sims.previous();
            }
            // Load the selected simulation
            KeyCode::Enter => {
                return ProcessResult::LoadNetwork(self.sims.items[self.sims.state.selected().unwrap()].clone());
            }
            // Ignore all other keys
            _ => {}
        }

        return ProcessResult::NoOp;
    }

    fn get_index(&self) -> usize {
        1
    }

    fn update_runtime_data(&mut self, _: Option<Vec<String>>, _: Option<Vec<String>>, _: Option<Vec<String>>, _: u64, _: u64, _: f64) {
        return;
    }

    fn update_config_data(&mut self, sims: Option<Vec<String>>, _: Option<Vec<String>>, _: Option<Vec<String>>, _: Option<Vec<String>>) {
        self.sims.items = sims.unwrap_or(Vec::new());
    }

    fn esc_operation(&mut self) -> ProcessResult {
        ProcessResult::ExitPage
    }

    fn quit_operation(&mut self) -> ProcessResult {
        ProcessResult::Quit
    }
}
