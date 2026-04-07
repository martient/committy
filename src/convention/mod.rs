use std::collections::HashMap;
use std::path::Path;

use crate::config::convention::{ConventionConfig, ConventionType};
use crate::config::hierarchy::MergedConfig;
use crate::config::repository::BumpType;
use crate::error::CliError;
use minijinja::{context, Environment};
use regex::Regex;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ParsedCommit {
    pub commit_type: String,
    pub scope: String,
    pub description: String,
    pub body: String,
    pub breaking_change: bool,
}

#[derive(Debug, Clone)]
pub struct Convention {
    config: ConventionConfig,
    parser: Regex,
    aliases: HashMap<String, String>,
}

impl Convention {
    pub fn load(repo_path: &Path) -> Result<Self, CliError> {
        let merged = MergedConfig::load(repo_path).map_err(|e| CliError::Generic(e.to_string()))?;
        Self::from_config(merged.effective_convention())
    }

    pub fn from_config(config: ConventionConfig) -> Result<Self, CliError> {
        let parser = Regex::new(&config.parser).map_err(|e| CliError::RegexError(e.to_string()))?;
        let mut aliases = HashMap::new();
        for item in &config.types {
            aliases.insert(item.name.clone(), item.name.clone());
            for alias in &item.aliases {
                aliases.insert(alias.clone(), item.name.clone());
            }
        }
        Ok(Self {
            config,
            parser,
            aliases,
        })
    }

    pub fn config(&self) -> &ConventionConfig {
        &self.config
    }

    pub fn schema(&self) -> &str {
        &self.config.schema
    }

    pub fn info(&self) -> &str {
        &self.config.info
    }

    pub fn examples(&self) -> &[String] {
        &self.config.examples
    }

    pub fn parser_pattern(&self) -> &str {
        &self.config.parser
    }

    pub fn visible_types(&self) -> Vec<&ConventionType> {
        self.config
            .types
            .iter()
            .filter(|item| !item.hidden)
            .collect()
    }

    pub fn allowed_types(&self) -> Vec<String> {
        self.visible_types()
            .into_iter()
            .map(|item| item.name.clone())
            .collect()
    }

    pub fn canonical_type(&self, value: &str) -> Option<String> {
        self.aliases.get(value).cloned()
    }

    pub fn validate_message(&self, message: &str) -> Vec<String> {
        let mut issues = Vec::new();
        let trimmed = message.trim();
        let first_line = trimmed.lines().next().unwrap_or("");

        let Some(parsed) = self.parse_message(trimmed) else {
            if !first_line.contains(": ") {
                issues
                    .push("Missing ': ' separator between type/scope and description".to_string());
            } else if first_line.contains("()") {
                issues.push("Empty scope parenthesis".to_string());
            } else if first_line.contains('(') && !first_line.contains(')') {
                issues.push("Unclosed scope parenthesis".to_string());
            } else if first_line.contains(')') && !first_line.contains('(') {
                issues.push("Unopened scope parenthesis".to_string());
            } else if let Some(candidate) = extract_commit_type(first_line) {
                if self.type_by_name(candidate).is_none() {
                    issues.push(format!(
                        "Commit type must be one of: {}",
                        self.allowed_types().join(", ")
                    ));
                } else {
                    issues.push(format!(
                        "Commit message format should be: {}",
                        self.config.schema
                    ));
                }
            } else {
                issues.push(format!(
                    "Commit message format should be: {}",
                    self.config.schema
                ));
            }
            return issues;
        };

        if self.type_by_name(&parsed.commit_type).is_none() {
            issues.push(format!(
                "Commit type must be one of: {}",
                self.allowed_types().join(", ")
            ));
        }

        if first_line.len() < 10 {
            issues.push(format!(
                "Commit message is too short (got {} characters, minimum is 10)",
                first_line.len()
            ));
        }

        if first_line.len() > self.config.max_subject_length {
            issues.push(format!(
                "First line of commit message is too long (got {} characters, maximum is {})",
                first_line.len(),
                self.config.max_subject_length
            ));
        }

        if self.config.require_body && parsed.body.trim().is_empty() {
            issues.push("Commit body is required by repository configuration".to_string());
        }

        for (idx, line) in trimmed.lines().skip(1).enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if line.len() > self.config.max_body_line_length {
                issues.push(format!(
                    "Body line {} is too long (got {} characters, maximum is {})",
                    idx + 2,
                    line.len(),
                    self.config.max_body_line_length
                ));
            }
        }

