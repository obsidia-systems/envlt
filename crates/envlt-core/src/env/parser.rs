use std::{collections::BTreeMap, fs, path::Path};

use crate::error::{EnvltError, Result};

pub fn parse_env_file(path: &Path) -> Result<BTreeMap<String, String>> {
    let content = fs::read_to_string(path)?;
    parse_env_str(path, &content)
}

pub fn parse_env_str(path: &Path, content: &str) -> Result<BTreeMap<String, String>> {
    let mut variables = BTreeMap::new();

    for (index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Handle optional `export` prefix
        let line = line.strip_prefix("export ").unwrap_or(line);

        let Some((key, value)) = line.split_once('=') else {
            return Err(EnvltError::EnvParse {
                path: path.to_path_buf(),
                message: format!("line {} is missing '='", index + 1),
            });
        };

        let key = key.trim();
        if key.is_empty() {
            return Err(EnvltError::EnvParse {
                path: path.to_path_buf(),
                message: format!("line {} has an empty key", index + 1),
            });
        }

        let value = parse_value(value)?;
        variables.insert(key.to_owned(), value);
    }

    Ok(variables)
}

fn parse_value(value: &str) -> Result<String> {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return Ok(String::new());
    }

    // Single-quoted: literal, no escape processing
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') && trimmed.len() > 1 {
        return Ok(trimmed[1..trimmed.len() - 1].to_owned());
    }

    // Double-quoted: support escapes
    if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() > 1 {
        return parse_escaped_value(&trimmed[1..trimmed.len() - 1]);
    }

    // Unquoted: trim whitespace
    Ok(trimmed.to_owned())
}

fn parse_escaped_value(value: &str) -> Result<String> {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some('\'') => result.push('\''),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(ch);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn parse(input: &str) -> BTreeMap<String, String> {
        parse_env_str(Path::new("<test>"), input).unwrap()
    }

    #[test]
    fn parses_simple_key_value() {
        let vars = parse("FOO=bar\n");
        assert_eq!(vars.get("FOO"), Some(&"bar".to_owned()));
    }

    #[test]
    fn ignores_empty_lines_and_comments() {
        let vars = parse("\n# comment\nFOO=bar\n\n");
        assert_eq!(vars.get("FOO"), Some(&"bar".to_owned()));
        assert_eq!(vars.len(), 1);
    }

    #[test]
    fn handles_spaces_around_equals() {
        let vars = parse("FOO = bar\n");
        assert_eq!(vars.get("FOO"), Some(&"bar".to_owned()));
    }

    #[test]
    fn handles_empty_value() {
        let vars = parse("FOO=\n");
        assert_eq!(vars.get("FOO"), Some(&"".to_owned()));
    }

    #[test]
    fn handles_single_quotes() {
        let vars = parse("FOO='hello world'\n");
        assert_eq!(vars.get("FOO"), Some(&"hello world".to_owned()));
    }

    #[test]
    fn handles_double_quotes_with_escapes() {
        let vars = parse("FOO=\"hello\\nworld\"\n");
        assert_eq!(vars.get("FOO"), Some(&"hello\nworld".to_owned()));
    }

    #[test]
    fn handles_export_prefix() {
        let vars = parse("export FOO=bar\n");
        assert_eq!(vars.get("FOO"), Some(&"bar".to_owned()));
    }

    #[test]
    fn preserves_unquoted_value_with_internal_spaces() {
        let vars = parse("FOO=hello world\n");
        assert_eq!(vars.get("FOO"), Some(&"hello world".to_owned()));
    }

    #[test]
    fn handles_backslash_escape_in_double_quotes() {
        let vars = parse("FOO=\"path\\\\to\\\\file\"\n");
        assert_eq!(vars.get("FOO"), Some(&"path\\to\\file".to_owned()));
    }

    #[test]
    fn handles_tab_escape() {
        let vars = parse("FOO=\"a\\tb\"\n");
        assert_eq!(vars.get("FOO"), Some(&"a\tb".to_owned()));
    }

    #[test]
    fn handles_carriage_return_escape() {
        let vars = parse("FOO=\"a\\rb\"\n");
        assert_eq!(vars.get("FOO"), Some(&"a\rb".to_owned()));
    }

    #[test]
    fn handles_quoted_single_quote_in_double_quotes() {
        let vars = parse("FOO=\"it's fine\"\n");
        assert_eq!(vars.get("FOO"), Some(&"it's fine".to_owned()));
    }

    #[test]
    fn handles_quoted_double_quote_in_double_quotes() {
        let vars = parse("FOO=\"say \\\"hello\\\"\"\n");
        assert_eq!(vars.get("FOO"), Some(&"say \"hello\"".to_owned()));
    }

    #[test]
    fn handles_multiple_variables() {
        let input = "FOO=bar\nBAZ=qux\nEMPTY=\n";
        let vars = parse(input);
        assert_eq!(vars.get("FOO"), Some(&"bar".to_owned()));
        assert_eq!(vars.get("BAZ"), Some(&"qux".to_owned()));
        assert_eq!(vars.get("EMPTY"), Some(&"".to_owned()));
        assert_eq!(vars.len(), 3);
    }

    #[test]
    fn error_on_missing_equals() {
        let result = parse_env_str(Path::new("<test>"), "FOO");
        assert!(result.is_err());
    }

    #[test]
    fn error_on_empty_key() {
        let result = parse_env_str(Path::new("<test>"), "=bar");
        assert!(result.is_err());
    }
}
