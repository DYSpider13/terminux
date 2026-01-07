use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalSettings {
    pub font_family: String,
    pub font_size: u32,
    pub scrollback_lines: u32,
    pub cursor_blink: bool,
    pub cursor_shape: String,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            font_family: "Monospace".to_string(),
            font_size: 11,
            scrollback_lines: 10000,
            cursor_blink: true,
            cursor_shape: "block".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorScheme {
    pub name: String,
    pub foreground: String,
    pub background: String,
    pub palette: [String; 16],
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            name: "Default Dark".to_string(),
            foreground: "#e0e0e0".to_string(),
            background: "#1e1e1e".to_string(),
            palette: [
                "#1e1e1e".to_string(), // Black
                "#f44747".to_string(), // Red
                "#6a9955".to_string(), // Green
                "#dcdcaa".to_string(), // Yellow
                "#569cd6".to_string(), // Blue
                "#c586c0".to_string(), // Magenta
                "#4ec9b0".to_string(), // Cyan
                "#d4d4d4".to_string(), // White
                "#808080".to_string(), // Bright Black
                "#f44747".to_string(), // Bright Red
                "#6a9955".to_string(), // Bright Green
                "#dcdcaa".to_string(), // Bright Yellow
                "#569cd6".to_string(), // Bright Blue
                "#c586c0".to_string(), // Bright Magenta
                "#4ec9b0".to_string(), // Bright Cyan
                "#e0e0e0".to_string(), // Bright White
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowSettings {
    pub width: i32,
    pub height: i32,
    pub sidebar_width: i32,
    pub sidebar_visible: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            width: 1200,
            height: 800,
            sidebar_width: 300,
            sidebar_visible: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    pub terminal: TerminalSettings,
    pub colors: ColorScheme,
    pub window: WindowSettings,
}

impl Settings {
    /// Load settings from config file
    pub fn load() -> anyhow::Result<Self> {
        let config_path = Self::get_config_path()?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let settings: Settings = toml::from_str(&content)?;
            Ok(settings)
        } else {
            // Return default settings
            Ok(Self::default())
        }
    }

    /// Save settings to config file
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = Self::get_config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;

        Ok(())
    }

    /// Get the config file path
    fn get_config_path() -> anyhow::Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find config directory"))?;

        Ok(config_dir.join("terminux").join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.terminal.font_size, 11);
        assert!(settings.window.sidebar_visible);
    }

    #[test]
    fn test_settings_serialization() {
        let settings = Settings::default();
        let toml_str = toml::to_string_pretty(&settings).unwrap();
        let parsed: Settings = toml::from_str(&toml_str).unwrap();
        assert_eq!(settings.terminal.font_size, parsed.terminal.font_size);
    }
}
