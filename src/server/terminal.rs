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
        // Setup terminal
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
        // Update channels list
        let available_channels = self.message_history.get_channels().await;
        {
            let mut channels = self.channels.write().await;
            *channels = available_channels;
            if channels.is_empty() {
                channels.push("public".to_string());
            }
        }

        // Get current channel
        let current_channel = {
            let channel = self.current_channel.read().await;
            channel.clone()
        };

        // Get messages for current channel
        let messages = self
            .message_history
            .get_messages(&current_channel, Some(50))
            .await;

        // Get channels for display
        let channels = {
            let channels_lock = self.channels.read().await;
            channels_lock.clone()
        };

        // Format messages outside the closure to avoid borrowing issues
        // We need to format before calling terminal.draw to avoid borrow conflicts
        let format_fn = |msg: &Message| -> Line {
            let timestamp = msg.timestamp.format("%H:%M:%S").to_string();

            // Extract agent type from metadata
            let agent_type = msg
                .metadata
                .as_ref()
                .and_then(|m| m.get("agent_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let (content_text, color) = match &msg.content {
                MessageContent::Notification { text, priority } => {
                    let color = match priority {
                        NotificationPriority::Urgent => Color::Red,
                        NotificationPriority::High => Color::Yellow,
                        NotificationPriority::Normal => Color::Green,
                        NotificationPriority::Low => Color::Blue,
                    };
                    (text.clone(), color)
                }
                MessageContent::Question { text, .. } => (format!("❓ {}", text), Color::Cyan),
                MessageContent::Authorization { action, .. } => {
                    (format!("🔐 Authorization: {}", action), Color::Magenta)
                }
                MessageContent::Response {
                    answer,
                    response_type,
                } => {
                    let answer_text = answer.as_deref().unwrap_or("(no answer)");
                    (
                        format!("📤 Response: {} ({:?})", answer_text, response_type),
                        Color::Blue,
                    )
                }
                MessageContent::Navigate { url } => {
                    (format!("🌐 Navigate to: {}", url), Color::Cyan)
                }
            };

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
        };

        let message_lines: Vec<Line> = if messages.is_empty() {
            vec![Line::from("No messages in this channel yet.")]
        } else {
            messages
                .iter()
                .rev() // Show newest first
                .take(30) // Limit display
                .map(format_fn)
                .collect()
        };

        // Prepare all data for the closure (clone what we need)
        let channels_for_draw = channels.clone();
        let message_lines_for_draw = message_lines.clone();
        let current_channel_for_draw = current_channel.clone();
        let message_count = messages.len();
        let scroll_offset = (message_count.saturating_sub(30)).max(0) as u16;

        // Now do the terminal drawing
        self.terminal.draw(|f| {
            let size = f.size();

            // Create layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Header
                    Constraint::Min(10),   // Main content
                    Constraint::Length(3), // Footer
                ])
                .split(size);

            // Header
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
            f.render_widget(header, chunks[0]);

            // Main content area - split into channels list and messages
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(25), Constraint::Percentage(75)])
                .split(chunks[1]);

            // Left side - Channel list (using prepared data)
            let channel_items: Vec<ListItem> = channels_for_draw
                .iter()
                .map(|ch| {
                    let style = if ch == &current_channel_for_draw {
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
            f.render_widget(channel_list, main_chunks[0]);

            // Right side - Messages (using prepared data)
            let message_widget = Paragraph::new(message_lines_for_draw)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!("Messages - {}", current_channel_for_draw)),
                )
                .wrap(Wrap { trim: true })
                .scroll((scroll_offset, 0));
            f.render_widget(message_widget, main_chunks[1]);

            // Footer
            let footer_text = format!(
                "Tab: Switch channel | q: Quit | Channel: {} | Messages: {}",
                current_channel_for_draw, message_count
            );
            let footer = Paragraph::new(footer_text)
                .block(Block::default().borders(Borders::ALL))
                .alignment(Alignment::Center);
            f.render_widget(footer, chunks[2]);
        })?;

        Ok(())
    }

    /// Format a message for display
    fn format_message(&self, message: &Message) -> Line<'_> {
        let timestamp = message.timestamp.format("%H:%M:%S").to_string();

        // Extract agent type from metadata
        let agent_type = message
            .metadata
            .as_ref()
            .and_then(|m| m.get("agent_type"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let (content_text, color) = match &message.content {
            MessageContent::Notification { text, priority } => {
                let color = match priority {
                    NotificationPriority::Urgent => Color::Red,
                    NotificationPriority::High => Color::Yellow,
                    NotificationPriority::Normal => Color::Green,
                    NotificationPriority::Low => Color::Blue,
                };
                (text.clone(), color)
            }
            MessageContent::Question { text, .. } => (format!("❓ {}", text), Color::Cyan),
            MessageContent::Authorization { action, .. } => {
                (format!("🔐 Authorization: {}", action), Color::Magenta)
            }
            MessageContent::Response {
                answer,
                response_type,
            } => {
                let answer_text = answer.as_deref().unwrap_or("(no answer)");
                (
                    format!("📤 Response: {} ({:?})", answer_text, response_type),
                    Color::Blue,
                )
            }
            MessageContent::Navigate { url } => (format!("🌐 Navigate to: {}", url), Color::Cyan),
        };

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

    /// Handle keyboard input for channel switching
    pub async fn handle_input(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            return Ok(true); // Quit
                        }
                        KeyCode::Tab => {
                            // Switch to next channel
                            let channels = self.channels.read().await;
                            if !channels.is_empty() {
                                let current = self.current_channel.read().await.clone();
                                let current_idx =
                                    channels.iter().position(|c| c == &current).unwrap_or(0);
                                let next_idx = (current_idx + 1) % channels.len();
                                let mut current_ch = self.current_channel.write().await;
                                *current_ch = channels[next_idx].clone();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(false) // Continue
    }

    /// Suspend terminal UI (exit alternate screen, disable raw mode)
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

    /// Resume terminal UI (enter alternate screen, enable raw mode)
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
        let _ = self.cleanup(); // Best effort cleanup
    }
}
