mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn get_light_returns_connected_false_when_device_absent() {
    let app = common::make_app(false);
    let headers = common::auth_headers(b"");
    let mut builder = Request::builder().method("GET").uri("/api/light");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app
        .oneshot(builder.body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["connected"], false);
}

#[tokio::test]
async fn post_light_returns_503_when_device_not_connected() {
    let app = common::make_app(false);
    let body = serde_json::json!({"on": true, "r": 255, "g": 0, "b": 0}).to_string();
    let body_bytes = body.as_bytes().to_vec();
    let headers = common::auth_headers(&body_bytes);
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/light")
        .header("Content-Type", "application/json");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn post_light_returns_200_when_device_connected() {
    let app = common::make_app(true);
    let body = serde_json::json!({"on": true, "r": 0, "g": 255, "b": 0}).to_string();
    let body_bytes = body.as_bytes().to_vec();
    let headers = common::auth_headers(&body_bytes);
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/light")
        .header("Content-Type", "application/json");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn post_light_returns_400_for_invalid_blink_ms() {
    let app = common::make_app(true);
    let body = serde_json::json!({
        "on": true, "r": 255, "g": 0, "b": 0,
        "blink": true, "blink_on_ms": 10, "blink_off_ms": 500
    })
    .to_string();
    let body_bytes = body.as_bytes().to_vec();
    let headers = common::auth_headers(&body_bytes);
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/light")
        .header("Content-Type", "application/json");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn post_light_returns_400_for_malformed_json() {
    let app = common::make_app(true);
    let body = "not json at all";
    let headers = common::auth_headers(body.as_bytes());
    let mut builder = Request::builder()
        .method("POST")
        .uri("/api/light")
        .header("Content-Type", "application/json");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let resp = app
        .oneshot(builder.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
