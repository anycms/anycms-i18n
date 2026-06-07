//! Basic usage with YAML format.
//!
//! Run: `cargo run -p anycms-i18n --example yaml_basic --features "yaml-backend,init"`

use anycms_i18n::{set_global, t, I18nBuilder};

fn main() {
    let i18n = I18nBuilder::new()
        .default_locale("en")
        .fallback_locale("en")
        .yaml_translations(&[
            ("en", include_str!("../../../locales/en.yaml")),
            ("zh-CN", include_str!("../../../locales/zh-CN.yaml")),
            ("ja", include_str!("../../../locales/ja.yaml")),
        ])
        .unwrap()
        .build()
        .unwrap();

    set_global(i18n).unwrap();

    // Show translations
    println!("=== YAML Backend ===");
    println!("welcome (en):    {}", t!("welcome", locale = "en"));
    println!("welcome (zh-CN): {}", t!("welcome", locale = "zh-CN"));
    println!("welcome (ja):    {}", t!("welcome", locale = "ja"));

    // Nested keys
    println!("\n=== Nested Keys ===");
    println!(
        "errors.not_found (en): {}",
        t!("errors.not_found", locale = "en")
    );
    println!("navigation.home (ja):  {}", t!("navigation.home", locale = "ja"));

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
