//! # anycms-i18n-sqlx
//!
//! SQLx database backend for [`anycms-i18n`].
//!
//! Loads translations from any SQLx-supported database (PostgreSQL, MySQL, SQLite)
//! into an in-memory cache at startup, then serves translations synchronously
//! via the [`Backend`] trait.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use anycms_i18n_sqlx::SqlxBackend;
//!
//! // PostgreSQL
//! let pool = sqlx::PgPool::connect("postgres://...").await?;
//! let backend = SqlxBackend::from_postgres(&pool).await?;
//!
//! // MySQL
//! let pool = sqlx::MySqlPool::connect("mysql://...").await?;
//! let backend = SqlxBackend::from_mysql(&pool).await?;
//!
//! // SQLite
//! let pool = sqlx::SqlitePool::connect("sqlite:translations.db").await?;
//! let backend = SqlxBackend::from_sqlite(&pool).await?;
//!
//! // Use as a Backend
//! use anycms_i18n::Backend;
//! assert!(backend.has_locale("en"));
//! ```
//!
//! ## Custom Table / Column Names
//!
//! Use [`SqlxBackendBuilder`] to customize the table and column names:
//!
//! ```rust,ignore
//! let backend = SqlxBackendBuilder::new()
//!     .table("my_translations")
//!     .locale_col("lang")
//!     .key_col("msg_key")
//!     .value_col("msg_value")
//!     .build_postgres(&pool)
//!     .await?;
//! ```

use std::collections::HashMap;

use anycms_i18n::Backend;
#[cfg(any(feature = "postgres", feature = "mysql", feature = "sqlite"))]
use anycms_i18n::I18nError;
use dashmap::DashMap;

// ---------------------------------------------------------------------------
// SqlxBackend
// ---------------------------------------------------------------------------

/// Database-backed translation backend powered by SQLx.
///
/// Translations are loaded asynchronously into an in-memory [`DashMap`] cache.
/// All [`Backend`] trait methods are synchronous and read from cache only.
pub struct SqlxBackend {
    cache: DashMap<String, HashMap<String, String>>,
}

impl SqlxBackend {
    /// Create an empty backend with no translations.
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }

    /// Create from an iterator of `(locale, key, value)` tuples.
    ///
    /// This is the core constructor used by all database-specific loaders.
    /// You can also call this directly if you have translations from another source.
    ///
    /// # Example
    ///
    /// ```
    /// use anycms_i18n::Backend;
    /// use anycms_i18n_sqlx::SqlxBackend;
    ///
    /// let backend = SqlxBackend::from_translations(vec![
    ///     ("en".to_string(), "hello".to_string(), "Hello".to_string()),
    ///     ("zh-CN".to_string(), "hello".to_string(), "你好".to_string()),
    /// ]);
    ///
    /// assert_eq!(backend.get("en", "hello").as_deref(), Some("Hello"));
    /// assert_eq!(backend.get("zh-CN", "hello").as_deref(), Some("你好"));
    /// assert!(backend.has_locale("en"));
    /// assert!(!backend.has_locale("ja"));
    /// ```
    pub fn from_translations(
        translations: impl IntoIterator<Item = (String, String, String)>,
    ) -> Self {
        let cache = DashMap::new();
        for (locale, key, value) in translations {
            cache
                .entry(locale)
                .or_insert_with(HashMap::new)
                .insert(key, value);
        }
        Self { cache }
    }

    /// Async reload: clear the cache and re-populate from an iterator.
    ///
    /// This is a convenience wrapper that clears the internal cache and rebuilds
    /// it from the provided translations. In practice you would call one of the
    /// database-specific reload methods instead.
    pub fn reload_from_translations(
        &self,
        translations: impl IntoIterator<Item = (String, String, String)>,
    ) {
        self.cache.clear();
        for (locale, key, value) in translations {
            self.cache.entry(locale).or_default().insert(key, value);
        }
    }

    // -- PostgreSQL -----------------------------------------------------------

    /// Load all translations from a PostgreSQL pool.
    ///
    /// Queries `SELECT locale, key, value FROM i18n_translations`.
    #[cfg(feature = "postgres")]
    pub async fn from_postgres(pool: &sqlx::PgPool) -> Result<Self, I18nError> {
        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT locale, key, value FROM i18n_translations")
                .fetch_all(pool)
                .await
                .map_err(|e| I18nError::DatabaseError(e.to_string()))?;

        Ok(Self::from_translations(rows))
    }

    /// Reload translations from a PostgreSQL pool.
    #[cfg(feature = "postgres")]
    pub async fn reload_postgres(&self, pool: &sqlx::PgPool) -> Result<(), I18nError> {
        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT locale, key, value FROM i18n_translations")
                .fetch_all(pool)
                .await
                .map_err(|e| I18nError::DatabaseError(e.to_string()))?;

        self.reload_from_translations(rows);
        Ok(())
    }

    // -- MySQL ----------------------------------------------------------------

    /// Load all translations from a MySQL pool.
    ///
    /// Queries `SELECT locale, key, value FROM i18n_translations`.
    #[cfg(feature = "mysql")]
    pub async fn from_mysql(pool: &sqlx::MySqlPool) -> Result<Self, I18nError> {
        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT locale, key, value FROM i18n_translations")
                .fetch_all(pool)
                .await
                .map_err(|e| I18nError::DatabaseError(e.to_string()))?;

        Ok(Self::from_translations(rows))
    }

    /// Reload translations from a MySQL pool.
    #[cfg(feature = "mysql")]
    pub async fn reload_mysql(&self, pool: &sqlx::MySqlPool) -> Result<(), I18nError> {
        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT locale, key, value FROM i18n_translations")
                .fetch_all(pool)
                .await
                .map_err(|e| I18nError::DatabaseError(e.to_string()))?;

        self.reload_from_translations(rows);
        Ok(())
    }

    // -- SQLite ---------------------------------------------------------------

    /// Load all translations from a SQLite pool.
    ///
    /// Queries `SELECT locale, key, value FROM i18n_translations`.
    #[cfg(feature = "sqlite")]
    pub async fn from_sqlite(pool: &sqlx::SqlitePool) -> Result<Self, I18nError> {
        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT locale, key, value FROM i18n_translations")
                .fetch_all(pool)
                .await
                .map_err(|e| I18nError::DatabaseError(e.to_string()))?;

        Ok(Self::from_translations(rows))
    }

    /// Reload translations from a SQLite pool.
    #[cfg(feature = "sqlite")]
    pub async fn reload_sqlite(&self, pool: &sqlx::SqlitePool) -> Result<(), I18nError> {
        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT locale, key, value FROM i18n_translations")
                .fetch_all(pool)
                .await
                .map_err(|e| I18nError::DatabaseError(e.to_string()))?;

        self.reload_from_translations(rows);
        Ok(())
    }
}

