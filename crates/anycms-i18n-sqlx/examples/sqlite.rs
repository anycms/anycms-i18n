//! Example: Loading translations from SQLite.
//!
//! Uses an in-memory SQLite database so it runs without any external setup.
//! Inserts sample data, then loads it into a `SqlxBackend`.
//!
//! Run: `cargo run -p anycms-i18n-sqlx --example sqlite --features sqlite`

use std::sync::Arc;

use anycms_i18n::{Backend, I18nBuilder};
use anycms_i18n_sqlx::SqlxBackend;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to an in-memory SQLite database
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;

    // Create the translations table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS i18n_translations (
            locale  TEXT NOT NULL,
            key     TEXT NOT NULL,
            value   TEXT NOT NULL,
            PRIMARY KEY (locale, key)
        )
        "#,
    )
    .execute(&pool)
    .await?;

    // Insert sample translations
    let translations = vec![
        // English
        ("en", "welcome", "Welcome to Anycms!"),
        ("en", "greeting", "Hello, %{name}!"),
        ("en", "errors.not_found", "Page not found"),
        ("en", "items.zero", "No items"),
        ("en", "items.one", "%{count} item"),
        ("en", "items.other", "%{count} items"),
        // Chinese
        ("zh-CN", "welcome", "欢迎使用 Anycms！"),
        ("zh-CN", "greeting", "你好，%{name}！"),
        ("zh-CN", "errors.not_found", "页面未找到"),
        ("zh-CN", "items.zero", "没有项目"),
        ("zh-CN", "items.one", "%{count} 个项目"),
        ("zh-CN", "items.other", "%{count} 个项目"),
        // Japanese
        ("ja", "welcome", "Anycms へようこそ！"),
        ("ja", "greeting", "こんにちは、%{name}さん！"),
        ("ja", "errors.not_found", "ページが見つかりません"),
    ];

    for (locale, key, value) in &translations {
        sqlx::query(
            "INSERT INTO i18n_translations (locale, key, value) VALUES (?, ?, ?)",
        )
        .bind(locale)
        .bind(key)
        .bind(value)
        .execute(&pool)
        .await?;
    }

    println!("Inserted {} translation rows.\n", translations.len());

    // Load translations from database into SqlxBackend
    let backend = Arc::new(SqlxBackend::from_sqlite(&pool).await?);
    println!("Loaded locales: {:?}\n", backend.available_locales());

    // Build I18n instance
    let i18n = I18nBuilder::new()
        .default_locale("en")
        .fallback_locale("en")
        .add_backend(backend.clone())
        .build()?;

    // ---- Translate ----
    println!("=== Simple Translations ===");
    println!("welcome (en):    {}", i18n.t_with_locale("welcome", "en"));
    println!("welcome (zh-CN): {}", i18n.t_with_locale("welcome", "zh-CN"));
    println!("welcome (ja):    {}", i18n.t_with_locale("welcome", "ja"));

    println!("\n=== Nested Keys ===");
    println!(
        "errors.not_found (en):    {}",
        i18n.t_with_locale("errors.not_found", "en")
    );
    println!(
        "errors.not_found (zh-CN): {}",
        i18n.t_with_locale("errors.not_found", "zh-CN")
    );

    println!("\n=== Interpolation ===");
    println!(
        "greeting (en):    {}",
        i18n.t_with_args("greeting", "en", &[("name", "world")])
    );
    println!(
        "greeting (zh-CN): {}",
        i18n.t_with_args("greeting", "zh-CN", &[("name", "世界")])
    );

    println!("\n=== Plural ===");
    for count in [0, 1, 5] {
        println!(
            "items ({count}) [en]:    {}",
            i18n.t_with_count("items", "en", count, &[])
        );
    }

    // ---- Reload from DB ----
    println!("\n=== Hot Reload Simulation ===");
    println!("Adding a new key via SQL...");
    sqlx::query(
        "INSERT INTO i18n_translations (locale, key, value) VALUES ('en', 'goodbye', 'Goodbye!')",
    )
    .execute(&pool)
    .await?;

    // Reload the backend in-place
    backend.reload_sqlite(&pool).await?;
    println!(
        "After reload — goodbye (en): {}",
        i18n.t_with_locale("goodbye", "en")
    );

    Ok(())
}
