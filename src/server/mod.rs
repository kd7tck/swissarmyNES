use axum::Router;
use tower_http::services::ServeDir;

pub fn app() -> Router {
    Router::new().nest_service("/", ServeDir::new("static"))
}