impl Default for SqlxBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Backend for SqlxBackend {
    fn get(&self, locale: &str, key: &str) -> Option<String> {
        self.cache.get(locale).and_then(|map| map.get(key).cloned())
    }

    fn available_locales(&self) -> Vec<String> {
        self.cache.iter().map(|r| r.key().clone()).collect()
    }

    fn has_locale(&self, locale: &str) -> bool {
        self.cache.contains_key(locale)
    }

    fn dump(&self, locale: &str) -> HashMap<String, String> {
        self.cache
            .get(locale)
            .map(|m| m.clone())
            .unwrap_or_default()
    }
}

// ---------------------------------------------------------------------------
// SqlxBackendBuilder
// ---------------------------------------------------------------------------

/// Builder for [`SqlxBackend`] with custom table and column names.
///
/// ```rust,ignore
/// let backend = SqlxBackendBuilder::new()
///     .table("my_translations")
///     .locale_col("lang")
///     .build_postgres(&pool)
///     .await?;
/// ```
pub struct SqlxBackendBuilder {
    table: String,
    locale_col: String,
    key_col: String,
    value_col: String,
}

impl SqlxBackendBuilder {
    /// Create a new builder with default names:
    /// - table: `i18n_translations`
    /// - locale column: `locale`
    /// - key column: `key`
    /// - value column: `value`
    pub fn new() -> Self {
        Self {
            table: "i18n_translations".into(),
            locale_col: "locale".into(),
            key_col: "key".into(),
            value_col: "value".into(),
        }
    }

    /// Set a custom table name.
    pub fn table(mut self, name: impl Into<String>) -> Self {
        self.table = name.into();
        self
    }

    /// Set a custom locale column name.
    pub fn locale_col(mut self, name: impl Into<String>) -> Self {
        self.locale_col = name.into();
        self
    }

    /// Set a custom key column name.
    pub fn key_col(mut self, name: impl Into<String>) -> Self {
        self.key_col = name.into();
        self
    }

    /// Set a custom value column name.
    pub fn value_col(mut self, name: impl Into<String>) -> Self {
        self.value_col = name.into();
        self
    }

    /// Build the SQL query string from configured names.
    #[allow(dead_code)] // used by feature-gated build_* methods
    fn query(&self) -> String {
        format!(
            "SELECT {} AS locale, {} AS key, {} AS value FROM {}",
            self.locale_col, self.key_col, self.value_col, self.table,
        )
    }

