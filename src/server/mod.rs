use axum::{routing::post, Router};
use tower_http::services::ServeDir;

pub mod api;

pub fn app() -> Router {
    Router::new()
        .route("/api/compile", post(api::compile))
        .nest_service("/", ServeDir::new("static"))
}
