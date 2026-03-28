use crate::app::CachedFolder;
use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    buffer::Buffer,
};

pub struct SwitchCacheWidget<'a> {
    folders: &'a [CachedFolder],
    selected: usize,
    current_folder: &'a str,
    theme_border: Color,
    theme_title: Color,
}

impl<'a> SwitchCacheWidget<'a> {
    pub fn new(
        folders: &'a [CachedFolder],
        selected: usize,
        current_folder: &'a str,
        theme_border: Color,
        theme_title: Color,
    ) -> Self {
        Self {
            folders,
            selected,
            current_folder,
            theme_border,
            theme_title,
        }
    }
}

impl Widget for SwitchCacheWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);
        
        // Fixed lines: title(1) + empty(1) + empty(1) + hint(1) = 4
        // Plus borders(2) = 6
        // Remaining space for folders
        let max_folders_visible = area.height.saturating_sub(6) as usize;
        
        // Calculate scroll to keep selected item visible
        let scroll_offset = if self.folders.len() <= max_folders_visible || max_folders_visible == 0 {
            0
        } else if self.selected < max_folders_visible / 2 {
            0
        } else if self.selected >= self.folders.len() - max_folders_visible / 2 {
            self.folders.len() - max_folders_visible
        } else {
            self.selected - max_folders_visible / 2
        };
        
        let visible_folders = if max_folders_visible == 0 {
            &self.folders[0..0]
        } else if scroll_offset + max_folders_visible >= self.folders.len() {
            &self.folders[scroll_offset..]
        } else {
            &self.folders[scroll_offset..scroll_offset + max_folders_visible]
        };
        
        let mut lines = vec![
            Line::from(Span::styled(
                "切换音乐库",
                Style::default()
                    .fg(self.theme_title)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
        ];
        
        if self.folders.is_empty() {
            lines.push(Line::from("  没有可用的缓存"));
        } else {
            for (i, folder) in visible_folders.iter().enumerate() {
                let actual_index = scroll_offset + i;
                let is_current = folder.music_folder == self.current_folder;
                let is_selected = actual_index == self.selected;
                
                let prefix = if is_selected { "  > " } else { "    " };
                let marker = if is_current { " [当前]" } else { "" };
                
                // Truncate folder path if too long
                let max_len = 40;
                let display_path = if folder.music_folder.len() > max_len {
                    format!("...{}", &folder.music_folder[folder.music_folder.len().saturating_sub(max_len - 3)..])
                } else {
                    folder.music_folder.clone()
                };
                
                let line = if is_selected {
                    Line::from(vec![
                        Span::styled(prefix, Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!("{} ({} 首){}", display_path, folder.song_count, marker),
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                        ),
                    ])
                } else {
                    Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(
                            format!("{} ({} 首){}", display_path, folder.song_count, marker),
                            Style::default().fg(if is_current { Color::Cyan } else { Color::Reset }),
                        ),
                    ])
                };
                lines.push(line);
            }
        }
        
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Enter 切换 | Esc 取消",
            Style::default().fg(Color::DarkGray),
        )));
        
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(self.theme_border));
        
        let paragraph = Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Left);
        
        // Calculate popup size - always show hint
        let content_lines = visible_folders.len() + 4;
        let popup_width = 50u16.min(area.width);
        let popup_height = (content_lines as u16 + 2).min(area.height);
        
        let popup_area = Rect {
            x: area.x + (area.width.saturating_sub(popup_width)) / 2,
            y: area.y + (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width,
            height: popup_height,
        };
        
        paragraph.render(popup_area, buf);
    }
}
