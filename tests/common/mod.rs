use rustylight_server::api::{build_router, AppState};
use rustylight_server::device::{LightState, SharedState};
use std::sync::{Arc, Mutex};

pub fn test_psk() -> Vec<u8> {
    b"test-psk-for-integration-tests!!".to_vec()
}

pub fn make_app(connected: bool) -> axum::Router {
    let shared = Arc::new(Mutex::new(SharedState {
        connected,
        light_state: LightState::default(),
        state_dirty: false,
    }));
    let state = AppState {
        psk: Arc::new(test_psk()),
        shared,
    };
    build_router(state)
}

pub fn auth_headers(body: &[u8]) -> Vec<(&'static str, String)> {
    use rustylight_server::api::auth::{compute_signature, current_unix_time};
    let ts = current_unix_time().to_string();
    let sig = compute_signature(&test_psk(), &ts, body);
    vec![("X-Timestamp", ts), ("X-Signature", sig)]
}
