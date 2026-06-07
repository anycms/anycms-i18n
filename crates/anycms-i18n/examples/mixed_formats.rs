//! Demonstrates mixing multiple translation formats.
//!
//! JSON overrides take priority (added first), TOML serves as fallback.
//!
//! Run: `cargo run -p anycms-i18n --example mixed_formats --features "all-backends,init"`

use anycms_i18n::{set_global, t, I18nBuilder};

fn main() {
    let i18n = I18nBuilder::new()
        .default_locale("en")
        .fallback_locale("en")
        // JSON overrides take priority (added first)
        .json_translations(&[("en", r#"{"welcome": "Hello from JSON!"}"#)])
        .unwrap()
        // TOML as fallback
        .embedded_translations(&[("en", include_str!("../../../locales/en.toml"))])
        .unwrap()
        .build()
        .unwrap();

    set_global(i18n).unwrap();

    println!("=== Mixed Formats (JSON override + TOML fallback) ===");

    // "welcome" comes from JSON (higher priority)
    println!(
        "welcome (from JSON override): {}",
        t!("welcome", locale = "en")
    );

    // Other keys fall through to TOML
    println!(
        "greeting (from TOML fallback): {}",
        t!("greeting", locale = "en", name = "world")
    );
    println!(
        "errors.not_found (from TOML fallback): {}",
        t!("errors.not_found", locale = "en")
    );
    println!(
        "navigation.home (from TOML fallback): {}",
        t!("navigation.home", locale = "en")
    );

    // Plural also falls through
    println!("\n=== Plural (from TOML fallback) ===");
    println!("items (0): {}", t!("items", count = 0, locale = "en"));
    println!("items (1): {}", t!("items", count = 1, locale = "en"));
    println!("items (5): {}", t!("items", count = 5, locale = "en"));
}
