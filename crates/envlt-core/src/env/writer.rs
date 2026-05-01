use std::collections::BTreeMap;

/// Renders variables as a `.env` formatted string.
///
/// Values containing spaces, quotes, backslashes, `#`, newlines, or `=`
/// are wrapped in double quotes with proper escaping.
pub fn render_env(variables: &BTreeMap<String, String>) -> String {
    let mut output = String::new();

    for (key, value) in variables {
        output.push_str(key);
        output.push('=');

        if needs_quoting(value) {
            output.push('"');
            output.push_str(&escape_value(value));
            output.push('"');
        } else {
            output.push_str(value);
        }

        output.push('\n');
    }

    output
}

fn needs_quoting(value: &str) -> bool {
    value.is_empty()
        || value.contains(' ')
        || value.contains('\t')
        || value.contains('"')
        || value.contains('\'')
        || value.contains('#')
        || value.contains('\n')
        || value.contains('\r')
        || value.contains('=')
}

fn escape_value(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            other => result.push(other),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn renders_simple_values() {
        let input = vars(&[("FOO", "bar"), ("BAZ", "qux")]);
        // BTreeMap iterates in key order
        assert_eq!(render_env(&input), "BAZ=qux\nFOO=bar\n");
    }

    #[test]
    fn quotes_values_with_spaces() {
        let input = vars(&[("FOO", "hello world")]);
        assert_eq!(render_env(&input), "FOO=\"hello world\"\n");
    }

    #[test]
    fn quotes_empty_values() {
        let input = vars(&[("FOO", "")]);
        assert_eq!(render_env(&input), "FOO=\"\"\n");
    }

    #[test]
    fn escapes_double_quotes() {
        let input = vars(&[("FOO", "say \"hello\"")]);
        assert_eq!(render_env(&input), "FOO=\"say \\\"hello\\\"\"\n");
    }

    #[test]
    fn escapes_backslashes_when_quoted() {
        // Backslash alone does not require quoting, but combined with a space it does
        let input = vars(&[("FOO", "path\\to\\file name")]);
        assert_eq!(render_env(&input), "FOO=\"path\\\\to\\\\file name\"\n");
    }

    #[test]
    fn escapes_newlines() {
        let input = vars(&[("FOO", "line1\nline2")]);
        assert_eq!(render_env(&input), "FOO=\"line1\\nline2\"\n");
    }

    #[test]
    fn escapes_tabs() {
        let input = vars(&[("FOO", "a\tb")]);
        assert_eq!(render_env(&input), "FOO=\"a\\tb\"\n");
    }

    #[test]
    fn quotes_values_with_equals() {
        let input = vars(&[("FOO", "a=b")]);
        assert_eq!(render_env(&input), "FOO=\"a=b\"\n");
    }

    #[test]
    fn quotes_values_with_hash() {
        let input = vars(&[("FOO", "value#123")]);
        assert_eq!(render_env(&input), "FOO=\"value#123\"\n");
    }

    #[test]
    fn preserves_single_quotes_unescaped() {
        let input = vars(&[("FOO", "it's fine")]);
        assert_eq!(render_env(&input), "FOO=\"it's fine\"\n");
    }

    #[test]
    fn roundtrip_preserved() {
        let original = vars(&[
            ("FOO", "bar"),
            ("WITH_SPACE", "hello world"),
            ("EMPTY", ""),
            ("QUOTE", "say \"hi\""),
            ("PATH", "path\\to\\file"),
            ("MULTI", "line1\nline2"),
        ]);

        let rendered = render_env(&original);
        let parsed =
            crate::env::parse_env_str(std::path::Path::new("<roundtrip>"), &rendered).unwrap();

        assert_eq!(original, parsed);
    }
}
