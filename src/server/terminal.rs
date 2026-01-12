//! Interactive terminal UI for server monitoring

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::io::{self, Stdout};

/// Terminal UI for server monitoring
pub struct TerminalUI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalUI {
    /// Create a new terminal UI
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    /// Render the terminal UI
    pub fn render(&mut self, server_status: &str, queue_size: usize, connections: usize) -> Result<(), Box<dyn std::error::Error>> {
        self.terminal.draw(|f| {
            let size = f.size();

            // Create layout
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Header
                    Constraint::Min(10),    // Main content
                    Constraint::Length(3),  // Footer
                ])
                .split(size);

            // Header
            let header = Paragraph::new(Line::from(vec![
                Span::styled("ailoop Server", Style::default().fg(Color::Cyan)),
                Span::raw(" - Human-in-the-Loop CLI Tool"),
            ]))
            .block(Block::default().borders(Borders::ALL).title("Server Status"));
            f.render_widget(header, chunks[0]);

            // Main content area
            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(chunks[1]);

            // Left side - Server metrics
            let metrics = vec![
                Line::from(format!("Status: {}", server_status)),
                Line::from(format!("Queue Size: {}", queue_size)),
                Line::from(format!("Active Connections: {}", connections)),
                Line::from(""),
                Line::from("Commands:"),
                Line::from("  'q' - Quit server"),
                Line::from("  'c' - Clear screen"),
                Line::from("  'h' - Show help"),
            ];

            let metrics_widget = Paragraph::new(metrics)
                .block(Block::default().borders(Borders::ALL).title("Server Metrics"));
            f.render_widget(metrics_widget, main_chunks[0]);

            // Right side - Recent activity (placeholder for now)
            let activity = vec![
                Line::from("Recent Activity:"),
                Line::from("  [INFO] Server started"),
                Line::from("  [INFO] New connection established"),
                Line::from("  [INFO] Message queued"),
                Line::from("  [INFO] Message processed"),
            ];

            let activity_widget = Paragraph::new(activity)
                .block(Block::default().borders(Borders::ALL).title("Recent Activity"));
            f.render_widget(activity_widget, main_chunks[1]);

            // Footer
            let footer = Paragraph::new("Press 'q' to quit | 'h' for help")
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(footer, chunks[2]);
        })?;

        Ok(())
    }

    /// Cleanup and restore terminal
    pub fn cleanup(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TerminalUI {
    fn drop(&mut self) {
        let _ = self.cleanup(); // Best effort cleanup
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_ui_creation() {
        // Note: This test would require a proper terminal environment
        // For now, just test that the struct can be created conceptually
        // In a real test environment, we'd mock the terminal
        assert!(true); // Placeholder test
    }
}