use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct GitConfig {
    #[serde(default)]
    pub config_overrides: Vec<String>,
}

pub fn validate_git_config_overrides(overrides: &[String]) -> Result<()> {
    for override_value in overrides {
        validate_git_config_override(override_value)?;
    }
    Ok(())
}

pub fn validate_git_config_override(override_value: &str) -> Result<()> {
    let Some((key, _value)) = override_value.split_once('=') else {
        return Err(anyhow!(
            "Invalid git config override '{override_value}'. Expected key=value"
        ));
    };

    if key.trim().is_empty() {
        return Err(anyhow!(
            "Invalid git config override '{override_value}'. Key must not be empty"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_git_config_override, validate_git_config_overrides};

    #[test]
    fn validate_git_config_override_accepts_empty_value() {
        assert!(validate_git_config_override("core.hooksPath=").is_ok());
    }

    #[test]
    fn validate_git_config_override_rejects_missing_equals() {
        assert!(validate_git_config_override("core.hooksPath").is_err());
    }

    #[test]
    fn validate_git_config_overrides_rejects_empty_key() {
        assert!(validate_git_config_overrides(&["=value".to_string()]).is_err());
    }
}
