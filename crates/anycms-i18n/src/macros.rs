//! The `t!()` translation macro.

/// Translate a key using the global i18n instance.
///
/// # Usage
///
/// ```rust,ignore
/// use anycms_i18n::t;
///
/// // Simple key lookup
/// t!("welcome");
///
/// // With explicit locale
/// t!("welcome", locale = "zh-CN");
///
/// // With interpolation
/// t!("greeting", name = "world");
///
/// // With plural count
/// t!("items", count = 5);
///
/// // Combined: locale + interpolation + count
/// t!("items", locale = "zh-CN", count = 5, name = "test");
/// ```
///
/// # Requirements
///
/// An [`crate::I18n`] instance must be set via [`crate::set_global`]
/// before using this macro. If no global instance is set, the macro
/// returns the key string as-is.
#[macro_export]
macro_rules! t {
    // t!("key")
    ($key:expr) => {{
        $crate::__t_inner!($key, None, &[], None)
    }};

    // t!("key", locale = "...")
    ($key:expr, locale = $locale:expr) => {{
        $crate::__t_inner!($key, Some($locale), &[], None)
    }};

    // t!("key", count = N)
    ($key:expr, count = $count:expr) => {{
        $crate::__t_inner!($key, None, &[], Some($count))
    }};

    // t!("key", locale = "...", count = N)
    ($key:expr, locale = $locale:expr, count = $count:expr) => {{
        $crate::__t_inner!($key, Some($locale), &[], Some($count))
    }};

    // t!("key", key = value, ...) — named args for interpolation
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {{
        let args: Vec<(&str, String)> = vec![
            $((stringify!($name), ($value).to_string())),+
        ];

        let mut locale_opt: Option<&str> = None;
        let mut count_opt: Option<i64> = None;
        let mut filtered: Vec<(&str, &str)> = Vec::new();
        for (k, v) in &args {
            match *k {
                "locale" => locale_opt = Some(v.as_str()),
                "count" => count_opt = Some(v.parse::<i64>().unwrap_or(0)),
                _ => filtered.push((*k, v.as_str())),
            }
        }

        $crate::__t_inner!(
            $key,
            locale_opt,
            &filtered,
            count_opt
        )
    }};
}

/// Internal helper used by `t!`. Not part of the public API.
///
/// Locale resolution order:
/// 1. Explicit `locale = "..."` argument (highest priority)
/// 2. Task-local locale (set by web framework middleware)
/// 3. Global default locale (set at init time)
#[macro_export]
macro_rules! __t_inner {
    ($key:expr, $locale:expr, $args:expr, $count:expr) => {{
        let i18n = $crate::global();
        match i18n {
            Some(i18n) => {
                let locale = $locale
                    .map(|l: &str| l.to_string())
                    .or_else(|| $crate::task_locale())
                    .unwrap_or_else(|| i18n.default_locale().to_string());
                i18n.translate($key, &locale, $args, $count)
            }
            None => $key.to_string(),
        }
    }};
}
