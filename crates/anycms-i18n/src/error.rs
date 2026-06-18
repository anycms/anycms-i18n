//! Error types for anycms-i18n.

use std::path::PathBuf;

/// i18n error type
#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    /// Failed to parse a locale string
    #[error("invalid locale: {0}")]
    InvalidLocale(String),

    /// Translation key not found
    #[error("translation key not found: {key} (locale: {locale})")]
    TranslationNotFound { key: String, locale: String },

    /// Failed to parse TOML translation file
    #[cfg(feature = "toml-backend")]
    #[error("failed to parse TOML for locale '{locale}': {source}")]
    TomlParseError {
        locale: String,
        #[source]
        source: toml::de::Error,
    },

    /// Failed to parse JSON translation file
    #[cfg(feature = "json-backend")]
    #[error("failed to parse JSON for locale '{locale}': {source}")]
    JsonParseError {
        locale: String,
        #[source]
        source: serde_json::Error,
    },

    /// Failed to parse YAML translation file
    #[cfg(feature = "yaml-backend")]
    #[error("failed to parse YAML for locale '{locale}': {source}")]
    YamlParseError {
        locale: String,
        #[source]
        source: serde_yaml::Error,
    },

    /// Database error
    #[error("database error: {0}")]
    DatabaseError(String),

    /// Failed to read translation file from disk
    #[error("failed to read translation file: {path}")]
    IoError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to interpolate variables in translation string
    #[error("interpolation error for key '{key}': missing variable '{var}'")]
    InterpolationError { key: String, var: String },

    /// Configuration error
    #[error("i18n configuration error: {0}")]
    ConfigError(String),

    /// No backend configured
    #[error("no i18n backend configured")]
    NoBackend,

    /// Builder error
    #[error("builder error: {0}")]
    BuilderError(String),

    /// File watcher error (hot-reload)
    #[error("file watcher error: {0}")]
    WatchError(String),
}
