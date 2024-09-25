use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    prelude::*,
    widgets::*,
};

use crate::shared::*;

pub struct LoadTab {
    pub sims: StatefulList<String>
}

impl BlastTab for LoadTab {
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

    fn init(&mut self) {
        self.sims.next();
    }

    fn close(&mut self) {
        self.sims.clear();
    }

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
            KeyCode::Enter => {
                return ProcessResult::LoadNetwork(self.sims.items[self.sims.state.selected().unwrap()].clone());
            }
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
