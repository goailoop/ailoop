use ailoop_core::terminal::countdown::{CountdownRenderer, InputResult};
use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, IsTerminal, Write};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

pub fn read_user_input_with_countdown(
    timeout: Duration,
    cancelled: Arc<AtomicBool>,
) -> Result<InputResult> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return read_user_input_fallback(timeout);
    }

    if enable_raw_mode().is_err() {
        return read_user_input_fallback(timeout);
    }

    let result = read_with_countdown_inner(timeout, cancelled);
    disable_raw_mode().ok();
    result
}

fn read_with_countdown_inner(timeout: Duration, cancelled: Arc<AtomicBool>) -> Result<InputResult> {
    let mut buffer = String::new();
    let mut countdown = CountdownRenderer::new(timeout);
    let mut countdown_enabled = true;

    println!("\x1B[s");
    io::stdout().flush().ok();

    loop {
        if cancelled.load(Ordering::Relaxed) {
            print!("\r\x1B[2K\x1B[u");
            io::stdout().flush().ok();
            println!();
            return Ok(InputResult::Cancelled);
        }

        if countdown.remaining_secs() == 0 {
            print!("{}", countdown.render_final());
            io::stdout().flush().ok();
            return Ok(InputResult::Timeout);
        }

        match event::poll(Duration::from_millis(100)) {
            Ok(true) => {
                if let Ok(Event::Key(key_event)) = event::read() {
                    if key_event.kind == KeyEventKind::Press {
                        match key_event.code {
                            KeyCode::Enter => {
                                print!("\r\x1B[2K\x1B[u");
                                io::stdout().flush().ok();
                                println!();
                                return Ok(InputResult::Submitted(buffer.trim().to_string()));
                            }
                            KeyCode::Esc => {
                                print!("\r\x1B[2K\x1B[u");
                                io::stdout().flush().ok();
                                println!();
                                return Ok(InputResult::Cancelled);
                            }
                            KeyCode::Char(c) => {
                                buffer.push(c);
                                print!("\x1B[u{}\x1B[s\x1B[B\r", c);
                                io::stdout().flush().ok();
                            }
                            KeyCode::Backspace if !buffer.is_empty() => {
                                buffer.pop();
                                print!("\x1B[u\x08 \x08\x1B[s\x1B[B\r");
                                io::stdout().flush().ok();
                            }
                            _ => {}
                        }
                    }
                }
            }
            Ok(false) => {
                if countdown_enabled {
                    if let Some(update) = countdown.render_update() {
                        let mut stdout = io::stdout();
                        if stdout.write_all(update.as_bytes()).is_ok() {
                            let _ = stdout.flush();
                        } else {
                            countdown_enabled = false;
                        }
                    }
                }
            }
            Err(_) => {
                print!("\r\x1B[2K\x1B[u");
                io::stdout().flush().ok();
                println!();
                return Ok(InputResult::Timeout);
            }
        }
    }
}

fn read_user_input_fallback(timeout: Duration) -> Result<InputResult> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut buffer = String::new();
        let result = io::stdin().read_line(&mut buffer);
        let _ = tx.send(result.map(|_| buffer));
    });
    match rx.recv_timeout(timeout) {
        Ok(Ok(buffer)) => Ok(InputResult::Submitted(buffer.trim().to_string())),
        Ok(Err(e)) => Err(e).context("Failed to read from stdin"),
        Err(_) => Ok(InputResult::Timeout),
    }
}
