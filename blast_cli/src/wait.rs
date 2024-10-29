// TUI libraries
use ratatui::{
    crossterm::event::KeyEvent,
    prelude::*,
    widgets::*,
};

// BLAST libraries
use crate::shared::*;

// The New Tab structure
pub struct WaitTab {
    pub message: String
}

// The Wait Tab is a window that displays some status while waiting for a task to complete
impl BlastTab for WaitTab {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Paragraph::new(self.message.as_str()).block(Block::bordered()),
            area,
        );
    }

    /// This is called when the wait tab is first displayed
    fn init(&mut self) {
        return;
    }

    /// This is called when the wait tab is closing
    fn close(&mut self) {
        return;
    }

    /// This is called when a key is pressed while on the wait tab
    fn process(&mut self, key: KeyEvent) -> ProcessResult {
        // Ignore all keys
        match key.code {
            _ => {}
        }

        return ProcessResult::NoOp;
    }

    fn get_index(&self) -> usize {
        4
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
        ProcessResult::NoOp
    }
}
