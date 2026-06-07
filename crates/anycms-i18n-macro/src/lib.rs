//! Procedural macros for anycms-i18n.
//!
//! Provides the `i18n!()` macro for one-line i18n initialization.

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{parse_macro_input, Ident, LitStr, Token};

// ---- Argument parsing ----

/// Parsed arguments for `i18n!("path", default = "en", fallback = "en", hot_reload)`.
struct I18nArgs {
    /// Path to the locales directory (relative to `CARGO_MANIFEST_DIR`).
    path: LitStr,
    /// Default locale. Defaults to `"en"`.
    default_locale: Option<LitStr>,
    /// Fallback locale. Defaults to the default locale.
    fallback_locale: Option<LitStr>,
    /// Enable hot-reload (runtime file watching).
    hot_reload: bool,
}

impl Parse for I18nArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;

        let mut default_locale = None;
        let mut fallback_locale = None;
        let mut hot_reload = false;

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
                            format!("unknown argument `{other}`; expected `default`, `fallback`, or `hot_reload`"),
                        ));
                    }
                }
            } else {
                // Flag-style (no = value)
                match key.to_string().as_str() {
                    "hot_reload" => hot_reload = true,
                    other => {
                        return Err(syn::Error::new(
                            key.span(),
                            format!("unknown flag `{other}`; expected `hot_reload`"),
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
        })
    }
}

// ---- Macro implementation ----

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
        // Default mode: compile-time embedding
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let full_dir = std::path::Path::new(&manifest_dir).join(&dir_path);

        if !full_dir.is_dir() {
            let msg = format!(
                "i18n!: directory `{}` does not exist (resolved: `{}`)",
                dir_path,
                full_dir.display()
            );
            return syn::Error::new(args.path.span(), msg)
                .to_compile_error()
                .into();
        }

        // Scan for .toml files
        let mut entries: Vec<(String, std::path::PathBuf)> = Vec::new();
        match std::fs::read_dir(&full_dir) {
            Ok(rd) => {
                for entry in rd {
                    let entry = match entry {
                        Ok(e) => e,
                        Err(e) => {
                            let msg = format!("i18n!: failed to read directory entry: {e}");
                            return syn::Error::new(args.path.span(), msg)
                                .to_compile_error()
                                .into();
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
                    "i18n!: failed to read directory `{}`: {e}",
                    full_dir.display()
                );
                return syn::Error::new(args.path.span(), msg)
                    .to_compile_error()
                    .into();
            }
        }

        if entries.is_empty() {
            let msg = format!(
                "i18n!: no `.toml` files found in `{}`",
                full_dir.display()
            );
            return syn::Error::new(args.path.span(), msg)
                .to_compile_error()
                .into();
        }

        // Sort by locale name for deterministic output
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        // Generate include_str! calls
        let translation_pairs: Vec<_> = entries
            .iter()
            .map(|(locale, path)| {
                let locale_lit = locale;
                let path_str = path.to_string_lossy().to_string();
                quote! {
                    (#locale_lit, include_str!(#path_str))
                }
            })
            .collect();

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
