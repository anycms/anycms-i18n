//! Builder pattern example for anycms-i18n.
//!
//! Demonstrates I18nBuilder with all configuration options:
//! default_locale, fallback_locale, embedded_translations,
//! and chained backends.

use std::sync::Arc;

use anycms_i18n::{ChainedBackend, I18n, I18nBuilder, TomlBackend};

fn main() {
    // --- Basic builder with all options ---
    println!("=== Basic Builder ===");

    let i18n = I18nBuilder::new()
        .default_locale("en")
        .fallback_locale("en")
        .embedded_translations(&[
            ("en", include_str!("../../../locales/en.toml")),
            ("zh-CN", include_str!("../../../locales/zh-CN.toml")),
            ("ja", include_str!("../../../locales/ja.toml")),
        ])
        .expect("failed to load embedded translations")
        .build()
        .expect("failed to build I18n");

    println!("Default locale:  {}", i18n.default_locale());
    println!("Fallback locale: {}", i18n.fallback_locale());
    println!("Available locales: {:?}", i18n.available_locales());
    println!("Debug view: {:?}", i18n);
    println!();

    // --- Default locale affects t() calls ---
    println!("=== Default Locale Effect ===");
    let i18n_zh = I18nBuilder::new()
        .default_locale("zh-CN")
        .fallback_locale("en")
        .embedded_translations(&[
            ("en", include_str!("../../../locales/en.toml")),
            ("zh-CN", include_str!("../../../locales/zh-CN.toml")),
        ])
        .expect("failed to load embedded translations")
        .build()
        .expect("failed to build I18n");

    // t() uses the default locale (zh-CN in this case)
    println!("t('welcome') with zh-CN default: {}", i18n_zh.t("welcome"));
    println!(
        "t_with_locale('welcome', 'en'):   {}",
        i18n_zh.t_with_locale("welcome", "en")
    );
    println!();

    // --- Chained backends ---
    // Build two separate TomlBackend instances, then chain them.
    // The first backend added has highest priority.
    println!("=== Chained Backends ===");

    // Backend 1: custom overrides (highest priority)
    let backend1 = {
        let b = TomlBackend::new();
        b.add_locale_from_str("en", r#"welcome = "Hey from override!""#)
            .unwrap();
        b.add_locale_from_str("zh-CN", r#"welcome = "覆盖翻译！""#)
            .unwrap();
        Arc::new(b) as Arc<dyn anycms_i18n::Backend>
    };

    // Backend 2: full locale files (lower priority)
    let backend2 = {
        let b = TomlBackend::new();
        b.add_locale_from_str("en", include_str!("../../../locales/en.toml"))
            .unwrap();
        b.add_locale_from_str("zh-CN", include_str!("../../../locales/zh-CN.toml"))
            .unwrap();
        Arc::new(b) as Arc<dyn anycms_i18n::Backend>
    };

    let chained = ChainedBackend::new(vec![backend1, backend2]);
    let i18n_chained = I18n::new(Arc::new(chained), "en", "en");

    // "welcome" comes from backend1 (override)
    println!("chained welcome (en):    {}", i18n_chained.t("welcome"));
    println!(
        "chained welcome (zh-CN): {}",
        i18n_chained.t_with_locale("welcome", "zh-CN")
    );

    // "errors.not_found" falls through to backend2
    println!(
        "chained errors.not_found (en): {}",
        i18n_chained.t("errors.not_found")
    );
    println!();

    // --- Using add_backend to chain via the builder ---
    println!("=== Builder with add_backend ===");

    let custom = {
        let b = TomlBackend::new();
        b.add_locale_from_str("en", r#"app.title = "My Custom App""#)
            .unwrap();
        Arc::new(b) as Arc<dyn anycms_i18n::Backend>
    };

    let i18n_custom = I18nBuilder::new()
        .default_locale("en")
        .fallback_locale("en")
        .add_backend(custom)
        .embedded_translations(&[("en", include_str!("../../../locales/en.toml"))])
        .expect("failed to load embedded translations")
        .build()
        .expect("failed to build I18n");

    // "app.title" from the custom backend (added first = highest priority)
    println!("app.title (custom backend): {}", i18n_custom.t("app.title"));

    // "welcome" falls through to embedded translations
    println!("welcome (embedded): {}", i18n_custom.t("welcome"));
}
