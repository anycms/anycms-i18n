## [0.1.0] - 2025-06-07

### 🚀 Features

- **Core**: `I18n` struct with `TomlBackend`, `ChainedBackend`, fallback chain, plural rules (CLDR)
- **Builder**: `I18nBuilder` for programmatic setup with `embedded_translations()`, `add_backend()`
- **Macro `i18n!`**: One-line compile-time init — `i18n!("locales", default = "zh-CN", fallback = "en")`
- **Macro `t!`**: Runtime translation with locale override, interpolation, plural count
- **Plural**: CLDR plural categories for en/zh/ja/ru/ar and more
- **Locale**: BCP 47 parsing, fallback chain (`zh-Hans-CN` → `zh-CN` → `zh` → fallback), Accept-Language negotiation
- **anycms-i18n-axum**: Axum `Locale` extractor with Accept-Language negotiation
- **anycms-i18n-actix**: Actix-web `LocaleExtractor` with Accept-Language negotiation
- **Locales**: Built-in en, zh-CN, ja translation files
- **Examples**: basic, builder, fallback, plural

### 🗑️ Removed

- **anycms-config integration**: Removed `config` feature, `I18nConfig`, `with_config` example — over-engineered for an i18n library
