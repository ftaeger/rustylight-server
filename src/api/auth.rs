use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

pub const TIMESTAMP_WINDOW_SECS: u64 = 30;

pub fn current_unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn compute_signature(psk: &[u8], timestamp: &str, body: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(psk).expect("HMAC accepts any key size");
    mac.update(timestamp.as_bytes());
    mac.update(body);
    hex::encode(mac.finalize().into_bytes())
}

pub fn verify_signature(psk: &[u8], timestamp: &str, body: &[u8], provided_hex: &str) -> bool {
    let Ok(provided_bytes) = hex::decode(provided_hex) else {
        return false;
    };
    let mut mac = HmacSha256::new_from_slice(psk).expect("HMAC accepts any key size");
    mac.update(timestamp.as_bytes());
    mac.update(body);
    mac.verify_slice(&provided_bytes).is_ok()
}

pub fn timestamp_in_window(ts: u64) -> bool {
    let now = current_unix_time();
    now.abs_diff(ts) <= TIMESTAMP_WINDOW_SECS
}

pub struct AuthGuard(pub Bytes);

pub enum AuthError {
    MissingHeader(&'static str),
    NonNumericTimestamp,
    TimestampOutOfWindow { server_time: u64 },
    InvalidSignature,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::MissingHeader(h) => (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": format!("missing header: {h}")})),
            )
                .into_response(),
            AuthError::NonNumericTimestamp => (
                StatusCode::UNAUTHORIZED,
                axum::Json(serde_json::json!({"error": "X-Timestamp must be a unix timestamp"})),
            )
                .into_response(),
            AuthError::TimestampOutOfWindow { server_time } => {
                let mut resp = (
                    StatusCode::FORBIDDEN,
                    axum::Json(serde_json::json!({"error": "timestamp outside ±30s window"})),
                )
                    .into_response();
                resp.headers_mut().insert(
                    "X-Server-Time",
                    server_time.to_string().parse().unwrap(),
                );
                resp
            }
            AuthError::InvalidSignature => (
                StatusCode::FORBIDDEN,
                axum::Json(serde_json::json!({"error": "invalid signature"})),
            )
                .into_response(),
        }
    }
}

#[axum::async_trait]
impl<S> FromRequest<S> for AuthGuard
where
    S: Send + Sync,
    crate::api::AppState: axum::extract::FromRef<S>,
{
    type Rejection = AuthError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        use axum::extract::FromRef;
        let app_state = crate::api::AppState::from_ref(state);
        let psk = app_state.psk.as_ref();

        let ts_header = req
            .headers()
            .get("X-Timestamp")
            .ok_or(AuthError::MissingHeader("X-Timestamp"))?
            .to_str()
            .map_err(|_| AuthError::NonNumericTimestamp)?
            .to_owned();

        let sig_header = req
            .headers()
            .get("X-Signature")
            .ok_or(AuthError::MissingHeader("X-Signature"))?
            .to_str()
            .map_err(|_| AuthError::InvalidSignature)?
            .to_owned();

        let ts: u64 = ts_header
            .parse()
            .map_err(|_| AuthError::NonNumericTimestamp)?;

        if !timestamp_in_window(ts) {
            return Err(AuthError::TimestampOutOfWindow {
                server_time: current_unix_time(),
            });
        }

        let body = Bytes::from_request(req, state)
            .await
            .unwrap_or_default();

        if !verify_signature(psk, &ts_header, &body, &sig_header) {
            return Err(AuthError::InvalidSignature);
        }

        Ok(AuthGuard(body))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn psk() -> Vec<u8> {
        b"test-psk-bytes-32-chars-xxxxxxxxx".to_vec()
    }

    #[test]
    fn valid_signature_returns_ok() {
        let ts = "1747394400";
        let body = b"{}";
        let sig = compute_signature(&psk(), ts, body);
        assert!(verify_signature(&psk(), ts, body, &sig));
    }

    #[test]
    fn wrong_signature_returns_false() {
        let ts = "1747394400";
        assert!(!verify_signature(&psk(), ts, b"{}", "deadbeef"));
    }

    #[test]
    fn timestamp_within_window_is_ok() {
        let now = current_unix_time();
        assert!(timestamp_in_window(now));
        assert!(timestamp_in_window(now + 29));
        assert!(timestamp_in_window(now - 29));
    }

    #[test]
    fn timestamp_outside_window_is_rejected() {
        let now = current_unix_time();
        assert!(!timestamp_in_window(now + 31));
        assert!(!timestamp_in_window(now - 31));
    }
}
