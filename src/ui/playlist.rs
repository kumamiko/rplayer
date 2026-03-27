use crate::app::App;
use crate::ui::utils;
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
        // Show album column when width >= 60, otherwise hide it
        let show_album = area.width >= 60;

        // Fixed overhead: borders(2) + prefix(2) + index(4) + spaces + duration(6)
        // With album:    borders(2) + prefix(1) + index(4) + spaces(5) + duration(6) = 18
        // Without album: borders(2) + prefix(1) + index(4) + spaces(4) + duration(6) = 17
        let fixed_overhead = if show_album { 18 } else { 17 };
        let available_width = area.width.saturating_sub(fixed_overhead) as usize;

        let (title_width, artist_width, album_width) = if show_album {
            let tw = (available_width * 2 / 5).max(8);
            let aw = (available_width * 1 / 5).max(5);
            let alw = available_width.saturating_sub(tw + aw).max(5);
            (tw, aw, Some(alw))
        } else {
            let tw = (available_width * 3 / 5).max(10);
            let aw = (available_width - tw).max(5);
            (tw, aw, None)
        };

        let theme_bg = self.app.theme_color();
        let theme_title = self.app.theme_color_bright();
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
                let title = Self::pad_to_width(&utils::truncate_to_width(&song.title, title_width), title_width);
                let artist = Self::pad_to_width(&utils::truncate_to_width(&song.artist, artist_width), artist_width);
                let album = album_width.map(|w| {
                    Self::pad_to_width(&utils::truncate_to_width(&song.album, w), w)
                });

                let duration = utils::format_duration_wide(song.duration);
                
                let (prefix, prefix_color) = if is_playing {
                    if self.app.is_playing {
                        ("▶", theme_title.unwrap_or(Color::Cyan))
                    } else {
                        ("⏸", Color::Yellow)
                    }
                } else {
                    (" ", Color::Green)
                };
                
                // Index (1-based)
                let index = display_idx + 1;
                let index_str = format!("{:3}.", index);
                
                let style = if is_selected {
                    Style::default()
                        .bg(theme_bg.unwrap_or(Color::Blue))
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let mut spans = vec![
                    Span::styled(format!("{} ", prefix), Style::default().fg(prefix_color)),
                    Span::styled(format!("{} ", index_str), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{} ", title), style),
                    Span::styled(format!("{} ", artist), Style::default().fg(Color::DarkGray)),
                ];
                if let Some(album) = album {
                    spans.push(Span::styled(format!("{} ", album), Style::default().fg(Color::DarkGray)));
                }
                spans.push(Span::styled(duration, Style::default().fg(Color::Gray)));
                
                (Line::from(spans), style)
            })
            .map(|(line, style)| ListItem::new(line).style(style))
            .collect();
        
        let theme_border = self.app.theme_color().unwrap_or(Color::Cyan);
        let theme_title = self.app.theme_color_bright().unwrap_or(Color::Cyan);
        let title = format!(" 歌曲列表 [{} 首] ", self.app.filtered_indices.len());
        
        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .title_style(Style::default().fg(theme_title))
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(theme_border))
            );
        
        Widget::render(list, area, buf);
    }
}

impl<'a> StatefulWidget for PlaylistWidget<'a> {
    type State = ();
    
    fn render(self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        Widget::render(self, area, buf);
    }
}
