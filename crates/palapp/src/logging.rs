use std::{collections::VecDeque, sync::Arc};

use palcore::{BackendEvent, LogEvent, LogLevel, LogSource};
use tauri::{AppHandle, Emitter};
use tokio::sync::{broadcast, Mutex};
use tracing_subscriber::Layer;

pub const MAX_LOG_HISTORY: usize = 1000;

#[derive(Clone)]
pub struct LogHistory {
    entries: Arc<Mutex<VecDeque<LogEvent>>>,
    max_entries: usize,
}

impl LogHistory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(Mutex::new(VecDeque::with_capacity(max_entries))),
            max_entries,
        }
    }

    pub async fn snapshot(&self) -> Vec<LogEvent> {
        self.entries.lock().await.iter().cloned().collect()
    }

    async fn push(&self, event: LogEvent) {
        let mut entries = self.entries.lock().await;
        entries.push_back(event);
        while entries.len() > self.max_entries {
            entries.pop_front();
        }
    }
}

pub struct TauriLogLayer {
    sender: broadcast::Sender<LogEvent>,
    history: LogHistory,
}

impl TauriLogLayer {
    pub fn new(sender: broadcast::Sender<LogEvent>, history: LogHistory) -> Self {
        Self { sender, history }
    }
}

impl<S> Layer<S> for TauriLogLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let metadata = event.metadata();
        let level = match *metadata.level() {
            tracing::Level::TRACE => LogLevel::Trace,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::ERROR => LogLevel::Error,
        };

        let mut visitor = LogMessageVisitor::default();
        event.record(&mut visitor);

        let log_event = LogEvent {
            level,
            source: LogSource::Backend,
            target: metadata.target().to_string(),
            message: visitor.message,
        };

        let history = self.history.clone();
        let event_for_history = log_event.clone();
        tauri::async_runtime::spawn(async move {
            history.push(event_for_history).await;
        });

        let _ = self.sender.send(log_event);
    }
}

pub fn spawn_log_relay(app: AppHandle, mut receiver: broadcast::Receiver<LogEvent>) {
    tauri::async_runtime::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            let _ = app.emit("backend_event", BackendEvent::Log(event));
        }
    });
}

#[derive(Default)]
struct LogMessageVisitor {
    message: String,
}

impl tracing::field::Visit for LogMessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        }
    }
}
