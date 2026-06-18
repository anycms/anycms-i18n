//! Plural category selection based on locale and count.

/// CLDR plural categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PluralCategory {
    Zero,
    One,
    Two,
    Few,
    Many,
    Other,
}

impl PluralCategory {
    /// Get the TOML key suffix for this plural category.
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::Zero => "zero",
            Self::One => "one",
            Self::Two => "two",
            Self::Few => "few",
            Self::Many => "many",
            Self::Other => "other",
        }
    }
}

/// Determine the plural category for a given locale and count.
///
/// Supports common language families. Languages not explicitly
/// handled default to the Germanic/English rule (one/other).
///
/// # Examples
///
/// ```
/// use anycms_i18n::{plural_category, PluralCategory};
///
/// // English: one vs. other
/// assert_eq!(plural_category("en", 1), PluralCategory::One);
/// assert_eq!(plural_category("en", 5), PluralCategory::Other);
///
/// // Chinese/Japanese/Korean: no plural distinction
/// assert_eq!(plural_category("zh-CN", 5), PluralCategory::Other);
///
/// // Arabic has all six CLDR categories
/// assert_eq!(plural_category("ar", 0), PluralCategory::Zero);
/// assert_eq!(plural_category("ar", 2), PluralCategory::Two);
/// ```
pub fn plural_category(locale: &str, count: i64) -> PluralCategory {
    let lang = locale
        .split('-')
        .next()
        .unwrap_or(locale)
        .to_ascii_lowercase();

    match lang.as_str() {
        // Chinese, Japanese, Korean, Vietnamese, Thai — no plural distinction
        "zh" | "ja" | "ko" | "vi" | "th" | "id" | "lo" => PluralCategory::Other,

        // Arabic — complex plural rules (must come before the generic match)
        "ar" => arabic_plural(count),

        // Polish — distinct CLDR plural rules, differs from Russian
        "pl" => polish_plural(count),

        // Slavic languages (Russian, Ukrainian, Czech, etc.) — complex plural rules
        "ru" | "uk" | "cs" | "sk" | "hr" | "sr" | "bg" | "sl" => slavic_plural(count),

        // Hebrew — distinct CLDR plural rules (integer-only simplified version)
        "he" => hebrew_plural(count),

        // English, German, Spanish, Italian, Portuguese, etc. — one/other
        "en" | "de" | "es" | "it" | "pt" | "nl" | "sv" | "da" | "no" | "nb" | "nn" | "fi"
        | "el" | "hi" | "tr" => {
            if count == 1 {
                PluralCategory::One
            } else {
                PluralCategory::Other
            }
        }

        // Default: English/Germanic rule
        _ => {
            if count == 1 {
                PluralCategory::One
            } else {
                PluralCategory::Other
            }
        }
    }
}

/// Slavic plural rules (Russian, Polish, Czech, etc.)
///
/// Rules based on the last two digits of the count:
/// - one: ends in 1, but not 11
/// - few: ends in 2-4, but not 12-14
/// - many: ends in 0, 5-20, or 12-14
/// - other: everything else
fn slavic_plural(count: i64) -> PluralCategory {
    let abs = count.unsigned_abs();
    let mod10 = abs % 10;
    let mod100 = abs % 100;

    if mod10 == 1 && mod100 != 11 {
        PluralCategory::One
    } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
        PluralCategory::Few
    } else if mod10 == 0 || (5..=20).contains(&mod100) || (5..=9).contains(&mod10) {
        PluralCategory::Many
    } else {
        PluralCategory::Other
    }
}

/// Polish plural rules (simplified CLDR, integer n).
///
/// - one: n == 1
/// - few: n%10 in 2..=4 and n%100 not in 12..=14
/// - many: n != 1 and (n%10 == 0 or n%10 == 1 or n%10 in 5..=9 or n%100 in 11..=14)
/// - other: everything else
fn polish_plural(count: i64) -> PluralCategory {
    let abs = count.unsigned_abs();
    let mod10 = abs % 10;
    let mod100 = abs % 100;

    if abs == 1 {
        PluralCategory::One
    } else if (2..=4).contains(&mod10) && !(12..=14).contains(&mod100) {
        PluralCategory::Few
    } else if mod10 == 0 || mod10 == 1 || (5..=9).contains(&mod10) || (11..=14).contains(&mod100) {
        PluralCategory::Many
    } else {
        PluralCategory::Other
    }
}

