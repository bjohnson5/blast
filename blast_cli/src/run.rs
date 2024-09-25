use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    prelude::*,
    widgets::*,
};

use crate::shared::*;

pub enum RunSection {
    Events,
    Activity,
    Stats
}

pub struct RunTab {
    pub events: StatefulList<String>,
    pub activity: StatefulList<String>,
    pub stats: StatefulList<String>,
    pub current_section: RunSection,
    pub progress: f64,
    pub window: [f64; 2],
    pub success_rate_data: [(f64, f64); 21],
    pub points: usize
}

impl RunTab {
    pub fn new() -> Self {
        Self {
            events: StatefulList::with_items(Vec::new()),
            activity: StatefulList::with_items(Vec::new()),
            stats: StatefulList::with_items(Vec::new()),
            current_section: RunSection::Events,
            progress: 0.0,
            window: [0.0, 20.0],
            success_rate_data: [(0.0, 0.0); 21],
            points: 0
        }
    }
}

impl BlastTab for RunTab {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::new(
            Direction::Vertical,
            [Constraint::Percentage(5), Constraint::Percentage(10), Constraint::Percentage(85)],
        )
        .split(area);
    
        let msg = vec![
            "Press ".into(),
            "s".bold(),
            " to stop sim, ".into(),
            "Tab".bold(),
            " to change sections".into()
        ];
        let text = Text::from(Line::from(msg)).patch_style(Style::default());
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, layout[0]);
    
        let line_gauge = LineGauge::default()
            .block(Block::new().title("Simulation Progress:"))
            .filled_style(Style::default().fg(Color::LightBlue))
            .line_set(symbols::line::THICK)
            .ratio(self.progress);
        frame.render_widget(line_gauge, layout[1]);
    
        let layout2 = Layout::new(
            Direction::Horizontal,
            [Constraint::Percentage(50), Constraint::Percentage(50)],
        )
        .split(layout[2]);
    
        let layout3 = Layout::new(
            Direction::Vertical,
            [Constraint::Percentage(33), Constraint::Percentage(50), Constraint::Percentage(33)],
        )
        .split(layout2[0]);
    
        let e: Vec<ListItem> = self.events.items.clone().iter()
        .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
        let etasks = List::new(e)
        .block(Block::bordered().title("Events"))
        .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
        frame.render_stateful_widget(etasks, layout3[0], &mut self.events.state);
    
        let a: Vec<ListItem> = self.activity.items.clone().iter()
        .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
        let atasks = List::new(a)
        .block(Block::bordered().title("Activity"))
        .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
        frame.render_stateful_widget(atasks,  layout3[1], &mut self.activity.state);
    
        let s: Vec<ListItem> = self.stats.items.clone().iter()
        .map(|i| ListItem::new(vec![text::Line::from(Span::raw(i.clone()))])).collect();
        let stasks = List::new(s)
        .block(Block::bordered().title("Stats"))
        .highlight_style(Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
        frame.render_stateful_widget(stasks, layout3[2], &mut self.stats.state);
    
        let x_labels = vec![
            Span::styled(
                format!("{}", self.window[0]),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{}",
                (self.window[0] + self.window[1]) / 2.0
            )),
            Span::styled(
                format!("{}", self.window[1]),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        let datasets = vec![
            Dataset::default()
                .name("All")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::LightYellow))
                .data(&self.success_rate_data),
        ];
        let chart = Chart::new(datasets)
            .block(
                Block::bordered().title(Span::styled(
                    "Payment Chart",
                    Style::default()
                        .fg(Color::LightBlue)
                        .add_modifier(Modifier::BOLD),
                )),
            )
            .x_axis(
                Axis::default()
                    .title("Sim Time")
                    .style(Style::default().fg(Color::Gray))
                    .bounds(self.window)
                    .labels(x_labels),
            )
            .y_axis(
                Axis::default()
                    .title("Success Rate")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, 100.0])
                    .labels(vec![
                        Span::styled("0", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw("50"),
                        Span::styled("100", Style::default().add_modifier(Modifier::BOLD)),
                    ]),
            );
        frame.render_widget(chart, layout2[1]);
    }

    fn init(&mut self) {
        self.current_section = RunSection::Events;
        self.events.next();
    }

    fn close(&mut self) {
        self.events.clear();
        self.activity.clear();
        self.stats.clear();
        self.progress = 0.0;
        self.window[0] = 0.0;
        self.window[1] = 20.0;
        self.success_rate_data = [(0.0, 0.0); 21];
        self.points = 0;
    }

    fn process(&mut self, key: KeyEvent) -> ProcessResult {
        match key.code {
            // The Run page is mainly readonly and will show the status of the running simulation, use `s` to stop the simulation and go back to the Configure page
            KeyCode::Char('s') => {
                self.close();
                return ProcessResult::StopSim;
            }
            KeyCode::Tab => {
                match self.current_section {
                    RunSection::Events => {
                        self.current_section = RunSection::Activity;
                        self.events.clear();
                        self.activity.next();
                    }
                    RunSection::Activity => {
                        self.current_section = RunSection::Stats;
                        self.activity.clear();
                        self.stats.next();
                    }
                    RunSection::Stats => {
                        self.current_section = RunSection::Events;
                        self.stats.clear();
                        self.events.next();
                    }
                }
            }
            KeyCode::Up => {
                match self.current_section {
                    RunSection::Events => {
                        self.events.previous();
                    }
                    RunSection::Activity => {
                        self.activity.previous();
                    }
                    RunSection::Stats => {
                        self.stats.previous();
                    }
                }
            }
            KeyCode::Down => {
                match self.current_section {
                    RunSection::Events => {
                        self.events.next();
                    }
                    RunSection::Activity => {
                        self.activity.next();
                    }
                    RunSection::Stats => {
                        self.stats.next();
                    }
                }
            }
            _ => {}
        }

        return ProcessResult::NoOp;
    }

    fn get_index(&self) -> usize {
        3
    }

    fn update_runtime_data(&mut self, events: Option<Vec<String>>, activity: Option<Vec<String>>, stats: Option<Vec<String>>, frame: u64, num_frames: u64, succes_rate: f64) {
        self.events.items = events.unwrap_or(Vec::new());
        self.activity.items = activity.unwrap_or(Vec::new());
        self.stats.items = stats.unwrap_or(Vec::new());

        if frame >= num_frames {
            self.progress = 1.0;
        } else {
            self.progress = frame as f64 / num_frames as f64;
        }

        if self.points <= 20 {
            self.window[0] = 0.0;
            self.window[1] = 20.0;
            self.success_rate_data[self.points] = (frame as f64, succes_rate);
            self.points += 1;
        } else {
            self.window[0] = frame as f64 - 20.0;
            self.window[1] = frame as f64;
            for i in 0..20 {
                self.success_rate_data[i] = self.success_rate_data[i + 1];
            }
    
            self.success_rate_data[20] = (frame as f64, succes_rate);
        }
    }

    fn update_config_data(&mut self, _: Option<Vec<String>>, _: Option<Vec<String>>, _: Option<Vec<String>>, _: Option<Vec<String>>) {
        return;
    }

    fn esc_operation(&mut self) -> ProcessResult {
        ProcessResult::NoOp
    }

    fn quit_operation(&mut self) -> ProcessResult {
        ProcessResult::NoOp
    }
}
