//! YAML translation backend.
//!
//! `YamlBackend` loads translations from YAML files or embedded strings,
//! flattening nested objects into dot-separated keys using the shared
//! [`crate::flat_backend::flatten_json`] helper.

use std::collections::HashMap;
#[cfg(feature = "fs-loader")]
use std::path::Path;

use crate::core::Reloadable;
use crate::error::I18nError;
use crate::flat_backend::{flatten_json, FlatBackend};

// ---- YAML parsing ----

/// Parse a YAML string into a flat `key -> value` map.
///
/// Nested YAML mappings are flattened to dot-separated keys.
/// Plural groups (containing zero/one/two/few/many/other) are stored as `key.category`.
fn parse_yaml(locale: &str, content: &str) -> Result<HashMap<String, String>, I18nError> {
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(content).map_err(|e| I18nError::YamlParseError {
            locale: locale.to_string(),
            source: e,
        })?;

    // Convert serde_yaml::Value -> serde_json::Value so we can reuse flatten_json()
    // serde_yaml::Value and serde_json::Value both implement Serialize/Deserialize,
    // so we round-trip through a JSON string.
    let json_value: serde_json::Value = serde_json::to_value(&yaml_value).map_err(|e| {
        I18nError::YamlParseError {
            locale: locale.to_string(),
            source: serde_yaml::from_str::<serde_yaml::Value>(&format!(
                "failed to convert YAML to JSON: {e}"
            ))
            .unwrap_err(),
        }
    })?;

    let mut messages = HashMap::new();
    flatten_json(&json_value, "", &mut messages);
    Ok(messages)
}

// ---- YamlBackend ----

/// A compile-time-embeddable YAML translation backend.
///
/// Stores translations in a [`FlatBackend`] (thread-safe `DashMap`).
/// Supports both compile-time embedding (via `include_str!`)
/// and runtime file loading.
pub struct YamlBackend {
    inner: FlatBackend,
}

impl Default for YamlBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl YamlBackend {
    /// Create an empty backend.
    pub fn new() -> Self {
        Self {
            inner: FlatBackend::new(),
        }
    }

    /// Build from a slice of `(locale, yaml_content)` pairs.
    ///
    /// Typical usage with compile-time embedding:
    /// ```rust,ignore
    /// YamlBackend::from_embedded(&[
    ///     ("en", include_str!("../../locales/en.yaml")),
    ///     ("zh-CN", include_str!("../../locales/zh-CN.yaml")),
    /// ])
    /// ```
    pub fn from_embedded(pairs: &[(&str, &str)]) -> Result<Self, I18nError> {
        let backend = Self::new();
        for (locale, content) in pairs {
            backend.add_locale_from_str(locale, content)?;
        }
        Ok(backend)
    }

    /// Build by loading `.yaml` files from a directory at runtime.
    ///
    /// Expects files named `<locale>.yaml`, e.g. `en.yaml`, `zh-CN.yaml`.
    #[cfg(feature = "fs-loader")]
    pub fn from_dir(path: impl AsRef<Path>) -> Result<Self, I18nError> {
        let backend = Self::new();
        backend
            .inner
            .load_dir(path, "yaml", &|locale, content| parse_yaml(locale, content))?;
        Ok(backend)
    }

    /// Parse a YAML string and merge it into the backend under the given locale.
    pub fn add_locale_from_str(&self, locale: &str, content: &str) -> Result<(), I18nError> {
        let messages = parse_yaml(locale, content)?;
        self.inner.add_locale(locale, messages);
        Ok(())
    }

    /// Get a reference to the inner [`FlatBackend`].
    pub fn inner(&self) -> &FlatBackend {
        &self.inner
    }
}

impl crate::core::Backend for YamlBackend {
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

impl Reloadable for YamlBackend {
    fn reload_from_str(&self, locale: &str, content: &str) -> Result<(), I18nError> {
        self.add_locale_from_str(locale, content)
    }

    fn file_extension(&self) -> &'static str {
        "yaml"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Backend;

    #[test]
    fn test_flatten_simple() {
        let yaml_str = r#"
app.title: My App
app.desc: A cool app
"#;
        let backend = YamlBackend::new();
        backend.add_locale_from_str("en", yaml_str).unwrap();

        assert_eq!(backend.get("en", "app.title").unwrap(), "My App");
        assert_eq!(backend.get("en", "app.desc").unwrap(), "A cool app");
    }

    #[test]
    fn test_flatten_nested_object() {
        let yaml_str = r#"
errors:
  not_found: Not found
  unauthorized: Unauthorized
  auth:
    expired: Session expired
"#;
        let backend = YamlBackend::new();
        backend.add_locale_from_str("en", yaml_str).unwrap();

        assert_eq!(backend.get("en", "errors.not_found").unwrap(), "Not found");
        assert_eq!(
            backend.get("en", "errors.auth.expired").unwrap(),
            "Session expired"
        );
    }

    #[test]
    fn test_plural_group() {
        let yaml_str = r#"
items:
  zero: No items
  one: One item
  other: "%{count} items"
"#;
        let backend = YamlBackend::new();
        backend.add_locale_from_str("en", yaml_str).unwrap();

        assert_eq!(backend.get("en", "items.zero").unwrap(), "No items");
        assert_eq!(backend.get("en", "items.one").unwrap(), "One item");
        assert_eq!(
            backend.get("en", "items.other").unwrap(),
            "%{count} items"
        );
    }

    #[test]
    fn test_yaml_backend_implements_reloadable() {
        let backend = YamlBackend::new();
        assert_eq!(backend.file_extension(), "yaml");
        backend
            .reload_from_str("en", "hello: Hi")
            .unwrap();
        assert_eq!(backend.get("en", "hello").unwrap(), "Hi");
    }

    #[test]
    fn test_reload_replaces_locale() {
        let backend = YamlBackend::new();
        backend
            .add_locale_from_str("en", "greeting: Hello")
            .unwrap();
        assert_eq!(backend.get("en", "greeting").unwrap(), "Hello");

        // Reload with new content — should replace the locale entirely
        backend
            .reload_from_str("en", "farewell: Goodbye")
            .unwrap();
        assert!(backend.get("en", "greeting").is_none());
        assert_eq!(backend.get("en", "farewell").unwrap(), "Goodbye");
    }

    #[test]
    fn test_from_embedded() {
        let backend = YamlBackend::from_embedded(&[
            ("en", "hello: Hello\nbye: Goodbye"),
            ("zh-CN", "hello: 你好\nbye: 再见"),
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
    fn test_invalid_yaml_returns_error() {
        let backend = YamlBackend::new();
        let result = backend.add_locale_from_str("en", "not: valid: yaml: {{{");
        assert!(result.is_err());
    }

    #[cfg(feature = "fs-loader")]
    #[test]
    fn test_from_dir() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(
            dir.path().join("en.yaml"),
            "welcome: Welcome!\ngreeting: Hello, %{name}!",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("zh-CN.yaml"),
            "welcome: 欢迎使用！\ngreeting: 你好，%{name}！",
        )
        .unwrap();

        let backend = YamlBackend::from_dir(dir.path()).unwrap();

        assert_eq!(backend.get("en", "welcome").unwrap(), "Welcome!");
        assert_eq!(backend.get("zh-CN", "welcome").unwrap(), "欢迎使用！");
        assert_eq!(backend.available_locales().len(), 2);
    }
}
