use axum::{
    http::{header::CACHE_CONTROL, HeaderValue},
    routing::{get, post},
    Router,
};
use tower_http::{services::ServeDir, set_header::SetResponseHeader};

pub mod api;
pub mod game_combat;
pub mod game_design;
pub mod game_events;
pub mod game_spawners;
pub mod game_visuals;
pub mod project;

pub fn app() -> Router {
    // IDE assets have stable names, so cached JS/WASM must be revalidated after edits.
    let static_files = SetResponseHeader::if_not_present(
        ServeDir::new("static"),
        CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
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
        .route("/api/projects/:name/flow/test", post(api::test_game_flow))
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
        .nest_service("/", static_files)
}
