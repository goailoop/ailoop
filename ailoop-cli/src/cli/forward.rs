//! Forward command for streaming agent output to ailoop server

use crate::cli::message_converter::MessageConverter;
use crate::parser::{create_parser, InputFormat};
use ailoop_core::transport::factory::{create_transport, TransportConfig, TransportType};
use anyhow::{Context, Result};
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use tokio::io::AsyncBufReadExt;

/// Forward command configuration
pub struct ForwardConfig {
    pub channel: String,
    pub agent_type: Option<String>,
    pub format: InputFormat,
    pub transport_type: TransportType,
    pub url: Option<String>,
    pub file_path: Option<PathBuf>,
    pub client_id: Option<String>,
    pub input_file: Option<PathBuf>,
}

/// Execute the forward command
pub async fn execute_forward(config: ForwardConfig) -> Result<()> {
    // Validate channel name
    ailoop_core::channel::validation::validate_channel_name(&config.channel)
        .map_err(|e| anyhow::anyhow!("Invalid channel name: {}", e))?;

    // Create parser
    let mut parser = create_parser(config.agent_type.clone(), config.format)
        .context("Failed to create parser")?;

    // Create message converter
    let mut converter = MessageConverter::new(
        config.channel.clone(),
        config.client_id.clone(),
        parser.agent_type().to_string(),
    );

    // Create transport
    let transport_config = TransportConfig {
        transport_type: config.transport_type.clone(),
        url: config.url.clone(),
        file_path: config
            .file_path
            .clone()
            .map(|p| p.to_string_lossy().to_string()),
        channel: config.channel.clone(),
        client_id: config.client_id.clone(),
    };
    let mut transport = create_transport(transport_config).context("Failed to create transport")?;

    // Determine input source
    if let Some(input_file) = config.input_file {
        // Read from file
        process_file_input(&mut *parser, &mut converter, &mut *transport, input_file).await?;
    } else {
        // Read from stdin
        process_stdin_input(&mut *parser, &mut converter, &mut *transport).await?;
    }

    // Flush and close transport
    transport
        .flush()
        .await
        .context("Failed to flush transport")?;
    transport
        .close()
        .await
        .context("Failed to close transport")?;

    Ok(())
}

/// Process input from stdin
async fn process_stdin_input(
    parser: &mut dyn crate::parser::AgentParser,
    converter: &mut MessageConverter,
    transport: &mut dyn ailoop_core::transport::Transport,
) -> Result<()> {
    let stdin = io::stdin();
    let reader = BufReader::new(stdin.lock());

    for line_result in reader.lines() {
        let line = line_result.context("Failed to read line from stdin")?;

        // Parse line (skip malformed lines with warning)
        match parser.parse_line(&line).await {
            Ok(Some(event)) => {
                // Convert event to messages
                let messages = converter.convert(event);

                // Send each message through transport
                for message in messages {
                    if let Err(e) = transport.send(message).await {
                        eprintln!("Warning: Failed to send message: {}", e);
                        // Continue processing despite transport errors
                    }
                }
            }
            Ok(None) => {
                // Line was skipped (empty or comment)
            }
            Err(e) => {
                // Malformed line - log warning and continue
                eprintln!("Warning: Failed to parse line (skipping): {}", e);
                eprintln!("  Line: {}", line);
            }
        }
    }

    Ok(())
}

/// Process input from file
async fn process_file_input(
    parser: &mut dyn crate::parser::AgentParser,
    converter: &mut MessageConverter,
    transport: &mut dyn ailoop_core::transport::Transport,
    file_path: PathBuf,
) -> Result<()> {
    let file = tokio::fs::File::open(&file_path)
        .await
        .with_context(|| format!("Failed to open file: {:?}", file_path))?;

    let mut reader = tokio::io::BufReader::new(file);
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        let line_trimmed = line.trim_end();

        // Parse line (skip malformed lines with warning)
        match parser.parse_line(line_trimmed).await {
            Ok(Some(event)) => {
                // Convert event to messages
                let messages = converter.convert(event);

                // Send each message through transport
                for message in messages {
                    if let Err(e) = transport.send(message).await {
                        eprintln!("Warning: Failed to send message: {}", e);
                        // Continue processing despite transport errors
                    }
                }
            }
            Ok(None) => {
                // Line was skipped (empty or comment)
            }
            Err(e) => {
                // Malformed line - log warning and continue
                eprintln!("Warning: Failed to parse line (skipping): {}", e);
                eprintln!("  Line: {}", line_trimmed);
            }
        }

        line.clear();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ailoop_core::models::{Message, MessageContent};
    use tempfile::NamedTempFile;

    fn write_temp_file(content: &str) -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;
        use std::io::Write;
        file.write_all(content.as_bytes())?;
        file.flush()?;
        Ok(file)
    }

    #[tokio::test]
    async fn test_forward_opencode_stream_json_to_file() -> Result<()> {
        let output_file = NamedTempFile::new()?;
        let output_path = output_file.path().to_path_buf();

        let input_content = r#"{"type":"step_start","timestamp":1700000000000,"sessionID":"sess-1","part":{"type":"step-start","snapshot":{}}}
{"type":"text","timestamp":1700000001000,"sessionID":"sess-1","part":{"type":"text","text":"Hello from OpenCode"}}
{"type":"step_finish","timestamp":1700000002000,"sessionID":"sess-1","part":{"type":"step-finish","reason":"stop","cost":1.2,"tokens":12}}
"#;
        let input_file = write_temp_file(input_content)?;

        let config = ForwardConfig {
            channel: "opencode-channel".to_string(),
            agent_type: Some("opencode".to_string()),
            format: InputFormat::StreamJson,
            transport_type: TransportType::File,
            url: None,
            file_path: Some(output_path.clone()),
            client_id: None,
            input_file: Some(input_file.path().to_path_buf()),
        };

        execute_forward(config).await?;

        let output = std::fs::read_to_string(&output_path)?;
        let mut messages = Vec::new();
        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let message: Message = serde_json::from_str(line)?;
            messages.push(message);
        }

        assert_eq!(messages.len(), 2);
        if let MessageContent::Notification { text, .. } = &messages[0].content {
            assert!(text.contains("Hello from OpenCode"));
        } else {
            anyhow::bail!("Expected notification message");
        }

        if let MessageContent::Notification { text, .. } = &messages[1].content {
            assert!(text.contains("Result"));
        } else {
            anyhow::bail!("Expected notification message");
        }

        Ok(())
    }
}
