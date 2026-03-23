use crate::lyrics::LyricsManager;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
    layout::{Rect, Alignment},
    buffer::Buffer,
};
use std::time::Duration;

pub struct LyricsWidget<'a> {
    lyrics: &'a LyricsManager,
    position: Duration,
}

impl<'a> LyricsWidget<'a> {
    pub fn new(lyrics: &'a LyricsManager, position: Duration) -> Self {
        Self { lyrics, position }
    }
}

impl<'a> Widget for LyricsWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Get current and next lyrics line for karaoke effect
        let (current_line, next_line) = self.lyrics.get_current_and_next(self.position);
        
        let lines: Vec<Line> = if let Some(current) = current_line {
            vec![
                // Current line - highlighted (karaoke style)
                Line::from(vec![
                    Span::styled(
                        "♪ ",
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
                    ),
                    Span::styled(
                        current,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    ),
                ]),
                // Next line - dimmed preview
                if let Some(next) = next_line {
                    Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(next, Style::default().fg(Color::DarkGray)),
                    ])
                } else {
                    Line::raw("")
                },
            ]
        } else {
            vec![
                Line::from(Span::styled(
                    "♪ No lyrics ♪",
                    Style::default().fg(Color::DarkGray)
                )),
            ]
        };
        
        let paragraph = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(" ♪ ")
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray))
            )
            .alignment(Alignment::Center);
        
        Widget::render(paragraph, area, buf);
    }
}
