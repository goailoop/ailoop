//! Embedded web UI served by `ailoop serve --web`

use warp::Filter;

pub static UI_HTML: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/ailoop-ui.html"
));

/// Warp route: GET / → serves the embedded HTML UI
pub fn ui_route() -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone
{
    warp::path::end().and(warp::get()).map(|| {
        warp::http::Response::builder()
            .status(200)
            .header("content-type", "text/html; charset=utf-8")
            .body(UI_HTML)
            .unwrap()
    })
}
