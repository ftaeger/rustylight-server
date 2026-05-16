use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init(level: &str, log_file: &str) -> Result<()> {
    let filter = EnvFilter::try_new(level)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let log_dir = std::path::Path::new(log_file)
        .parent()
        .unwrap_or(std::path::Path::new("/var/log/rustylight"));
    let file_name = std::path::Path::new(log_file)
        .file_name()
        .unwrap_or(std::ffi::OsStr::new("rustylight.log"));

    let file_appender = tracing_appender::rolling::never(log_dir, file_name);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Leak the guard so it lives for the process lifetime
    Box::leak(Box::new(guard));

    fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    Ok(())
}
