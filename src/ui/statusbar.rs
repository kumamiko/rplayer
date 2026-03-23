use crate::app::{App, Mode, PlayMode};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
    layout::Rect,
    buffer::Buffer,
};
use unicode_width::UnicodeWidthStr;

pub struct StatusbarWidget<'a> {
    app: &'a App,
}

impl<'a> StatusbarWidget<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
    
    fn format_duration(d: std::time::Duration) -> String {
        let total_secs = d.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{:02}:{:02}", mins, secs)
    }
    
    fn truncate_to_width(s: &str, max_width: usize) -> String {
        let mut width = 0;
        let mut result = String::new();
        
        for ch in s.chars() {
            let ch_width = UnicodeWidthStr::width(ch.to_string().as_str());
            if width + ch_width > max_width - 3 {
                break;
            }
            result.push(ch);
            width += ch_width;
        }
        
        if result.len() < s.len() {
            format!("{}...", result)
        } else {
            result
        }
    }
}

impl<'a> Widget for StatusbarWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Left: Mode indicator
        let mode_style = match self.app.mode {
            Mode::Normal => Style::default()
                .bg(Color::Green)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            Mode::Search => Style::default()
                .bg(Color::Yellow)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
            Mode::ConfirmRefresh => Style::default()
                .bg(Color::Magenta)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            Mode::Help => Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        };
        let mode_text = format!(" {} ", self.app.mode.as_str());
        
        // Play mode indicator (only show if not None)
        let play_mode_text = if self.app.play_mode != PlayMode::None {
            format!(" {}", self.app.play_mode.icon())
        } else {
            String::new()
        };
        
        // Right: Song name + progress
        let pos = Self::format_duration(self.app.current_pos);
        let dur = Self::format_duration(self.app.duration);
        
        let song_info = if let Some(idx) = self.app.current_song_index {
            if let Some(song) = self.app.songs.get(idx) {
                let name = Self::truncate_to_width(&song.title, 30);
                format!("{} {}/{}", name, pos, dur)
            } else {
                format!("{}/{}", pos, dur)
            }
        } else {
            format!("{}/{}", pos, dur)
        };
        
        // Calculate padding to right-align song info
        let mode_width = UnicodeWidthStr::width(mode_text.as_str()) + UnicodeWidthStr::width(play_mode_text.as_str()) + 1; // +1 for space
        let info_width = UnicodeWidthStr::width(song_info.as_str());
        let total_width = area.width as usize;
        let padding = total_width.saturating_sub(mode_width + info_width);
        
        // Build line: [MODE][icon] <padding> [song info (right aligned)]
        let line = Line::from(vec![
            Span::styled(mode_text, mode_style),
            Span::styled(play_mode_text, Style::default().fg(Color::Cyan)),
            Span::raw(" ".repeat(padding)),
            Span::styled(song_info, Style::default().fg(Color::White)),
        ]);
        
        let paragraph = Paragraph::new(line);
        Widget::render(paragraph, area, buf);
    }
}
