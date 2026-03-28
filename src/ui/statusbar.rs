use crate::app::{App, Mode, PlayMode};
use crate::ui::utils;
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
            Mode::Theme => Style::default()
                .bg(Color::Magenta)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
            Mode::SwitchCache => Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
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
        let pos = utils::format_duration_compact(self.app.current_pos);
        let dur = utils::format_duration_compact(self.app.duration);
        
        let song_info = if let Some(idx) = self.app.current_song_index {
            if let Some(song) = self.app.songs.get(idx) {
                let name = utils::truncate_to_width(&song.title, 30);
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
