use std::path::Path;
use std::time::SystemTime;

use serde::Serialize;
use tauri::Emitter;
use tracing::Subscriber;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

#[derive(Clone, Serialize)]
pub struct LogEvent {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

pub struct TauriEventLayer {
    app_handle: tauri::AppHandle,
}

struct MessageVisitor {
    message: String,
}

impl MessageVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
        }
    }
}

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}

impl<S> Layer<S> for TauriEventLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = MessageVisitor::new();
        event.record(&mut visitor);

        let metadata = event.metadata();

        let log_event = LogEvent {
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            level: metadata.level().to_string(),
            target: metadata.target().to_string(),
            message: visitor.message,
        };

        let _ = self.app_handle.emit("log-event", log_event);
    }
}

pub fn init_logging(app_handle: tauri::AppHandle, data_dir: &Path, level: &str) {
    let logs_dir = data_dir.join("logs");
    let _ = std::fs::create_dir_all(&logs_dir);

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "app");

    let level_filter = match level {
        "error" => tracing_subscriber::filter::LevelFilter::ERROR,
        "warn" => tracing_subscriber::filter::LevelFilter::WARN,
        "debug" => tracing_subscriber::filter::LevelFilter::DEBUG,
        "trace" => tracing_subscriber::filter::LevelFilter::TRACE,
        _ => tracing_subscriber::filter::LevelFilter::INFO,
    };

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_filter(level_filter);

    let event_layer = TauriEventLayer { app_handle }.with_filter(level_filter);

    let subscriber = tracing_subscriber::Registry::default()
        .with(file_layer)
        .with(event_layer);

    if let Err(e) = tracing::subscriber::set_global_default(subscriber) {
        eprintln!("Failed to set global default subscriber: {e}");
    }
}

pub fn cleanup_old_logs(data_dir: &Path) {
    let logs_dir = data_dir.join("logs");
    let entries = match std::fs::read_dir(&logs_dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    let cutoff = SystemTime::now() - std::time::Duration::from_secs(7 * 24 * 60 * 60);

    for entry in entries.flatten() {
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        if modified < cutoff {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
