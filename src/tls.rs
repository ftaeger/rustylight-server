use anyhow::{Context, Result};
use axum_server::tls_rustls::RustlsConfig;
use rcgen::{CertificateParams, KeyPair, PKCS_ECDSA_P256_SHA256};
use std::path::Path;

pub fn generate_self_signed() -> Result<(String, String)> {
    let key_pair = KeyPair::generate_for(&PKCS_ECDSA_P256_SHA256)
        .context("generating ECC key pair")?;
    let mut params = CertificateParams::new(vec!["rustylight".to_owned()])
        .context("creating cert params")?;
    params.not_after = time::OffsetDateTime::now_utc()
        .checked_add(time::Duration::days(3650))
        .context("computing cert expiry")?;
    let cert = params.self_signed(&key_pair).context("self-signing cert")?;
    Ok((cert.pem(), key_pair.serialize_pem()))
}

pub fn load_or_generate(cert_path: &str, key_path: &str) -> Result<()> {
    let cert_missing = !Path::new(cert_path).exists();
    let key_missing = !Path::new(key_path).exists();
    if cert_missing || key_missing {
        let (cert_pem, key_pem) = generate_self_signed()
            .context("generating self-signed TLS certificate")?;
        std::fs::write(cert_path, cert_pem)
            .with_context(|| format!("writing cert to {cert_path}"))?;
        std::fs::write(key_path, key_pem)
            .with_context(|| format!("writing key to {key_path}"))?;
        tracing::info!("generated self-signed TLS certificate at {cert_path}");
    }
    Ok(())
}

pub async fn rustls_config(cert_path: &str, key_path: &str) -> Result<RustlsConfig> {
    RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .context("loading TLS config from PEM files")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_pem_cert_and_key() {
        let (cert_pem, key_pem) = generate_self_signed().unwrap();
        assert!(cert_pem.contains("BEGIN CERTIFICATE"));
        assert!(key_pem.contains("BEGIN"));
    }

    #[test]
    fn load_or_generate_creates_files_if_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cert_path = dir.path().join("tls.crt").to_str().unwrap().to_owned();
        let key_path = dir.path().join("tls.key").to_str().unwrap().to_owned();
        load_or_generate(&cert_path, &key_path).unwrap();
        assert!(std::path::Path::new(&cert_path).exists());
        assert!(std::path::Path::new(&key_path).exists());
    }
}
