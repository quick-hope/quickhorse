//! Tracing initialization for QuickHorse
//!
//! Provides logging setup with file and terminal output.

#![allow(dead_code)] // Future use: init functions

use tracing_subscriber::{
    fmt, EnvFilter, Layer, Registry,
    layer::SubscriberExt,
    util::SubscriberInitExt,
};
use std::fs::{self, File};
use std::io;
use std::sync::Arc;

use crate::log::config::LogConfig;

/// Initialize logging system with given configuration
pub fn init_logging(config: &LogConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure log directory exists
    if config.write_to_file {
        fs::create_dir_all(&config.log_dir)?;
    }

    // Build log level filter
    let level_filter = config.tracing_level();
    let env_filter = EnvFilter::builder()
        .with_default_directive(level_filter.parse().unwrap_or(tracing::Level::INFO.into()))
        .from_env_lossy();

    // Terminal layer (human-readable)
    let terminal_layer = if config.write_to_terminal {
        Some(fmt::layer()
            .with_writer(io::stdout)
            .with_target(config.debug)
            .with_file(config.debug)
            .with_line_number(config.debug)
            .with_ansi(true)
            .pretty()
            .with_filter(env_filter.clone()))
    } else {
        None
    };

    // File layer (JSON format)
    let file_layer = if config.write_to_file {
        let log_file = File::create(config.log_file_path())?;
        Some(fmt::layer()
            .with_writer(Arc::new(log_file))
            .with_target(true)
            .with_file(true)
            .with_line_number(true)
            .with_ansi(false)
            .json()
            .with_filter(env_filter))
    } else {
        None
    };

    // Combine layers
    let subscriber = Registry::default()
        .with(terminal_layer)
        .with(file_layer);

    subscriber.init();

    tracing::info!(
        log_dir = %config.log_dir.display(),
        level = level_filter,
        verbose = config.verbose,
        debug = config.debug,
        "Logging initialized"
    );

    Ok(())
}

/// Initialize default logging (quiet mode)
pub fn init_default() -> Result<(), Box<dyn std::error::Error>> {
    init_logging(&LogConfig::default())
}

/// Initialize verbose logging (info level)
pub fn init_verbose() -> Result<(), Box<dyn std::error::Error>> {
    init_logging(&LogConfig::verbose())
}

/// Initialize debug logging (debug level)
pub fn init_debug() -> Result<(), Box<dyn std::error::Error>> {
    init_logging(&LogConfig::debug())
}

/// Initialize logging from CLI flags
pub fn init_from_cli(verbose: bool, debug: bool) -> Result<(), Box<dyn std::error::Error>> {
    let config = if debug {
        LogConfig::debug()
    } else if verbose {
        LogConfig::verbose()
    } else {
        LogConfig::default()
    };

    init_logging(&config)
}