//! Procedural macros for anycms-i18n.
//!
//! Provides:
//! - `i18n!()` — one-line i18n initialization
//! - `embed_locales!()` — compile-time embedding helper for use with `I18nBuilder`

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, LitStr, Token};

// ---- Shared directory scanning ----

/// Scan a directory for `.toml` files and return `(locale, absolute_path)` pairs,
/// sorted by locale name for deterministic output.
///
/// Returns a `syn::Error` if the directory does not exist, cannot be read,
/// or contains no `.toml` files.
fn scan_toml_dir(dir_path: &str, span: proc_macro2::Span) -> Result<Vec<(String, std::path::PathBuf)>, syn::Error> {
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let full_dir = std::path::Path::new(&manifest_dir).join(dir_path);

    if !full_dir.is_dir() {
        let msg = format!(
            "directory `{}` does not exist (resolved: `{}`)",
            dir_path,
            full_dir.display()
        );
        return Err(syn::Error::new(span, msg));
    }

    let mut entries: Vec<(String, std::path::PathBuf)> = Vec::new();
    match std::fs::read_dir(&full_dir) {
        Ok(rd) => {
            for entry in rd {
                let entry = match entry {
                    Ok(e) => e,
                    Err(e) => {
                        let msg = format!("failed to read directory entry: {e}");
                        return Err(syn::Error::new(span, msg));
                    }
                };
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("toml") {
                    let locale = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    entries.push((locale, path));
                }
            }
        }
        Err(e) => {
            let msg = format!(
                "failed to read directory `{}`: {e}",
                full_dir.display()
            );
            return Err(syn::Error::new(span, msg));
        }
    }

    if entries.is_empty() {
        let msg = format!("no `.toml` files found in `{}`", full_dir.display());
        return Err(syn::Error::new(span, msg));
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

/// Generate the `include_str!` token pairs from scanned entries.
fn generate_translation_pairs(entries: &[(String, std::path::PathBuf)]) -> Vec<proc_macro2::TokenStream> {
    entries
        .iter()
        .map(|(locale, path)| {
            let locale_lit = locale;
            let path_str = path.to_string_lossy().to_string();
            quote! {
                (#locale_lit, include_str!(#path_str))
            }
        })
        .collect()
}

// ---- embed_locales! macro ----

/// Compile-time embedding helper.
///
/// Scans the given directory for `.toml` files at compile time and expands to
/// a `&[(&str, &str)]` array suitable for passing to
/// [`TomlBackend::from_embedded`] or [`I18nBuilder::embedded_translations`].
///
/// # Usage
///
/// ```rust,ignore
/// use anycms_i18n::I18nBuilder;
///
/// let i18n = I18nBuilder::new()
///     .default_locale("en")
///     .fallback_locale("en")
///     .embedded_translations(embed_locales!("locales"))?
///     .build()?;
/// ```
///
/// # Panics
///
/// Compile-time error if the directory does not exist or contains no `.toml` files.
#[proc_macro]
pub fn embed_locales(input: TokenStream) -> TokenStream {
    let dir_path = parse_macro_input!(input as LitStr);
    let path_str = dir_path.value();
    let span = dir_path.span();

    let entries = match scan_toml_dir(&path_str, span) {
        Ok(e) => e,
        Err(e) => return e.to_compile_error().into(),
    };

    let pairs = generate_translation_pairs(&entries);

    let expanded = quote! {
        &[#(#pairs),*]
    };
    expanded.into()
}

// ---- i18n! argument parsing ----

/// Parsed arguments for `i18n!("path", default = "en", fallback = "en", hot_reload, allow_override)`.
struct I18nArgs {
    /// Path to the locales directory (relative to `CARGO_MANIFEST_DIR`).
    path: LitStr,
    /// Default locale. Defaults to `"en"`.
    default_locale: Option<LitStr>,
    /// Fallback locale. Defaults to the default locale.
    fallback_locale: Option<LitStr>,
    /// Enable hot-reload (runtime file watching).
    hot_reload: bool,
    /// Enable runtime override: compile-embed translations, then optionally
    /// load from the same directory at runtime (non-fatal if missing).
    /// Runtime keys override compiled keys via `ChainedBackend`.
    allow_override: bool,
}

impl Parse for I18nArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;

        let mut default_locale = None;
        let mut fallback_locale = None;
        let mut hot_reload = false;
        let mut allow_override = false;

        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let key: Ident = input.parse()?;

            if input.peek(Token![=]) {
                // key = "value" style
                input.parse::<Token![=]>()?;
                let value: LitStr = input.parse()?;

                match key.to_string().as_str() {
                    "default" => default_locale = Some(value),
                    "fallback" => fallback_locale = Some(value),
                    other => {
                        return Err(syn::Error::new(
                            key.span(),
                            format!("unknown argument `{other}`; expected `default` or `fallback`"),
                        ));
                    }
                }
            } else {
                // Flag-style (no = value)
                match key.to_string().as_str() {
                    "hot_reload" => hot_reload = true,
                    "allow_override" => allow_override = true,
                    other => {
                        return Err(syn::Error::new(
                            key.span(),
                            format!("unknown flag `{other}`; expected `hot_reload` or `allow_override`"),
                        ));
                    }
                }
            }
        }

        Ok(I18nArgs {
            path,
            default_locale,
            fallback_locale,
            hot_reload,
            allow_override,
        })
    }
}

// ---- i18n! macro ----

/// Initialize i18n with compile-time embedded translations.
///
/// Scans the given directory for `.toml` files, embeds them into the binary,
/// creates a global [`anycms_i18n::I18n`] instance, and registers it via
/// [`anycms_i18n::set_global`].
///
/// # Usage
///
/// ```rust,ignore
/// // Minimal — auto-discovers all .toml files in "locales/"
/// i18n!("locales");
///
/// // With options
/// i18n!("locales", default = "zh-CN", fallback = "en");
///
/// // With hot-reload (loads from filesystem at runtime, watches for changes)
/// // Requires features: hot-reload
/// let (i18n, _reloader) = i18n!("locales", default = "zh-CN", fallback = "en", hot_reload);
///
/// // With runtime override (compile-embed + optional local file override)
/// // Runtime files in the same directory override compiled translations.
/// // If the directory doesn't exist at runtime, compiled defaults are used.
/// // Requires features: fs-loader
/// i18n!("locales", allow_override);
/// ```
///
/// # File naming
///
/// Each `.toml` file is treated as one locale. The filename (without extension)
/// becomes the locale identifier:
///
/// ```text
/// locales/
/// ├── en.toml       → locale "en"
/// ├── zh-CN.toml    → locale "zh-CN"
/// └── ja.toml       → locale "ja"
/// ```
///
/// # Hot-reload
///
/// When `hot_reload` is set, translations are loaded from the filesystem at
/// runtime (not embedded) and a [`anycms_i18n::HotReloader`] is returned.
/// The reloader must be kept alive (not dropped) for watching to continue.
///
/// # Runtime override (`allow_override`)
///
/// When `allow_override` is set, translations are compiled into the binary as
/// defaults, then optionally loaded from the same directory at runtime. Runtime
/// files take priority (via [`anycms_i18n::ChainedBackend`]), so you can
/// override individual keys without rebuilding. If the runtime directory is
/// absent, the compiled defaults are used silently.
///
/// # Panics
///
/// Compile-time panic if the directory does not exist or contains invalid TOML.
#[proc_macro]
pub fn i18n(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as I18nArgs);

    let dir_path = args.path.value();
    let default_locale = args
        .default_locale
        .map(|l| l.value())
        .unwrap_or_else(|| "en".to_string());
    let fallback_locale = args
        .fallback_locale
        .map(|l| l.value())
        .unwrap_or_else(|| default_locale.clone());

    let default_lit = &default_locale;
    let fallback_lit = &fallback_locale;

    if args.hot_reload {
        // Hot-reload mode: runtime loading + file watcher
        let expanded = quote! {
            {
                let __backend = std::sync::Arc::new(
                    ::anycms_i18n::TomlBackend::from_dir(#dir_path)
                        .expect("i18n!: failed to load translations from directory")
                );
                let __reloader = ::anycms_i18n::HotReloader::watch(#dir_path, __backend.clone())
                    .expect("i18n!: failed to start hot-reload watcher");
                let __i18n = ::anycms_i18n::I18n::new(
                    __backend.clone(),
                    #default_lit,
                    #fallback_lit,
                );
                ::anycms_i18n::set_global(__i18n.clone()).expect("i18n!: global instance already set");
                ::anycms_i18n::set_global_reloader(__reloader).expect("i18n!: global reloader already set");
                __i18n
            }
        };
        expanded.into()
    } else {
        // Compile-time embedding (with or without runtime override)
        let entries = match scan_toml_dir(&dir_path, args.path.span()) {
            Ok(e) => e,
            Err(e) => return e.to_compile_error().into(),
        };

        let translation_pairs = generate_translation_pairs(&entries);

        if args.allow_override {
            // allow_override mode: compile-embed + optional runtime file override.
            // Runtime directory is loaded first (highest priority); if it
            // doesn't exist the backend is empty and all lookups fall through
            // to the compiled defaults.
            let expanded = quote! {
                {
                    let __runtime = ::anycms_i18n::TomlBackend::try_from_dir(#dir_path)
                        .expect("i18n!: failed to load runtime translations");
                    let __compiled = ::anycms_i18n::TomlBackend::from_embedded(&[
                        #(#translation_pairs),*
                    ]).expect("i18n!: failed to parse compiled translation files");
                    let __i18n = ::anycms_i18n::I18nBuilder::new()
                        .default_locale(#default_lit)
                        .fallback_locale(#fallback_lit)
                        .add_backend(std::sync::Arc::new(__runtime))
                        .add_backend(std::sync::Arc::new(__compiled))
                        .build()
                        .expect("i18n!: failed to build I18n");
                    ::anycms_i18n::set_global(__i18n.clone()).expect("i18n!: global instance already set");
                    __i18n
                }
            };
            expanded.into()
        } else {
            // Default mode: compile-time embedding only
            let expanded = quote! {
                {
                    let __i18n_backend = ::anycms_i18n::TomlBackend::from_embedded(&[
                        #(#translation_pairs),*
                    ]).expect("i18n!: failed to parse translation files");
                    let __i18n = ::anycms_i18n::I18n::new(
                        std::sync::Arc::new(__i18n_backend),
                        #default_lit,
                        #fallback_lit,
                    );
                    ::anycms_i18n::set_global(__i18n.clone()).expect("i18n!: global instance already set");
                    __i18n
                }
            };
            expanded.into()
        }
    }
}
