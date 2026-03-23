use crate::app::App;
use ratatui::{
    style::{Color, Style},
    text::Line,
    widgets::{Paragraph, Widget},
    layout::Rect,
    buffer::Buffer,
};

pub struct MessageWidget<'a> {
    app: &'a App,
}

impl<'a> MessageWidget<'a> {
    pub fn new(app: &'a App) -> Self {
        Self { app }
    }
}

impl<'a> Widget for MessageWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let message = if !self.app.status_message.is_empty() {
            &self.app.status_message
        } else {
            ""
        };
        
        let line = Line::styled(
            format!(" {}", message),
            Style::default().fg(Color::Yellow)
        );
        
        let paragraph = Paragraph::new(line);
        Widget::render(paragraph, area, buf);
    }
}
