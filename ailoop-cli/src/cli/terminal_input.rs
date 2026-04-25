use ailoop_core::terminal::countdown::{CountdownRenderer, InputResult};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, IsTerminal, Write};
use std::time::Duration;

pub fn read_user_input_with_countdown(timeout: Duration) -> Result<InputResult> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return read_user_input_fallback();
    }

    if enable_raw_mode().is_err() {
        return read_user_input_fallback();
    }

    let result = read_with_countdown_inner(timeout);
    disable_raw_mode().ok();
    result
}

fn read_with_countdown_inner(timeout: Duration) -> Result<InputResult> {
    let mut buffer = String::new();
    let mut countdown = CountdownRenderer::new(timeout);

    loop {
        let elapsed = countdown.remaining_secs();
        if elapsed == 0 {
            println!();
            return Ok(InputResult::Timeout);
        }

        match event::poll(Duration::from_millis(100)) {
            Ok(true) => {
                if let Ok(Event::Key(key_event)) = event::read() {
                    if key_event.kind == KeyEventKind::Press {
                        match key_event.code {
                            KeyCode::Enter => {
                                println!();
                                let answer = buffer.trim().to_string();
                                return Ok(InputResult::Submitted(answer));
                            }
                            KeyCode::Esc => {
                                println!();
                                return Ok(InputResult::Cancelled);
                            }
                            KeyCode::Char(c) => {
                                buffer.push(c);
                                print!("{}", c);
                                io::stdout().flush().ok();
                            }
                            KeyCode::Backspace if !buffer.is_empty() => {
                                buffer.pop();
                                print!("\x08 \x08");
                                io::stdout().flush().ok();
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(false) => {
                if let Some(update) = countdown.render_update() {
                    let mut stdout = io::stdout();
                    if stdout.write_all(update.as_bytes()).is_ok() {
                        let _ = stdout.flush();
                    }
                }
            }
            Err(_) => {
                return Ok(InputResult::Timeout);
            }
        }
    }
}

fn read_user_input_fallback() -> Result<InputResult> {
    let mut buffer = String::new();
    io::stdin()
        .read_line(&mut buffer)
        .context("Failed to read from stdin")?;
    Ok(InputResult::Submitted(buffer.trim().to_string()))
}
