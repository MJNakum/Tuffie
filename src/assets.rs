//! Serves the web UI that is embedded in the binary.

use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/web/build"]
struct Ui;

pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if let Some(r) = file(path) {
        return r;
    }
    // Single-page app: unknown paths get index.html.
    match file("index.html") {
        Some(r) => r,
        None => (
            StatusCode::NOT_FOUND,
            "The web UI is not built. Run `npm --prefix web ci && npm --prefix web run build`, then rebuild Tuffie.",
        )
            .into_response(),
    }
}

fn file(path: &str) -> Option<Response> {
    let f = Ui::get(if path.is_empty() { "index.html" } else { path })?;
    let mime = mime_guess::from_path(path).first_or(mime_guess::mime::TEXT_HTML);
    // Hashed asset files never change; index.html must always revalidate.
    let cache = if path.starts_with("_app/immutable/") { "public, max-age=31536000, immutable" } else { "no-cache" };
    Some(([(header::CONTENT_TYPE, mime.as_ref().to_string()), (header::CACHE_CONTROL, cache.to_string())], f.data).into_response())
}
