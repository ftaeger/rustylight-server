use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const CONFIG_PATH: &str = "/etc/rustylight/rustylight.conf";

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub server: ServerConfig,
    pub tls: TlsConfig,
    pub auth: AuthConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { port: 8443 }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TlsConfig {
    pub cert_file: String,
    pub key_file: String,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            cert_file: "/etc/rustylight/tls.crt".to_owned(),
            key_file: "/etc/rustylight/tls.key".to_owned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AuthConfig {
    pub psk: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
    pub log_file: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_owned(),
            log_file: "/var/log/rustylight/rustylight.log".to_owned(),
        }
    }
}

pub fn load_or_create(path: &str) -> Result<Config> {
    if Path::new(path).exists() {
        let raw =
            std::fs::read_to_string(path).with_context(|| format!("reading config at {path}"))?;
        let cfg: Config = toml::from_str(&raw).context("parsing config TOML")?;
        Ok(cfg)
    } else {
        Ok(Config::default())
    }
}

pub fn ensure_psk(cfg: &mut Config, path: &str) -> Result<()> {
    if cfg.auth.psk.is_empty() {
        cfg.auth.psk = generate_psk();
        save(cfg, path).context("writing generated PSK to config")?;
    }
    Ok(())
}

fn save(cfg: &Config, path: &str) -> Result<()> {
    let content = toml::to_string_pretty(cfg).context("serialising config")?;
    std::fs::write(path, content).with_context(|| format!("writing config to {path}"))?;
    Ok(())
}

pub fn generate_psk() -> String {
    let bytes: [u8; 32] = rand::random();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_correct() {
        let cfg = Config::default();
        assert_eq!(cfg.server.port, 8443);
        assert_eq!(cfg.tls.cert_file, "/etc/rustylight/tls.crt");
        assert_eq!(cfg.logging.level, "info");
        assert_eq!(cfg.auth.psk, "");
    }

    #[test]
    fn parses_full_toml() {
        let toml_str = r#"
[server]
port = 9443

[tls]
cert_file = "/tmp/tls.crt"
key_file = "/tmp/tls.key"

[auth]
psk = "abc123"

[logging]
level = "debug"
log_file = "/tmp/rustylight.log"
"#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.server.port, 9443);
        assert_eq!(cfg.auth.psk, "abc123");
        assert_eq!(cfg.logging.level, "debug");
    }

    #[test]
    fn parses_partial_toml_with_defaults() {
        let toml_str = "[auth]\npsk = \"mykey\"";
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.server.port, 8443);
        assert_eq!(cfg.auth.psk, "mykey");
    }

    #[test]
    fn generates_non_empty_psk() {
        let psk = generate_psk();
        assert!(!psk.is_empty());
        assert_eq!(psk.len(), 64);
    }
}
