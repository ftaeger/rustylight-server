use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::net::SocketAddr;

use crate::{
    api::{auth::AuthGuard, AppState},
    device::LightState,
};

pub fn validate_post_body(state: &LightState) -> Result<(), String> {
    if state.blink {
        let on_ms = state.blink_on_ms.unwrap_or(500);
        let off_ms = state.blink_off_ms.unwrap_or(500);
        if on_ms < 50 || on_ms > 10000 {
            return Err(format!("blink_on_ms must be 50–10000, got {on_ms}"));
        }
        if off_ms < 50 || off_ms > 10000 {
            return Err(format!("blink_off_ms must be 50–10000, got {off_ms}"));
        }
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/light",
    responses(
        (status = 200, description = "Current busylight state"),
        (status = 401, description = "Missing auth headers"),
        (status = 403, description = "Invalid signature or timestamp"),
        (status = 503, description = "Busylight not connected"),
    ),
    params(
        ("X-Timestamp" = String, Header, description = "Unix timestamp (seconds UTC)"),
        ("X-Signature" = String, Header, description = "HMAC-SHA256(psk, timestamp+body) as lowercase hex"),
    )
)]
pub async fn get_light(
    _auth: AuthGuard,
    addr: Option<ConnectInfo<SocketAddr>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::debug!(
        "GET /api/light from {}",
        addr.map_or("unknown".to_owned(), |a| a.0.to_string())
    );
    let shared = state.shared.lock().unwrap();
    let mut body = serde_json::to_value(&shared.light_state).unwrap();
    body["connected"] = serde_json::Value::Bool(shared.connected);
    Json(body)
}

#[utoipa::path(
    post,
    path = "/api/light",
    request_body = LightState,
    responses(
        (status = 200, description = "State applied"),
        (status = 400, description = "Invalid request body"),
        (status = 401, description = "Missing auth headers"),
        (status = 403, description = "Invalid signature or timestamp"),
        (status = 503, description = "Busylight not connected"),
    ),
    params(
        ("X-Timestamp" = String, Header, description = "Unix timestamp (seconds UTC)"),
        ("X-Signature" = String, Header, description = "HMAC-SHA256(psk, timestamp+body) as lowercase hex"),
    )
)]
pub async fn post_light(
    AuthGuard(body_bytes): AuthGuard,
    addr: Option<ConnectInfo<SocketAddr>>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let light_state: LightState = match serde_json::from_slice(&body_bytes) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("invalid JSON: {e}")})),
            )
                .into_response();
        }
    };

    if let Err(msg) = validate_post_body(&light_state) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": msg})),
        )
            .into_response();
    }

    let mut shared = state.shared.lock().unwrap();

    if !shared.connected {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Busylight not connected"})),
        )
            .into_response();
    }

    tracing::debug!(
        "POST /api/light from {}",
        addr.map_or("unknown".to_owned(), |a| a.0.to_string())
    );
    shared.light_state = light_state;
    shared.state_dirty = true;

    (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::LightState;

    #[test]
    fn validate_light_state_rejects_blink_ms_below_minimum() {
        let state = LightState {
            on: true,
            r: 255,
            g: 0,
            b: 0,
            blink: true,
            blink_on_ms: Some(40),
            blink_off_ms: Some(500),
            ..Default::default()
        };
        assert!(validate_post_body(&state).is_err());
    }

    #[test]
    fn validate_light_state_rejects_blink_ms_above_maximum() {
        let state = LightState {
            on: true,
            r: 255,
            g: 0,
            b: 0,
            blink: true,
            blink_on_ms: Some(500),
            blink_off_ms: Some(11000),
            ..Default::default()
        };
        assert!(validate_post_body(&state).is_err());
    }

    #[test]
    fn validate_light_state_accepts_valid_blink() {
        let state = LightState {
            on: true,
            r: 255,
            g: 0,
            b: 0,
            blink: true,
            blink_on_ms: Some(500),
            blink_off_ms: Some(500),
            ..Default::default()
        };
        assert!(validate_post_body(&state).is_ok());
    }
}
