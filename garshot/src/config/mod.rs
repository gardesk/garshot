//! Configuration module for garshot.
//!
//! Supports both Lua (primary, shared with gar) and TOML (fallback).

mod naming;
mod toml_config;

pub use naming::{generate_filename, NamingConfig};
pub use toml_config::{load_config, Config, SelectionSettings};
