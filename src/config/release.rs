use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ReleaseConfig {
    pub provider: String,
    pub tag_format: String,
    pub version_source: String,
    pub update_files: bool,
    pub annotated_tag: bool,
    pub signed_tag: bool,
    pub bump_commit_message: String,
    pub pre_bump_hooks: Vec<String>,
    pub post_bump_hooks: Vec<String>,
    pub publish: bool,
    pub prerelease_suffix: String,
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            provider: "auto".to_string(),
            tag_format: "v{{ version }}".to_string(),
            version_source: "provider".to_string(),
            update_files: true,
            annotated_tag: true,
            signed_tag: false,
            bump_commit_message: "chore: bump version to {{ version }}".to_string(),
            pre_bump_hooks: vec![],
            post_bump_hooks: vec![],
            publish: false,
            prerelease_suffix: "beta".to_string(),
        }
    }
}
