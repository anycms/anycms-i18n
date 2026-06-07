//! Locale fallback chain example for anycms-i18n.
//!
//! Demonstrates:
//! - The fallback chain: zh-Hans-CN -> zh-CN -> zh -> en
//! - Locale parsing and fallback_chain generation
//! - negotiate_locale with Accept-Language header values
//! - How missing translations fall back through the chain
//!
//! Run: `cargo run -p anycms-i18n --example fallback --features init`

use anycms_i18n::{i18n, negotiate_locale, Locale, t};

fn main() {
    i18n!("../../locales", default = "en", fallback = "en");

    // --- Locale parsing ---
    println!("=== Locale Parsing ===");
    for raw in ["en", "zh-CN", "zh-Hans-CN", "zh_Hans_TW"] {
        let loc = Locale::parse(raw).unwrap();
        println!(
            "{:<15} => language={}, script={:?}, region={:?}",
            raw, loc.language, loc.script, loc.region
        );
    }

    // --- Fallback chain generation ---
    println!("\n=== Fallback Chains (toward fallback='en') ===");
    for raw in ["zh-Hans-CN", "zh-CN", "zh", "en"] {
        let loc = Locale::parse(raw).unwrap();
        let chain = loc.fallback_chain("en");
        println!("{:<15} => {:?}", raw, chain);
    }

    // --- Translation fallback in action ---
    println!("\n=== Translation Fallback in Action ===");
    println!("welcome (zh-Hans-CN, falls to zh-CN): {}", t!("welcome", locale = "zh-Hans-CN"));
    println!("welcome (zh-TW, falls to en):          {}", t!("welcome", locale = "zh-TW"));
    println!("welcome (fr, falls to en):              {}", t!("welcome", locale = "fr"));

    // --- Key-level fallback ---
    println!("\n=== Key-Level Fallback ===");
    println!("items.zero (en):              {}", t!("items.zero", locale = "en"));
    println!("items.zero (zh-CN, falls en): {}", t!("items.zero"));

    // --- Accept-Language negotiation ---
    println!("\n=== Accept-Language Negotiation ===");
    let available = ["en", "zh-CN", "ja"];
    for (header, desc) in [
        ("zh-CN,en;q=0.9", "exact zh-CN match"),
        ("zh-TW,en;q=0.9", "zh-TW -> prefix matches zh-CN"),
        ("ja,en;q=0.5", "exact ja match"),
        ("fr,en;q=0.9", "fr not available, falls to en"),
        ("zh-Hans-CN,zh-CN;q=0.9,en;q=0.8", "prefix matches zh-CN"),
        ("de", "de not available, falls to default en"),
        ("", "empty header, falls to default en"),
    ] {
        let negotiated = negotiate_locale(header, &available, "en");
        println!("{:<35} => {} ({})", header, negotiated, desc);
    }
}
