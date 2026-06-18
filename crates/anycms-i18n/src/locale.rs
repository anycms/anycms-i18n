//! Locale parsing, fallback chain generation, and Accept-Language negotiation.

use crate::error::I18nError;

/// A parsed locale identifier.
///
/// Supports BCP 47-style tags:
/// - `"en"` — language only
/// - `"zh-CN"` — language + region
/// - `"zh-Hans-CN"` — language + script + region
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Locale {
    /// ISO 639-1 language code (lowercase), e.g. `"zh"`, `"en"`.
    pub language: String,
    /// ISO 15924 script code (title case), e.g. `"Hans"`, `"Latn"`.
    pub script: Option<String>,
    /// ISO 3166-1 region code (uppercase), e.g. `"CN"`, `"TW"`.
    pub region: Option<String>,
}

impl Locale {
    /// Parse a locale string.
    ///
    /// Accepts formats: `"en"`, `"zh-CN"`, `"zh-Hans-CN"`, `"zh_Hans_CN"`.
    /// Separators `-` and `_` are both supported.
    ///
    /// # Examples
    ///
    /// ```
    /// use anycms_i18n::Locale;
    ///
    /// let loc = Locale::parse("zh-Hans-CN")?;
    /// assert_eq!(loc.language, "zh");
    /// assert_eq!(loc.script.as_deref(), Some("Hans"));
    /// assert_eq!(loc.region.as_deref(), Some("CN"));
    /// # Ok::<(), anycms_i18n::I18nError>(())
    /// ```
    pub fn parse(input: &str) -> Result<Self, I18nError> {
        let normalized = input.replace('_', "-");
        let parts: Vec<&str> = normalized.split('-').collect();

        if parts.is_empty() || parts[0].is_empty() {
            return Err(I18nError::InvalidLocale(input.to_string()));
        }

        let language = parts[0].to_ascii_lowercase();

        if language.len() != 2 && language.len() != 3 {
            return Err(I18nError::InvalidLocale(format!(
                "invalid language code: {input}"
            )));
        }

        let mut script = None;
        let mut region = None;

        for part in parts.iter().skip(1) {
            if part.len() == 4 && part.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                // 4-char, starts with uppercase → script (e.g. "Hans")
                script = Some(part.to_string());
            } else if part.len() == 2 && part.chars().all(|c| c.is_ascii_uppercase())
                || part.len() == 3 && part.chars().all(|c| c.is_ascii_digit())
            {
                // 2-char uppercase → region (e.g. "CN"), or 3-digit
                region = Some(part.to_string());
            } else {
                // Unknown pattern → treat as region
                region = Some(part.to_ascii_uppercase());
            }
        }

        Ok(Self {
            language,
            script,
            region,
        })
    }

    /// Create a locale with language only.
    pub fn language_only(lang: &str) -> Self {
        Self {
            language: lang.to_ascii_lowercase(),
            script: None,
            region: None,
        }
    }

    /// Generate a fallback chain from this locale to the given default.
    ///
    /// Order: full tag → drop region keep script → drop script keep region →
    /// language only → default.
    ///
    /// Example: `"zh-Hans-CN"` → `["zh-Hans-CN", "zh-Hans", "zh-CN", "zh", <default>]`
    ///
    /// ```
    /// use anycms_i18n::Locale;
    ///
    /// let loc = Locale::parse("zh-Hans-CN")?;
    /// assert_eq!(
    ///     loc.fallback_chain("en"),
    ///     vec!["zh-Hans-CN", "zh-Hans", "zh-CN", "zh", "en"]
    /// );
    /// # Ok::<(), anycms_i18n::I18nError>(())
    /// ```
    pub fn fallback_chain(&self, default: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Full tag: zh-Hans-CN
        let full = self.to_string();
        if seen.insert(full.clone()) {
            chain.push(full);
        }

        // Drop region, keep script: zh-Hans
        if let Some(script) = self.script.as_deref()
            && self.region.is_some()
        {
            let tag = format!("{}-{}", self.language, script);
            if seen.insert(tag.clone()) {
                chain.push(tag);
            }
        }

        // Drop script, keep region: zh-CN
        if let Some(region) = self.region.as_deref()
            && self.script.is_some()
        {
            let tag = format!("{}-{}", self.language, region);
            if seen.insert(tag.clone()) {
                chain.push(tag);
            }
        }

        // Language only: zh
        if seen.insert(self.language.clone()) {
            chain.push(self.language.clone());
        }

        // Default fallback
        if seen.insert(default.to_string()) {
            chain.push(default.to_string());
        }

        chain
    }

    /// Format as a BCP 47 string.
    pub fn to_tag(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for Locale {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.language)?;
        if let Some(ref s) = self.script {
            write!(f, "-{s}")?;
        }
        if let Some(ref r) = self.region {
            write!(f, "-{r}")?;
        }
        Ok(())
    }
}

