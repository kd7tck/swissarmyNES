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
        .route(
            "/api/projects/:name/files",
            get(api::list_files).post(api::create_file),
        )
        .route(
            "/api/projects/:name/files/:filename",
            get(api::get_file)
                .post(api::save_file)
                .delete(api::delete_file),
        )
        .nest_service("/", ServeDir::new("static"))
}
