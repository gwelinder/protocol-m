// JCS (JSON Canonicalization Scheme) - RFC 8785 implementation

use anyhow::Result;
use serde::Serialize;

/// Canonicalizes a serializable value according to RFC 8785 (JCS) and returns the UTF-8 bytes.
///
/// This ensures deterministic JSON serialization for signing:
/// - Object keys are sorted lexicographically
/// - No unnecessary whitespace
/// - Numbers are serialized consistently
///
/// # Arguments
/// * `value` - Any serializable value
///
/// # Returns
/// * `Ok(Vec<u8>)` - UTF-8 bytes of the canonical JSON
/// * `Err` - If serialization fails
pub fn jcs_canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let canonical = serde_jcs::to_string(value)?;
    Ok(canonical.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[test]
    fn test_field_order_independence() {
        // Test that two objects with same content but different field order
        // produce identical canonical bytes

        // Create JSON with fields in one order
        let json1 = r#"{"zebra": 1, "alpha": 2, "middle": 3}"#;
        // Create JSON with fields in different order
        let json2 = r#"{"alpha": 2, "middle": 3, "zebra": 1}"#;

        // Parse both as generic JSON values
        let value1: serde_json::Value = serde_json::from_str(json1).unwrap();
        let value2: serde_json::Value = serde_json::from_str(json2).unwrap();

        // Canonicalize both
        let bytes1 = jcs_canonical_bytes(&value1).unwrap();
        let bytes2 = jcs_canonical_bytes(&value2).unwrap();

        // They should produce identical bytes
        assert_eq!(bytes1, bytes2);

        // And the output should have keys in sorted order
        let canonical_str = String::from_utf8(bytes1).unwrap();
        assert_eq!(canonical_str, r#"{"alpha":2,"middle":3,"zebra":1}"#);
    }

    #[test]
    fn test_nested_object_sorting() {
        let json = r#"{"outer": {"z": 1, "a": 2}, "inner": {"y": 3, "b": 4}}"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let bytes = jcs_canonical_bytes(&value).unwrap();
        let canonical_str = String::from_utf8(bytes).unwrap();

        // Both outer and inner keys should be sorted
        assert_eq!(canonical_str, r#"{"inner":{"b":4,"y":3},"outer":{"a":2,"z":1}}"#);
    }

    #[test]
    fn test_struct_serialization() {
        #[derive(Serialize, Deserialize)]
        struct TestStruct {
            zebra: i32,
            alpha: String,
        }

        let value = TestStruct {
            zebra: 42,
            alpha: "hello".to_string(),
        };

        let bytes = jcs_canonical_bytes(&value).unwrap();
        let canonical_str = String::from_utf8(bytes).unwrap();

        // Struct fields should be in their declaration order, but JCS sorts them
        // Note: serde_jcs sorts by key name
        assert_eq!(canonical_str, r#"{"alpha":"hello","zebra":42}"#);
    }
}
