// File-based logging via tracing. Writes to ~/.local/share/clisten/clisten.log.

use tracing_appender::rolling;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init() -> anyhow::Result<()> {
    let data_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("clisten");
    std::fs::create_dir_all(&data_dir)?;

    let file_appender = rolling::never(&data_dir, "clisten.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(non_blocking).with_ansi(false))
        .with(EnvFilter::from_default_env().add_directive("clisten=debug".parse()?))
        .init();

    // The guard must outlive the program — leak it so the file writer stays open.
    std::mem::forget(guard);
    Ok(())
}
