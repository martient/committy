// YAML file handler for dependency updates

use anyhow::{Context, Result};
use serde_norway::Value;
use std::fs;
use std::path::Path;

/// Read version from a YAML file using a dot-notation field path
/// Example: "image.tag" reads yaml["image"]["tag"]
pub fn read_version(file_path: &Path, field: &str) -> Result<String> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    let yaml: Value = serde_norway::from_str(&content)
        .with_context(|| format!("Failed to parse YAML in {}", file_path.display()))?;

    let version = get_nested_value(&yaml, field)?;

    Ok(version
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Version field '{}' is not a string", field))?
        .to_string())
}

/// Update version in a YAML file using a dot-notation field path
pub fn update_version(file_path: &Path, field: &str, new_version: &str) -> Result<()> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    let mut yaml: Value = serde_norway::from_str(&content)
        .with_context(|| format!("Failed to parse YAML in {}", file_path.display()))?;

    set_nested_value(&mut yaml, field, Value::String(new_version.to_string()))?;

    let updated_content = serde_norway::to_string(&yaml).context("Failed to serialize YAML")?;

    fs::write(file_path, updated_content)
        .with_context(|| format!("Failed to write {}", file_path.display()))?;

    Ok(())
}

/// Get a nested value from YAML using dot notation
fn get_nested_value<'a>(yaml: &'a Value, path: &str) -> Result<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = yaml;

    for part in parts {
        current = current
            .get(part)
            .ok_or_else(|| anyhow::anyhow!("Field '{}' not found in path '{}'", part, path))?;
    }

    Ok(current)
}

/// Set a nested value in YAML using dot notation
fn set_nested_value(yaml: &mut Value, path: &str, value: Value) -> Result<()> {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty path"));
    }

    let mut current = yaml;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - set the value
            if let Value::Mapping(map) = current {
                map.insert(Value::String(part.to_string()), value);
                return Ok(());
            } else {
                return Err(anyhow::anyhow!(
                    "Cannot set field '{}' on non-mapping",
                    part
                ));
            }
        } else {
            // Intermediate part - navigate deeper
            if let Value::Mapping(map) = current {
                current = map
                    .get_mut(Value::String(part.to_string()))
                    .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", part))?;
            } else {
                return Err(anyhow::anyhow!(
                    "Cannot navigate through non-mapping at '{}'",
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
        let file_path = temp_dir.path().join("values.yaml");

        fs::write(
            &file_path,
            r#"
image:
  tag: "1.0.0"
  repository: "myapp"
"#,
        )
        .unwrap();

        let version = read_version(&file_path, "image.tag").unwrap();
        assert_eq!(version, "1.0.0");
    }

    #[test]
    fn test_update_version() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("values.yaml");

        fs::write(
            &file_path,
            r#"
image:
  tag: "1.0.0"
  repository: "myapp"
"#,
        )
        .unwrap();

        update_version(&file_path, "image.tag", "2.0.0").unwrap();

        let version = read_version(&file_path, "image.tag").unwrap();
        assert_eq!(version, "2.0.0");
    }

    #[test]
    fn test_nested_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.yaml");

        fs::write(
            &file_path,
            r#"
app:
  backend:
    version: "1.0.0"
"#,
        )
        .unwrap();

        let version = read_version(&file_path, "app.backend.version").unwrap();
        assert_eq!(version, "1.0.0");

        update_version(&file_path, "app.backend.version", "1.5.0").unwrap();

        let version = read_version(&file_path, "app.backend.version").unwrap();
        assert_eq!(version, "1.5.0");
    }
}
