//! Generic in-memory translation backend.
//!
//! `FlatBackend` stores translations as flat `key → value` maps per locale,
//! using `DashMap` for thread-safe concurrent access. All format-specific
//! backends (TOML, JSON, YAML) delegate storage to this type.

use std::collections::HashMap;
use std::path::Path;

use dashmap::DashMap;

use crate::core::Backend;
use crate::error::I18nError;

/// Generic in-memory translation storage.
///
/// Stores translations as `locale → (flat_key → translated_string)`.
/// Thread-safe via `DashMap`.
pub struct FlatBackend {
    translations: DashMap<String, HashMap<String, String>>,
}

impl Default for FlatBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl FlatBackend {
    /// Create an empty backend.
    pub fn new() -> Self {
        Self {
            translations: DashMap::new(),
        }
    }

    /// Add or replace all messages for a locale.
    pub fn add_locale(&self, locale: &str, messages: HashMap<String, String>) {
        self.translations.insert(locale.to_string(), messages);
    }

    /// Remove a locale from the backend.
    pub fn remove_locale(&self, locale: &str) {
        self.translations.remove(locale);
    }

    /// Load translations from a directory.
    ///
    /// Scans for files matching `extension` (e.g. `"toml"`, `"json"`, `"yaml"`),
    /// reads each file, and calls `parse_fn` to convert content to flat key-value pairs.
    ///
    /// File naming: `<locale>.<extension>` → locale identifier.
    pub fn load_dir(
        &self,
        path: impl AsRef<Path>,
        extension: &str,
        parse_fn: &dyn Fn(&str, &str) -> Result<HashMap<String, String>, I18nError>,
    ) -> Result<(), I18nError> {
        let dir = path.as_ref();

        if !dir.is_dir() {
            return Err(I18nError::IoError {
                path: dir.to_path_buf(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "locale directory not found",
                ),
            });
        }

        self.load_dir_inner(dir, extension, parse_fn)
    }

    /// Like [`load_dir`], but returns `Ok(())` when the directory does not exist
    /// instead of erroring. This enables the "compiled defaults + optional runtime
    /// override" pattern: if the runtime directory is absent, translations fall
    /// through to the compiled/embedded backend.
    pub fn try_load_dir(
        &self,
        path: impl AsRef<Path>,
        extension: &str,
        parse_fn: &dyn Fn(&str, &str) -> Result<HashMap<String, String>, I18nError>,
    ) -> Result<(), I18nError> {
        let dir = path.as_ref();

        if !dir.is_dir() {
            tracing::debug!(
                path = %dir.display(),
                "locale directory not found, skipping runtime override"
            );
            return Ok(());
        }

        self.load_dir_inner(dir, extension, parse_fn)
    }

    /// Shared implementation used by both [`load_dir`] and [`try_load_dir`].
    fn load_dir_inner(
        &self,
        dir: &Path,
        extension: &str,
        parse_fn: &dyn Fn(&str, &str) -> Result<HashMap<String, String>, I18nError>,
    ) -> Result<(), I18nError> {

        for entry in std::fs::read_dir(dir).map_err(|e| I18nError::IoError {
            path: dir.to_path_buf(),
            source: e,
        })? {
            let entry = entry.map_err(|e| I18nError::IoError {
                path: dir.to_path_buf(),
                source: e,
            })?;
            let file_path = entry.path();

            if file_path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|ext| ext == extension)
            {
                let locale = file_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let content =
                    std::fs::read_to_string(&file_path).map_err(|e| I18nError::IoError {
                        path: file_path.clone(),
                        source: e,
                    })?;

                let messages = parse_fn(&locale, &content)?;
                self.add_locale(&locale, messages);
            }
        }

        Ok(())
    }

    /// Get a translation for the given locale and key.
    pub fn get_message(&self, locale: &str, key: &str) -> Option<String> {
        self.translations
            .get(locale)
            .and_then(|m| m.get(key).cloned())
    }

    /// Get all locale identifiers.
    pub fn locales(&self) -> Vec<String> {
        self.translations.iter().map(|e| e.key().clone()).collect()
    }

    /// Check if a locale exists.
    pub fn contains_locale(&self, locale: &str) -> bool {
        self.translations.contains_key(locale)
    }
}

impl Backend for FlatBackend {
    fn get(&self, locale: &str, key: &str) -> Option<String> {
        self.get_message(locale, key)
    }

    fn available_locales(&self) -> Vec<String> {
        self.locales()
    }

    fn has_locale(&self, locale: &str) -> bool {
        self.contains_locale(locale)
    }
}

// ---- Shared flatten helpers ----

