// Scope matcher for detecting file-based scopes

use anyhow::Result;
use glob::Pattern;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Mapping from file patterns to scopes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeMapping {
    /// Glob pattern to match files
    pub pattern: String,
    /// Scope to assign when pattern matches
    pub scope: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl ScopeMapping {
    /// Create a new scope mapping
    pub fn new(pattern: String, scope: String) -> Self {
        Self {
            pattern,
            scope,
            description: None,
        }
    }

    /// Create a new scope mapping with description
    #[cfg(test)]
    pub fn with_description(pattern: String, scope: String, description: String) -> Self {
        Self {
            pattern,
            scope,
            description: Some(description),
        }
    }

    /// Check if a file path matches this mapping
    pub fn matches(&self, path: &Path) -> Result<bool> {
        let pattern = Pattern::new(&self.pattern)?;
        let path_str = path.to_string_lossy();
        Ok(pattern.matches(&path_str))
    }
}

/// File matcher for determining scopes from file paths
pub struct FileMatcher {
    mappings: Vec<ScopeMapping>,
}

impl FileMatcher {
    /// Create a new file matcher with the given mappings
    pub fn new(mappings: Vec<ScopeMapping>) -> Self {
        Self { mappings }
    }

    /// Find scopes for a list of file paths
    /// Returns unique scopes that match the given files
    pub fn find_scopes(&self, files: &[&Path]) -> Result<Vec<String>> {
        let mut scopes = Vec::new();

        for file in files {
            for mapping in &self.mappings {
                if mapping.matches(file)? && !scopes.contains(&mapping.scope) {
                    scopes.push(mapping.scope.clone());
                }
            }
        }

        Ok(scopes)
    }

    /// Find the first matching scope for a file path
    #[cfg(test)]
    pub fn find_scope(&self, file: &Path) -> Result<Option<String>> {
        for mapping in &self.mappings {
            if mapping.matches(file)? {
                return Ok(Some(mapping.scope.clone()));
            }
        }
        Ok(None)
    }

    /// Get all mappings
    #[cfg(test)]
    pub fn mappings(&self) -> &[ScopeMapping] {
        &self.mappings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_mapping_matches() {
        let mapping = ScopeMapping::new("src/**/*.rs".to_string(), "core".to_string());

        assert!(mapping.matches(Path::new("src/main.rs")).unwrap());
        assert!(mapping.matches(Path::new("src/lib/mod.rs")).unwrap());
        assert!(!mapping.matches(Path::new("tests/test.rs")).unwrap());
    }

    #[test]
    fn test_scope_mapping_with_description() {
        let mapping = ScopeMapping::with_description(
            "docs/**/*.md".to_string(),
            "docs".to_string(),
            "Documentation files".to_string(),
        );

        assert_eq!(mapping.scope, "docs");
        assert_eq!(mapping.description, Some("Documentation files".to_string()));
        assert!(mapping.matches(Path::new("docs/README.md")).unwrap());
    }

    #[test]
    fn test_file_matcher_find_scopes() {
        let mappings = vec![
            ScopeMapping::new("src/**/*.rs".to_string(), "core".to_string()),
            ScopeMapping::new("tests/**/*.rs".to_string(), "tests".to_string()),
            ScopeMapping::new("docs/**/*.md".to_string(), "docs".to_string()),
        ];

        let matcher = FileMatcher::new(mappings);

        let files = vec![
            Path::new("src/main.rs"),
            Path::new("src/lib.rs"),
            Path::new("docs/README.md"),
        ];

        let scopes = matcher.find_scopes(&files).unwrap();
        assert_eq!(scopes.len(), 2);
        assert!(scopes.contains(&"core".to_string()));
        assert!(scopes.contains(&"docs".to_string()));
    }

    #[test]
    fn test_file_matcher_find_scope() {
        let mappings = vec![
            ScopeMapping::new("src/**/*.rs".to_string(), "core".to_string()),
            ScopeMapping::new("tests/**/*.rs".to_string(), "tests".to_string()),
        ];

        let matcher = FileMatcher::new(mappings);

        assert_eq!(
            matcher.find_scope(Path::new("src/main.rs")).unwrap(),
            Some("core".to_string())
        );
        assert_eq!(
            matcher.find_scope(Path::new("tests/test.rs")).unwrap(),
            Some("tests".to_string())
        );
        assert_eq!(matcher.find_scope(Path::new("README.md")).unwrap(), None);
    }

    #[test]
    fn test_file_matcher_unique_scopes() {
        let mappings = vec![ScopeMapping::new(
            "src/**/*.rs".to_string(),
            "core".to_string(),
        )];

        let matcher = FileMatcher::new(mappings);

        let files = vec![
            Path::new("src/main.rs"),
            Path::new("src/lib.rs"),
            Path::new("src/config.rs"),
        ];

        let scopes = matcher.find_scopes(&files).unwrap();
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0], "core");
    }

    #[test]
    fn test_file_matcher_package_paths() {
        let mappings = vec![
            ScopeMapping::new("cli/**/*".to_string(), "cli".to_string()),
            ScopeMapping::new("server/**/*".to_string(), "server".to_string()),
            ScopeMapping::new("docs/**/*".to_string(), "docs".to_string()),
        ];

        let matcher = FileMatcher::new(mappings);

        let files = vec![
            Path::new("cli/src/main.rs"),
            Path::new("server/package.json"),
        ];

        let scopes = matcher.find_scopes(&files).unwrap();
        assert_eq!(scopes.len(), 2);
        assert!(scopes.contains(&"cli".to_string()));
        assert!(scopes.contains(&"server".to_string()));
    }

    #[test]
    fn test_file_matcher_mappings_accessor() {
        let mappings = vec![
            ScopeMapping::new("src/**/*.rs".to_string(), "core".to_string()),
            ScopeMapping::new("docs/**/*.md".to_string(), "docs".to_string()),
        ];

        let matcher = FileMatcher::new(mappings.clone());
        let exposed = matcher.mappings();

        assert_eq!(exposed.len(), mappings.len());
        assert_eq!(exposed[0].pattern, mappings[0].pattern);
        assert_eq!(exposed[1].scope, mappings[1].scope);
    }
}
