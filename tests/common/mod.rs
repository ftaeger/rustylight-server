use rustylight_server::api::{build_router, AppState};
use rustylight_server::device::{LightState, SharedState};
use std::sync::{Arc, Mutex};

pub fn test_psk() -> String {
    "test-psk-for-integration-tests!!".to_string()
}

pub fn make_app(connected: bool) -> axum::Router {
    let log_file = std::env::temp_dir()
        .join("rustylight-test.log")
        .to_str()
        .unwrap()
        .to_string();
    make_app_with_log(connected, log_file)
}

pub fn make_app_with_log(connected: bool, log_file: String) -> axum::Router {
    let shared = Arc::new(Mutex::new(SharedState {
        connected,
        light_state: LightState::default(),
        state_dirty: false,
    }));
    let state = AppState {
        psk: Arc::new(test_psk()),
        shared,
        log_file: Arc::new(log_file),
    };
    build_router(state)
}

#[allow(dead_code)]
pub fn auth_headers() -> Vec<(&'static str, String)> {
    vec![("X-Api-Key", test_psk())]
}
