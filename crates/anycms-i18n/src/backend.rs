//! Translation backend implementations.

use std::collections::HashMap;
use std::sync::Arc;

use dashmap::DashMap;
use serde::Deserialize;

use crate::core::Backend;
use crate::error::I18nError;

// ---- TOML structures ----

/// Top-level structure of a locale TOML file.
#[derive(Debug, Default, Deserialize)]
struct LocaleData {
    #[serde(flatten)]
    messages: toml::map::Map<String, toml::Value>,
}

// ---- TomlBackend ----

/// Translation storage for a single locale.
#[derive(Debug, Default)]
struct LocaleTranslations {
    /// Flat key → translated string.
    /// Nested TOML keys like `[errors.auth]` → `errors.auth.not_found`
    messages: HashMap<String, String>,
}

/// A compile-time-embeddable TOML translation backend.
///
/// Stores translations in a `DashMap` keyed by locale string.
/// Supports both compile-time embedding (via `include_str!`)
/// and runtime file loading.
pub struct TomlBackend {
    translations: DashMap<String, LocaleTranslations>,
}

impl TomlBackend {
    /// Create an empty backend.
    pub fn new() -> Self {
        Self {
            translations: DashMap::new(),
        }
    }

    /// Build from a slice of `(locale, toml_content)` pairs.
    ///
    /// Typical usage with compile-time embedding:
    /// ```rust,ignore
    /// TomlBackend::from_embedded(&[
    ///     ("en", include_str!("../../locales/en.toml")),
    ///     ("zh-CN", include_str!("../../locales/zh-CN.toml")),
    /// ])
    /// ```
    pub fn from_embedded(pairs: &[(&str, &str)]) -> Result<Self, I18nError> {
        let backend = Self::new();
        for (locale, content) in pairs {
            backend.add_locale_from_str(locale, content)?;
        }
        Ok(backend)
    }

    /// Build by loading `.toml` files from a directory at runtime.
    ///
    /// Expects files named `<locale>.toml`, e.g. `en.toml`, `zh-CN.toml`.
    #[cfg(feature = "fs-loader")]
    pub fn from_dir(path: impl AsRef<Path>) -> Result<Self, I18nError> {
        let backend = Self::new();
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

        for entry in std::fs::read_dir(dir)
            .map_err(|e| I18nError::IoError {
                path: dir.to_path_buf(),
                source: e,
            })?
        {
            let entry = entry.map_err(|e| I18nError::IoError {
                path: dir.to_path_buf(),
                source: e,
            })?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                let locale = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let content = std::fs::read_to_string(&path).map_err(|e| I18nError::IoError {
                    path: path.clone(),
                    source: e,
                })?;

                backend.add_locale_from_str(&locale, &content)?;
            }
        }

        Ok(backend)
    }

    /// Parse a TOML string and merge it into the backend under the given locale.
    pub fn add_locale_from_str(&self, locale: &str, content: &str) -> Result<(), I18nError> {
        let data: LocaleData = toml::from_str(content).map_err(|e| I18nError::TomlParseError {
            locale: locale.to_string(),
            source: e,
        })?;

        let mut translations = LocaleTranslations::default();
        Self::flatten_toml(&data.messages, "", &mut translations.messages);

        self.translations.insert(locale.to_string(), translations);
        Ok(())
    }

    /// Recursively flatten nested TOML tables into `a.b.c` dot-separated keys.
    fn flatten_toml(
        values: &toml::map::Map<String, toml::Value>,
        prefix: &str,
        out: &mut HashMap<String, String>,
    ) {
        for (key, value) in values {
            let full_key = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };

            match value {
                toml::Value::String(s) => {
                    out.insert(full_key, s.clone());
                }
                toml::Value::Table(table) => {
                    // Check if this is a plural group (contains zero/one/two/few/many/other)
                    let is_plural = table
                        .keys()
                        .any(|k| matches!(k.as_str(), "zero" | "one" | "two" | "few" | "many" | "other"));

                    if is_plural {
                        // Store each plural form as "key.category"
                        for (cat, val) in table {
                            if let toml::Value::String(s) = val {
                                out.insert(format!("{full_key}.{cat}"), s.clone());
                            }
                        }
                    } else {
                        // Recurse into nested table
                        Self::flatten_toml(table, &full_key, out);
                    }
                }
                toml::Value::Integer(i) => {
                    out.insert(full_key, i.to_string());
                }
                _ => {
                    // Skip arrays, floats, bools, datetimes
                }
            }
        }
    }
}

