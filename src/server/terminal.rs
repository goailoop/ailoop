//! Interactive terminal UI for server monitoring with channel switching

mod terminal_render;
use terminal_render::{
    create_layout, format_message_content, render_footer, render_header, render_main_content,
    RenderData,
};

use crate::models::Message;
use crate::server::history::MessageHistory;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, text::Line, Terminal};
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
        self.update_channels().await;

        let render_data = self.prepare_render_data().await;

        self.terminal.draw(|f| {
            let size = f.size();
            let chunks = create_layout(size);

            render_header(f, chunks[0]);
            render_main_content(f, chunks[1], &render_data);
            render_footer(f, chunks[2], &render_data);
        })?;

        Ok(())
    }

    /// Update channels list from message history
    async fn update_channels(&self) {
        let available_channels = self.message_history.get_channels().await;
        let mut channels = self.channels.write().await;
        *channels = available_channels;
        if channels.is_empty() {
            channels.push("public".to_string());
        }
    }

    /// Prepare data for rendering
    async fn prepare_render_data(&self) -> RenderData {
        let current_channel = self.current_channel.read().await.clone();
        let messages = self
            .message_history
            .get_messages(&current_channel, Some(50))
            .await;
        let channels = self.channels.read().await.clone();

        let message_lines = Self::format_messages(&messages);
        let message_count = messages.len();
        let scroll_offset = (message_count.saturating_sub(30)).max(0) as u16;

        RenderData {
            channels,
            current_channel,
            message_lines,
            message_count,
            scroll_offset,
        }
    }

    /// Format messages for display
    fn format_messages(messages: &[Message]) -> Vec<Line<'static>> {
        if messages.is_empty() {
            return vec![Line::from("No messages in this channel yet.")];
        }

        messages
            .iter()
            .rev()
            .take(30)
            .map(|msg| Self::format_message(msg))
            .collect()
    }

    /// Format a message for display
    fn format_message(message: &Message) -> Line<'static> {
        let timestamp = message.timestamp.format("%H:%M:%S").to_string();
        let agent_type = Self::extract_agent_type(message);
        let (content_text, color) = format_message_content(&message.content);

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
    fn extract_agent_type(message: &Message) -> String {
        message
            .metadata
            .as_ref()
            .and_then(|m| m.get("agent_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string()
    }

    /// Handle keyboard input for channel switching
    pub async fn handle_input(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(true);
                        }
                        KeyCode::Tab => {
                            self.switch_to_next_channel().await;
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(false)
    }

    /// Switch to next channel
    async fn switch_to_next_channel(&self) {
        let channels = self.channels.read().await;
        if !channels.is_empty() {
            let current = self.current_channel.read().await.clone();
            let current_idx = channels.iter().position(|c| c == &current).unwrap_or(0);
            let next_idx = (current_idx + 1) % channels.len();
            let mut current_ch = self.current_channel.write().await;
            *current_ch = channels[next_idx].clone();
        }
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

/// Data prepared for rendering
struct RenderData {
    channels: Vec<String>,
    current_channel: String,
    message_lines: Vec<Line<'static>>,
    message_count: usize,
    scroll_offset: u16,
}
