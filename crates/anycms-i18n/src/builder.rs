//! Builder for constructing an [`I18n`] instance.

use std::sync::Arc;

use crate::core::{Backend, I18n};
use crate::error::I18nError;

/// Builder for constructing an [`I18n`] instance.
///
/// # Example
///
/// ```rust,ignore
/// use anycms_i18n::I18nBuilder;
///
/// let i18n = I18nBuilder::new()
///     .default_locale("en")
///     .fallback_locale("en")
///     .embedded_translations(&[
///         ("en", include_str!("../../locales/en.toml")),
///         ("zh-CN", include_str!("../../locales/zh-CN.toml")),
///     ])
///     .build()
///     .unwrap();
/// ```
pub struct I18nBuilder {
    default_locale: String,
    fallback_locale: String,
    backends: Vec<Arc<dyn Backend>>,
}

impl Default for I18nBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl I18nBuilder {
    /// Create a new builder with defaults (`"en"` locale).
    pub fn new() -> Self {
        Self {
            default_locale: "en".to_string(),
            fallback_locale: "en".to_string(),
            backends: Vec::new(),
        }
    }

    /// Set the default locale (used when no locale is specified).
    pub fn default_locale(mut self, locale: impl Into<String>) -> Self {
        self.default_locale = locale.into();
        self
    }

    /// Set the fallback locale (last resort in the fallback chain).
    pub fn fallback_locale(mut self, locale: impl Into<String>) -> Self {
        self.fallback_locale = locale.into();
        self
    }

    // ---- TOML backend (feature-gated) ----

    /// Add compile-time-embedded TOML translations.
    ///
    /// Can be called multiple times; each call adds a new backend to the chain.
    #[cfg(feature = "toml-backend")]
    pub fn embedded_translations(self, pairs: &[(&str, &str)]) -> Result<Self, I18nError> {
        let backend = crate::backend::TomlBackend::from_embedded(pairs)?;
        Ok(self.add_backend(Arc::new(backend)))
    }

    /// Add TOML translations from a directory (requires `fs-loader` + `toml-backend` features).
    #[cfg(all(feature = "fs-loader", feature = "toml-backend"))]
    pub fn translations_from_dir(self, path: impl AsRef<std::path::Path>) -> Result<Self, I18nError> {
        let backend = crate::backend::TomlBackend::from_dir(path)?;
        Ok(self.add_backend(Arc::new(backend)))
    }

    // ---- JSON backend (feature-gated) ----

    /// Add compile-time-embedded JSON translations.
    ///
    /// Can be called multiple times; each call adds a new backend to the chain.
    #[cfg(feature = "json-backend")]
    pub fn json_translations(self, pairs: &[(&str, &str)]) -> Result<Self, I18nError> {
        let backend = crate::json_backend::JsonBackend::from_embedded(pairs)?;
        Ok(self.add_backend(Arc::new(backend)))
    }

    /// Add JSON translations from a directory (requires `fs-loader` + `json-backend` features).
    #[cfg(all(feature = "fs-loader", feature = "json-backend"))]
    pub fn json_from_dir(self, path: impl AsRef<std::path::Path>) -> Result<Self, I18nError> {
        let backend = crate::json_backend::JsonBackend::from_dir(path)?;
        Ok(self.add_backend(Arc::new(backend)))
    }

    // ---- YAML backend (feature-gated) ----

    /// Add compile-time-embedded YAML translations.
    ///
    /// Can be called multiple times; each call adds a new backend to the chain.
    #[cfg(feature = "yaml-backend")]
    pub fn yaml_translations(self, pairs: &[(&str, &str)]) -> Result<Self, I18nError> {
        let backend = crate::yaml_backend::YamlBackend::from_embedded(pairs)?;
        Ok(self.add_backend(Arc::new(backend)))
    }

    /// Add YAML translations from a directory (requires `fs-loader` + `yaml-backend` features).
    #[cfg(all(feature = "fs-loader", feature = "yaml-backend"))]
    pub fn yaml_from_dir(self, path: impl AsRef<std::path::Path>) -> Result<Self, I18nError> {
        let backend = crate::yaml_backend::YamlBackend::from_dir(path)?;
        Ok(self.add_backend(Arc::new(backend)))
    }

    // ---- Generic ----

    /// Add a custom [`Backend`] implementation.
    pub fn add_backend(mut self, backend: Arc<dyn Backend>) -> Self {
        self.backends.push(backend);
        self
    }

    /// Build the [`I18n`] instance.
    ///
    /// If multiple backends were added, they are wrapped in a
    /// [`crate::ChainedBackend`] (first added = highest priority).
    pub fn build(self) -> Result<I18n, I18nError> {
        let backend: Arc<dyn Backend> = match self.backends.len() {
            0 => return Err(I18nError::NoBackend),
            1 => self.backends.into_iter().next().unwrap(),
            _ => Arc::new(crate::backend::ChainedBackend::new(self.backends)),
        };

        Ok(I18n::new(backend, self.default_locale, self.fallback_locale))
    }
}
