//! File transport implementation for testing and output

use super::Transport;
use crate::models::Message;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// File transport for writing messages to a file (useful for testing and debugging)
pub struct FileTransport {
    file_path: PathBuf,
    _channel: String,
    file: Option<std::fs::File>,
}

impl FileTransport {
    /// Create a new file transport
    pub fn new(file_path: impl Into<PathBuf>, _channel: String) -> Result<Self> {
        let path = file_path.into();
        Ok(Self {
            file_path: path,
            _channel,
            file: None,
        })
    }

    /// Open or get the file handle
    fn get_file(&mut self) -> Result<&mut std::fs::File> {
        if self.file.is_none() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.file_path)
                .with_context(|| format!("Failed to open file: {:?}", self.file_path))?;
            self.file = Some(file);
        }
        Ok(self.file.as_mut().unwrap())
    }
}

#[async_trait]
impl Transport for FileTransport {
    async fn send(&mut self, message: Message) -> Result<()> {
        let json = serde_json::to_string(&message).context("Failed to serialize message")?;

        let file_path = self.file_path.clone();
        let file = self.get_file()?;
        writeln!(file, "{}", json)
            .with_context(|| format!("Failed to write to file: {:?}", file_path))?;

        file.flush()
            .with_context(|| format!("Failed to flush file: {:?}", file_path))?;

        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        if let Some(file) = &mut self.file {
            file.flush()
                .with_context(|| format!("Failed to flush file: {:?}", self.file_path))?;
        }
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.flush().await?;
        self.file = None;
        Ok(())
    }

    fn name(&self) -> &str {
        "file"
    }
}
