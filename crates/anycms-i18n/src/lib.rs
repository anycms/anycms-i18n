//! # anycms-i18n
//!
//! Internationalization support for the anycms-rs ecosystem.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use anycms_i18n::{I18nBuilder, t};
//!
//! let i18n = I18nBuilder::new()
//!     .default_locale("en")
//!     .fallback_locale("en")
//!     .embedded_translations(&[
//!         ("en", include_str!("../../locales/en.toml")),
//!         ("zh-CN", include_str!("../../locales/zh-CN.toml")),
//!     ])
//!     .build()
//!     .unwrap();
//!
//! // Simple translation
//! let msg = t!("welcome");
//!
//! // With locale override
//! let msg = t!("welcome", locale = "zh-CN");
//!
//! // With interpolation
//! let msg = t!("greeting", name = "world");
//!
//! // With plural
//! let msg = t!("items", count = 5);
//! ```

mod backend;
mod builder;
mod core;
mod error;
mod interpolate;
mod locale;
mod macros;
mod plural;

// ---- public API ----

pub use backend::{ChainedBackend, TomlBackend};
pub use builder::I18nBuilder;
pub use core::{Backend, I18n};
pub use error::I18nError;
pub use interpolate::interpolate;
pub use locale::{negotiate_locale, Locale};
pub use plural::{plural_category, PluralCategory};

// Note: `t!` and `__t_inner!` are #[macro_export] and automatically at crate root.
// No explicit re-export needed.

// i18n!() init macro (feature-gated)
#[cfg(feature = "init")]
pub use anycms_i18n_macro::i18n;

// ---- Global I18n instance ----

use std::sync::OnceLock;

static GLOBAL_I18N: OnceLock<I18n> = OnceLock::new();

/// Set the global [`I18n`] instance used by the `t!` macro.
///
/// Can only be called once; returns `Err` if already set.
pub fn set_global(i18n: I18n) -> Result<(), I18n> {
    GLOBAL_I18N.set(i18n)
}

/// Get a reference to the global [`I18n`] instance.
///
/// Returns `None` if [`set_global`] has not been called.
pub fn global() -> Option<&'static I18n> {
    GLOBAL_I18N.get()
}

// ---- Task-local locale (for async web frameworks) ----

#[cfg(feature = "task-local")]
tokio::task_local! {
    /// Task-local locale, set by web framework middleware.
    ///
    /// The `t!()` macro checks this before falling back to the global default.
    /// Use [`set_task_locale`] to wrap a future with a locale scope.
    pub static CURRENT_LOCALE: String;
}

/// Get the current task-local locale, if set.
///
/// Returns `None` if not inside a [`CURRENT_LOCALE`] scope
/// (i.e., not in a web request context).
#[cfg(feature = "task-local")]
pub fn task_locale() -> Option<String> {
    CURRENT_LOCALE.try_with(|l| l.clone()).ok()
}

#[cfg(not(feature = "task-local"))]
pub fn task_locale() -> Option<String> {
    None
}
