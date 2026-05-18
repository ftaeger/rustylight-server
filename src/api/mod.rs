pub mod auth;
pub mod handlers;
pub mod openapi;

use crate::device::SharedState;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppState {
    pub psk: Arc<String>,
    pub shared: Arc<Mutex<SharedState>>,
    pub log_file: Arc<String>,
}

pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/light", get(handlers::get_light))
        .route("/light", post(handlers::post_light))
        .route("/public/version", get(handlers::get_version))
        .with_state(state);

    let swagger_routes = openapi::swagger_router();

    Router::new().nest("/api", api_routes).merge(swagger_routes)
}
