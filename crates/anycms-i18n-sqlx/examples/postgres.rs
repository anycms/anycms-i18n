//! Example: Loading translations from PostgreSQL.
//!
//! Requires a running PostgreSQL instance with the `i18n_translations` table.
//!
//! Expected schema:
//! ```sql
//! CREATE TABLE i18n_translations (
//!     locale  TEXT NOT NULL,
//!     key     TEXT NOT NULL,
//!     value   TEXT NOT NULL,
//!     PRIMARY KEY (locale, key)
//! );
//! ```
//!
//! Run: `cargo run -p anycms-i18n-sqlx --example postgres --features postgres`

use std::sync::Arc;

use anycms_i18n::{Backend, I18nBuilder};
use anycms_i18n_sqlx::SqlxBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = sqlx::PgPool::connect("postgres://user:pass@localhost/i18n").await?;

    let backend = SqlxBackend::from_postgres(&pool).await?;
    println!("Loaded locales: {:?}", backend.available_locales());

    let i18n = I18nBuilder::new()
        .default_locale("en")
        .fallback_locale("en")
        .add_backend(Arc::new(backend))
        .build()?;

    println!("welcome: {}", i18n.t("welcome"));
    Ok(())
}
