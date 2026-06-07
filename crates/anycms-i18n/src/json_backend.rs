//! JSON translation backend.
//!
//! `JsonBackend` loads translations from JSON files or embedded strings,
//! flattening nested objects into dot-separated keys using the shared
//! [`crate::flat_backend::flatten_json`] helper.

use std::collections::HashMap;
#[cfg(feature = "fs-loader")]
use std::path::Path;

use crate::core::Reloadable;
use crate::error::I18nError;
use crate::flat_backend::{flatten_json, FlatBackend};

// ---- JSON parsing ----

/// Parse a JSON string into a flat `key -> value` map.
///
/// Nested JSON objects are flattened to dot-separated keys.
/// Plural groups (containing zero/one/two/few/many/other) are stored as `key.category`.
fn parse_json(locale: &str, content: &str) -> Result<HashMap<String, String>, I18nError> {
    let value: serde_json::Value =
        serde_json::from_str(content).map_err(|e| I18nError::JsonParseError {
            locale: locale.to_string(),
            source: e,
        })?;

    let mut messages = HashMap::new();
    flatten_json(&value, "", &mut messages);
    Ok(messages)
}

// ---- JsonBackend ----

/// A compile-time-embeddable JSON translation backend.
///
/// Stores translations in a [`FlatBackend`] (thread-safe `DashMap`).
/// Supports both compile-time embedding (via `include_str!`)
/// and runtime file loading.
pub struct JsonBackend {
    inner: FlatBackend,
}

impl Default for JsonBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonBackend {
    /// Create an empty backend.
    pub fn new() -> Self {
        Self {
            inner: FlatBackend::new(),
        }
    }

    /// Build from a slice of `(locale, json_content)` pairs.
    ///
    /// Typical usage with compile-time embedding:
    /// ```rust,ignore
    /// JsonBackend::from_embedded(&[
    ///     ("en", include_str!("../../locales/en.json")),
    ///     ("zh-CN", include_str!("../../locales/zh-CN.json")),
    /// ])
    /// ```
    pub fn from_embedded(pairs: &[(&str, &str)]) -> Result<Self, I18nError> {
        let backend = Self::new();
        for (locale, content) in pairs {
            backend.add_locale_from_str(locale, content)?;
        }
        Ok(backend)
    }

    /// Build by loading `.json` files from a directory at runtime.
    ///
    /// Expects files named `<locale>.json`, e.g. `en.json`, `zh-CN.json`.
    #[cfg(feature = "fs-loader")]
    pub fn from_dir(path: impl AsRef<Path>) -> Result<Self, I18nError> {
        let backend = Self::new();
        backend
            .inner
            .load_dir(path, "json", &|locale, content| parse_json(locale, content))?;
        Ok(backend)
    }

    /// Parse a JSON string and merge it into the backend under the given locale.
    pub fn add_locale_from_str(&self, locale: &str, content: &str) -> Result<(), I18nError> {
        let messages = parse_json(locale, content)?;
        self.inner.add_locale(locale, messages);
        Ok(())
    }

    /// Get a reference to the inner [`FlatBackend`].
    pub fn inner(&self) -> &FlatBackend {
        &self.inner
    }
}

impl crate::core::Backend for JsonBackend {
    fn get(&self, locale: &str, key: &str) -> Option<String> {
        self.inner.get(locale, key)
    }

    fn available_locales(&self) -> Vec<String> {
        self.inner.available_locales()
    }

    fn has_locale(&self, locale: &str) -> bool {
        self.inner.has_locale(locale)
    }
}

impl Reloadable for JsonBackend {
    fn reload_from_str(&self, locale: &str, content: &str) -> Result<(), I18nError> {
        self.add_locale_from_str(locale, content)
    }

    fn file_extension(&self) -> &'static str {
        "json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Backend;

    #[test]
    fn test_flatten_simple() {
        let json_str = r#"{
            "app.title": "My App",
            "app.desc": "A cool app"
        }"#;
        let backend = JsonBackend::new();
        backend.add_locale_from_str("en", json_str).unwrap();

