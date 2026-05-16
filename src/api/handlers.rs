use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use std::net::SocketAddr;
use axum::extract::ConnectInfo;
use crate::{api::AppState, device::LightState};

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

pub async fn get_light(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    Json(serde_json::json!({"connected": false, "on": false, "r": 0, "g": 0, "b": 0, "blink": false}))
}

pub async fn post_light(
    State(_state): State<AppState>,
    Json(_body): Json<LightState>,
) -> impl IntoResponse {
    StatusCode::OK
}
