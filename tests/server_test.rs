#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use swissarmynes::server;
    use tower::ServiceExt;
    use axum::http::Method;
    use std::fs;

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

    #[tokio::test]
    async fn test_project_api() {
        let app = server::app();

        // 1. Create a project
        let _ = fs::remove_dir_all("projects/test_api_project");

        let create_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/projects")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"name": "test_api_project"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_res.status(), StatusCode::CREATED);

        // 2. List projects
        let list_res = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/projects")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_res.status(), StatusCode::OK);

        // 3. Cleanup
        let _ = fs::remove_dir_all("projects/test_api_project");
    }
}
