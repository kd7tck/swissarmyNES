#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use swissarmynes::server;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_static_file_serving() {
        let app = server::app();

        // Test index.html
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/index.html")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test css/style.css
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/css/style.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test js/app.js
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/js/app.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Test 404
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/nonexistent.html")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
