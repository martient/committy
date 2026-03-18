// JSON file handler for dependency updates

use anyhow::{Context, Result};
use serde_json::Value;
use std::fs;
use std::path::Path;

/// Read version from a JSON file using a dot-notation field path
pub fn read_version(file_path: &Path, field: &str) -> Result<String> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    let json: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON in {}", file_path.display()))?;

    let version = get_nested_value(&json, field)?;

    Ok(version
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Version field '{}' is not a string", field))?
        .to_string())
}

/// Update version in a JSON file using a dot-notation field path
pub fn update_version(file_path: &Path, field: &str, new_version: &str) -> Result<()> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    let mut json: Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON in {}", file_path.display()))?;

    set_nested_value(&mut json, field, Value::String(new_version.to_string()))?;

    let updated_content =
        serde_json::to_string_pretty(&json).context("Failed to serialize JSON")?;

    fs::write(file_path, updated_content + "\n")
        .with_context(|| format!("Failed to write {}", file_path.display()))?;

    Ok(())
}

/// Get a nested value from JSON using dot notation
fn get_nested_value<'a>(json: &'a Value, path: &str) -> Result<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;

    for part in parts {
        current = current
            .get(part)
            .ok_or_else(|| anyhow::anyhow!("Field '{}' not found in path '{}'", part, path))?;
    }

    Ok(current)
}

/// Set a nested value in JSON using dot notation
fn set_nested_value(json: &mut Value, path: &str, value: Value) -> Result<()> {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty path"));
    }

    let mut current = json;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - set the value
            if let Value::Object(map) = current {
                map.insert(part.to_string(), value);
                return Ok(());
            } else {
                return Err(anyhow::anyhow!("Cannot set field '{}' on non-object", part));
            }
        } else {
            // Intermediate part - navigate deeper
            if let Value::Object(map) = current {
                current = map
                    .get_mut(*part)
                    .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", part))?;
            } else {
                return Err(anyhow::anyhow!(
                    "Cannot navigate through non-object at '{}'",
                    part
                ));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_read_version() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        fs::write(
            &file_path,
            r#"{
  "app": {
    "version": "1.0.0"
  }
}"#,
        )
        .unwrap();

        let version = read_version(&file_path, "app.version").unwrap();
        assert_eq!(version, "1.0.0");
    }

    #[test]
    fn test_update_version() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.json");

        fs::write(
            &file_path,
            r#"{
  "app": {
    "version": "1.0.0"
  }
}"#,
        )
        .unwrap();

        update_version(&file_path, "app.version", "2.0.0").unwrap();

        let version = read_version(&file_path, "app.version").unwrap();
        assert_eq!(version, "2.0.0");
    }

    #[test]
    fn test_nested_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("package.json");

        fs::write(
            &file_path,
            r#"{
  "dependencies": {
    "mypackage": "1.0.0"
  }
}"#,
        )
        .unwrap();

        let version = read_version(&file_path, "dependencies.mypackage").unwrap();
        assert_eq!(version, "1.0.0");

        update_version(&file_path, "dependencies.mypackage", "1.5.0").unwrap();

        let version = read_version(&file_path, "dependencies.mypackage").unwrap();
        assert_eq!(version, "1.5.0");
    }
}