/// Hebrew plural rules (simplified CLDR, integer-only version).
///
/// Note: this omits the fractional-only rules from full CLDR; it handles
/// integer counts only.
/// - one: n == 1
/// - two: n == 2
/// - many: n in 10..=19 or (n != 0 and n%10 == 0)
/// - other: everything else
fn hebrew_plural(count: i64) -> PluralCategory {
    let abs = count.unsigned_abs();
    let mod10 = abs % 10;

    if abs == 1 {
        PluralCategory::One
    } else if abs == 2 {
        PluralCategory::Two
    } else if (10..=19).contains(&abs) || (abs != 0 && mod10 == 0) {
        PluralCategory::Many
    } else {
        PluralCategory::Other
    }
}

/// Arabic plural rules.
///
/// - zero: 0
/// - one: 1
/// - two: 2
/// - few: 3-10
/// - many: 11-99
/// - other: everything else (100+)
fn arabic_plural(count: i64) -> PluralCategory {
    let abs = count.unsigned_abs();

    match abs {
        0 => PluralCategory::Zero,
        1 => PluralCategory::One,
        2 => PluralCategory::Two,
        3..=10 => PluralCategory::Few,
        11..=99 => PluralCategory::Many,
        _ => PluralCategory::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_english() {
        assert_eq!(plural_category("en", 0), PluralCategory::Other);
        assert_eq!(plural_category("en", 1), PluralCategory::One);
        assert_eq!(plural_category("en", 2), PluralCategory::Other);
        assert_eq!(plural_category("en", 5), PluralCategory::Other);
    }

    #[test]
    fn test_chinese() {
        assert_eq!(plural_category("zh-CN", 0), PluralCategory::Other);
        assert_eq!(plural_category("zh-CN", 1), PluralCategory::Other);
        assert_eq!(plural_category("zh-CN", 5), PluralCategory::Other);
    }

    #[test]
    fn test_japanese() {
        assert_eq!(plural_category("ja", 1), PluralCategory::Other);
    }

    #[test]
    fn test_russian() {
        assert_eq!(plural_category("ru", 1), PluralCategory::One);
        assert_eq!(plural_category("ru", 21), PluralCategory::One);
        assert_eq!(plural_category("ru", 11), PluralCategory::Many);
        assert_eq!(plural_category("ru", 2), PluralCategory::Few);
        assert_eq!(plural_category("ru", 5), PluralCategory::Many);
        assert_eq!(plural_category("ru", 0), PluralCategory::Many);
    }

    #[test]
    fn test_arabic() {
        assert_eq!(plural_category("ar", 0), PluralCategory::Zero);
        assert_eq!(plural_category("ar", 1), PluralCategory::One);
        assert_eq!(plural_category("ar", 2), PluralCategory::Two);
        assert_eq!(plural_category("ar", 5), PluralCategory::Few);
        assert_eq!(plural_category("ar", 50), PluralCategory::Many);
        assert_eq!(plural_category("ar", 100), PluralCategory::Other);
    }

    #[test]
    fn test_locale_with_region() {
        assert_eq!(plural_category("en-US", 1), PluralCategory::One);
        assert_eq!(plural_category("pt-BR", 1), PluralCategory::One);
    }

    #[test]
    fn test_polish() {
        assert_eq!(plural_category("pl", 1), PluralCategory::One);
        assert_eq!(plural_category("pl", 2), PluralCategory::Few);
        assert_eq!(plural_category("pl", 5), PluralCategory::Many);
        assert_eq!(plural_category("pl", 22), PluralCategory::Few);
        assert_eq!(plural_category("pl", 100), PluralCategory::Many);
    }

    #[test]
    fn test_hebrew() {
        assert_eq!(plural_category("he", 1), PluralCategory::One);
        assert_eq!(plural_category("he", 2), PluralCategory::Two);
        assert_eq!(plural_category("he", 15), PluralCategory::Many);
        assert_eq!(plural_category("he", 20), PluralCategory::Many);
        assert_eq!(plural_category("he", 3), PluralCategory::Other);
    }
}
