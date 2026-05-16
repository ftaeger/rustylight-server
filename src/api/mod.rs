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
    pub psk: Arc<Vec<u8>>,
    pub shared: Arc<Mutex<SharedState>>,
}

pub fn build_router(state: AppState) -> Router {
    let api_routes = Router::new()
        .route("/light", get(handlers::get_light))
        .route("/light", post(handlers::post_light))
        .with_state(state);

    let swagger_routes = openapi::swagger_router();

    Router::new()
        .nest("/api", api_routes)
        .merge(swagger_routes)
}