/// Negotiate the best available locale from an `Accept-Language` header value.
///
/// Tries each language in the header (in quality order) against the
/// `available` list, returning the first match. Falls back to `default`.
///
/// ```
/// use anycms_i18n::negotiate_locale;
///
/// let result = negotiate_locale("zh-CN,en;q=0.9", &["en", "zh-CN", "ja"], "en");
/// assert_eq!(result, "zh-CN");
///
/// // No match → falls back to default.
/// let result = negotiate_locale("fr,en;q=0.9", &["en", "zh-CN", "ja"], "en");
/// assert_eq!(result, "en");
/// ```
pub fn negotiate_locale(accept_language: &str, available: &[&str], default: &str) -> String {
    let parsed = parse_accept_language(accept_language);

    for (tag, _quality) in parsed {
        // Exact match
        if available.contains(&tag.as_str()) {
            return tag;
        }

        // Try matching language only
        if let Ok(locale) = Locale::parse(&tag) {
            let lang = &locale.language;
            if let Some(matched) = available.iter().find(|a| {
                a.to_ascii_lowercase().starts_with(&format!("{lang}-"))
                    || a.eq_ignore_ascii_case(lang)
            }) {
                return matched.to_string();
            }
        }
    }

    default.to_string()
}

/// Parse an `Accept-Language` header into `(tag, quality)` pairs, sorted by quality desc.
fn parse_accept_language(header: &str) -> Vec<(String, f32)> {
    let mut entries: Vec<(String, f32)> = header
        .split(',')
        .filter_map(|part| {
            let part = part.trim();
            if part.is_empty() {
                return None;
            }

            let (tag, quality) = if let Some((t, q)) = part.split_once(';') {
                let q = q.trim();
                let quality = if let Some(stripped) = q.strip_prefix("q=") {
                    stripped.parse::<f32>().unwrap_or(1.0)
                } else {
                    1.0
                };
                (t.trim().to_string(), quality)
            } else {
                (part.to_string(), 1.0)
            };

            Some((tag, quality))
        })
        .collect();

    entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_en() {
        let loc = Locale::parse("en").unwrap();
        assert_eq!(loc.language, "en");
        assert_eq!(loc.script, None);
        assert_eq!(loc.region, None);
    }

    #[test]
    fn test_parse_zh_cn() {
        let loc = Locale::parse("zh-CN").unwrap();
        assert_eq!(loc.language, "zh");
        assert_eq!(loc.script, None);
        assert_eq!(loc.region, Some("CN".into()));
    }

    #[test]
    fn test_parse_zh_hans_cn() {
        let loc = Locale::parse("zh-Hans-CN").unwrap();
        assert_eq!(loc.language, "zh");
        assert_eq!(loc.script, Some("Hans".into()));
        assert_eq!(loc.region, Some("CN".into()));
        assert_eq!(loc.to_string(), "zh-Hans-CN");
    }

    #[test]
    fn test_parse_underscore_separator() {
        let loc = Locale::parse("zh_Hans_TW").unwrap();
        assert_eq!(loc.language, "zh");
        assert_eq!(loc.script, Some("Hans".into()));
        assert_eq!(loc.region, Some("TW".into()));
    }

    #[test]
    fn test_fallback_chain() {
        let loc = Locale::parse("zh-Hans-CN").unwrap();
        let chain = loc.fallback_chain("en");
        assert_eq!(chain, vec!["zh-Hans-CN", "zh-Hans", "zh-CN", "zh", "en"]);
    }

    #[test]
    fn test_fallback_chain_language_only() {
        let loc = Locale::parse("en").unwrap();
        let chain = loc.fallback_chain("en");
        assert_eq!(chain, vec!["en"]);
    }

    #[test]
    fn test_fallback_chain_region_only() {
        let loc = Locale::parse("zh-CN").unwrap();
        let chain = loc.fallback_chain("en");
        assert_eq!(chain, vec!["zh-CN", "zh", "en"]);
    }

    #[test]
    fn test_negotiate_exact() {
        let result = negotiate_locale("zh-CN,en;q=0.9", &["en", "zh-CN", "ja"], "en");
        assert_eq!(result, "zh-CN");
    }

    #[test]
    fn test_negotiate_fallback() {
        let result = negotiate_locale("fr,en;q=0.9", &["en", "zh-CN", "ja"], "en");
        assert_eq!(result, "en");
    }

    #[test]
    fn test_negotiate_language_match() {
        let result = negotiate_locale("zh-TW", &["zh-CN", "en"], "en");
        // "zh-TW" doesn't match exactly, but "zh" prefix matches "zh-CN"
        assert_eq!(result, "zh-CN");
    }

    #[test]
    fn test_negotiate_empty_header() {
        let result = negotiate_locale("", &["en", "zh-CN"], "en");
        assert_eq!(result, "en");
    }
}
