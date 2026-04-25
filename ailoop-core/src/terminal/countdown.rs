use std::time::{Duration, Instant};

pub struct CountdownRenderer {
    timeout: Duration,
    start: Instant,
    last_rendered_secs: Option<u64>,
}

pub enum InputResult {
    Submitted(String),
    Cancelled,
    Timeout,
}

impl CountdownRenderer {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            start: Instant::now(),
            last_rendered_secs: None,
        }
    }

    pub fn remaining_secs(&self) -> u64 {
        let elapsed = self.start.elapsed();
        self.timeout
            .checked_sub(elapsed)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn needs_update(&self) -> bool {
        let current = self.remaining_secs();
        match self.last_rendered_secs {
            Some(last) => current != last,
            None => true,
        }
    }

    pub fn render_update(&mut self) -> Option<String> {
        if self.needs_update() {
            let remaining = self.remaining_secs();
            self.last_rendered_secs = Some(remaining);
            Some(format!("\r\x1B[2KTimeout: {} seconds", remaining))
        } else {
            None
        }
    }

    pub fn render_final(&self) -> String {
        "\r\x1B[2KTimeout: 0 seconds\n".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_renderer(timeout: Duration, elapsed: Duration) -> CountdownRenderer {
        let mut renderer = CountdownRenderer::new(timeout);
        renderer.start = Instant::now() - elapsed;
        renderer
    }

    #[test]
    fn test_initial_needs_update() {
        let renderer = make_renderer(Duration::from_secs(10), Duration::ZERO);
        assert!(renderer.needs_update());
    }

    #[test]
    fn test_initial_render_update() {
        let mut renderer = make_renderer(Duration::from_secs(100), Duration::ZERO);
        let result = renderer.render_update();
        assert!(result.is_some());
        let text = result.unwrap();
        assert!(text.contains("Timeout:"));
        assert!(text.contains("seconds"));
        assert!(text.starts_with('\r'));
        assert!(text.contains('\x1B'));
        let remaining = renderer.remaining_secs();
        assert!((99..=100).contains(&remaining));
    }

    #[test]
    fn test_no_update_when_second_unchanged() {
        let mut renderer = make_renderer(Duration::from_secs(100), Duration::ZERO);
        let _ = renderer.render_update();
        assert!(!renderer.needs_update());
        assert!(renderer.render_update().is_none());
    }

    #[test]
    fn test_remaining_secs_at_zero() {
        let renderer = make_renderer(Duration::from_secs(0), Duration::from_secs(1));
        assert_eq!(renderer.remaining_secs(), 0);
    }

    #[test]
    fn test_render_final() {
        let renderer = CountdownRenderer::new(Duration::from_secs(5));
        let final_text = renderer.render_final();
        assert!(final_text.contains("Timeout: 0 seconds"));
        assert!(final_text.ends_with('\n'));
    }

    #[test]
    fn test_render_update_tracks_last_rendered() {
        let mut renderer = make_renderer(Duration::from_secs(100), Duration::ZERO);
        let _ = renderer.render_update();
        let tracked = renderer.last_rendered_secs.unwrap();
        assert!((99..=100).contains(&tracked));
    }

    #[test]
    fn test_remaining_secs_decreases() {
        let renderer = make_renderer(Duration::from_secs(10), Duration::from_millis(2500));
        assert_eq!(renderer.remaining_secs(), 7);
    }
}
