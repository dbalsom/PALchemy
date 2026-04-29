use crate::state::LogEntry;
use leptos::prelude::*;
use palcore::LogLevel;
use stylance::import_style;

import_style!(style, "log_viewer.module.scss");

#[component]
pub fn LogViewer(log_entries: RwSignal<std::collections::VecDeque<LogEntry>>) -> impl IntoView {
    let severity_filter = RwSignal::new(default_log_level());

    let severity_filter_for_events = severity_filter;
    crate::events::listen_log_severity(move |event| {
        severity_filter_for_events.set(event.level);
    });

    view! {
        <div id="bottom-pane" class=style::bottom_pane>
            <div id="status-log" class=style::log_view>
                <For
                    each=move || {
                        let filter = severity_filter.get();
                        log_entries
                            .get()
                            .into_iter()
                            .filter(|entry| entry.level >= filter)
                            .collect::<Vec<_>>()
                    }
                    key=|entry| format!("{}{}{}", entry.level as u8, entry.source as u8, entry.text)
                    let:entry
                >
                    {
                        let class_style = match entry.level {
                            LogLevel::Trace => style::trace,
                            LogLevel::Debug => style::debug,
                            LogLevel::Info => style::sys,
                            LogLevel::Warn => style::warn,
                            LogLevel::Error => style::err,
                        };

                        let source_str = match entry.source {
                            palcore::LogSource::Frontend => "[Front]",
                            palcore::LogSource::Backend => "[Core]",
                        };

                        let level_str = match entry.level {
                            LogLevel::Trace => "[TRC]",
                            LogLevel::Debug => "[DBG]",
                            LogLevel::Info => "[SYS]",
                            LogLevel::Warn => "[WRN]",
                            LogLevel::Error => "[ERR]",
                        };

                        view! {
                            <div class=format!("{} {}", style::log_entry, class_style)>
                                <strong>{source_str}</strong>" "{level_str}" "{entry.text.clone()}
                            </div>
                        }
                    }
                </For>
            </div>
        </div>
    }
}

fn default_log_level() -> LogLevel {
    if cfg!(debug_assertions) {
        LogLevel::Debug
    } else {
        LogLevel::Info
    }
}
