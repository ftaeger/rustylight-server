pub mod manager;
pub mod models;
pub mod report;

use thiserror::Error;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default, PartialEq, utoipa::ToSchema)]
pub struct LightState {
    pub on: bool,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    #[serde(default)]
    pub blink: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blink_on_ms: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blink_off_ms: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r2: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub g2: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub b2: Option<u8>,
}

impl LightState {
    pub fn effective_blink_on_ms(&self) -> u16 {
        self.blink_on_ms.unwrap_or(500)
    }

    pub fn effective_blink_off_ms(&self) -> u16 {
        self.blink_off_ms.unwrap_or(500)
    }
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("device not connected")]
    NotConnected,
    #[error("HID error: {0}")]
    Hid(String),
}

pub trait BuslightDevice: Send + Sync {
    fn set_state(&self, state: &LightState) -> Result<(), DeviceError>;
    fn is_connected(&self) -> bool;
}

pub struct SharedState {
    pub connected: bool,
    pub light_state: LightState,
    pub state_dirty: bool,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            connected: false,
            light_state: LightState::default(),
            state_dirty: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_state_default_is_off() {
        let state = LightState::default();
        assert!(!state.on);
        assert!(!state.blink);
        assert_eq!(state.r, 0);
    }

    #[test]
    fn light_state_serialises_without_blink_fields_when_not_blinking() {
        let state = LightState { on: true, r: 255, g: 0, b: 0, blink: false, ..Default::default() };
        let json = serde_json::to_value(&state).unwrap();
        assert!(json.get("blink_on_ms").is_none());
        assert!(json.get("r2").is_none());
    }

    #[test]
    fn light_state_serialises_blink_fields_when_blinking() {
        let state = LightState {
            on: true, r: 255, g: 0, b: 0,
            blink: true, blink_on_ms: Some(500), blink_off_ms: Some(300),
            r2: Some(0), g2: Some(0), b2: Some(255),
        };
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["blink_on_ms"], 500);
        assert_eq!(json["b2"], 255);
    }
}
