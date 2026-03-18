// TOML file handler for dependency updates

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use toml_edit::{value, DocumentMut, Item};

/// Read version from a TOML file using a dot-notation field path
pub fn read_version(file_path: &Path, field: &str) -> Result<String> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    let doc: DocumentMut = content
        .parse()
        .with_context(|| format!("Failed to parse TOML in {}", file_path.display()))?;

    let version = get_nested_value(&doc, field)?;

    Ok(version
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Version field '{}' is not a string", field))?
        .to_string())
}

/// Update version in a TOML file using a dot-notation field path
pub fn update_version(file_path: &Path, field: &str, new_version: &str) -> Result<()> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read {}", file_path.display()))?;

    let mut doc: DocumentMut = content
        .parse()
        .with_context(|| format!("Failed to parse TOML in {}", file_path.display()))?;

    set_nested_value(&mut doc, field, value(new_version))?;

    fs::write(file_path, doc.to_string())
        .with_context(|| format!("Failed to write {}", file_path.display()))?;

    Ok(())
}

/// Get a nested value from TOML using dot notation
fn get_nested_value<'a>(doc: &'a DocumentMut, path: &str) -> Result<&'a Item> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = doc.as_item();

    for part in parts {
        current = current
            .get(part)
            .ok_or_else(|| anyhow::anyhow!("Field '{}' not found in path '{}'", part, path))?;
    }

    Ok(current)
}

/// Set a nested value in TOML using dot notation
fn set_nested_value(doc: &mut DocumentMut, path: &str, value: Item) -> Result<()> {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty path"));
    }

    let mut current = doc.as_item_mut();

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part - set the value
            if let Some(table) = current.as_table_mut() {
                table[part] = value;
                return Ok(());
            } else {
                return Err(anyhow::anyhow!("Cannot set field '{}' on non-table", part));
            }
        } else {
            // Intermediate part - navigate deeper
            current = current
                .get_mut(part)
                .ok_or_else(|| anyhow::anyhow!("Field '{}' not found", part))?;
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
        let file_path = temp_dir.path().join("config.toml");

        fs::write(
            &file_path,
            r#"
[package]
version = "1.0.0"
"#,
        )
        .unwrap();

        let version = read_version(&file_path, "package.version").unwrap();
        assert_eq!(version, "1.0.0");
    }

    #[test]
    fn test_update_version() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");

        fs::write(
            &file_path,
            r#"
[package]
version = "1.0.0"
name = "test"
"#,
        )
        .unwrap();

        update_version(&file_path, "package.version", "2.0.0").unwrap();

        let version = read_version(&file_path, "package.version").unwrap();
        assert_eq!(version, "2.0.0");
    }

    #[test]
    fn test_nested_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("Cargo.toml");

        fs::write(
            &file_path,
            r#"
[package]
name = "myapp"
version = "1.0.0"

[dependencies]
mylib = "1.0.0"
"#,
        )
        .unwrap();

        let version = read_version(&file_path, "dependencies.mylib").unwrap();
        assert_eq!(version, "1.0.0");

        update_version(&file_path, "dependencies.mylib", "1.5.0").unwrap();

        let version = read_version(&file_path, "dependencies.mylib").unwrap();
        assert_eq!(version, "1.5.0");
    }
}
