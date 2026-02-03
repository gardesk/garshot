//! TOML configuration for garshot.
//!
//! Fallback configuration when not using Lua (gar integration).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{GarshotError, Result};

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// General settings.
    pub general: GeneralSettings,
    /// Selection overlay settings.
    pub selection: SelectionSettings,
    /// Naming settings.
    pub naming: NamingSettings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralSettings::default(),
            selection: SelectionSettings::default(),
            naming: NamingSettings::default(),
        }
    }
}

/// General screenshot settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    /// Directory to save screenshots.
    pub save_dir: PathBuf,
    /// Default output format (png, jpeg, webp, ppm).
    pub format: String,
    /// JPEG/WebP quality (1-100).
    pub quality: u8,
    /// Include cursor in screenshots by default.
    pub include_cursor: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        let save_dir = dirs::picture_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Screenshots");

        Self {
            save_dir,
            format: "png".to_string(),
            quality: 90,
            include_cursor: false,
        }
    }
}

/// Selection overlay settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SelectionSettings {
    /// Blur radius in pixels.
    pub blur_radius: usize,
    /// Selection line color (hex format: #RRGGBB).
    pub line_color: String,
    /// Selection line width in pixels.
    pub line_width: u32,
}

impl Default for SelectionSettings {
    fn default() -> Self {
        Self {
            blur_radius: 15,
            line_color: "#ff6600".to_string(),
            line_width: 2,
        }
    }
}

impl SelectionSettings {
    /// Parse the line color as a u32 RGB value.
    pub fn line_color_u32(&self) -> u32 {
        parse_color(&self.line_color).unwrap_or(0xFF6600)
    }
}

/// Naming settings for screenshot filenames.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NamingSettings {
    /// Filename pattern with strftime and custom tokens.
    /// Supports: %Y, %m, %d, %H, %M, %S (strftime)
    /// Custom: %n (daily counter), %N (global counter), %w (window name)
    pub pattern: String,
}

impl Default for NamingSettings {
    fn default() -> Self {
        Self {
            pattern: "screenshot-%Y%m%d-%H%M%S".to_string(),
        }
    }
}

/// Load configuration from file.
///
/// Tries to load from:
/// 1. `~/.config/garshot/config.toml`
/// 2. `~/.config/gar/init.lua` (extracts gar.shot table) - TODO
/// 3. Default configuration
pub fn load_config() -> Result<Config> {
    let config_path = config_path();

    if config_path.exists() {
        tracing::debug!("Loading config from {}", config_path.display());
        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            GarshotError::ConfigError(format!("Failed to read config file: {}", e))
        })?;

        let mut config: Config = toml::from_str(&content).map_err(|e| {
            GarshotError::ConfigError(format!("Failed to parse config file: {}", e))
        })?;

        // Expand tilde in save_dir
        config.general.save_dir = expand_tilde(&config.general.save_dir);

        Ok(config)
    } else {
        tracing::debug!("No config file found, using defaults");
        Ok(Config::default())
    }
}

/// Expand leading `~` in a path to the user's home directory.
fn expand_tilde(path: &PathBuf) -> PathBuf {
    let path_str = path.to_string_lossy();
    if path_str == "~" {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
    } else if let Some(rest) = path_str.strip_prefix("~/") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(rest)
    } else {
        path.clone()
    }
}

/// Get the config file path.
pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("garshot")
        .join("config.toml")
}

/// Parse a color string (#RRGGBB or RRGGBB) to u32.
fn parse_color(s: &str) -> Option<u32> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    u32::from_str_radix(s, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_color() {
        assert_eq!(parse_color("#ff6600"), Some(0xFF6600));
        assert_eq!(parse_color("ff6600"), Some(0xFF6600));
        assert_eq!(parse_color("#000000"), Some(0x000000));
        assert_eq!(parse_color("#ffffff"), Some(0xFFFFFF));
        assert_eq!(parse_color("invalid"), None);
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.general.format, "png");
        assert_eq!(config.selection.blur_radius, 15);
        assert!(config.naming.pattern.contains("screenshot"));
    }

    #[test]
    fn test_deserialize_config() {
        let toml_str = concat!(
            "[general]\n",
            "format = \"jpeg\"\n",
            "quality = 85\n",
            "include_cursor = true\n",
            "\n",
            "[selection]\n",
            "blur_radius = 20\n",
            "line_color = \"#00ff00\"\n",
            "line_width = 3\n",
            "\n",
            "[naming]\n",
            "pattern = \"shot-date\"\n",
        );
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.general.format, "jpeg");
        assert_eq!(config.general.quality, 85);
        assert!(config.general.include_cursor);
        assert_eq!(config.selection.blur_radius, 20);
        assert_eq!(config.selection.line_color, "#00ff00");
        assert_eq!(config.naming.pattern, "shot-date");
    }

    #[test]
    fn test_expand_tilde() {
        let home = dirs::home_dir().unwrap();

        // ~/foo/bar should expand
        let path = PathBuf::from("~/Pictures/Screenshots");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, home.join("Pictures/Screenshots"));

        // Just ~ should expand to home
        let path = PathBuf::from("~");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, home);

        // Absolute paths should not change
        let path = PathBuf::from("/tmp/screenshots");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, PathBuf::from("/tmp/screenshots"));

        // Relative paths should not change
        let path = PathBuf::from("screenshots");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, PathBuf::from("screenshots"));

        // Tilde in middle should not expand
        let path = PathBuf::from("/foo/~/bar");
        let expanded = expand_tilde(&path);
        assert_eq!(expanded, PathBuf::from("/foo/~/bar"));
    }
}