/// Flatten a `serde_json::Value` (object) into dot-separated keys.
///
/// Handles:
/// - String values → stored directly
/// - Nested objects → recursively flattened (`a.b.c`)
/// - Plural groups (containing zero/one/two/few/many/other) → stored as `key.category`
/// - Numbers → stored as string representation
#[cfg(any(feature = "json-backend", feature = "yaml-backend", test))]
pub fn flatten_json(value: &serde_json::Value, prefix: &str, out: &mut HashMap<String, String>) {
    match value {
        serde_json::Value::String(s) => {
            if !prefix.is_empty() {
                out.insert(prefix.to_string(), s.clone());
            }
        }
        serde_json::Value::Object(map) => {
            // Check if this is a plural group
            let is_plural = map
                .keys()
                .any(|k| matches!(k.as_str(), "zero" | "one" | "two" | "few" | "many" | "other"));

            if is_plural && !prefix.is_empty() {
                for (cat, val) in map {
                    if let serde_json::Value::String(s) = val {
                        out.insert(format!("{prefix}.{cat}"), s.clone());
                    }
                }
            } else {
                for (key, val) in map {
                    let full_key = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    flatten_json(val, &full_key, out);
                }
            }
        }
        serde_json::Value::Number(n) => {
            if !prefix.is_empty() {
                out.insert(prefix.to_string(), n.to_string());
            }
        }
        // Skip arrays, bools, null
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_backend_basic() {
        let backend = FlatBackend::new();
        let mut messages = HashMap::new();
        messages.insert("welcome".to_string(), "Welcome!".to_string());
        messages.insert("bye".to_string(), "Goodbye!".to_string());
        backend.add_locale("en", messages);

        assert_eq!(backend.get("en", "welcome").unwrap(), "Welcome!");
        assert_eq!(backend.get("en", "bye").unwrap(), "Goodbye!");
        assert!(backend.get("en", "missing").is_none());
    }

    #[test]
    fn test_flat_backend_multiple_locales() {
        let backend = FlatBackend::new();

        let mut en = HashMap::new();
        en.insert("hello".to_string(), "Hello".to_string());
        backend.add_locale("en", en);

        let mut zh = HashMap::new();
        zh.insert("hello".to_string(), "你好".to_string());
        backend.add_locale("zh-CN", zh);

        assert_eq!(backend.get("en", "hello").unwrap(), "Hello");
        assert_eq!(backend.get("zh-CN", "hello").unwrap(), "你好");
        assert_eq!(backend.available_locales().len(), 2);
    }

    #[test]
    fn test_flatten_json_simple() {
        let json: serde_json::Value = serde_json::json!({
            "app.title": "My App",
            "app.desc": "A cool app"
        });
        let mut out = HashMap::new();
        flatten_json(&json, "", &mut out);
        assert_eq!(out.get("app.title").unwrap(), "My App");
    }

    #[test]
    fn test_flatten_json_nested() {
        let json: serde_json::Value = serde_json::json!({
            "errors": {
                "not_found": "Not found",
                "auth": {
                    "expired": "Session expired"
                }
            }
        });
        let mut out = HashMap::new();
        flatten_json(&json, "", &mut out);
        assert_eq!(out.get("errors.not_found").unwrap(), "Not found");
        assert_eq!(out.get("errors.auth.expired").unwrap(), "Session expired");
    }

    #[test]
    fn test_flatten_json_plural() {
        let json: serde_json::Value = serde_json::json!({
            "items": {
                "zero": "No items",
                "one": "One item",
                "other": "%{count} items"
            }
        });
        let mut out = HashMap::new();
        flatten_json(&json, "", &mut out);
        assert_eq!(out.get("items.zero").unwrap(), "No items");
        assert_eq!(out.get("items.one").unwrap(), "One item");
        assert_eq!(out.get("items.other").unwrap(), "%{count} items");
    }

    #[test]
    fn test_load_dir_errors_on_missing() {
        let backend = FlatBackend::new();
        let result = backend.load_dir("/no/such/directory", "toml", &|_locale, _content| {
            Ok(HashMap::new())
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_try_load_dir_ok_on_missing() {
        let backend = FlatBackend::new();
        let result = backend.try_load_dir("/no/such/directory", "toml", &|_locale, _content| {
            Ok(HashMap::new())
        });
        assert!(result.is_ok());
        // Backend should be empty
        assert!(backend.available_locales().is_empty());
    }

    #[test]
    fn test_try_load_dir_loads_existing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("en.toml"), r#"hello = "Hi""#).unwrap();

        let backend = FlatBackend::new();
        let result = backend.try_load_dir(dir.path(), "toml", &|_locale, content| {
            let data: toml::map::Map<String, toml::Value> = toml::from_str(content).unwrap();
            let mut out = HashMap::new();
            for (k, v) in data {
                if let toml::Value::String(s) = v {
                    out.insert(k, s);
                }
            }
            Ok(out)
        });
        assert!(result.is_ok());
        assert_eq!(backend.get("en", "hello").unwrap(), "Hi");
    }
}
