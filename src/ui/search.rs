use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
    layout::Rect,
    buffer::Buffer,
};

pub struct SearchWidget<'a> {
    query: &'a str,
}

impl<'a> SearchWidget<'a> {
    pub fn new(query: &'a str) -> Self {
        Self { query }
    }
}

impl<'a> Widget for SearchWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let line = Line::from(vec![
            Span::styled(" SEARCH: ", Style::default().bg(Color::Yellow).fg(Color::Black)),
            Span::styled(self.query, Style::default().fg(Color::White)),
            Span::raw("█"),
        ]);
        
        let paragraph = Paragraph::new(line);
        Widget::render(paragraph, area, buf);
    }
}
