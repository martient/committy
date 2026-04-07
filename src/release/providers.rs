use std::fs;
use std::path::{Path, PathBuf};

use git2::Repository;
use serde::Serialize;
use serde_json::Value;
use toml_edit::{value, DocumentMut};

use crate::config::hierarchy::MergedConfig;
use crate::config::repository::{RepositoryConfig, VersioningStrategy};
use crate::error::CliError;
use crate::release::changelog::latest_semver_tag;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    Cargo,
    Npm,
    Composer,
    Pep621,
    Poetry,
    Uv,
    Scm,
    MultiPackage,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectVersion {
    pub provider: String,
    pub version: Option<String>,
    pub read_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_versions: Option<Vec<(String, String)>>,
}

pub fn builtin_provider_ids() -> Vec<&'static str> {
    vec![
        "cargo",
        "npm",
        "composer",
        "pep621",
        "poetry",
        "uv",
        "scm",
        "multi-package",
    ]
}

pub fn builtin_template_ids() -> Vec<&'static str> {
    vec!["conventional"]
}

impl ProviderKind {
    pub fn id(&self) -> &'static str {
        match self {
            ProviderKind::Cargo => "cargo",
            ProviderKind::Npm => "npm",
            ProviderKind::Composer => "composer",
            ProviderKind::Pep621 => "pep621",
            ProviderKind::Poetry => "poetry",
            ProviderKind::Uv => "uv",
            ProviderKind::Scm => "scm",
            ProviderKind::MultiPackage => "multi-package",
        }
    }

    pub fn is_read_only(&self) -> bool {
        matches!(self, ProviderKind::Scm)
    }

    pub fn managed_files(
        &self,
        repo_path: &Path,
        repo_config: Option<&RepositoryConfig>,
    ) -> Vec<PathBuf> {
        match self {
            ProviderKind::Cargo => vec![repo_path.join("Cargo.toml")],
            ProviderKind::Npm => vec![repo_path.join("package.json")],
            ProviderKind::Composer => vec![repo_path.join("composer.json")],
            ProviderKind::Pep621 | ProviderKind::Poetry | ProviderKind::Uv => {
                vec![repo_path.join("pyproject.toml")]
            }
            ProviderKind::Scm => vec![],
            ProviderKind::MultiPackage => repo_config
                .map(|config| {
                    config
                        .packages
                        .iter()
                        .map(|pkg| repo_path.join(&pkg.path).join(&pkg.version_file))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    pub fn read(
        &self,
        repo_path: &Path,
        repo_config: Option<&RepositoryConfig>,
    ) -> Result<ProjectVersion, CliError> {
        match self {
            ProviderKind::Cargo => Ok(ProjectVersion {
                provider: self.id().to_string(),
                version: Some(read_toml_string(
                    &repo_path.join("Cargo.toml"),
                    &["package", "version"],
                )?),
                read_only: false,
                package_versions: None,
            }),
            ProviderKind::Npm => Ok(ProjectVersion {
                provider: self.id().to_string(),
                version: Some(read_json_string(
                    &repo_path.join("package.json"),
                    "version",
                )?),
                read_only: false,
                package_versions: None,
            }),
            ProviderKind::Composer => Ok(ProjectVersion {
                provider: self.id().to_string(),
                version: Some(read_json_string(
                    &repo_path.join("composer.json"),
                    "version",
                )?),
                read_only: false,
                package_versions: None,
            }),
            ProviderKind::Pep621 | ProviderKind::Uv => Ok(ProjectVersion {
                provider: self.id().to_string(),
                version: Some(read_toml_string(
                    &repo_path.join("pyproject.toml"),
                    &["project", "version"],
                )?),
                read_only: false,
                package_versions: None,
            }),
            ProviderKind::Poetry => Ok(ProjectVersion {
                provider: self.id().to_string(),
                version: Some(read_toml_string(
                    &repo_path.join("pyproject.toml"),
                    &["tool", "poetry", "version"],
                )?),
                read_only: false,
                package_versions: None,
            }),
            ProviderKind::Scm => Ok(ProjectVersion {
                provider: self.id().to_string(),
                version: latest_tag_version(repo_path)?,
                read_only: true,
                package_versions: None,
            }),
            ProviderKind::MultiPackage => {
                let Some(config) = repo_config else {
                    return Err(CliError::InputError(
                        "multi-package provider requires .committy/config.toml".to_string(),
                    ));
                };

                let mut versions = vec![];
                for package in &config.packages {
                    let version = read_version_file(
                        repo_path.join(&package.path).join(&package.version_file),
                        &package.version_field,
                    )?;
                    versions.push((package.name.clone(), version));
                }

                let primary = match config.versioning.strategy {
                    VersioningStrategy::Unified => config.versioning.unified_version.clone(),
                    _ => versions.first().map(|(_, version)| version.clone()),
                };

                Ok(ProjectVersion {
                    provider: self.id().to_string(),
                    version: primary,
                    read_only: false,
                    package_versions: Some(versions),
                })
            }
        }
    }

    pub fn write(
        &self,
        repo_path: &Path,
        repo_config: Option<&RepositoryConfig>,
        new_version: &str,
    ) -> Result<Vec<PathBuf>, CliError> {
        match self {
            ProviderKind::Cargo => {
                let path = repo_path.join("Cargo.toml");
                update_toml_string(&path, &["package", "version"], new_version)?;
                Ok(vec![path])
            }
            ProviderKind::Npm => {
                let path = repo_path.join("package.json");
                update_json_string(&path, "version", new_version)?;
                Ok(vec![path])
            }
            ProviderKind::Composer => {
                let path = repo_path.join("composer.json");
                update_json_string(&path, "version", new_version)?;
                Ok(vec![path])
            }
            ProviderKind::Pep621 | ProviderKind::Uv => {
                let path = repo_path.join("pyproject.toml");
                update_toml_string(&path, &["project", "version"], new_version)?;
                Ok(vec![path])
            }
            ProviderKind::Poetry => {
                let path = repo_path.join("pyproject.toml");
                update_toml_string(&path, &["tool", "poetry", "version"], new_version)?;
                Ok(vec![path])
            }
            ProviderKind::Scm => Ok(vec![]),
            ProviderKind::MultiPackage => {
                let Some(config) = repo_config else {
                    return Err(CliError::InputError(
                        "multi-package provider requires .committy/config.toml".to_string(),
                    ));
                };
                let mut files = vec![];
                for package in &config.packages {
                    let path = repo_path.join(&package.path).join(&package.version_file);
                    update_version_file(&path, &package.version_field, new_version)?;
                    files.push(path);
                }
                Ok(files)
            }
        }
    }
}

pub fn resolve_provider(
    repo_path: &Path,
    merged_config: &MergedConfig,
) -> Result<ProviderKind, CliError> {
    let configured = merged_config.effective_release().provider;
    if configured != "auto" {
        return provider_from_string(&configured);
    }

    if let Some(repository) = merged_config.repository_config() {
        if repository.is_multi_package() {
            return Ok(ProviderKind::MultiPackage);
        }
    }

    if repo_path.join("Cargo.toml").exists() {
        return Ok(ProviderKind::Cargo);
    }
    if repo_path.join("package.json").exists() {
        return Ok(ProviderKind::Npm);
    }
    if repo_path.join("composer.json").exists() {
        return Ok(ProviderKind::Composer);
    }
    if repo_path.join("pyproject.toml").exists() {
        let content =
            fs::read_to_string(repo_path.join("pyproject.toml")).map_err(CliError::IoError)?;
        if content.contains("[tool.poetry]") {
            return Ok(ProviderKind::Poetry);
        }
        if content.contains("[project]") {
            if content.contains("uv") {
                return Ok(ProviderKind::Uv);
            }
            return Ok(ProviderKind::Pep621);
        }
    }

    Ok(ProviderKind::Scm)
}

pub fn provider_from_string(value: &str) -> Result<ProviderKind, CliError> {
    match value {
        "cargo" => Ok(ProviderKind::Cargo),
        "npm" => Ok(ProviderKind::Npm),
        "composer" => Ok(ProviderKind::Composer),
        "pep621" => Ok(ProviderKind::Pep621),
        "poetry" => Ok(ProviderKind::Poetry),
        "uv" => Ok(ProviderKind::Uv),
        "scm" => Ok(ProviderKind::Scm),
        "multi-package" | "committy" => Ok(ProviderKind::MultiPackage),
        other => Err(CliError::InputError(format!(
            "Unknown release provider '{other}'"
        ))),
    }
}

fn read_toml_string(path: &Path, segments: &[&str]) -> Result<String, CliError> {
    let content = fs::read_to_string(path).map_err(CliError::IoError)?;
    let document = content
        .parse::<DocumentMut>()
        .map_err(|e| CliError::Generic(e.to_string()))?;
    let mut current = document.as_item();
    for segment in segments {
        current = current.get(segment).ok_or_else(|| {
            CliError::InputError(format!("Missing TOML key {}", segments.join(".")))
        })?;
    }
    current
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| CliError::InputError(format!("Expected string at {}", segments.join("."))))
}

fn update_toml_string(path: &Path, segments: &[&str], new_value: &str) -> Result<(), CliError> {
    let content = fs::read_to_string(path).map_err(CliError::IoError)?;
    let mut document = content
        .parse::<DocumentMut>()
        .map_err(|e| CliError::Generic(e.to_string()))?;

    if segments.len() == 2 {
        document[segments[0]][segments[1]] = value(new_value);
    } else if segments.len() == 3 {
        document[segments[0]][segments[1]][segments[2]] = value(new_value);
    } else {
        return Err(CliError::InputError(
            "Unsupported TOML path depth".to_string(),
        ));
    }

    fs::write(path, document.to_string()).map_err(CliError::IoError)
}

fn read_json_string(path: &Path, key: &str) -> Result<String, CliError> {
    let content = fs::read_to_string(path).map_err(CliError::IoError)?;
    let parsed: Value =
        serde_json::from_str(&content).map_err(|e| CliError::Generic(e.to_string()))?;
    parsed[key]
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| CliError::InputError(format!("Missing JSON string key '{key}'")))
}

fn update_json_string(path: &Path, key: &str, new_value: &str) -> Result<(), CliError> {
    let content = fs::read_to_string(path).map_err(CliError::IoError)?;
    let mut parsed: Value =
        serde_json::from_str(&content).map_err(|e| CliError::Generic(e.to_string()))?;
    parsed[key] = Value::String(new_value.to_string());
    let rendered =
        serde_json::to_string_pretty(&parsed).map_err(|e| CliError::Generic(e.to_string()))?;
    fs::write(path, format!("{rendered}\n")).map_err(CliError::IoError)
}

fn latest_tag_version(repo_path: &Path) -> Result<Option<String>, CliError> {
    let repo = Repository::open(repo_path).map_err(CliError::from)?;
    Ok(latest_semver_tag(&repo)?.map(|tag| tag.trim_start_matches('v').to_string()))
}

pub fn write_package_version(
    repo_path: &Path,
    repo_config: &RepositoryConfig,
    package_name: &str,
    new_version: &str,
) -> Result<PathBuf, CliError> {
    let package = repo_config
        .packages
        .iter()
        .find(|pkg| pkg.name == package_name)
        .ok_or_else(|| {
            CliError::InputError(format!(
                "Package '{}' missing from repository config",
                package_name
            ))
        })?;
    let path = repo_path.join(&package.path).join(&package.version_file);
    update_version_file(&path, &package.version_field, new_version)?;
    Ok(path)
}

fn read_version_file(path: PathBuf, field: &str) -> Result<String, CliError> {
    if path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name == "package.json" || name == "composer.json")
    {
        return read_json_string(&path, field.rsplit('.').next().unwrap_or(field));
    }

    let segments = field.split('.').collect::<Vec<_>>();
    read_toml_string(&path, &segments)
}

fn update_version_file(path: &Path, field: &str, new_version: &str) -> Result<(), CliError> {
    if path
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name == "package.json" || name == "composer.json")
    {
        return update_json_string(path, field.rsplit('.').next().unwrap_or(field), new_version);
    }

    let segments = field.split('.').collect::<Vec<_>>();
    update_toml_string(path, &segments, new_version)
}
