use serde::{Deserialize, Serialize};

use super::repository::BumpType;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ConventionConfig {
    pub name: String,
    pub description: Option<String>,
    pub schema: String,
    pub parser: String,
    pub commit_template: String,
    pub info: String,
    pub examples: Vec<String>,
    pub types: Vec<ConventionType>,
    pub questions: Vec<ConventionQuestion>,
    pub max_subject_length: usize,
    pub max_body_line_length: usize,
    pub require_body: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ConventionType {
    pub name: String,
    pub description: String,
    #[serde(default = "default_type_contexts")]
    pub contexts: Vec<ConventionContext>,
    pub bump: BumpType,
    pub changelog_section: String,
    pub aliases: Vec<String>,
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConventionContext {
    Commit,
    Branch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ConventionQuestion {
    pub key: String,
    pub kind: QuestionKind,
    pub prompt: String,
    pub required: bool,
    pub default: Option<String>,
    pub choices: Vec<String>,
    pub multiline: bool,
    pub when_field: Option<String>,
    pub when_equals: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum QuestionKind {
    Select,
    Text,
    Confirm,
    Multiline,
}

impl Default for ConventionConfig {
    fn default() -> Self {
        Self {
            name: "conventional-commits".to_string(),
            description: Some(
                "Committy conventional commit flow with SemVer-aware release metadata"
                    .to_string(),
            ),
            schema: "<type>(<scope>)!: <description>".to_string(),
            parser: r"^(?P<type>[a-z0-9-]+)(?:\((?P<scope>[a-z0-9,-]+)\))?(?P<breaking>!)?: (?P<description>.+)$".to_string(),
            commit_template: "{{ type }}{% if scope %}({{ scope }}){% endif %}{% if breaking %}!{% endif %}: {{ description }}{% if body %}\n\n{{ body }}{% endif %}".to_string(),
            info: "Conventional Commits with configurable types, changelog sections, and SemVer bump mapping.".to_string(),
            examples: vec![
                "feat(api): add release planning endpoint".to_string(),
                "fix(cli): respect alternate ssh identity".to_string(),
                "docs: document changelog workflow".to_string(),
            ],
            types: default_types(),
            questions: default_questions(),
            max_subject_length: 72,
            max_body_line_length: 100,
            require_body: false,
        }
    }
}

impl Default for ConventionType {
    fn default() -> Self {
        Self {
            name: "chore".to_string(),
            description: "Other changes that do not modify src or test files".to_string(),
            contexts: default_type_contexts(),
            bump: BumpType::None,
            changelog_section: "Maintenance".to_string(),
            aliases: vec![],
            hidden: false,
        }
    }
}

fn default_type_contexts() -> Vec<ConventionContext> {
    vec![ConventionContext::Commit]
}

fn commit_and_branch_contexts() -> Vec<ConventionContext> {
    vec![ConventionContext::Commit, ConventionContext::Branch]
}

fn branch_contexts() -> Vec<ConventionContext> {
    vec![ConventionContext::Branch]
}

impl Default for ConventionQuestion {
    fn default() -> Self {
        Self {
            key: "description".to_string(),
            kind: QuestionKind::Text,
            prompt: "Describe the change".to_string(),
            required: true,
            default: None,
            choices: vec![],
            multiline: false,
            when_field: None,
            when_equals: None,
        }
    }
}

fn default_questions() -> Vec<ConventionQuestion> {
    vec![
        ConventionQuestion {
            key: "type".to_string(),
            kind: QuestionKind::Select,
            prompt: "Select the change type".to_string(),
            required: true,
            default: Some("feat".to_string()),
            choices: default_types().into_iter().map(|item| item.name).collect(),
            multiline: false,
            when_field: None,
            when_equals: None,
        },
        ConventionQuestion {
            key: "scope".to_string(),
            kind: QuestionKind::Text,
            prompt: "Scope (optional)".to_string(),
            required: false,
            default: None,
            choices: vec![],
            multiline: false,
            when_field: None,
            when_equals: None,
        },
        ConventionQuestion {
            key: "description".to_string(),
            kind: QuestionKind::Text,
            prompt: "Short description".to_string(),
            required: true,
            default: None,
            choices: vec![],
            multiline: false,
            when_field: None,
            when_equals: None,
        },
        ConventionQuestion {
            key: "breaking".to_string(),
            kind: QuestionKind::Confirm,
            prompt: "Is this a breaking change?".to_string(),
            required: false,
            default: Some("false".to_string()),
            choices: vec![],
            multiline: false,
            when_field: None,
            when_equals: None,
        },
        ConventionQuestion {
            key: "body".to_string(),
            kind: QuestionKind::Multiline,
            prompt: "Long description (optional)".to_string(),
            required: false,
            default: None,
            choices: vec![],
            multiline: true,
            when_field: None,
            when_equals: None,
        },
    ]
}

pub fn default_types() -> Vec<ConventionType> {
    vec![
        ConventionType {
            name: "feat".to_string(),
            description: "A new feature".to_string(),
            contexts: commit_and_branch_contexts(),
            bump: BumpType::Minor,
            changelog_section: "Features".to_string(),
            aliases: vec!["feature".to_string()],
            hidden: false,
        },
        ConventionType {
            name: "fix".to_string(),
            description: "A bug fix".to_string(),
            contexts: commit_and_branch_contexts(),
            bump: BumpType::Patch,
            changelog_section: "Bug Fixes".to_string(),
            aliases: vec![],
            hidden: false,
        },
        ConventionType {
            name: "docs".to_string(),
            description: "Documentation only changes".to_string(),
            contexts: commit_and_branch_contexts(),
            bump: BumpType::Patch,
            changelog_section: "Documentation".to_string(),
            aliases: vec![],
            hidden: false,
        },
        ConventionType {
            name: "perf".to_string(),
            description: "A code change that improves performance".to_string(),
            contexts: commit_and_branch_contexts(),
            bump: BumpType::Patch,
            changelog_section: "Performance".to_string(),
            aliases: vec![],
            hidden: false,
        },
        ConventionType {
            name: "refactor".to_string(),
            description: "A code change that neither fixes a bug nor adds a feature".to_string(),
            contexts: commit_and_branch_contexts(),
            bump: BumpType::Patch,
            changelog_section: "Refactoring".to_string(),
            aliases: vec![],
            hidden: false,
        },
        ConventionType {
            name: "build".to_string(),
            description: "Changes that affect the build system".to_string(),
            contexts: default_type_contexts(),
            bump: BumpType::Patch,
            changelog_section: "Build".to_string(),
            aliases: vec![],
            hidden: false,
        },
        ConventionType {
            name: "ci".to_string(),
            description: "CI configuration changes".to_string(),
            contexts: default_type_contexts(),
            bump: BumpType::Patch,
            changelog_section: "CI".to_string(),
            aliases: vec![],
            hidden: false,
        },
        ConventionType {
            name: "chore".to_string(),
            description: "Other changes that do not modify src or test files".to_string(),
            contexts: default_type_contexts(),
            bump: BumpType::None,
            changelog_section: "Maintenance".to_string(),
            aliases: vec![],
            hidden: false,
        },
        ConventionType {
            name: "test".to_string(),
            description: "Adding or fixing tests".to_string(),
            contexts: commit_and_branch_contexts(),
            bump: BumpType::Patch,
            changelog_section: "Tests".to_string(),
            aliases: vec![],
            hidden: false,
        },
        ConventionType {
            name: "style".to_string(),
            description: "Code style changes".to_string(),
            contexts: default_type_contexts(),
            bump: BumpType::Patch,
            changelog_section: "Style".to_string(),
            aliases: vec![],
            hidden: false,
        },
        branch_type("security", "Security-focused work"),
        branch_type("hotfix", "Urgent production fix"),
        branch_type("release", "Release preparation"),
        branch_type("spike", "Time-boxed investigation"),
        branch_type("tooling", "Developer tooling work"),
    ]
}

fn branch_type(name: &str, description: &str) -> ConventionType {
    ConventionType {
        name: name.to_string(),
        description: description.to_string(),
        contexts: branch_contexts(),
        bump: BumpType::None,
        changelog_section: "Other".to_string(),
        aliases: vec![],
        hidden: false,
    }
}
