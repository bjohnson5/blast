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
    add: bool
}

impl RunTab {
    pub fn new() -> Self {
        // TODO: this is a placeholder, initialize with the actual saved simulations
        let mut events_list: Vec<String> = Vec::new();
        let mut activity_list: Vec<String> = Vec::new();
        let mut stats_list: Vec<String> = Vec::new();
        events_list.push(String::from("10s OpenChannel (blast_lnd0000 --> blast_lnd0001: 2000msat)"));
        events_list.push(String::from("20s CloseChannel (0)"));
        activity_list.push(String::from("blast_ldk0000 --> blast_lnd0004: 2000msat, 5s"));
        activity_list.push(String::from("blast_ldk0001 --> blast_lnd0005: 1000msat, 15s"));
        activity_list.push(String::from("blast_ldk0002 --> blast_lnd0006: 8000msat, 10s"));
        activity_list.push(String::from("blast_ldk0003 --> blast_lnd0007: 5000msat, 25s"));
        stats_list.push(String::from("Number of Nodes:          15"));
        stats_list.push(String::from("Total Payment Attempts:   76"));
        stats_list.push(String::from("Payment Success Rate:     100%"));

        Self {
            events: StatefulList::with_items(events_list),
            activity: StatefulList::with_items(activity_list),
            stats: StatefulList::with_items(stats_list),
            current_section: RunSection::Events,
            progress: 0.0,
            window: [0.0, 20.0],
            success_rate_data: [(0.0, 0.0); 21],
            add: true
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
            "q".bold(),
            " to exit, ".into(),
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
            .ratio(self.progress/100.0);
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

    fn update_runtime_data(&mut self) {
        self.progress = self.progress + 1.0;
        self.window[0] += 1.0;
        self.window[1] += 1.0;
        for i in 0..20 {
            self.success_rate_data[i] = self.success_rate_data[i + 1];
        }

        if self.add {
            self.success_rate_data[20] = (self.window[1], self.success_rate_data[19].1 + 5.0);
        } else {
            self.success_rate_data[20] = (self.window[1], self.success_rate_data[19].1 - 5.0);
        }

        if self.window[1] % 10.0 == 0.0 {
            self.add = !self.add;
        }
    }

    fn update_config_data(&mut self, _: Vec<String>, _: Vec<String>, _: Vec<String>, _: Vec<String>) {
        return;
    }

    fn esc_operation(&mut self) -> ProcessResult {
        ProcessResult::NoOp
    }
}
