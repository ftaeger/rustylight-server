use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::handlers;
use crate::device::LightState;

#[derive(OpenApi)]
#[openapi(
    paths(handlers::get_light, handlers::post_light),
    components(schemas(LightState)),
    info(
        title = "rustylight-server API",
        version = "0.1.0",
        description = "REST API for controlling a Kuando Busylight USB device.\n\n\
            ## Authentication\n\
            Every `/api/light` request requires two headers:\n\
            - `X-Timestamp`: Unix timestamp (seconds, UTC)\n\
            - `X-Signature`: `HMAC-SHA256(psk_bytes, timestamp_string + request_body)` as lowercase hex\n\n\
            The server rejects requests with timestamps outside ±30 seconds of server time."
    )
)]
pub struct ApiDoc;

pub fn swagger_router() -> Router {
    Router::new().merge(SwaggerUi::new("/api").url("/api/openapi.json", ApiDoc::openapi()))
}
