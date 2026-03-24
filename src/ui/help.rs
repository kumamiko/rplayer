use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
    buffer::Buffer,
};

pub struct HelpWidget;

impl HelpWidget {
    pub fn new() -> Self {
        Self
    }
}

impl Widget for HelpWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear the area first
        Clear.render(area, buf);
        
        let help_lines = vec![
            Line::from(Span::styled("快捷键帮助", Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD))),
            Line::from(""),
            Line::from(vec![
                Span::styled("  j/k, ↑/↓    ", Style::default().fg(Color::Yellow)),
                Span::raw("上下移动"),
            ]),
            Line::from(vec![
                Span::styled("  g/G          ", Style::default().fg(Color::Yellow)),
                Span::raw("跳到首/尾"),
            ]),
            Line::from(vec![
                Span::styled("  h/l          ", Style::default().fg(Color::Yellow)),
                Span::raw("快退/快进 10秒"),
            ]),
            Line::from(vec![
                Span::styled("  PageUp/Down  ", Style::default().fg(Color::Yellow)),
                Span::raw("翻页"),
            ]),
            Line::from(vec![
                Span::styled("  ←/→          ", Style::default().fg(Color::Yellow)),
                Span::raw("翻页"),
            ]),
            Line::from(vec![
                Span::styled("  u/d          ", Style::default().fg(Color::Yellow)),
                Span::raw("上/下翻页"),
            ]),
            Line::from(vec![
                Span::styled("  `/\'          ", Style::default().fg(Color::Yellow)),
                Span::raw("跳到当前播放"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  Enter        ", Style::default().fg(Color::Yellow)),
                Span::raw("播放选中"),
            ]),
            Line::from(vec![
                Span::styled("  Space        ", Style::default().fg(Color::Yellow)),
                Span::raw("暂停/继续"),
            ]),
            Line::from(vec![
                Span::styled("  s            ", Style::default().fg(Color::Yellow)),
                Span::raw("停止"),
            ]),
            Line::from(vec![
                Span::styled("  n/p          ", Style::default().fg(Color::Yellow)),
                Span::raw("下/上一曲"),
            ]),
            Line::from(vec![
                Span::styled("  r            ", Style::default().fg(Color::Yellow)),
                Span::raw("切换播放模式"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  +/-          ", Style::default().fg(Color::Yellow)),
                Span::raw("音量 ±10%"),
            ]),
            Line::from(vec![
                Span::styled("  /, f         ", Style::default().fg(Color::Yellow)),
                Span::raw("搜索"),
            ]),
            Line::from(vec![
                Span::styled("  Ctrl+f       ", Style::default().fg(Color::Yellow)),
                Span::raw("切换搜索字段"),
            ]),
            Line::from(vec![
                Span::styled("  F            ", Style::default().fg(Color::Yellow)),
                Span::raw("取消搜索"),
            ]),
            Line::from(vec![
                Span::styled("  R            ", Style::default().fg(Color::Yellow)),
                Span::raw("重新扫描媒体库"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled("  q            ", Style::default().fg(Color::Yellow)),
                Span::raw("退出"),
            ]),
            Line::from(""),
            Line::from(Span::styled("按任意键关闭", Style::default()
                .fg(Color::DarkGray))),
        ];
        
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        
        let paragraph = Paragraph::new(help_lines)
            .block(block)
            .alignment(Alignment::Left);
        
        // Calculate centered popup position
        let popup_width = 36u16;
        let popup_height = 26u16;
        let popup_area = Rect {
            x: area.x + (area.width.saturating_sub(popup_width)) / 2,
            y: area.y + (area.height.saturating_sub(popup_height)) / 2,
            width: popup_width.min(area.width),
            height: popup_height.min(area.height),
        };
        
        paragraph.render(popup_area, buf);
    }
}
