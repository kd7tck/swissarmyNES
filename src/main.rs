use swissarmynes::server;

#[tokio::main]
async fn main() {
    // build our application with a route to serve static files
    let app = server::app();

    // run it with hyper on localhost:3000
    // Warning: Bound to 127.0.0.1 for local development security. Change to 0.0.0.0 for external access (e.g. Docker)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
