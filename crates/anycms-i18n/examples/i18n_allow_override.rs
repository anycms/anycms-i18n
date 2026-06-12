//! Demonstrates `i18n!("...", allow_override)` — compile-time embedding
//! with optional runtime file override.
//!
//! - Translations are compiled into the binary as defaults.
//! - If the runtime directory `locales/` exists alongside the binary,
//!   those files override matching keys at runtime.
//! - If `locales/` is absent, compiled defaults are used silently.
//!
//! Run:
//!   cargo run -p anycms-i18n --example i18n_allow_override --features "init,fs-loader"
//!
//! To test runtime override, create a `locales/` directory next to the binary
//! with a partial override file, e.g. `locales/zh-CN.toml`:
//!   welcome = "运行时覆盖的欢迎词！"

use anycms_i18n::{i18n, t};

fn main() {
    // Compile-embed all .toml files from ../../locales,
    // then try loading from a "locales/" runtime directory (non-fatal if missing).
    // Runtime keys take priority over compiled keys.
    i18n!("../../locales", default = "zh-CN", fallback = "en", allow_override);

    println!("=== i18n! with allow_override ===");
    println!("(if locales/ dir exists, runtime files override compiled defaults)\n");

    println!("welcome (default/zh-CN): {}", t!("welcome"));
    println!("welcome (en):            {}", t!("welcome", locale = "en"));
    println!("welcome (ja):            {}", t!("welcome", locale = "ja"));

    println!("\n=== Nested Keys ===");
    println!("errors.not_found (en):    {}", t!("errors.not_found", locale = "en"));
    println!("errors.not_found (zh-CN): {}", t!("errors.not_found"));

    println!("\n=== Interpolation ===");
    println!("greeting (en):    {}", t!("greeting", locale = "en", name = "world"));
    println!("greeting (zh-CN): {}", t!("greeting", name = "世界"));

    println!("\n=== Available Locales ===");
    if let Some(g) = anycms_i18n::global() {
        println!("{:?}", g.available_locales());
    }
}
