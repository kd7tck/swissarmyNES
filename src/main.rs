use axum::Router;
use tower_http::services::ServeDir;

pub mod compiler;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() {
    // build our application with a route to serve static files
    let app = Router::new()
        .nest_service("/", ServeDir::new("static"));

    // run it with hyper on localhost:3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
