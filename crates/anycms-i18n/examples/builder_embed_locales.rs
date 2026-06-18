//! Demonstrates `embed_locales!("path")` macro + `I18nBuilder` —
//! automatic compile-time scanning without manually listing each locale file.
//!
//! Also shows `optional_translations_from_dir` for runtime override.
//!
//! Run:
//!   cargo run -p anycms-i18n --example builder_embed_locales --features "init,fs-loader"
//!
//! Compare with the manual approach in `builder.rs`:
//!   .embedded_translations(&[
//!       ("en", include_str!("../../../locales/en.toml")),
//!       ("zh-CN", include_str!("../../../locales/zh-CN.toml")),
//!       ("ja", include_str!("../../../locales/ja.toml")),
//!   ])
//!
//! vs the automatic approach here:
//!   .embedded_translations(embed_locales!("../../locales"))

use anycms_i18n::{I18nBuilder, embed_locales};

fn main() {
    println!("=== embed_locales! + I18nBuilder ===\n");

    // embed_locales! scans the directory at compile time and generates
    // the same &[("locale", include_str!(...))] array — no manual listing needed.
    let i18n = I18nBuilder::new()
        .default_locale("zh-CN")
        .fallback_locale("en")
        // Optional: runtime directory override (non-fatal if missing)
        .optional_translations_from_dir("locales")
        .expect("optional_translations_from_dir failed")
        // Auto-scan & embed all .toml files at compile time
        .embedded_translations(embed_locales!("../../locales"))
        .expect("embedded_translations failed")
        .build()
        .expect("build failed");

    println!("Default locale:  {}", i18n.default_locale());
    println!("Fallback locale: {}", i18n.fallback_locale());
    println!("Available locales: {:?}", i18n.available_locales());
    println!();

    println!("welcome (default/zh-CN): {}", i18n.t("welcome"));
    println!(
        "welcome (en):            {}",
        i18n.t_with_locale("welcome", "en")
    );
    println!(
        "welcome (ja):            {}",
        i18n.t_with_locale("welcome", "ja")
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
    println!("items (0): {}", i18n.t_with_count("items", "zh-CN", 0, &[]));
    println!("items (1): {}", i18n.t_with_count("items", "zh-CN", 1, &[]));
    println!("items (5): {}", i18n.t_with_count("items", "zh-CN", 5, &[]));

    println!("\n=== Fallback Chain ===");
    // ja has no items.zero/one — falls back to items.other
    println!(
        "items (1, ja): {}",
        i18n.t_with_count("items", "ja", 1, &[])
    );
    // nonexistent key → returns the key itself
    println!("nonexistent: {}", i18n.t("nonexistent.key"));
}
