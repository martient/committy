use crate::error::CliError;
use regex::Regex;
use std::path::Path;

#[derive(Debug)]
pub struct VersionFile {
    path: String,
    pattern: Regex,
    format: String,
}

impl VersionFile {
    pub fn new(path: &str, pattern: &str, format: &str) -> Result<Self, CliError> {
        Ok(VersionFile {
            path: path.to_string(),
            pattern: Regex::new(pattern).map_err(|e| CliError::RegexError(e.to_string()))?,
            format: format.to_string(),
        })
    }

    pub fn update_version(&self, new_version: &str) -> Result<(), CliError> {
        let path = Path::new(&self.path);
        if !path.exists() {
            return Ok(()); // Skip if file doesn't exist
        }

        let content = std::fs::read_to_string(path).map_err(CliError::IoError)?;

        let version_without_v = new_version.trim_start_matches('v');
        let new_content = self
            .pattern
            .replace_all(&content, &self.format.replace("{}", version_without_v))
            .to_string();

        std::fs::write(path, new_content).map_err(CliError::IoError)?;

        Ok(())
    }
}

pub struct VersionManager {
    version_files: Vec<VersionFile>,
}

impl VersionManager {
    pub fn new() -> Self {
        Self {
            version_files: Vec::new(),
        }
    }

    pub fn register_common_files(&mut self) -> Result<(), CliError> {
        // Cargo.toml (Rust)
        self.add_version_file(
            "Cargo.toml",
            r#"(?m)^\s*version\s*=\s*"[^"]*""#, // (?m) enables multiline mode, ^ ensures start of line
            r#"version = "{}""#,
        )?;

        // package.json (Node.js)
        self.add_version_file(
            "package.json",
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )?;

        // pyproject.toml (Python)
        self.add_version_file(
            "pyproject.toml",
            r#"version\s*=\s*"[^"]*""#,
            r#"version = "{}""#,
        )?;

        // composer.json (PHP)
        self.add_version_file(
            "composer.json",
            r#""version"\s*:\s*"[^"]*""#,
            r#""version": "{}""#,
        )?;

        // pom.xml (Java)
        self.add_version_file(
            "pom.xml",
            r#"<version>[^<]*</version>"#,
            "<version>{}</version>",
        )?;

        // *.csproj (.NET)
        self.add_version_file(
            "*.csproj",
            r#"<Version>[^<]*</Version>"#,
            "<Version>{}</Version>",
        )?;

        Ok(())
    }

    pub fn add_version_file(
        &mut self,
        path: &str,
        pattern: &str,
        format: &str,
    ) -> Result<(), CliError> {
        let version_file = VersionFile::new(path, pattern, format)?;
        self.version_files.push(version_file);
        Ok(())
    }

    pub fn update_all_versions(&self, new_version: &str) -> Result<Vec<String>, CliError> {
        let mut updated_files = Vec::new();

        for file in &self.version_files {
            let path = Path::new(&file.path);
            if path.exists() {
                file.update_version(new_version)?;
                updated_files.push(file.path.clone());
            }
        }

        for file in &updated_files {
            std::fs::File::open(file)?; // Wait for the file to be fully written
        }

        Ok(updated_files)
    }
}
