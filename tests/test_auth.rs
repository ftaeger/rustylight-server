mod common;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn missing_timestamp_header_returns_401() {
    let app = common::make_app(true);
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Signature", "deadbeef")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn missing_signature_header_returns_401() {
    let app = common::make_app(true);
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Timestamp", "1747394400")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn invalid_signature_returns_403() {
    let app = common::make_app(true);
    use rustylight_server::api::auth::current_unix_time;
    let ts = current_unix_time().to_string();
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Timestamp", &ts)
        .header("X-Signature", "badsignature")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn stale_timestamp_returns_403_with_server_time_header() {
    let app = common::make_app(true);
    let stale_ts = "1000000000";
    let sig = rustylight_server::api::auth::compute_signature(&common::test_psk(), stale_ts, b"");
    let req = Request::builder()
        .method("GET")
        .uri("/api/light")
        .header("X-Timestamp", stale_ts)
        .header("X-Signature", sig)
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    assert!(resp.headers().contains_key("X-Server-Time"));
}

#[tokio::test]
async fn valid_auth_on_get_returns_200() {
    let app = common::make_app(true);
    let headers = common::auth_headers(b"");
    let mut builder = Request::builder().method("GET").uri("/api/light");
    for (k, v) in &headers {
        builder = builder.header(*k, v);
    }
    let req = builder.body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
