use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use subtle::ConstantTimeEq;

pub struct AuthGuard;

pub enum AuthError {
    MissingKey,
    InvalidKey,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::MissingKey => (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "missing header: X-Api-Key"})),
            )
                .into_response(),
            AuthError::InvalidKey => (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "invalid API key"})),
            )
                .into_response(),
        }
    }
}

impl<S> FromRequestParts<S> for AuthGuard
where
    S: Send + Sync,
    crate::api::AppState: axum::extract::FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        use axum::extract::FromRef;
        let app_state = crate::api::AppState::from_ref(state);

        let key = parts
            .headers
            .get("X-Api-Key")
            .ok_or(AuthError::MissingKey)?
            .to_str()
            .map_err(|_| AuthError::InvalidKey)?;

        let key_matches: bool = key.as_bytes().ct_eq(app_state.psk.as_bytes()).into();
        if !key_matches {
            return Err(AuthError::InvalidKey);
        }

        Ok(AuthGuard)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_error_missing_key_is_401() {
        let resp = AuthError::MissingKey.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn auth_error_invalid_key_is_401() {
        let resp = AuthError::InvalidKey.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }
}
