use axum::{
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

pub mod api;
pub mod project;

pub fn app() -> Router {
    Router::new()
        .route("/api/compile", post(api::compile))
        .route(
            "/api/projects",
            get(api::list_projects).post(api::create_project),
        )
        .route(
            "/api/projects/:name",
            get(api::get_project).post(api::save_project),
        )
        .nest_service("/", ServeDir::new("static"))
}
