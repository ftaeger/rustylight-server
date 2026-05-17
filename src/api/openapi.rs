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
            Every `/api/light` request requires an `X-Api-Key` header containing \
            the PSK from `/etc/rustylight/rustylight.conf`."
    )
)]
pub struct ApiDoc;

pub fn swagger_router() -> Router {
    Router::new().merge(SwaggerUi::new("/api").url("/api/openapi.json", ApiDoc::openapi()))
}