        issues
    }

    pub fn parse_message(&self, message: &str) -> Option<ParsedCommit> {
        let trimmed = message.trim();
        let first_line = trimmed.lines().next().unwrap_or("");
        let captures = self.parser.captures(first_line)?;
        let commit_type = captures.name("type")?.as_str().to_string();
        let scope = captures
            .name("scope")
            .map(|item| item.as_str().to_string())
            .unwrap_or_default();
        let description = captures.name("description")?.as_str().to_string();
        let breaking_change = captures.name("breaking").is_some();
        let body = trimmed
            .split_once("\n\n")
            .map(|(_, body)| body.to_string())
            .unwrap_or_default();

        Some(ParsedCommit {
            commit_type: self.canonical_type(&commit_type).unwrap_or(commit_type),
            scope,
            description,
            body,
            breaking_change,
        })
    }

    pub fn render_message(
        &self,
        commit_type: &str,
        scope: &str,
        description: &str,
        body: &str,
        breaking_change: bool,
    ) -> Result<String, CliError> {
        let canonical = self
            .canonical_type(commit_type)
            .unwrap_or_else(|| commit_type.to_string());
        let mut env = Environment::new();
        env.add_template("commit", &self.config.commit_template)
            .map_err(|e| CliError::Generic(e.to_string()))?;
        let rendered = env
            .get_template("commit")
            .map_err(|e| CliError::Generic(e.to_string()))?
            .render(context! {
                type => canonical,
                scope => scope,
                description => description,
                body => body,
                breaking => breaking_change,
            })
            .map_err(|e| CliError::Generic(e.to_string()))?;
        Ok(rendered.trim_end().to_string())
    }

    pub fn bump_for_message(&self, message: &str) -> BumpType {
        let Some(parsed) = self.parse_message(message) else {
            return BumpType::Patch;
        };

        if parsed.breaking_change || message.contains("BREAKING CHANGE:") {
            return BumpType::Major;
        }

        self.type_by_name(&parsed.commit_type)
            .map(|item| item.bump.clone())
            .unwrap_or(BumpType::Patch)
    }

    pub fn changelog_section_for_type(&self, commit_type: &str) -> String {
        self.type_by_name(commit_type)
            .map(|item| item.changelog_section.clone())
            .unwrap_or_else(|| "Other".to_string())
    }

    fn type_by_name(&self, commit_type: &str) -> Option<&ConventionType> {
        let canonical = self.canonical_type(commit_type)?;
        self.config.types.iter().find(|item| item.name == canonical)
    }
}

fn extract_commit_type(first_line: &str) -> Option<&str> {
    let separator_index = first_line.find(": ")?;
    let header = &first_line[..separator_index];
    let type_end = header.find(['(', '!']).unwrap_or(header.len());
    Some(&header[..type_end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::convention::ConventionConfig;

    #[test]
    fn test_render_and_parse_roundtrip() {
        let convention = Convention::from_config(ConventionConfig::default()).unwrap();
        let rendered = convention
            .render_message("feat", "cli", "add bump command", "Long body", true)
            .unwrap();
        assert_eq!(
            rendered,
            "feat(cli)!: add bump command\n\nLong body".to_string()
        );

        let parsed = convention.parse_message(&rendered).unwrap();
        assert_eq!(parsed.commit_type, "feat");
        assert_eq!(parsed.scope, "cli");
        assert!(parsed.breaking_change);
    }

    #[test]
    fn test_validate_bad_message() {
        let convention = Convention::from_config(ConventionConfig::default()).unwrap();
        let issues = convention.validate_message("invalid commit");
        assert!(!issues.is_empty());
    }
}
