use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use chrono::Utc;
use git2::{Oid, Repository};
use minijinja::{context, Environment};
use semver::Version;
use serde::Serialize;

use crate::config::changelog::ChangelogConfig;
use crate::convention::{Convention, ParsedCommit};
use crate::error::CliError;

#[derive(Debug, Clone, Serialize)]
pub struct ChangelogEntry {
    pub sha: String,
    pub commit_type: String,
    pub scope: String,
    pub description: String,
    pub body: String,
    pub breaking_change: bool,
    pub section: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChangelogPlan {
    pub previous_ref: Option<String>,
    pub next_ref: Option<String>,
    pub output_file: Option<String>,
    pub rendered: String,
    pub entry_count: usize,
}

pub fn latest_semver_tag(repo: &Repository) -> Result<Option<String>, CliError> {
    Ok(repo
        .tag_names(None)?
        .iter()
        .flatten()
        .filter_map(|tag| {
            Version::parse(tag.trim_start_matches('v'))
                .ok()
                .map(|version| (version, tag.to_string()))
        })
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, tag)| tag))
}

pub fn collect_entries(
    repo_path: &Path,
    convention: &Convention,
    from_ref: Option<&str>,
    to_ref: Option<&str>,
    changelog_config: &ChangelogConfig,
) -> Result<(Vec<ChangelogEntry>, Option<String>), CliError> {
    let repo = Repository::open(repo_path).map_err(CliError::from)?;
    let resolved_previous = if let Some(from_ref) = from_ref {
        Some(from_ref.to_string())
    } else {
        latest_semver_tag(&repo)?
    };
    let target = to_ref.unwrap_or("HEAD");
    let target_oid = repo.revparse_single(target)?.peel_to_commit()?.id();
    let mut revwalk = repo.revwalk()?;
    revwalk.push(target_oid)?;
    if let Some(previous) = &resolved_previous {
        if let Ok(obj) = repo.revparse_single(previous) {
            if let Ok(commit) = obj.peel_to_commit() {
                revwalk.hide(commit.id())?;
            }
        }
    }

    let mut entries = vec![];
    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;
        let message = commit.message().unwrap_or("").trim().to_string();
        let Some(parsed) = convention.parse_message(&message) else {
            continue;
        };
        if !type_included(&parsed, changelog_config) {
            continue;
        }
        entries.push(entry_from_commit(oid, parsed, convention, changelog_config));
    }

    Ok((entries, resolved_previous))
}

fn type_included(parsed: &ParsedCommit, changelog_config: &ChangelogConfig) -> bool {
    if !changelog_config.include_types.is_empty()
        && !changelog_config.include_types.contains(&parsed.commit_type)
    {
        return false;
    }
    !changelog_config.exclude_types.contains(&parsed.commit_type)
}

fn entry_from_commit(
    oid: Oid,
    parsed: ParsedCommit,
    convention: &Convention,
    changelog_config: &ChangelogConfig,
) -> ChangelogEntry {
    let section = if parsed.breaking_change || parsed.body.contains("BREAKING CHANGE:") {
        "Breaking Changes".to_string()
    } else {
        changelog_config
            .type_sections
            .get(&parsed.commit_type)
            .cloned()
            .unwrap_or_else(|| convention.changelog_section_for_type(&parsed.commit_type))
    };

    ChangelogEntry {
        sha: oid.to_string(),
        commit_type: parsed.commit_type,
        scope: parsed.scope,
        description: parsed.description,
        body: parsed.body,
        breaking_change: parsed.breaking_change,
        section,
    }
}

pub fn render_changelog(
    config: &ChangelogConfig,
    version: &str,
    previous_ref: Option<&str>,
    entries: &[ChangelogEntry],
) -> Result<String, CliError> {
    let mut grouped: BTreeMap<String, Vec<&ChangelogEntry>> = BTreeMap::new();
    for entry in entries {
        grouped
            .entry(entry.section.clone())
            .or_default()
            .push(entry);
    }
    let mut sections = vec![];
    for name in &config.section_order {
        if let Some(entries) = grouped.remove(name) {
            sections.push((name.clone(), entries));
        }
    }
    sections.extend(grouped.into_iter());

    let template = if let Some(path) = &config.template_path {
        fs::read_to_string(path).map_err(CliError::IoError)?
    } else {
        builtin_template(&config.template)
            .ok_or_else(|| {
                CliError::InputError(format!("Unknown changelog template '{}'", config.template))
            })?
            .to_string()
    };

    let mut env = Environment::new();
    env.add_template("changelog", &template)
        .map_err(|e| CliError::Generic(e.to_string()))?;
    env.get_template("changelog")
        .map_err(|e| CliError::Generic(e.to_string()))?
        .render(context! {
            version => version,
            generated_at => Utc::now().date_naive().to_string(),
            previous_ref => previous_ref.unwrap_or("initial release"),
            sections => sections,
        })
        .map_err(|e| CliError::Generic(e.to_string()))
}

pub fn write_changelog(path: &Path, content: &str) -> Result<(), CliError> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    let updated = if existing.is_empty() {
        content.to_string()
    } else {
        format!("{content}\n\n{existing}")
    };
    fs::write(path, updated).map_err(CliError::IoError)
}

fn builtin_template(name: &str) -> Option<&'static str> {
    match name {
        "conventional" => Some(
            r#"## {{ version }} - {{ generated_at }}
{% if previous_ref %}_Since {{ previous_ref }}_{% endif %}
{% for section_name, entries in sections %}
### {{ section_name }}
{% for entry in entries -%}
- {% if entry.scope %}**{{ entry.scope }}:** {% endif %}{{ entry.description }} ({{ entry.sha[:7] }})
{% endfor %}
{% endfor %}"#,
        ),
        _ => None,
    }
}
