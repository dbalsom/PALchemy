use std::{fs, path::PathBuf, sync::Arc};

use palcore::{AppError, AppSettings, ThemePreset};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;
use toml_edit::{value, DocumentMut};

const SETTINGS_FILE_NAME: &str = "settings.toml";
const SETTINGS_TEMPLATE: &str = r#"# PALchemy user settings
#
# theme:
#   classic - original PALchemy palette
#   wong    - colorblind-friendly palette
#
# high_contrast:
#   true  - strong outlines and flat chip rendering
#   false - standard gradients and bevels
#
# large_text:
#   true  - increase the base UI text size
#   false - use the standard text size
#
# interactive_poll_hz:
#   interactive GPIO polling frequency in hertz
#   valid range: 1-240

theme = "wong"
high_contrast = false
large_text = false
interactive_poll_hz = 30
"#;

#[derive(Clone)]
pub struct SettingsStore {
    path: PathBuf,
    state: Arc<Mutex<AppSettings>>,
}

impl SettingsStore {
    pub fn load(app: &AppHandle) -> Result<Self, AppError> {
        let config_dir = app.path().app_config_dir().map_err(|error| AppError::Internal {
            message: format!("Failed to resolve app config directory: {error}"),
        })?;

        fs::create_dir_all(&config_dir).map_err(|error| AppError::Internal {
            message: format!("Failed to create config directory: {error}"),
        })?;

        let path = config_dir.join(SETTINGS_FILE_NAME);
        if !path.exists() {
            fs::write(&path, SETTINGS_TEMPLATE).map_err(|error| AppError::Internal {
                message: format!("Failed to create settings file: {error}"),
            })?;
        }

        let settings = load_settings_from_path(&path)?;
        Ok(Self {
            path,
            state: Arc::new(Mutex::new(settings)),
        })
    }

    pub async fn get(&self) -> AppSettings {
        *self.state.lock().await
    }

    pub fn get_blocking(&self) -> AppSettings {
        *self.state.blocking_lock()
    }

    pub async fn update(&self, settings: AppSettings) -> Result<AppSettings, AppError> {
        write_settings_to_path(&self.path, settings)?;
        *self.state.lock().await = settings;
        Ok(settings)
    }
}

fn load_settings_from_path(path: &PathBuf) -> Result<AppSettings, AppError> {
    let contents = fs::read_to_string(path).map_err(|error| AppError::Internal {
        message: format!("Failed to read settings file: {error}"),
    })?;
    let document = contents.parse::<DocumentMut>().map_err(|error| AppError::Internal {
        message: format!("Failed to parse settings file: {error}"),
    })?;

    let theme = document
        .get("theme")
        .and_then(|item| item.as_str())
        .map(parse_theme)
        .unwrap_or(ThemePreset::Wong);
    let high_contrast = document
        .get("high_contrast")
        .and_then(|item| item.as_bool())
        .unwrap_or(false);
    let large_text = document
        .get("large_text")
        .and_then(|item| item.as_bool())
        .unwrap_or(false);
    let interactive_poll_hz = document
        .get("interactive_poll_hz")
        .and_then(|item| item.as_integer())
        .map(sanitize_poll_hz)
        .unwrap_or(30);

    Ok(AppSettings {
        theme,
        high_contrast,
        large_text,
        interactive_poll_hz,
    })
}

fn write_settings_to_path(path: &PathBuf, settings: AppSettings) -> Result<(), AppError> {
    let contents = fs::read_to_string(path).unwrap_or_else(|_| SETTINGS_TEMPLATE.to_string());
    let mut document = contents.parse::<DocumentMut>().map_err(|error| AppError::Internal {
        message: format!("Failed to parse settings file for update: {error}"),
    })?;

    document["theme"] = value(settings.theme.as_str());
    document["high_contrast"] = value(settings.high_contrast);
    document["large_text"] = value(settings.large_text);
    document["interactive_poll_hz"] = value(i64::from(sanitize_poll_hz(i64::from(settings.interactive_poll_hz))));

    fs::write(path, document.to_string()).map_err(|error| AppError::Internal {
        message: format!("Failed to write settings file: {error}"),
    })
}

fn parse_theme(value: &str) -> ThemePreset {
    match value {
        "classic" => ThemePreset::Classic,
        _ => ThemePreset::Wong,
    }
}

fn sanitize_poll_hz(value: i64) -> u16 {
    value.clamp(1, 240) as u16
}

#[cfg(test)]
mod tests {
    use super::sanitize_poll_hz;

    #[test]
    fn poll_rate_is_clamped_to_safe_range() {
        assert_eq!(sanitize_poll_hz(0), 1);
        assert_eq!(sanitize_poll_hz(30), 30);
        assert_eq!(sanitize_poll_hz(500), 240);
    }
}
