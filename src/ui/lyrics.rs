use crate::lyrics::{LyricsLine, LyricsManager};
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
    has_lyrics: bool,
}

impl<'a> LyricsWidget<'a> {
    pub fn new(lyrics: &'a LyricsManager, position: Duration, theme_border: Color, theme_title: Color) -> Self {
        Self { lyrics, position, theme_border, theme_title, has_lyrics: !lyrics.is_empty() }
    }
}

impl<'a> Widget for LyricsWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (current_line, next_line) = self.lyrics.get_current_and_next(self.position);

        let lines: Vec<Line> = match current_line {
            Some(line) => match line {
                LyricsLine::Timed { words, .. } => render_timed_line(words, self.position, self.theme_title),
                LyricsLine::Plain { text, .. } => {
                    if text.len() >= 2 {
                        // Bilingual: show original + translation
                        vec![
                            Line::from(Span::styled(
                                text[0].clone(),
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD)
                            )),
                            Line::from(Span::styled(
                                text[1].clone(),
                                Style::default().fg(Color::Gray)
                            )),
                        ]
                    } else if !text.is_empty() {
                        // Single language: show current + next line preview
                        let mut result = vec![
                            Line::from(Span::styled(
                                text[0].clone(),
                                Style::default()
                                    .fg(Color::White)
                                    .add_modifier(Modifier::BOLD)
                            )),
                        ];
                        if let Some(LyricsLine::Plain { text: next_text, .. }) = next_line {
                            if !next_text.is_empty() {
                                result.push(Line::from(vec![
                                    Span::styled(next_text[0].clone(), Style::default().fg(Color::Rgb(120, 120, 120))),
                                ]));
                            }
                        }
                        result
                    } else {
                        vec![Line::from(Span::styled("", Style::default().fg(Color::DarkGray)))]
                    }
                }
            }
            None => {
                // Before lyrics start (intro) - show upcoming line dimmed
                if let Some(LyricsLine::Plain { text, .. }) = next_line {
                    if !text.is_empty() {
                        vec![
                            Line::from(Span::styled(text[0].clone(), Style::default().fg(Color::DarkGray))),
                        ]
                    } else {
                        vec![Line::from(Span::styled("", Style::default().fg(Color::DarkGray)))]
                    }
                } else if let Some(LyricsLine::Timed { words, .. }) = next_line {
                    render_timed_line(words, Duration::ZERO, Color::DarkGray)
                } else {
                    let placeholder = if self.has_lyrics { "" } else { "No lyrics" };
                    vec![Line::from(Span::styled(placeholder, Style::default().fg(Color::DarkGray)))]
                }
            }
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

/// Render a timed (Enhanced LRC) lyrics line with word-by-word highlighting
fn render_timed_line(words: &[crate::lyrics::LyricsWord], position: Duration, highlight_color: Color) -> Vec<Line<'static>> {
    let mut spans: Vec<Span> = Vec::new();

    for word in words {
        let style = if word.start <= position {
            // Already sung: highlight with theme color + bold
            Style::default()
                .fg(highlight_color)
                .add_modifier(Modifier::BOLD)
        } else {
            // Not yet reached: dim
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(word.text.clone(), style));
    }

    vec![Line::from(spans)]
}
