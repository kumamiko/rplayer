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
    theme_border: Color,
    theme_title: Color,
}

impl<'a> LyricsWidget<'a> {
    pub fn new(lyrics: &'a LyricsManager, position: Duration, theme_border: Color, theme_title: Color) -> Self {
        Self { lyrics, position, theme_border, theme_title }
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
        } else if let Some(next) = next_line {
            // Before lyrics start (intro) - show upcoming line dimmed
            vec![
                Line::from(vec![
                    Span::styled("♪ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(next, Style::default().fg(Color::DarkGray)),
                ]),
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
                    .title_style(Style::default().fg(self.theme_title))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(self.theme_border))
            )
            .alignment(Alignment::Center);
        
        Widget::render(paragraph, area, buf);
    }
}
