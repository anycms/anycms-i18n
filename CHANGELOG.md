# Changelog

All notable changes to the anycms-i18n workspace (core, macro, axum, actix) are documented here.

## [0.2.2] - 2026-06-07

### 🚀 Features

- Implement hot-reload support and upgrade i18n! macro
- Add JSON/YAML backends, SQLx database backend, and generic hot-reload

### 🐛 Bug Fixes

- Add registry = r404 to internal crate dependencies
- Add -l flag to git-cliff --prepend in release hook
- Resolve git-cliff CWD issue in pre-release-hook

### 📚 Documentation

- Update CHANGELOG.md with 0.1.0 release notes
- Update README, remove config feature references

### ⚙️ Miscellaneous Tasks

- Track per-crate CHANGELOGs
- Consolidate sub-crate CHANGELOGs into root CHANGELOG.md
- Bump workspace version to 0.2.0
## [0.1.1] - 2026-06-07

### 🚀 Features

- **anycms-i18n**: Add `TomlBackend` with hot-reload support (`watch` feature), `ChainedBackend` for composable translation sources
- **anycms-i18n-macro**: `i18n!` and `t!` proc-macros for compile-time init and runtime translation
- **anycms-i18n-axum**: Axum `Locale` extractor with Accept-Language negotiation
- **anycms-i18n-actix**: Actix-web `LocaleExtractor` with Accept-Language negotiation

### 🚜 Refactor

- Remove `config` feature and `I18nConfig`; simplify to pure builder API

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
