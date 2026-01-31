// Metadata parsing for --meta flag
//
// Parses key=value pairs with support for nested keys using dot notation.
// Example: author.name=Alice becomes { "author": { "name": "Alice" } }

use anyhow::{anyhow, Result};
use serde_json::{json, Map, Value};

/// Parses a list of key=value strings into a JSON object.
///
/// Supports nested keys using dot notation:
/// - `author=Alice` -> `{ "author": "Alice" }`
/// - `author.name=Alice` -> `{ "author": { "name": "Alice" } }`
/// - `author.email=alice@example.com` -> merges with above
///
/// # Arguments
/// * `args` - A vector of strings in the format "key=value"
///
/// # Returns
/// * `Ok(Value)` - A JSON object containing all parsed metadata
/// * `Err` - If any argument is malformed
///
/// # Examples
/// ```
/// use openclaw_cli::metadata::parse_metadata;
///
/// let args = vec!["author=Alice".to_string(), "version=1.0".to_string()];
/// let result = parse_metadata(args).unwrap();
/// assert_eq!(result["author"], "Alice");
/// assert_eq!(result["version"], "1.0");
/// ```
pub fn parse_metadata(args: Vec<String>) -> Result<Value> {
    let mut root = Map::new();

    for arg in args {
        let (key, value) = parse_key_value(&arg)?;
        insert_nested(&mut root, &key, value)?;
    }

    Ok(Value::Object(root))
}

/// Parses a single "key=value" string.
fn parse_key_value(arg: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = arg.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(anyhow!(
            "Invalid metadata format: '{}'. Expected 'key=value'",
            arg
        ));
    }

    let key = parts[0].trim();
    let value = parts[1].trim();

    if key.is_empty() {
        return Err(anyhow!("Empty key in metadata: '{}'", arg));
    }

    Ok((key.to_string(), value.to_string()))
}

/// Inserts a value into a nested map structure using dot notation.
fn insert_nested(root: &mut Map<String, Value>, key: &str, value: String) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();

    if parts.is_empty() {
        return Err(anyhow!("Empty key path"));
    }

    // Navigate/create nested objects
    let mut current = root;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            return Err(anyhow!(
                "Invalid key path '{}': empty segment after dot",
                key
            ));
        }

        let is_last = i == parts.len() - 1;

        if is_last {
            // Insert the value at the final key
            current.insert(part.to_string(), json!(value));
        } else {
            // Navigate or create intermediate object
            if !current.contains_key(*part) {
                current.insert(part.to_string(), json!({}));
            }

            // Get mutable reference to the nested object
            let nested = current.get_mut(*part).unwrap();
            match nested {
                Value::Object(map) => {
                    current = map;
                }
                _ => {
                    return Err(anyhow!(
                        "Cannot create nested key '{}': '{}' is already a scalar value",
                        key,
                        part
                    ));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_key_value() {
        let args = vec!["author=Alice".to_string(), "version=1.0".to_string()];
        let result = parse_metadata(args).unwrap();

        assert_eq!(result["author"], "Alice");
        assert_eq!(result["version"], "1.0");
    }

    #[test]
    fn test_nested_keys() {
        let args = vec![
            "author.name=Alice".to_string(),
            "author.email=alice@example.com".to_string(),
        ];
        let result = parse_metadata(args).unwrap();

        assert_eq!(result["author"]["name"], "Alice");
        assert_eq!(result["author"]["email"], "alice@example.com");
    }

    #[test]
    fn test_deeply_nested_keys() {
        let args = vec!["a.b.c.d=value".to_string()];
        let result = parse_metadata(args).unwrap();

        assert_eq!(result["a"]["b"]["c"]["d"], "value");
    }

    #[test]
    fn test_mixed_nesting() {
        let args = vec![
            "simple=value".to_string(),
            "nested.key=other".to_string(),
            "nested.another=third".to_string(),
        ];
        let result = parse_metadata(args).unwrap();

        assert_eq!(result["simple"], "value");
        assert_eq!(result["nested"]["key"], "other");
        assert_eq!(result["nested"]["another"], "third");
    }

    #[test]
    fn test_empty_input() {
        let args: Vec<String> = vec![];
        let result = parse_metadata(args).unwrap();

        assert!(result.as_object().unwrap().is_empty());
    }

    #[test]
    fn test_value_with_equals_sign() {
        let args = vec!["equation=a=b+c".to_string()];
        let result = parse_metadata(args).unwrap();

        assert_eq!(result["equation"], "a=b+c");
    }

    #[test]
    fn test_invalid_format_no_equals() {
        let args = vec!["no_equals_sign".to_string()];
        let result = parse_metadata(args);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Expected 'key=value'"));
    }

    #[test]
    fn test_invalid_format_empty_key() {
        let args = vec!["=value".to_string()];
        let result = parse_metadata(args);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty key"));
    }

    #[test]
    fn test_invalid_format_empty_segment() {
        let args = vec!["a..b=value".to_string()];
        let result = parse_metadata(args);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("empty segment after dot"));
    }

    #[test]
    fn test_conflict_scalar_then_object() {
        let args = vec!["author=Alice".to_string(), "author.name=Bob".to_string()];
        let result = parse_metadata(args);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already a scalar value"));
    }

    #[test]
    fn test_whitespace_trimming() {
        let args = vec!["  key  =  value  ".to_string()];
        let result = parse_metadata(args).unwrap();

        assert_eq!(result["key"], "value");
    }
}