impl Backend for TomlBackend {
    fn get(&self, locale: &str, key: &str) -> Option<String> {
        self.translations
            .get(locale)
            .and_then(|t| t.messages.get(key).cloned())
    }

    fn available_locales(&self) -> Vec<String> {
        self.translations.iter().map(|e| e.key().clone()).collect()
    }

    fn has_locale(&self, locale: &str) -> bool {
        self.translations.contains_key(locale)
    }
}

// ---- ChainedBackend ----

/// A backend that chains multiple backends together.
///
/// Looks up translations in order; the first backend that returns
/// a result wins. This allows e.g. database overrides to take
/// priority over file-based translations.
pub struct ChainedBackend {
    backends: Vec<Arc<dyn Backend>>,
}

impl ChainedBackend {
    /// Create a new chained backend from an ordered list of backends.
    ///
    /// Backends are queried in order; the first match wins.
    pub fn new(backends: Vec<Arc<dyn Backend>>) -> Self {
        Self { backends }
    }

    /// Add a backend to the chain (highest priority — queried first).
    pub fn push_front(&mut self, backend: Arc<dyn Backend>) {
        self.backends.insert(0, backend);
    }

    /// Add a backend to the end of the chain (lowest priority).
    pub fn push_back(&mut self, backend: Arc<dyn Backend>) {
        self.backends.push(backend);
    }
}

impl Backend for ChainedBackend {
    fn get(&self, locale: &str, key: &str) -> Option<String> {
        for backend in &self.backends {
            if let Some(value) = backend.get(locale, key) {
                return Some(value);
            }
        }
        None
    }

    fn available_locales(&self) -> Vec<String> {
        let mut locales = std::collections::HashSet::new();
        for backend in &self.backends {
            for locale in backend.available_locales() {
                locales.insert(locale);
            }
        }
        locales.into_iter().collect()
    }

    fn has_locale(&self, locale: &str) -> bool {
        self.backends.iter().any(|b| b.has_locale(locale))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flatten_simple() {
        let toml_str = r#"
app.title = "My App"
app.desc = "A cool app"
"#;
        let backend = TomlBackend::new();
        backend.add_locale_from_str("en", toml_str).unwrap();

        assert_eq!(backend.get("en", "app.title").unwrap(), "My App");
        assert_eq!(backend.get("en", "app.desc").unwrap(), "A cool app");
    }

    #[test]
    fn test_flatten_nested_table() {
        let toml_str = r#"
[errors]
not_found = "Not found"
unauthorized = "Unauthorized"

[errors.auth]
expired = "Session expired"
"#;
        let backend = TomlBackend::new();
        backend.add_locale_from_str("en", toml_str).unwrap();

        assert_eq!(backend.get("en", "errors.not_found").unwrap(), "Not found");
        assert_eq!(backend.get("en", "errors.auth.expired").unwrap(), "Session expired");
    }

    #[test]
    fn test_plural_group() {
        let toml_str = r#"
[items]
zero = "No items"
one = "One item"
other = "%{count} items"
"#;
        let backend = TomlBackend::new();
        backend.add_locale_from_str("en", toml_str).unwrap();

        assert_eq!(backend.get("en", "items.zero").unwrap(), "No items");
        assert_eq!(backend.get("en", "items.one").unwrap(), "One item");
        assert_eq!(backend.get("en", "items.other").unwrap(), "%{count} items");
    }

    #[test]
    fn test_chained_backend() {
        let b1 = {
            let b = TomlBackend::new();
            b.add_locale_from_str("en", r#"hello = "Hi""#).unwrap();
            b.add_locale_from_str("zh-CN", r#"hello = "你好""#).unwrap();
            Arc::new(b) as Arc<dyn Backend>
        };
        let b2 = {
            let b = TomlBackend::new();
            b.add_locale_from_str("en", r#"hello = "Hello"
bye = "Goodbye""#).unwrap();
            Arc::new(b) as Arc<dyn Backend>
        };

        let chained = ChainedBackend::new(vec![b1, b2]);

        // b1 has priority for "en.hello"
        assert_eq!(chained.get("en", "hello").unwrap(), "Hi");
        // falls through to b2 for "en.bye"
        assert_eq!(chained.get("en", "bye").unwrap(), "Goodbye");
        // both have the locale
        assert!(chained.has_locale("zh-CN"));
    }
}
