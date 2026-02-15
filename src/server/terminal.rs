//! Interactive terminal UI for server monitoring with channel switching

use crate::models::{Message, MessageContent, NotificationPriority};
use crate::server::history::MessageHistory;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};
use std::io::{self, Stdout};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Terminal UI for server monitoring with channel switching
pub struct TerminalUI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    message_history: Arc<MessageHistory>,
    current_channel: Arc<RwLock<String>>,
    channels: Arc<RwLock<Vec<String>>>,
}

impl TerminalUI {
    /// Create a new terminal UI with message history
    pub fn new(message_history: Arc<MessageHistory>) -> Result<Self, Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self {
            terminal,
            message_history,
            current_channel: Arc::new(RwLock::new("public".to_string())),
            channels: Arc::new(RwLock::new(vec!["public".to_string()])),
        })
    }

    /// Render the terminal UI with channel switching and message display
    pub async fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.update_channels_list().await;

        let current_channel = self.get_current_channel().await;
        let messages = self.get_messages(&current_channel).await;
        let channels = self.get_channels().await;

        let message_lines = self.format_messages(&messages);
        let channel_items = self.build_channel_items(&channels, &current_channel);
        let scroll_offset = self.calculate_scroll_offset(messages.len());

        self.draw_ui(
            &current_channel,
            &message_lines,
            &channel_items,
            scroll_offset,
            messages.len(),
        )?;

        Ok(())
    }

    /// Update channels list from message history
    async fn update_channels_list(&mut self) {
        let available_channels = self.message_history.get_channels().await;
        let mut channels = self.channels.write().await;
        *channels = available_channels;
        if channels.is_empty() {
            channels.push("public".to_string());
        }
    }

    /// Get current channel
    async fn get_current_channel(&self) -> String {
        let channel = self.current_channel.read().await;
        channel.clone()
    }

    /// Get channels list
    async fn get_channels(&self) -> Vec<String> {
        let channels_lock = self.channels.read().await;
        channels_lock.clone()
    }

    /// Get messages for current channel
    async fn get_messages(&self, channel: &str) -> Vec<Message> {
        self.message_history.get_messages(channel, Some(50)).await
    }

    /// Format messages for display
    fn format_messages(&self, messages: &[Message]) -> Vec<Line> {
        if messages.is_empty() {
            return vec![Line::from("No messages in this channel yet.")];
        }

        messages
            .iter()
            .rev()
            .take(30)
            .map(|msg| self.format_message(msg))
            .collect()
    }

    /// Build channel list items
    fn build_channel_items(&self, channels: &[String], current_channel: &str) -> Vec<ListItem> {
        channels
            .iter()
            .map(|ch| {
                let style = if ch == current_channel {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(vec![Span::styled(format!("● {}", ch), style)]))
            })
            .collect()
    }

    /// Calculate scroll offset
    fn calculate_scroll_offset(&self, message_count: usize) -> u16 {
        (message_count.saturating_sub(30)).max(0) as u16
    }

    /// Draw the UI
    fn draw_ui(
        &mut self,
        current_channel: &str,
        message_lines: &[Line],
        channel_items: &[ListItem],
        scroll_offset: u16,
        message_count: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.draw(|f| {
            let size = f.size();

            let chunks = create_main_layout(size);

            self.draw_header(f, chunks[0]);
            self.draw_main_content(
                f,
                chunks[1],
                current_channel,
                message_lines,
                channel_items,
                scroll_offset,
            );
            self.draw_footer(f, chunks[2], current_channel, message_count);
        })?;

        Ok(())
    }

    /// Draw header
    fn draw_header(&self, f: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let header = create_header_widget();
        f.render_widget(header, area);
    }

    /// Draw main content area
    fn draw_main_content(
        &self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        current_channel: &str,
        message_lines: &[Line],
        channel_items: &[ListItem],
        scroll_offset: u16,
    ) {
        let main_chunks = create_horizontal_layout(area);

        self.draw_channel_list(f, main_chunks[0], channel_items);
        self.draw_message_list(
            f,
            main_chunks[1],
            current_channel,
            message_lines,
            scroll_offset,
        );
    }

    /// Draw channel list
    fn draw_channel_list(
        &self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        channel_items: &[ListItem],
    ) {
        let channel_list = create_channel_list_widget(channel_items);
        f.render_widget(channel_list, area);
    }

    /// Draw message list
    fn draw_message_list(
        &self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        current_channel: &str,
        message_lines: &[Line],
        scroll_offset: u16,
    ) {
        let message_widget = create_message_widget(current_channel, message_lines, scroll_offset);
        f.render_widget(message_widget, area);
    }

    /// Draw footer
    fn draw_footer(
        &self,
        f: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        current_channel: &str,
        message_count: usize,
    ) {
        let footer = create_footer_widget(current_channel, message_count);
        f.render_widget(footer, area);
    }

    /// Format a message for display
    fn format_message(&self, message: &Message) -> Line {
        let timestamp = message.timestamp.format("%H:%M:%S").to_string();
        let agent_type = self.extract_agent_type(message);
        let (content_text, color) = self.format_message_content(message);

        Line::from(vec![
            Span::styled(
                format!("[{}] ", timestamp),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("[{}] ", agent_type),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(content_text, Style::default().fg(color)),
        ])
    }

    /// Extract agent type from message metadata
    fn extract_agent_type(&self, message: &Message) -> &str {
        message
            .metadata
            .as_ref()
            .and_then(|m| m.get("agent_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
    }

    /// Format message content with appropriate color
    fn format_message_content(&self, message: &Message) -> (String, Color) {
        match &message.content {
            MessageContent::Notification { text, priority } => {
                let color = self.notification_color(priority);
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

    /// Get color for notification priority
    fn notification_color(&self, priority: &NotificationPriority) -> Color {
        match priority {
            NotificationPriority::Urgent => Color::Red,
            NotificationPriority::High => Color::Yellow,
            NotificationPriority::Normal => Color::Green,
            NotificationPriority::Low => Color::Blue,
        }
    }

    /// Handle keyboard input for channel switching
    pub async fn handle_input(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    return self.handle_key_event(key.code).await;
                }
            }
        }
        Ok(false)
    }

    /// Handle a key event
    async fn handle_key_event(
        &mut self,
        key_code: KeyCode,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        match key_code {
            KeyCode::Char('q') | KeyCode::Esc => Ok(true),
            KeyCode::Tab => self.switch_to_next_channel().await,
            _ => Ok(false),
        }
    }

    /// Switch to next channel
    async fn switch_to_next_channel(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let channels = self.channels.read().await;
        if !channels.is_empty() {
            let current = self.current_channel.read().await.clone();
            let current_idx = channels.iter().position(|c| c == &current).unwrap_or(0);
            let next_idx = (current_idx + 1) % channels.len();
            let mut current_ch = self.current_channel.write().await;
            *current_ch = channels[next_idx].clone();
        }
        Ok(false)
    }

    /// Suspend terminal UI
    pub fn suspend(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        disable_raw_mode()?;
        execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Resume terminal UI
    pub fn resume(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        Ok(())
    }

    /// Cleanup and restore terminal
    pub fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.suspend()
    }
}

impl Drop for TerminalUI {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Create main layout for terminal UI
fn create_main_layout(size: ratatui::layout::Rect) -> Vec<ratatui::layout::Rect> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(size)
}

/// Create horizontal layout for main content
fn create_horizontal_layout(area: ratatui::layout::Rect) -> Vec<ratatui::layout::Rect> {
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
        .split(area)
}

/// Create header widget
fn create_header_widget() -> Paragraph<'static> {
    Paragraph::new(Line::from(vec![
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
    .alignment(Alignment::Center)
}

/// Create channel list widget
fn create_channel_list_widget(items: &[ListItem]) -> List {
    List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Channels (Tab to switch)"),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
}

/// Create message widget
fn create_message_widget(
    current_channel: &str,
    message_lines: &[Line],
    scroll_offset: u16,
) -> Paragraph {
    Paragraph::new(message_lines.to_vec())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Messages - {}", current_channel)),
        )
        .wrap(Wrap { trim: true })
        .scroll((scroll_offset, 0))
}

/// Create footer widget
fn create_footer_widget(current_channel: &str, message_count: usize) -> Paragraph {
    let footer_text = format!(
        "Tab: Switch channel | q: Quit | Channel: {} | Messages: {}",
        current_channel, message_count
    );
    Paragraph::new(footer_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center)
}
