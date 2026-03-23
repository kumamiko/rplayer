use crate::app::App;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, StatefulWidget, Widget},
    layout::Rect,
    buffer::Buffer,
};
use unicode_width::UnicodeWidthStr;

pub struct PlaylistWidget<'a> {
    app: &'a App,
}

impl<'a> PlaylistWidget<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
    
    fn format_duration(d: std::time::Duration) -> String {
        let total_secs = d.as_secs();
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{:2}:{:02}", mins, secs)
    }
    
    /// Truncate string by display width, safe for UTF-8
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
    
    /// Pad string to specific display width
    fn pad_to_width(s: &str, width: usize) -> String {
        let current_width = UnicodeWidthStr::width(s);
        if current_width >= width {
            s.to_string()
        } else {
            format!("{}{}", s, " ".repeat(width - current_width))
        }
    }
}

impl<'a> Widget for PlaylistWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Calculate dynamic widths based on available area
        // Layout: [prefix(2)] [index(4)] [title] [artist] [duration(6)]
        // Subtract borders(2), prefix(2), index(4), spaces(4), duration(6)
        let available_width = area.width.saturating_sub(18) as usize;
        let title_width = (available_width * 3 / 5).max(10);  // 60% for title, min 10
        let artist_width = (available_width - title_width).max(5);  // remaining for artist, min 5
        
        let items: Vec<ListItem> = self.app.filtered_indices
            .iter()
            .enumerate()
            .skip(self.app.scroll_offset)
            .take(area.height.saturating_sub(2) as usize)
            .map(|(display_idx, &song_idx)| {
                let song = &self.app.songs[song_idx];
                let is_selected = display_idx == self.app.selected_index;
                let is_playing = self.app.current_song_index == Some(song_idx);
                
                // Truncate and pad for proper alignment with dynamic widths
                let title = Self::pad_to_width(&Self::truncate_to_width(&song.title, title_width), title_width);
                let artist = Self::pad_to_width(&Self::truncate_to_width(&song.artist, artist_width), artist_width);
                
                let duration = Self::format_duration(song.duration);
                
                let prefix = if is_playing {
                    if self.app.is_playing { "▶" } else { "⏸" }
                } else {
                    " "
                };
                
                // Index (1-based)
                let index = display_idx + 1;
                let index_str = format!("{:3}.", index);
                
                let style = if is_selected {
                    Style::default()
                        .bg(Color::Blue)
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                
                let line = Line::from(vec![
                    Span::styled(format!("{} ", prefix), Style::default().fg(Color::Green)),
                    Span::styled(format!("{} ", index_str), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{} ", title), style),
                    Span::styled(format!("{} ", artist), Style::default().fg(Color::DarkGray)),
                    Span::styled(duration, Style::default().fg(Color::Gray)),
                ]);
                
                ListItem::new(line).style(style)
            })
            .collect();
        
        let title = format!(" 歌曲列表 [{} 首] ", self.app.filtered_indices.len());
        
        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(Color::Cyan))
            );
        
        Widget::render(list, area, buf);
        
        // Update scroll offset if needed
        let visible_height = area.height.saturating_sub(2) as usize;
        if self.app.selected_index < self.app.scroll_offset {
            // Scroll up needed - but we can't mutate here
        } else if self.app.selected_index >= self.app.scroll_offset + visible_height {
            // Scroll down needed - but we can't mutate here
        }
    }
}

impl<'a> StatefulWidget for PlaylistWidget<'a> {
    type State = ();
    
    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        Widget::render(self, area, buf);
    }
}
