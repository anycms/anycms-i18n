//! Builder for constructing an [`I18n`] instance.

use std::sync::Arc;

use crate::backend::TomlBackend;
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

    /// Add compile-time-embedded translations as a [`TomlBackend`].
    ///
    /// Can be called multiple times; each call adds a new backend to the chain.
    pub fn embedded_translations(self, pairs: &[(&str, &str)]) -> Result<Self, I18nError> {
        let backend = TomlBackend::from_embedded(pairs)?;
        Ok(self.add_backend(Arc::new(backend)))
    }

    /// Add translations from a directory (requires `fs-loader` feature).
    #[cfg(feature = "fs-loader")]
    pub fn translations_from_dir(self, path: impl AsRef<std::path::Path>) -> Result<Self, I18nError> {
        let backend = TomlBackend::from_dir(path)?;
        Ok(self.add_backend(Arc::new(backend)))
    }

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
