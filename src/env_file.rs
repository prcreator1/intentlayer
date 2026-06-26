//! Minimal env-file parser for local provider keys.
//!
//! Supports `KEY=VALUE` lines. Blank lines and `#` comments are ignored.
//! Values are never printed in errors or debug output.
//!
//! No dependencies — manual implementation to avoid new crate bloat.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parse a `KEY=VALUE` env file into a map.
///
/// - Blank lines ignored
/// - Lines starting with `#` ignored
/// - Keys trimmed, values preserved after `=`
/// - Duplicate keys: last wins
pub fn parse_env_file(path: &Path) -> Result<HashMap<String, String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Cannot read env file '{}': {}", path.display(), e))?;

    let mut map = HashMap::new();

    for (line_num, raw_line) in content.lines().enumerate() {
        let ln = line_num + 1;
        let trimmed = raw_line.trim();

        // Skip blank lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Split on first `=`
        let eq_pos = match trimmed.find('=') {
            Some(pos) => pos,
            None => {
                return Err(format!(
                    "Malformed line {} in env file '{}': missing '='",
                    ln,
                    path.display()
                ));
            }
        };

        let key = trimmed[..eq_pos].trim().to_string();
        if key.is_empty() {
            return Err(format!(
                "Malformed line {} in env file '{}': empty key before '='",
                ln,
                path.display()
            ));
        }

        let value = trimmed[eq_pos + 1..].trim().to_string();

        map.insert(key, value);
    }

    Ok(map)
}

/// Load env-file values into the current process environment for any
/// keys not already present. Precedence: existing process env wins.
pub fn load_env_file_fill_missing(path: &Path) -> Result<usize, String> {
    let vars = parse_env_file(path)?;
    let mut loaded = 0;
    for (key, value) in &vars {
        if std::env::var(key).is_err() {
            std::env::set_var(key, value);
            loaded += 1;
        }
    }
    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp(content: &str, name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir();
        let path = dir.join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_parse_simple_key_value() {
        let path = write_temp("KEY1=value1\nKEY2=value2\n", "env_simple.env");
        let map = parse_env_file(&path).unwrap();
        assert_eq!(map.get("KEY1").unwrap(), "value1");
        assert_eq!(map.get("KEY2").unwrap(), "value2");
    }

    #[test]
    fn test_parse_ignores_comments_and_blanks() {
        let path = write_temp(
            "# this is a comment\n\nKEY=value\n  \n# another comment\n",
            "env_comments.env",
        );
        let map = parse_env_file(&path).unwrap();
        assert_eq!(map.len(), 1);
        assert_eq!(map.get("KEY").unwrap(), "value");
    }

    #[test]
    fn test_parse_trims_whitespace_around_key() {
        let path = write_temp("  KEY  =  value with spaces  \n", "env_ws.env");
        let map = parse_env_file(&path).unwrap();
        assert_eq!(map.get("KEY").unwrap(), "value with spaces");
    }

    #[test]
    fn test_parse_preserves_value_equals() {
        let path = write_temp("KEY=val=with=equals\n", "env_equals.env");
        let map = parse_env_file(&path).unwrap();
        assert_eq!(map.get("KEY").unwrap(), "val=with=equals");
    }

    #[test]
    fn test_parse_duplicate_keys_last_wins() {
        let path = write_temp("KEY=first\nKEY=second\n", "env_dup.env");
        let map = parse_env_file(&path).unwrap();
        assert_eq!(map.get("KEY").unwrap(), "second");
    }

    #[test]
    fn test_parse_missing_file_is_error() {
        let result = parse_env_file(Path::new("/nonexistent/path.env"));
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_malformed_line_no_equals_is_error() {
        let path = write_temp("MISSING_EQUALS\n", "env_malformed.env");
        let result = parse_env_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_key_before_equals_is_error() {
        let path = write_temp("  =value\n", "env_empty_key.env");
        let result = parse_env_file(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_env_file_fill_missing_only_when_absent() {
        let path = write_temp(
            "OPENROUTER_API_KEY=fake-key-from-file\n",
            "env_override.env",
        );
        std::env::set_var("OPENROUTER_API_KEY", "already-set-in-env");
        let loaded = load_env_file_fill_missing(&path).unwrap();
        // Should not override existing env — 0 new vars loaded
        assert_eq!(loaded, 0);
        assert_eq!(
            std::env::var("OPENROUTER_API_KEY").unwrap(),
            "already-set-in-env"
        );
        std::env::remove_var("OPENROUTER_API_KEY");
    }

    #[test]
    fn test_load_env_file_fills_missing_var() {
        std::env::remove_var("INTENTLAYER_031_TEST_KEY");
        let path = write_temp(
            "INTENTLAYER_031_TEST_KEY=fake-key-from-file\n",
            "env_fill.env",
        );
        let loaded = load_env_file_fill_missing(&path).unwrap();
        assert_eq!(loaded, 1);
        assert_eq!(
            std::env::var("INTENTLAYER_031_TEST_KEY").unwrap(),
            "fake-key-from-file"
        );
        std::env::remove_var("INTENTLAYER_031_TEST_KEY");
    }

    #[test]
    fn test_parse_does_not_print_values_in_error() {
        let path = write_temp("sk-secret-key\n", "env_bad_line.env");
        let result = parse_env_file(&path);
        let err = result.unwrap_err();
        assert!(!err.contains("sk-secret-key"));
    }
}
