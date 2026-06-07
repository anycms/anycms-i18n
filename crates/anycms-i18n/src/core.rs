//! Core traits and the I18n runtime.

use std::sync::Arc;

use crate::interpolate::interpolate;
use crate::locale::Locale;
use crate::plural::plural_category;

/// Translation backend trait.
///
/// Abstracts the source of translations (files, database, etc.).
/// Implementations must be `Send + Sync` for use across threads.
pub trait Backend: Send + Sync + 'static {
    /// Get the translated string for the given locale and key.
    fn get(&self, locale: &str, key: &str) -> Option<String>;

    /// Get all available locales.
    fn available_locales(&self) -> Vec<String>;

    /// Check if a locale is available.
    fn has_locale(&self, locale: &str) -> bool;
}

/// The core i18n runtime.
///
/// Holds a reference to a [`Backend`] and provides the translation API.
/// Thread-safe and cheaply cloneable (uses `Arc` internally).
#[derive(Clone)]
pub struct I18n {
    backend: Arc<dyn Backend>,
    default_locale: String,
    fallback_locale: String,
}

impl std::fmt::Debug for I18n {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I18n")
            .field("default_locale", &self.default_locale)
            .field("fallback_locale", &self.fallback_locale)
            .field("available_locales", &self.available_locales())
            .finish_non_exhaustive()
    }
}

impl I18n {
    /// Create a new I18n instance with the given backend and default locale.
    pub fn new(
        backend: Arc<dyn Backend>,
        default_locale: impl Into<String>,
        fallback_locale: impl Into<String>,
    ) -> Self {
        Self {
            backend,
            default_locale: default_locale.into(),
            fallback_locale: fallback_locale.into(),
        }
    }

    /// Translate a key using the default locale.
    pub fn t(&self, key: &str) -> String {
        self.translate(key, &self.default_locale, &[], None)
    }

    /// Translate a key with a specific locale.
    pub fn t_with_locale(&self, key: &str, locale: &str) -> String {
        self.translate(key, locale, &[], None)
    }

    /// Translate a key with interpolation arguments.
    pub fn t_with_args(&self, key: &str, locale: &str, args: &[(&str, &str)]) -> String {
        self.translate(key, locale, args, None)
    }

    /// Translate a key with interpolation and plural support.
    pub fn t_with_count(&self, key: &str, locale: &str, count: i64, args: &[(&str, &str)]) -> String {
        self.translate(key, locale, args, Some(count))
    }

    /// Full translation: locale + interpolation + plural.
    ///
    /// Follows the fallback chain:
    /// `locale` -> `locale without region` -> `language only` -> `fallback_locale`
    pub fn translate(
        &self,
        key: &str,
        locale: &str,
        args: &[(&str, &str)],
        count: Option<i64>,
    ) -> String {
        // Build the lookup key for plural forms
        let lookup_key = match count {
            Some(c) => {
                let cat = plural_category(locale, c);
                // Try "key.category" first, fall back to "key.other"
                let plural_key = format!("{key}.{}", cat.suffix());
                if self.backend.get(locale, &plural_key).is_some() {
                    plural_key
                } else {
                    key.to_string()
                }
            }
            None => key.to_string(),
        };

        // Try fallback chain
        let parsed = Locale::parse(locale).unwrap_or_else(|_| Locale::language_only(locale));
        let chain = parsed.fallback_chain(&self.fallback_locale);

        for loc in &chain {
            if let Some(template) = self.backend.get(loc, &lookup_key) {
                let mut all_args: Vec<(&str, String)> = args
                    .iter()
                    .map(|(k, v)| (*k, v.to_string()))
                    .collect();
                if let Some(c) = count {
                    all_args.push(("count", c.to_string()));
                }
                return interpolate(&template, &all_args);
            }
        }

        // Key not found — return the key itself as fallback
        tracing::warn!(key = %key, locale = %locale, "translation key not found");
        key.to_string()
    }

    /// Get the default locale.
    pub fn default_locale(&self) -> &str {
        &self.default_locale
    }

    /// Get the fallback locale.
    pub fn fallback_locale(&self) -> &str {
        &self.fallback_locale
    }

    /// Get all available locales from the backend.
    pub fn available_locales(&self) -> Vec<String> {
        self.backend.available_locales()
    }

    /// Get a reference to the backend (for advanced use).
    pub fn backend(&self) -> &Arc<dyn Backend> {
        &self.backend
    }
}
