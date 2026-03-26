use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
    layout::Rect,
    buffer::Buffer,
};

pub struct ThemeWidget<'a> {
    input: &'a str,
    cursor: usize,
}

impl<'a> ThemeWidget<'a> {
    pub fn new(input: &'a str, cursor: usize) -> Self {
        Self { input, cursor }
    }
}

impl<'a> Widget for ThemeWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chars: Vec<char> = self.input.chars().collect();
        let cursor = self.cursor.min(chars.len());
        let label_style = Style::default().bg(Color::Magenta).fg(Color::Black);
        let text_style = Style::default().fg(Color::Yellow);
        let cursor_style = Style::default().bg(Color::White).fg(Color::Black);

        let mut spans = vec![
            Span::styled(" THEME: ", label_style),
        ];

        for (i, &c) in chars.iter().enumerate() {
            let style = if i == cursor { cursor_style } else { text_style };
            spans.push(Span::styled(c.to_string(), style));
        }

        if cursor == chars.len() {
            spans.push(Span::styled(" ", cursor_style));
        }

        let paragraph = Paragraph::new(Line::from(spans));
        Widget::render(paragraph, area, buf);
    }
}
