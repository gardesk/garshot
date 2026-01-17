//! Filename generation with token support.
//!
//! Supported tokens:
//! - strftime tokens: %Y, %m, %d, %H, %M, %S, etc.
//! - %n: Daily counter (resets each day)
//! - %N: Global counter (persistent)
//! - %w: Window name (for window captures)

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use chrono::Local;

/// Global counter for %N token.
static GLOBAL_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Configuration for filename generation.
#[derive(Debug, Clone)]
pub struct NamingConfig {
    /// Filename pattern.
    pub pattern: String,
    /// Window name (optional, for %w token).
    pub window_name: Option<String>,
}

impl Default for NamingConfig {
    fn default() -> Self {
        Self {
            pattern: "screenshot-%Y%m%d-%H%M%S".to_string(),
            window_name: None,
        }
    }
}

/// Generate a filename from a pattern.
///
/// Handles strftime tokens via chrono and custom tokens:
/// - %n: Daily counter (currently just uses global counter)
/// - %N: Global counter
/// - %w: Window name
pub fn generate_filename(config: &NamingConfig, format: &str, save_dir: &Path) -> PathBuf {
    let now = Local::now();

    // Process custom tokens first
    let pattern = process_custom_tokens(&config.pattern, config);

    // Apply strftime formatting
    let filename = now.format(&pattern).to_string();

    // Add extension
    let filename_with_ext = format!("{}.{}", filename, format);
    let mut path = save_dir.join(&filename_with_ext);

    // Handle collisions
    path = handle_collision(path, format);

    path
}

/// Process custom tokens (%n, %N, %w).
fn process_custom_tokens(pattern: &str, config: &NamingConfig) -> String {
    let mut result = pattern.to_string();

    // %N - global counter
    if result.contains("%N") {
        let counter = GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst);
        result = result.replace("%N", &format!("{:04}", counter));
    }

    // %n - daily counter (for now, same as global)
    if result.contains("%n") {
        let counter = GLOBAL_COUNTER.fetch_add(1, Ordering::SeqCst);
        result = result.replace("%n", &format!("{:04}", counter));
    }

    // %w - window name
    if result.contains("%w") {
        let window_name = config
            .window_name
            .as_deref()
            .unwrap_or("window")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take(50)
            .collect::<String>();
        result = result.replace("%w", &window_name);
    }

    result
}

/// Handle filename collision by appending a number.
fn handle_collision(path: PathBuf, format: &str) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("screenshot");
    let parent = path.parent().unwrap_or(Path::new("."));

    for i in 1..1000 {
        let new_name = format!("{}-{}.{}", stem, i, format);
        let new_path = parent.join(&new_name);
        if !new_path.exists() {
            return new_path;
        }
    }

    // Fallback: use timestamp suffix
    let timestamp = Local::now().timestamp_millis();
    let new_name = format!("{}-{}.{}", stem, timestamp, format);
    parent.join(&new_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_generate_filename_basic() {
        let config = NamingConfig {
            pattern: "test".to_string(),
            window_name: None,
        };
        let dir = PathBuf::from("/tmp");
        let path = generate_filename(&config, "png", &dir);
        assert!(path.to_string_lossy().contains("test"));
        assert!(path.to_string_lossy().ends_with(".png"));
    }

    #[test]
    fn test_generate_filename_with_strftime() {
        let config = NamingConfig {
            pattern: "shot-%Y%m%d".to_string(),
            window_name: None,
        };
        let dir = PathBuf::from("/tmp");
        let path = generate_filename(&config, "png", &dir);
        let filename = path.file_name().unwrap().to_string_lossy();
        assert!(filename.starts_with("shot-20"));
    }

    #[test]
    fn test_generate_filename_with_window_name() {
        let config = NamingConfig {
            pattern: "shot-%w".to_string(),
            window_name: Some("Firefox".to_string()),
        };
        let dir = PathBuf::from("/tmp");
        let path = generate_filename(&config, "png", &dir);
        assert!(path.to_string_lossy().contains("Firefox"));
    }

    #[test]
    fn test_collision_handling() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.png");

        // Create the first file
        fs::write(&path, b"test").unwrap();

        // Generate should return test-1.png
        let config = NamingConfig {
            pattern: "test".to_string(),
            window_name: None,
        };
        let new_path = generate_filename(&config, "png", dir.path());

        assert!(new_path.to_string_lossy().contains("test-1.png"));
    }

    #[test]
    fn test_process_custom_tokens_global_counter() {
        let config = NamingConfig {
            pattern: "shot-%N".to_string(),
            window_name: None,
        };
        let result = process_custom_tokens(&config.pattern, &config);
        assert!(result.starts_with("shot-"));
        assert!(!result.contains("%N"));
    }
}