    /// Build from a PostgreSQL pool.
    #[cfg(feature = "postgres")]
    pub async fn build_postgres(&self, pool: &sqlx::PgPool) -> Result<SqlxBackend, I18nError> {
        let sql = self.query();
        let rows: Vec<(String, String, String)> = sqlx::query_as(sql.as_str())
            .fetch_all(pool)
            .await
            .map_err(|e| I18nError::DatabaseError(e.to_string()))?;

        Ok(SqlxBackend::from_translations(rows))
    }

    /// Build from a MySQL pool.
    #[cfg(feature = "mysql")]
    pub async fn build_mysql(&self, pool: &sqlx::MySqlPool) -> Result<SqlxBackend, I18nError> {
        let sql = self.query();
        let rows: Vec<(String, String, String)> = sqlx::query_as(sql.as_str())
            .fetch_all(pool)
            .await
            .map_err(|e| I18nError::DatabaseError(e.to_string()))?;

        Ok(SqlxBackend::from_translations(rows))
    }

    /// Build from a SQLite pool.
    #[cfg(feature = "sqlite")]
    pub async fn build_sqlite(&self, pool: &sqlx::SqlitePool) -> Result<SqlxBackend, I18nError> {
        let sql = self.query();
        let rows: Vec<(String, String, String)> = sqlx::query_as(sql.as_str())
            .fetch_all(pool)
            .await
            .map_err(|e| I18nError::DatabaseError(e.to_string()))?;

        Ok(SqlxBackend::from_translations(rows))
    }
}

impl Default for SqlxBackendBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_backend() {
        let backend = SqlxBackend::new();
        assert!(!backend.has_locale("en"));
        assert!(backend.available_locales().is_empty());
        assert_eq!(backend.get("en", "hello"), None);
    }

    #[test]
    fn from_translations() {
        let backend = SqlxBackend::from_translations(vec![
            ("en".into(), "hello".into(), "Hello".into()),
            ("en".into(), "world".into(), "World".into()),
            ("zh-CN".into(), "hello".into(), "你好".into()),
        ]);

        assert!(backend.has_locale("en"));
        assert!(backend.has_locale("zh-CN"));
        assert!(!backend.has_locale("ja"));
        assert_eq!(backend.get("en", "hello"), Some("Hello".into()));
        assert_eq!(backend.get("en", "world"), Some("World".into()));
        assert_eq!(backend.get("zh-CN", "hello"), Some("你好".into()));
        assert_eq!(backend.get("en", "missing"), None);

        let mut locales = backend.available_locales();
        locales.sort();
        assert_eq!(locales, vec!["en", "zh-CN"]);
    }

    #[test]
    fn reload_clears_old_data() {
        let backend =
            SqlxBackend::from_translations(vec![("en".into(), "hello".into(), "Hello".into())]);
        assert_eq!(backend.get("en", "hello"), Some("Hello".into()));

        backend.reload_from_translations(vec![("de".into(), "hello".into(), "Hallo".into())]);

        assert_eq!(backend.get("en", "hello"), None);
        assert_eq!(backend.get("de", "hello"), Some("Hallo".into()));
    }

    #[test]
    fn builder_default_query() {
        let builder = SqlxBackendBuilder::new();
        assert_eq!(
            builder.query(),
            "SELECT locale AS locale, key AS key, value AS value FROM i18n_translations"
        );
    }

    #[test]
    fn builder_custom_query() {
        let builder = SqlxBackendBuilder::new()
            .table("my_table")
            .locale_col("lang")
            .key_col("msg_key")
            .value_col("msg_val");

        assert_eq!(
            builder.query(),
            "SELECT lang AS locale, msg_key AS key, msg_val AS value FROM my_table"
        );
    }

    #[test]
    fn dump_returns_all_keys_for_locale() {
        let backend = SqlxBackend::from_translations(vec![
            ("en".into(), "hello".into(), "Hello".into()),
            ("en".into(), "world".into(), "World".into()),
            ("zh-CN".into(), "hello".into(), "你好".into()),
        ]);

        let en = backend.dump("en");
        assert_eq!(en.len(), 2);
        assert_eq!(en.get("hello").unwrap(), "Hello");
        assert_eq!(en.get("world").unwrap(), "World");

        let zh = backend.dump("zh-CN");
        assert_eq!(zh.len(), 1);
        assert_eq!(zh.get("hello").unwrap(), "你好");

        // Missing locale -> empty map.
        assert!(backend.dump("ja").is_empty());
    }
}
