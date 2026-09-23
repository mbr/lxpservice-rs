//! Configures structured diagnostics independently of command output.

use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::EnvFilter;

/// Installs stderr diagnostics with an environment-overridable verbosity level.
pub fn init(verbosity: u8) -> Result<(), SetGlobalDefaultError> {
    let default_level = match verbosity {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
}
