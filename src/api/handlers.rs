use axum::{
    body::Bytes,
    extract::{ConnectInfo, Extension, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use std::net::SocketAddr;

use time::format_description::well_known::Rfc3339;

use crate::{
    api::{auth::AuthGuard, AppState},
    device::LightState,
};

pub fn validate_post_body(state: &LightState) -> Result<(), String> {
    if state.blink {
        let on_ms = state.blink_on_ms.unwrap_or(500);
        let off_ms = state.blink_off_ms.unwrap_or(500);
        if !(50..=10000).contains(&on_ms) {
            return Err(format!("blink_on_ms must be 50–10000, got {on_ms}"));
        }
        if !(50..=10000).contains(&off_ms) {
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
        (status = 401, description = "Missing or invalid API key"),
        (status = 503, description = "Busylight not connected"),
    ),
    params(
        ("X-Api-Key" = String, Header, description = "Pre-shared API key from config"),
    )
)]
pub async fn get_light(
    addr: Option<Extension<ConnectInfo<SocketAddr>>>,
    State(state): State<AppState>,
    _auth: AuthGuard,
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
        (status = 401, description = "Missing or invalid API key"),
        (status = 503, description = "Busylight not connected"),
    ),
    params(
        ("X-Api-Key" = String, Header, description = "Pre-shared API key from config"),
    )
)]
pub async fn post_light(
    addr: Option<Extension<ConnectInfo<SocketAddr>>>,
    State(state): State<AppState>,
    _auth: AuthGuard,
    body: Bytes,
) -> impl IntoResponse {
    let light_state: LightState = match serde_json::from_slice(&body) {
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

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct VersionResponse {
    pub version: &'static str,
    pub time: String,
}

#[utoipa::path(
    get,
    path = "/api/public/version",
    responses(
        (status = 200, description = "Server version and current UTC time", body = VersionResponse),
    )
)]
pub async fn get_version() -> impl IntoResponse {
    let time = time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());
    Json(VersionResponse {
        version: env!("CARGO_PKG_VERSION"),
        time,
    })
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: &'static str,
    pub busylight_connected: bool,
    pub log_writable: bool,
}

#[utoipa::path(
    get,
    path = "/api/public/healthcheck",
    responses(
        (status = 200, description = "All checks pass", body = HealthResponse),
        (status = 503, description = "One or more checks failed", body = HealthResponse),
    )
)]
pub async fn get_healthcheck(State(state): State<AppState>) -> impl IntoResponse {
    let busylight_connected = state.shared.lock().unwrap().connected;
    let log_writable = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(state.log_file.as_str())
        .is_ok();

    let healthy = busylight_connected && log_writable;
    let response = HealthResponse {
        status: if healthy { "ok" } else { "degraded" },
        busylight_connected,
        log_writable,
    };
    let status_code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (status_code, Json(response)).into_response()
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
