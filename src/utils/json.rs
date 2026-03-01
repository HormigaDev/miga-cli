use anyhow::{Context, Result};
use serde_json::Value;

/// Compacts a JSON string by removing all whitespace and newlines.
pub fn minify(input: &str) -> Result<String> {
    let value: Value = serde_json::from_str(input).context("Invalid JSON")?;
    serde_json::to_string(&value).context("Failed to serialize JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minify() {
        let input = r#"{ "name": "test", "value": 42 }"#;
        let output = minify(input).unwrap();
        assert!(!output.contains('\n'));
        assert!(!output.contains("  "));
    }
}
