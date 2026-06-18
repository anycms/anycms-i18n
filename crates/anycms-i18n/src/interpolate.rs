//! String interpolation for `%{name}` placeholders.

/// Replace `%{name}` placeholders in a template string with provided values.
///
/// # Examples
/// ```
/// use anycms_i18n::interpolate;
///
/// let result = interpolate("Hello, %{name}!", &[("name", "world".to_string())]);
/// assert_eq!(result, "Hello, world!");
///
/// // Placeholders without a matching argument are left as-is.
/// let result = interpolate("Hi %{x} and %{name}!", &[("name", "world".to_string())]);
/// assert_eq!(result, "Hi %{x} and world!");
/// ```
pub fn interpolate(template: &str, args: &[(&str, String)]) -> String {
    let mut result = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Detect a `%{name}` placeholder starting at the current position.
        // We require the closing `}` to be on the same UTF-8 char boundary as
        // the opening `%{`, which is guaranteed here because `%`, `{` and `}`
        // are all single-byte ASCII characters and `i` is always at a char
        // boundary (we advance whole UTF-8 sequences otherwise).
        if bytes[i] == b'%'
            && i + 1 < bytes.len()
            && bytes[i + 1] == b'{'
            && let Some(close_rel) = template[i + 2..].find('}')
        {
            let close = i + 2 + close_rel;
            let name = &template[i + 2..close];
            if !name.is_empty() {
                match args.iter().find(|(key, _)| *key == name) {
                    Some((_, value)) => {
                        result.push_str(value);
                        i = close + 1;
                        continue;
                    }
                    None => {
                        // No matching argument: keep the placeholder as-is.
                        result.push_str(&template[i..close + 1]);
                        i = close + 1;
                        continue;
                    }
                }
            }
        }

        // Not a recognized placeholder: copy the next UTF-8 character verbatim.
        let ch = template[i..].chars().next().expect("valid utf-8");
        result.push(ch);
        i += ch.len_utf8();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_interpolation() {
        let result = interpolate("Hello, %{name}!", &[("name", "world".into())]);
        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_multiple_interpolations() {
        let result = interpolate(
            "%{greeting}, %{name}! You have %{count} messages.",
            &[
                ("greeting", "Hello".into()),
                ("name", "Alice".into()),
                ("count", "3".into()),
            ],
        );
        assert_eq!(result, "Hello, Alice! You have 3 messages.");
    }

    #[test]
    fn test_missing_argument_left_as_is() {
        let result = interpolate("Hello, %{name}!", &[]);
        assert_eq!(result, "Hello, %{name}!");
    }

    #[test]
    fn test_no_placeholders() {
        let result = interpolate("No placeholders here.", &[("name", "test".into())]);
        assert_eq!(result, "No placeholders here.");
    }

    #[test]
    fn test_repeated_placeholder() {
        let result = interpolate("%{x} and %{x}", &[("x", "1".into())]);
        assert_eq!(result, "1 and 1");
    }

    #[test]
    fn test_no_cascade_substitution() {
        // A value that itself contains a `%{name}` placeholder must not be
        // substituted by a later arg. Single-pass scanning prevents the cascade.
        let result = interpolate("%{a}", &[("a", "%{b}".into()), ("b", "X".into())]);
        assert_eq!(result, "%{b}");
    }
}
