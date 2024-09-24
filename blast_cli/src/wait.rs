use ratatui::{
    crossterm::event::KeyEvent,
    prelude::*,
    widgets::*,
};

use crate::shared::*;

pub struct WaitTab {
    pub message: String
}

impl BlastTab for WaitTab {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Paragraph::new(self.message.as_str()).block(Block::bordered()),
            area,
        );
    }

    fn init(&mut self) {
        return;
    }

    fn close(&mut self) {
        return;
    }

    fn process(&mut self, key: KeyEvent) -> ProcessResult {
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
}
