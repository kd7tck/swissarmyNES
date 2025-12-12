#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        Router,
    };
    use tower::ServiceExt;
    use tower_http::services::ServeDir;

    #[tokio::test]
    async fn test_static_file_serving() {
        let app = Router::new()
            .nest_service("/", ServeDir::new("static"));

        // Test index.html
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/index.html").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test css/style.css
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/css/style.css").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

         // Test js/app.js
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/js/app.js").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test 404
         let response = app
            .clone()
            .oneshot(Request::builder().uri("/nonexistent.html").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
