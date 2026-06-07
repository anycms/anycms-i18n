//! Demonstrates hot-reloading translations from a locale directory.
//!
//! Watches `locales/` for `.toml` file changes. Edit any file while running
//! to see translations update live without restart.
//!
//! Run: `cargo run -p anycms-i18n --example hot_reload --features hot-reload`

use std::thread;
use std::time::Duration;

use anycms_i18n::{i18n, t};

fn main() {
    i18n!("locales", default = "en", fallback = "en", hot_reload);

    println!("Watching locales/ for changes... (Ctrl+C to stop)");
    println!("Edit any .toml file in locales/ and watch translations update.\n");

    loop {
        println!(
            "  t!(\"welcome\")               = {}",
            t!("welcome"),
        );
        println!(
            "  t!(\"welcome\", locale=zh-CN)   = {}",
            t!("welcome", locale = "zh-CN"),
        );
        println!(
            "  t!(\"welcome\", locale=ja)      = {}",
            t!("welcome", locale = "ja"),
        );
        println!();

        thread::sleep(Duration::from_secs(3));
    }
}
