//! Basic usage example for anycms-i18n.
//!
//! Demonstrates the `i18n!()` init macro and the `t!()` translation macro.
//!
//! Run: `cargo run -p anycms-i18n --example basic --features init`

use anycms_i18n::{i18n, t};

fn main() {
    // One line to initialize i18n — auto-scans all .toml files in the directory
    i18n!("../../locales", default = "zh-CN", fallback = "en");

    println!("=== Simple Translations (t! macro) ===");
    println!("welcome (default/zh-CN): {}", t!("welcome"));
    println!("welcome (en):            {}", t!("welcome", locale = "en"));
    println!("welcome (ja):            {}", t!("welcome", locale = "ja"));

    println!("\n=== Nested Keys ===");
    println!(
        "errors.not_found (en):    {}",
        t!("errors.not_found", locale = "en")
    );
    println!("errors.not_found (zh-CN): {}", t!("errors.not_found"));
    println!(
        "navigation.home (ja):     {}",
        t!("navigation.home", locale = "ja")
    );

    println!("\n=== Interpolation ===");
    println!(
        "greeting (en):    {}",
        t!("greeting", locale = "en", name = "world")
    );
    println!("greeting (zh-CN): {}", t!("greeting", name = "世界"));
    println!(
        "greeting (ja):    {}",
        t!("greeting", locale = "ja", name = "世界")
    );

    println!("\n=== Plural ===");
    println!("items (0): {}", t!("items", count = 0));
    println!("items (1): {}", t!("items", count = 1));
    println!("items (5): {}", t!("items", count = 5));

    println!("\n=== Missing Key (falls back to key string) ===");
    println!("nonexistent.key: {}", t!("nonexistent.key"));
}
