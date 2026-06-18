//! Demonstrates the compile-embed + runtime-override pattern in depth.
//!
//! Steps:
//! 1. Create a temporary runtime directory with a partial override file
//! 2. Build I18n with compiled defaults + runtime override
//! 3. Show that runtime keys override compiled, while non-overridden keys
//!    fall through to the compiled defaults.
//!
//! Run:
//!   cargo run -p anycms-i18n --example runtime_override --features "init,fs-loader"

use anycms_i18n::{I18nBuilder, embed_locales};

fn main() {
    println!("=== Compile + Runtime Override Demo ===\n");

    // --- Set up a temporary runtime directory with partial overrides ---
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let dir_path = dir.path();

    // Only override "welcome" and "greeting" in zh-CN — everything else
    // should come from compiled defaults.
    std::fs::write(
        dir_path.join("zh-CN.toml"),
        r#"welcome = "🔥 运行时覆盖的欢迎词！"
greeting = "嗨，%{name}！这是运行时覆盖的！""#,
    )
    .expect("failed to write override file");

    // Also add a brand-new key that doesn't exist in compiled defaults
    std::fs::write(
        dir_path.join("en.toml"),
        r#"runtime_only = "This key only exists at runtime""#,
    )
    .expect("failed to write override file");

    println!("Runtime override dir: {}", dir_path.display());
    println!();

    // --- Build with override ---
    let i18n = I18nBuilder::new()
        .default_locale("zh-CN")
        .fallback_locale("en")
        // 1st: runtime files (highest priority) — partial overrides
        .optional_translations_from_dir(dir_path)
        .expect("optional_translations_from_dir failed")
        // 2nd: compiled defaults (fallback)
        .embedded_translations(embed_locales!("../../locales"))
        .expect("embedded_translations failed")
        .build()
        .expect("build failed");

    // --- Verify overrides ---
    println!("--- Overridden keys (runtime wins) ---");
    println!(
        "welcome (zh-CN):    {}  ← should be 🔥 overridden",
        i18n.t("welcome")
    );
    println!(
        "greeting (zh-CN):   {}  ← should be overridden",
        i18n.t_with_args("greeting", "zh-CN", &[("name", "测试")])
    );

    println!("\n--- Non-overridden keys (compiled fallback) ---");
    println!(
        "welcome (en):       {}  ← compiled default (en not overridden)",
        i18n.t_with_locale("welcome", "en")
    );
    println!(
        "errors.not_found:   {}  ← compiled default (key not in override)",
        i18n.t("errors.not_found")
    );
    println!(
        "navigation.home:    {}  ← compiled default",
        i18n.t("navigation.home")
    );

    println!("\n--- Runtime-only key (not in compiled defaults) ---");
    println!(
        "runtime_only (en):  {}  ← only exists in runtime file",
        i18n.t_with_locale("runtime_only", "en")
    );

    println!("\n--- Plural (compiled, not overridden) ---");
    println!(
        "items (0):          {}",
        i18n.t_with_count("items", "zh-CN", 0, &[])
    );
    println!(
        "items (5):          {}",
        i18n.t_with_count("items", "zh-CN", 5, &[])
    );

    println!("\n--- Available locales (union of runtime + compiled) ---");
    println!("{:?}", i18n.available_locales());

    // --- Now test WITHOUT runtime directory ---
    println!("\n=== Without Runtime Override (no dir) ===\n");

    let i18n_pure = I18nBuilder::new()
        .default_locale("zh-CN")
        .fallback_locale("en")
        .optional_translations_from_dir("/no/such/dir/locales")
        .expect("should not fail on missing dir")
        .embedded_translations(embed_locales!("../../locales"))
        .expect("embedded_translations failed")
        .build()
        .expect("build failed");

    println!(
        "welcome (zh-CN):    {}  ← compiled default (no override)",
        i18n_pure.t("welcome")
    );
    println!(
        "errors.not_found:   {}  ← compiled default",
        i18n_pure.t("errors.not_found")
    );
    println!(
        "runtime_only (en):  {}  ← key missing (runtime_only not in compiled)",
        i18n_pure.t_with_locale("runtime_only", "en")
    );
}
