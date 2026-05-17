mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn missing_api_key_returns_401() {
    let app = common::make_app(true);
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn wrong_api_key_returns_401() {
    let app = common::make_app(true);
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Api-Key", "wrong-key")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn valid_api_key_on_get_returns_200() {
    let app = common::make_app(true);
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Api-Key", common::test_psk())
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
