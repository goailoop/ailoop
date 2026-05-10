//! Embedded web UI served by `ailoop serve --web`

pub static UI_HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ailoop-ui.html"
));
