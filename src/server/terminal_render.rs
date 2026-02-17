//! Terminal UI rendering utilities

use crate::models::{Message, MessageContent, NotificationPriority};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use std::io::Stdout;

/// Render the header section
pub fn render_header(
    f: &mut Frame<ratatui::backend::CrosstermBackend<Stdout>>,
    area: ratatui::layout::Rect,
) {
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            "ailoop Server",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" - Agent Message Streaming"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Server Status"),
    )
    .alignment(Alignment::Center);
    f.render_widget(header, area);
}

/// Render the main content area
pub fn render_main_content(
    f: &mut Frame<ratatui::backend::CrosstermBackend<Stdout>>,
    area: ratatui::layout::Rect,
    data: &RenderData,
) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area);

    render_channel_list(f, main_chunks[0], data);
    render_messages(f, main_chunks[1], data);
}

/// Render the channel list
pub fn render_channel_list(
    f: &mut Frame<ratatui::backend::CrosstermBackend<Stdout>>,
    area: ratatui::layout::Rect,
    data: &RenderData,
) {
    let channel_items: Vec<ListItem> = data
        .channels
        .iter()
        .map(|ch| {
            let style = if ch == &data.current_channel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![Span::styled(format!("● {}", ch), style)]))
        })
        .collect();

    let channel_list = List::new(channel_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Channels (Tab to switch)"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(channel_list, area);
}

/// Render the messages section
pub fn render_messages(
    f: &mut Frame<ratatui::backend::CrosstermBackend<Stdout>>,
    area: ratatui::layout::Rect,
    data: &RenderData,
) {
    let message_widget = Paragraph::new(data.message_lines.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Messages - {}", data.current_channel)),
        )
        .wrap(Wrap { trim: true })
        .scroll((data.scroll_offset, 0));
    f.render_widget(message_widget, area);
}

/// Render the footer section
pub fn render_footer(
    f: &mut Frame<ratatui::backend::CrosstermBackend<Stdout>>,
    area: ratatui::layout::Rect,
    data: &RenderData,
) {
    let footer_text = format!(
        "Tab: Switch channel | q: Quit | Channel: {} | Messages: {}",
        data.current_channel, data.message_count
    );
    let footer = Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(footer, area);
}

/// Create layout chunks for the UI
pub fn create_layout(size: ratatui::layout::Rect) -> Vec<ratatui::layout::Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(size)
}

/// Format message content with appropriate color
pub fn format_message_content(content: &MessageContent) -> (String, Color) {
    match content {
        MessageContent::Notification { text, priority } => {
            let color = priority_to_color(priority);
            (text.clone(), color)
        }
        MessageContent::Question { text, .. } => (format!("Question: {}", text), Color::Cyan),
        MessageContent::Authorization { action, .. } => {
            (format!("Authorization: {}", action), Color::Magenta)
        }
        MessageContent::Response {
            answer,
            response_type,
        } => {
            let answer_text = answer.as_deref().unwrap_or("(no answer)");
            (
                format!("Response: {} ({:?})", answer_text, response_type),
                Color::Blue,
            )
        }
        MessageContent::Navigate { url } => (format!("Navigate to: {}", url), Color::Cyan),
    }
}

/// Convert notification priority to color
pub fn priority_to_color(priority: &NotificationPriority) -> Color {
    match priority {
        NotificationPriority::Urgent => Color::Red,
        NotificationPriority::High => Color::Yellow,
        NotificationPriority::Normal => Color::Green,
        NotificationPriority::Low => Color::Blue,
    }
}

/// Data prepared for rendering
#[derive(Debug)]
pub struct RenderData {
    pub channels: Vec<String>,
    pub current_channel: String,
    pub message_lines: Vec<Line<'static>>,
    pub message_count: usize,
    pub scroll_offset: u16,
}
