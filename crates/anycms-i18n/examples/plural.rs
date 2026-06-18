//! Plural forms example for anycms-i18n.
//!
//! Demonstrates plural category selection for different language families:
//! - English: one/other (1 item vs 2 items)
//! - Chinese: other only (no plural distinction)
//! - Russian: one/few/many/other (complex Slavic rules)
//! - Arabic: zero/one/two/few/many/other (most complex)

use anycms_i18n::{I18nBuilder, plural_category, set_global};

fn main() {
    // Build I18n with embedded locales. We include en, zh-CN, ja from files,
    // and manually add Russian and Arabic with inline plural data.
    let i18n = I18nBuilder::new()
        .default_locale("en")
        .fallback_locale("en")
        .embedded_translations(&[
            ("en", include_str!("../../../locales/en.toml")),
            ("zh-CN", include_str!("../../../locales/zh-CN.toml")),
            ("ja", include_str!("../../../locales/ja.toml")),
            ("ru", RU_TOML),
            ("ar", AR_TOML),
        ])
        .expect("failed to load embedded translations")
        .build()
        .expect("failed to build I18n");

    set_global(i18n.clone()).expect("failed to set global i18n");

    // --- Plural categories directly ---
    println!("=== Plural Categories (direct) ===");
    println!("English:");
    print_plural_categories("en", &[0, 1, 2, 5]);

    println!("Chinese:");
    print_plural_categories("zh-CN", &[0, 1, 2, 5]);

    println!("Japanese:");
    print_plural_categories("ja", &[0, 1, 2, 5]);

    println!("Russian:");
    print_plural_categories("ru", &[0, 1, 2, 3, 4, 5, 11, 12, 21, 22, 25, 101]);

    println!("Arabic:");
    print_plural_categories("ar", &[0, 1, 2, 3, 5, 10, 11, 50, 99, 100, 200]);

    // --- English plurals with translations ---
    println!();
    println!("=== English Plural Translations (items) ===");
    for count in [0, 1, 2, 5, 100] {
        println!(
            "  count={:>3} => {}",
            count,
            i18n.t_with_count("items", "en", count, &[])
        );
    }

    // --- Chinese plurals (no distinction) ---
    println!();
    println!("=== Chinese Plural Translations (items) ===");
    for count in [0, 1, 2, 5, 100] {
        println!(
            "  count={:>3} => {}",
            count,
            i18n.t_with_count("items", "zh-CN", count, &[])
        );
    }

    // --- Japanese plurals (no distinction) ---
    println!();
    println!("=== Japanese Plural Translations (items) ===");
    for count in [0, 1, 5] {
        println!(
            "  count={:>3} => {}",
            count,
            i18n.t_with_count("items", "ja", count, &[])
        );
    }

    // --- Russian plurals ---
    println!();
    println!("=== Russian Plural Translations (apples) ===");
    for count in [0, 1, 2, 5, 11, 21, 22, 25, 101, 111] {
        println!(
            "  count={:>3} => {}",
            count,
            i18n.t_with_count("apples", "ru", count, &[])
        );
    }

    // --- Arabic plurals ---
    println!();
    println!("=== Arabic Plural Translations (files) ===");
    for count in [0, 1, 2, 3, 5, 10, 11, 50, 99, 100, 200] {
        println!(
            "  count={:>3} => {}",
            count,
            i18n.t_with_count("files", "ar", count, &[])
        );
    }

    // --- Using the t!() macro with count ---
    println!();
    println!("=== t!() Macro with Count ===");
    println!(
        "t!(\"items\", count=0)  => {}",
        anycms_i18n::t!("items", count = 0)
    );
    println!(
        "t!(\"items\", count=1)  => {}",
        anycms_i18n::t!("items", count = 1)
    );
    println!(
        "t!(\"items\", count=5)  => {}",
        anycms_i18n::t!("items", count = 5)
    );
    println!(
        "t!(\"items\", locale=\"zh-CN\", count=5) => {}",
        anycms_i18n::t!("items", locale = "zh-CN", count = 5)
    );
}

fn print_plural_categories(locale: &str, counts: &[i64]) {
    let categories: Vec<String> = counts
        .iter()
        .map(|c| format!("{}={}", c, plural_category(locale, *c).suffix()))
        .collect();
    println!("  {}", categories.join(", "));
}

// Inline TOML for Russian plural translations
const RU_TOML: &str = r#"
welcome = "Добро пожаловать!"

[apples]
one = "%{count} яблоко"
few = "%{count} яблока"
many = "%{count} яблок"
other = "%{count} яблока"
"#;

// Inline TOML for Arabic plural translations
const AR_TOML: &str = r#"
welcome = "مرحبا!"

[files]
zero = "لا توجد ملفات"
one = "ملف واحد"
two = "ملفان"
few = "%{count} ملفات"
many = "%{count} ملفا"
other = "%{count} ملف"
"#;