        assert_eq!(backend.get("en", "app.title").unwrap(), "My App");
        assert_eq!(backend.get("en", "app.desc").unwrap(), "A cool app");
    }

    #[test]
    fn test_flatten_nested_object() {
        let json_str = r#"{
            "errors": {
                "not_found": "Not found",
                "unauthorized": "Unauthorized",
                "auth": {
                    "expired": "Session expired"
                }
            }
        }"#;
        let backend = JsonBackend::new();
        backend.add_locale_from_str("en", json_str).unwrap();

        assert_eq!(backend.get("en", "errors.not_found").unwrap(), "Not found");
        assert_eq!(
            backend.get("en", "errors.auth.expired").unwrap(),
            "Session expired"
        );
    }

    #[test]
    fn test_plural_group() {
        let json_str = r#"{
            "items": {
                "zero": "No items",
                "one": "One item",
                "other": "%{count} items"
            }
        }"#;
        let backend = JsonBackend::new();
        backend.add_locale_from_str("en", json_str).unwrap();

        assert_eq!(backend.get("en", "items.zero").unwrap(), "No items");
        assert_eq!(backend.get("en", "items.one").unwrap(), "One item");
        assert_eq!(
            backend.get("en", "items.other").unwrap(),
            "%{count} items"
        );
    }

    #[test]
    fn test_json_backend_implements_reloadable() {
        let backend = JsonBackend::new();
        assert_eq!(backend.file_extension(), "json");
        backend
            .reload_from_str("en", r#"{"hello": "Hi"}"#)
            .unwrap();
        assert_eq!(backend.get("en", "hello").unwrap(), "Hi");
    }

    #[test]
    fn test_reload_replaces_locale() {
        let backend = JsonBackend::new();
        backend
            .add_locale_from_str("en", r#"{"greeting": "Hello"}"#)
            .unwrap();
        assert_eq!(backend.get("en", "greeting").unwrap(), "Hello");

        // Reload with new content — should replace the locale entirely
        backend
            .reload_from_str("en", r#"{"farewell": "Goodbye"}"#)
            .unwrap();
        assert!(backend.get("en", "greeting").is_none());
        assert_eq!(backend.get("en", "farewell").unwrap(), "Goodbye");
    }

    #[test]
    fn test_from_embedded() {
        let backend = JsonBackend::from_embedded(&[
            ("en", r#"{"hello": "Hello", "bye": "Goodbye"}"#),
            ("zh-CN", r#"{"hello": "你好", "bye": "再见"}"#),
        ])
        .unwrap();

        assert_eq!(backend.get("en", "hello").unwrap(), "Hello");
        assert_eq!(backend.get("zh-CN", "hello").unwrap(), "你好");
        assert_eq!(backend.available_locales().len(), 2);
        assert!(backend.has_locale("en"));
        assert!(backend.has_locale("zh-CN"));
        assert!(!backend.has_locale("ja"));
    }

    #[test]
    fn test_invalid_json_returns_error() {
        let backend = JsonBackend::new();
        let result = backend.add_locale_from_str("en", "not valid json {{{");
        assert!(result.is_err());
    }

    #[cfg(feature = "fs-loader")]
    #[test]
    fn test_from_dir() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(
            dir.path().join("en.json"),
            r#"{"welcome": "Welcome!", "greeting": "Hello, %{name}!"}"#,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("zh-CN.json"),
            r#"{"welcome": "欢迎使用！", "greeting": "你好，%{name}！"}"#,
        )
        .unwrap();

        let backend = JsonBackend::from_dir(dir.path()).unwrap();

        assert_eq!(backend.get("en", "welcome").unwrap(), "Welcome!");
        assert_eq!(backend.get("zh-CN", "welcome").unwrap(), "欢迎使用！");
        assert_eq!(backend.available_locales().len(), 2);
    }
}
