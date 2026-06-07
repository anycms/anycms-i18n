//! String interpolation for `%{name}` placeholders.

/// Replace `%{name}` placeholders in a template string with provided values.
///
/// # Examples
/// ```rust,ignore
/// let result = interpolate("Hello, %{name}!", &[("name", "world")]);
/// assert_eq!(result, "Hello, world!");
/// ```
///
/// Placeholders without a matching argument are left as-is.
pub fn interpolate(template: &str, args: &[(&str, String)]) -> String {
    let mut result = template.to_string();

    for (key, value) in args {
        let placeholder = format!("%{{{key}}}");
        result = result.replace(&placeholder, value);
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
}
