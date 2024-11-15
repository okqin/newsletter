use once_cell::sync::Lazy;
use std::{path::Path, sync::Mutex};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

use crate::configuration::LogsSettings;

// Store guard in a global static variable to prevent the file writing thread from being destroyed
static GUARD: Lazy<Mutex<Option<WorkerGuard>>> = Lazy::new(|| Mutex::new(None));

pub fn setup_tracing(logs_conf: Option<&LogsSettings>) {
    // Extract common logic for getting log filter
    let get_filter = |conf: Option<&LogsSettings>| {
        conf.and_then(|c| c.directives.as_ref())
            .and_then(|directive| directive.parse::<EnvFilter>().ok())
    };

    // Configure file logging layer
    let file_layer = logs_conf.and_then(|c| c.path.as_ref()).map(|path| {
        let filter = get_filter(logs_conf);

        // Handle log file path
        let (path, filename) = if path.extension().is_some() {
            (
                path.parent().unwrap_or(Path::new("")),
                path.file_name().unwrap_or_default().as_ref(),
            )
        } else {
            (path.as_ref(), Path::new("newsletter.log"))
        };

        let file_appender = tracing_appender::rolling::daily(path, filename);
        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        // Safely store the guard
        if let Ok(mut guard_lock) = GUARD.lock() {
            *guard_lock = Some(guard);
        }

        tracing_subscriber::fmt::Layer::new()
            .with_span_events(FmtSpan::CLOSE)
            .with_writer(non_blocking)
            .with_filter(filter)
    });

    // Configure stdout logging layer
    let stdout_layer = tracing_subscriber::fmt::Layer::new()
        .with_filter(get_filter(logs_conf).unwrap_or(EnvFilter::new("info")));

    // Initialize the subscriber
    tracing_subscriber::registry()
        .with(file_layer)
        .with(stdout_layer)
        .init();
}
