//! Basic usage with JSON format.
//!
//! Run: `cargo run -p anycms-i18n --example json_basic --features "json-backend,init"`

use anycms_i18n::{I18nBuilder, set_global, t};

fn main() {
    let i18n = I18nBuilder::new()
        .default_locale("en")
        .fallback_locale("en")
        .json_translations(&[
            ("en", include_str!("../../../locales/en.json")),
            ("zh-CN", include_str!("../../../locales/zh-CN.json")),
            ("ja", include_str!("../../../locales/ja.json")),
        ])
        .unwrap()
        .build()
        .unwrap();

    set_global(i18n).unwrap();

    // Show translations
    println!("=== JSON Backend ===");
    println!("welcome (en):    {}", t!("welcome", locale = "en"));
    println!("welcome (zh-CN): {}", t!("welcome", locale = "zh-CN"));
    println!("welcome (ja):    {}", t!("welcome", locale = "ja"));

    // Nested keys
    println!("\n=== Nested Keys ===");
    println!(
        "errors.not_found (en): {}",
        t!("errors.not_found", locale = "en")
    );
    println!(
        "navigation.home (ja):  {}",
        t!("navigation.home", locale = "ja")
    );

    // Interpolation
    println!("\n=== Interpolation ===");
    println!(
        "greeting (en): {}",
        t!("greeting", locale = "en", name = "world")
    );

    // Plural
    println!("\n=== Plural ===");
    println!("items (0): {}", t!("items", count = 0, locale = "en"));
    println!("items (1): {}", t!("items", count = 1, locale = "en"));
    println!("items (5): {}", t!("items", count = 5, locale = "en"));
}
