use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ChangelogConfig {
    pub output_file: Option<String>,
    pub template: String,
    pub template_path: Option<String>,
    pub section_order: Vec<String>,
    pub type_sections: BTreeMap<String, String>,
    pub include_types: Vec<String>,
    pub exclude_types: Vec<String>,
}

impl Default for ChangelogConfig {
    fn default() -> Self {
        let mut type_sections = BTreeMap::new();
        type_sections.insert("feat".to_string(), "Features".to_string());
        type_sections.insert("fix".to_string(), "Bug Fixes".to_string());
        type_sections.insert("docs".to_string(), "Documentation".to_string());
        type_sections.insert("perf".to_string(), "Performance".to_string());
        type_sections.insert("refactor".to_string(), "Refactoring".to_string());
        type_sections.insert("build".to_string(), "Build".to_string());
        type_sections.insert("ci".to_string(), "CI".to_string());
        type_sections.insert("test".to_string(), "Tests".to_string());
        type_sections.insert("chore".to_string(), "Maintenance".to_string());

        Self {
            output_file: Some("CHANGELOG.md".to_string()),
            template: "conventional".to_string(),
            template_path: None,
            section_order: vec![
                "Breaking Changes".to_string(),
                "Features".to_string(),
                "Bug Fixes".to_string(),
                "Performance".to_string(),
                "Refactoring".to_string(),
                "Documentation".to_string(),
                "Build".to_string(),
                "CI".to_string(),
                "Tests".to_string(),
                "Maintenance".to_string(),
            ],
            type_sections,
            include_types: vec![],
            exclude_types: vec![],
        }
    }
}
