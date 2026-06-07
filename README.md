# anycms-i18n

Internationalization (i18n) support for the [anycms-rs](https://github.com/anycms) ecosystem.

A lightweight, TOML-based i18n library for Rust with compile-time translation embedding, locale fallback chains, plural rules, and web framework integrations.

## Features

- **TOML-based translations** — one file per locale, familiar & easy to edit
- **Compile-time embedding** — translations baked into the binary via `include_str!`, zero runtime file I/O
- **`t!()` macro** — ergonomic translation with locale override, interpolation, and plural count
- **Fallback chains** — `zh-Hans-CN` → `zh-CN` → `zh` → `en` automatic fallback
- **Plural rules** — English, Chinese, Japanese, Russian, Arabic and more out of the box
- **`%{name}` interpolation** — variable substitution in translation strings
- **Backend trait** — plug in custom translation sources (database, HTTP, etc.)
- **ChainedBackend** — stack multiple backends with priority (e.g. DB overrides > files)
- **`i18n!()` macro** — one-line compile-time initialization
- **Actix-web integration** — middleware + extractor + frontend API routes
- **Axum integration** — Layer + extractor + frontend API routes

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
anycms-i18n = "0.1"
```

Create translation files:

```toml
# locales/en.toml
welcome = "Welcome to Anycms!"
greeting = "Hello, %{name}!"

[errors]
not_found = "Page not found"

[items]
zero = "No items"
one = "%{count} item"
other = "%{count} items"
```

```toml
# locales/zh-CN.toml
welcome = "欢迎使用 Anycms！"
greeting = "你好，%{name}！"

[errors]
not_found = "页面未找到"

[items]
other = "%{count} 个项目"
```

Use in code:

```rust
use anycms_i18n::{I18nBuilder, set_global, t};

fn main() {
    let i18n = I18nBuilder::new()
        .default_locale("en")
        .fallback_locale("en")
        .embedded_translations(&[
            ("en", include_str!("../locales/en.toml")),
            ("zh-CN", include_str!("../locales/zh-CN.toml")),
        ])
        .build()
        .unwrap();

    set_global(i18n).unwrap();

    // Simple translation
    assert_eq!(t!("welcome"), "Welcome to Anycms!");

    // Locale override
    assert_eq!(t!("welcome", locale = "zh-CN"), "欢迎使用 Anycms！");

    // Interpolation
    assert_eq!(t!("greeting", name = "world"), "Hello, world!");

    // Plural
    assert_eq!(t!("items", count = 0), "No items");
    assert_eq!(t!("items", count = 1), "1 item");
    assert_eq!(t!("items", count = 5), "5 items");
}
```

## API Reference

### `t!()` Macro

```rust
t!("key")                                // Simple lookup
t!("key", locale = "zh-CN")              // With locale override
t!("key", name = "value")                // With interpolation
t!("key", count = 5)                     // With plural
t!("key", locale = "zh-CN", count = 5)   // Combined
```

### `I18n` Methods

```rust
let i18n = /* ... */;

i18n.t("welcome");                                    // Default locale
i18n.t_with_locale("welcome", "zh-CN");               // Specific locale
i18n.t_with_args("greeting", "en", &[("name", "A")]); // With interpolation
i18n.t_with_count("items", "en", 5, &[]);             // With plural
i18n.default_locale();                                // "en"
i18n.available_locales();                             // ["en", "zh-CN"]
```

### `I18nBuilder`

```rust
let i18n = I18nBuilder::new()
    .default_locale("en")
    .fallback_locale("en")
    .embedded_translations(&[ /* ... */ ])?
    .add_backend(my_custom_backend)
    .build()?;
```

### Locale Fallback

```rust
use anycms_i18n::Locale;

let locale = Locale::parse("zh-Hans-CN").unwrap();
let chain = locale.fallback_chain("en");
// ["zh-Hans-CN", "zh-CN", "zh", "en"]
```

### Accept-Language Negotiation

```rust
use anycms_i18n::negotiate_locale;

let locale = negotiate_locale("zh-CN,en;q=0.9", &["en", "zh-CN", "ja"], "en");
// "zh-CN"
```

## Web Framework Integration

### Actix-web

```toml
[dependencies]
anycms-i18n-actix = "0.1"
```

```rust
use actix_web::{web, App, HttpServer};
use anycms_i18n::I18nBuilder;
use anycms_i18n_actix::{I18nMiddleware, LocaleExtractor, I18nAppExt};
use std::sync::Arc;

let i18n = Arc::new(
    I18nBuilder::new()
        .embedded_translations(&[("en", "..."), ("zh-CN", "...")])?
        .build()?
);

HttpServer::new(move || {
    App::new()
        .wrap(I18nMiddleware::new(i18n.clone()))
        .i18n_routes(i18n.clone())  // /api/i18n/locales, /api/i18n/{locale}
        .route("/", web::get().to(index))
        .route("/greet/{name}", web::get().to(greet))
})

async fn index(locale: LocaleExtractor) -> String {
    format!("[{}] {}", locale.as_str(), locale.t("welcome"))
}

async fn greet(locale: LocaleExtractor, path: web::Path<String>) -> String {
    locale.t_with_args("greeting", &[("name", &path.into_inner())])
}
```

**Locale detection order:** query param `?lang=` → cookie `locale` → `Accept-Language` header → default.

### Axum

```toml
[dependencies]
anycms-i18n-axum = "0.1"
```

```rust
use axum::{routing::get, Router};
use anycms_i18n::I18nBuilder;
use anycms_i18n_axum::{I18nLayer, Locale, I18nRouterExt};
use std::sync::Arc;

let i18n = Arc::new(
    I18nBuilder::new()
        .embedded_translations(&[("en", "..."), ("zh-CN", "...")])?
        .build()?
);

let app = Router::new()
    .route("/", get(index))
    .route("/greet/{name}", get(greet))
    .i18n_routes(i18n.clone())
    .layer(I18nLayer::new(i18n));

async fn index(locale: Locale) -> String {
    format!("[{}] {}", locale.as_str(), locale.t("welcome"))
}

async fn greet(locale: Locale, axum::extract::Path(name): axum::extract::Path<String>) -> String {
    locale.t_with_args("greeting", &[("name", &name)])
}
```

## Custom Backend

Implement the `Backend` trait for database-driven or remote translations:

```rust
use anycms_i18n::Backend;

struct DatabaseBackend { /* ... */ }

impl Backend for DatabaseBackend {
    fn get(&self, locale: &str, key: &str) -> Option<String> {
        // Query database for translation
        todo!()
    }

    fn available_locales(&self) -> Vec<String> {
        todo!()
    }

    fn has_locale(&self, locale: &str) -> bool {
        todo!()
    }
}
```

Stack with `ChainedBackend` for priority:

```rust
use anycms_i18n::{I18nBuilder, ChainedBackend, TomlBackend};
use std::sync::Arc;

let db_backend: Arc<dyn Backend> = Arc::new(DatabaseBackend::new());
let file_backend: Arc<dyn Backend> = Arc::new(TomlBackend::from_embedded(&[...])?);

// DB overrides take priority, files serve as fallback
let i18n = I18nBuilder::new()
    .add_backend(db_backend)
    .add_backend(file_backend)
    .build()?;
```

## Plural Rules

Built-in support for major language families:

| Language | Categories | Example |
|----------|-----------|---------|
| English | one, other | 1 item / 2 items |
| Chinese | other | 2 个项目 |
| Japanese | other | 2 個のアイテム |
| Russian | one, few, many, other | 1 яблоко / 2 яблока / 5 яблок |
| Arabic | zero, one, two, few, many, other | 0 ملفات / 1 ملف / 2 ملفان |

## TOML Translation File Format

```toml
# Simple key-value
welcome = "Welcome!"

# Nested tables → dot-separated keys
[errors]
not_found = "Not found"          # key: errors.not_found

[errors.auth]
expired = "Session expired"      # key: errors.auth.expired

# Plural forms (table with zero/one/other keys)
[items]
zero = "No items"
one = "%{count} item"
other = "%{count} items"

# Interpolation with %{name}
greeting = "Hello, %{name}!"
```

## Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `toml-backend` | ✅ | TOML file translation backend |
| `init` | ✅ | `i18n!()` proc macro for one-line initialization |
| `task-local` | ✅ | Task-local locale support for async frameworks |
| `fs-loader` | ❌ | Runtime file loading from directory |
| `hot-reload` | ❌ | File watching + hot reload |

## Crate Structure

```
anycms-i18n/                       # Workspace root
├── crates/
│   ├── anycms-i18n/               # Core library
│   │   └── examples/
│   │       ├── basic.rs           # Basic usage + t!() macro
│   │       ├── builder.rs         # I18nBuilder + ChainedBackend
│   │       ├── fallback.rs        # Locale fallback chains
│   │       └── plural.rs          # Plural rules (en/zh/ru/ar)
│   ├── anycms-i18n-actix/         # Actix-web integration
│   │   └── examples/
│   │       └── actix_server.rs    # Complete Actix server
│   └── anycms-i18n-axum/          # Axum integration
│       └── examples/
│           └── axum_server.rs     # Complete Axum server
└── locales/                       # Example translation files
    ├── en.toml
    ├── zh-CN.toml
    └── ja.toml
```

## Running Examples

```bash
# Core examples
cargo run -p anycms-i18n --example basic
cargo run -p anycms-i18n --example builder
cargo run -p anycms-i18n --example fallback
cargo run -p anycms-i18n --example plural

# Actix-web server (http://localhost:8080)
cargo run -p anycms-i18n-actix --example actix_server

# Axum server (http://localhost:8081)
cargo run -p anycms-i18n-axum --example axum_server
```

Test the web servers:

```bash
# Default locale
curl http://localhost:8080/

# Chinese via query param
curl "http://localhost:8080/?lang=zh-CN"

# Chinese via Accept-Language header
curl -H "Accept-Language: zh-CN" http://localhost:8080/greet/Alice

# List available locales
curl http://localhost:8080/api/i18n/locales

# Get all Chinese translations
curl http://localhost:8080/api/i18n/zh-CN
```

## License

MIT
