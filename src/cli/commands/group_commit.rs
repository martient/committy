use crate::ai::{AiCommitSuggestion, LlmClient, LlmError, OllamaClient, OpenRouterClient};
use crate::cli::output::{MachineContext, API_VERSION};
use crate::cli::Command;
use crate::error::CliError;
use crate::git::format_commit_message;
use crate::git::{discover_repository_from, list_changed_files_from};
use crate::linter::check_message_format_for_repo;
use git2::Repository;
use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum GroupName {
    Docs,
    Tests,
    Ci,
    Deps,
    Build,
    Chore,
    Code,
}

#[derive(Debug, Serialize, Clone)]
pub struct PlanGroup {
    pub name: GroupName,
    pub commit_type: String,
    pub files: Vec<String>,
    pub suggested_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issues: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct GroupCommitPlanResult {
    pub api_version: u8,
    pub command: String,
    pub mode: String,
    pub ok: bool,
    pub dry_run: bool,
    pub groups: Vec<PlanGroup>,
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CommitRecord {
    pub group: GroupName,
    pub message: String,
    pub ok: bool,
    pub sha: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GroupCommitApplyResult {
    pub api_version: u8,
    pub command: String,
    pub mode: String,
    pub ok: bool,
    pub dry_run: bool,
    pub groups: Vec<PlanGroup>,
    pub commits: Vec<CommitRecord>,
    pub pushed: Option<bool>,
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, StructOpt)]
pub struct GroupCommitCommand {
    /// Mode: plan (default) or apply
    #[structopt(long, default_value = "plan", possible_values = &["plan", "apply"])]
    mode: String,

    /// Include unstaged changes
    #[structopt(long)]
    include_unstaged: bool,

    /// Auto-stage files per group in apply mode
    #[structopt(long)]
    auto_stage: bool,

    /// Push after apply
    #[structopt(long)]
    push: bool,

    /// Confirm the remote push when --push is used
    #[structopt(long)]
    confirm_push: bool,

    /// Output format: text or json
    #[structopt(long, default_value = "json", possible_values = &["text", "json"])]
    output: String,

    // AI flags
    /// Enable AI-assisted messages
    #[structopt(long = "ai")]
    ai: bool,

    /// Provider for AI
    #[structopt(long = "ai-provider", default_value = "openrouter", possible_values = &["openrouter", "ollama"])]
    ai_provider: String,

    /// AI model identifier
    #[structopt(long = "ai-model")]
    ai_model: Option<String>,

    /// Env var name to read the API key from (OpenRouter)
    #[structopt(long = "ai-api-key-env", default_value = "OPENROUTER_API_KEY")]
    ai_api_key_env: String,

    /// Base URL for the provider API
    #[structopt(long = "ai-base-url")]
    ai_base_url: Option<String>,

    /// Max tokens for AI response
    #[structopt(long = "ai-max-tokens", default_value = "256")]
    ai_max_tokens: u32,

    /// Temperature for AI sampling
    #[structopt(long = "ai-temperature", default_value = "0.2")]
    ai_temperature: f32,

    /// Timeout in milliseconds
    #[structopt(long = "ai-timeout-ms", default_value = "20000")]
    ai_timeout_ms: u64,

    /// Disable JSON mode for AI output
    #[structopt(long = "no-ai-json-mode")]
    no_ai_json_mode: bool,

    /// Custom system prompt
    #[structopt(long = "ai-system-prompt")]
    ai_system_prompt: Option<String>,

    /// System prompt file
    #[structopt(long = "ai-system-prompt-file")]
    ai_system_prompt_file: Option<String>,

    /// Max files per group sent to AI
    #[structopt(long = "ai-file-limit", default_value = "20")]
    ai_file_limit: usize,

    /// Allow sending sensitive content to external AI providers
    #[structopt(long = "ai-allow-sensitive")]
    ai_allow_sensitive: bool,

    #[structopt(
        long = "git-config",
        help = "Pass through git -c key=value overrides (repeatable)"
    )]
    git_config: Vec<String>,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

impl Default for GroupCommitCommand {
    fn default() -> Self {
        GroupCommitCommand {
            mode: "plan".into(),
            include_unstaged: false,
            auto_stage: false,
            push: false,
            confirm_push: false,
            output: "json".into(),
            ai: false,
            ai_provider: "openrouter".into(),
            ai_model: None,
            ai_api_key_env: "OPENROUTER_API_KEY".into(),
            ai_base_url: None,
            ai_max_tokens: 256,
            ai_temperature: 0.2,
            ai_timeout_ms: 20000,
            no_ai_json_mode: false,
            ai_system_prompt: None,
            ai_system_prompt_file: None,
            ai_file_limit: 20,
            ai_allow_sensitive: false,
            git_config: vec![],
            repo_path: PathBuf::from("."),
        }
    }
}

fn classify_file(file: &str) -> GroupName {
    let f = file.trim_start_matches("./");
    // CI
    if f.starts_with(".github/") {
        return GroupName::Ci;
    }
    // Docs
    if f.starts_with("docs/")
        || f.ends_with("README.md")
        || f.ends_with("README.MD")
        || f.ends_with(".md")
        || f.ends_with(".MD")
        || f.ends_with(".mdx")
        || f.ends_with(".MDX")
    {
        return GroupName::Docs;
    }
    // Tests
    if f.starts_with("tests/")
        || f.ends_with("_test.rs")
        || f.ends_with(".test.js")
        || f.ends_with(".test.ts")
        || f.ends_with(".spec.js")
        || f.ends_with(".spec.ts")
    {
        return GroupName::Tests;
    }
    // Deps (lockfiles)
    if f.ends_with("package-lock.json")
        || f.ends_with("npm-shrinkwrap.json")
        || f.ends_with("pnpm-lock.yaml")
        || f.ends_with("yarn.lock")
        || f.ends_with("Cargo.lock")
    {
        return GroupName::Deps;
    }
    // Build/config
    if f.ends_with("Cargo.toml")
        || f.ends_with("build.rs")
        || f.ends_with("package.json")
        || f.ends_with("tsconfig.json")
        || f.contains("eslint.")
        || f.ends_with(".eslintrc")
        || f.contains("vite.config")
        || f.ends_with("rollup.config.js")
        || f.ends_with("rollup.config.cjs")
        || f.ends_with("rollup.config.mjs")
    {
        return GroupName::Build;
    }
    // Chore (editor/config meta)
    if f.starts_with(".vscode/")
        || f.ends_with(".editorconfig")
        || f.ends_with(".gitignore")
        || f.ends_with(".npmrc")
    {
        return GroupName::Chore;
    }
    // Everything else
    GroupName::Code
}

fn default_type_for(name: &GroupName) -> &'static str {
    match name {
        GroupName::Docs => "docs",
        GroupName::Tests => "test",
        GroupName::Ci => "ci",
        GroupName::Deps => "chore",
        GroupName::Build => "build",
        GroupName::Chore => "chore",
        GroupName::Code => "chore",
    }
}

fn default_short_for(name: &GroupName) -> &'static str {
    match name {
        GroupName::Docs => "update docs",
        GroupName::Tests => "update tests",
        GroupName::Ci => "update CI",
        GroupName::Deps => "update dependencies",
        GroupName::Build => "update build config",
        GroupName::Chore => "misc maintenance",
        GroupName::Code => "update code",
    }
}

fn group_name_str(name: GroupName) -> &'static str {
    match name {
        GroupName::Docs => "docs",
        GroupName::Tests => "tests",
        GroupName::Ci => "ci",
        GroupName::Deps => "deps",
        GroupName::Build => "build",
        GroupName::Chore => "chore",
        GroupName::Code => "code",
    }
}

fn build_message_from_suggestion(
    s: &AiCommitSuggestion,
    fallback_type: &str,
    fallback_short: &str,
) -> String {
    if let Some(msg) = &s.message {
        return msg.trim().to_string();
    }
    let commit_type = s.commit_type.as_deref().unwrap_or(fallback_type);
    let short = s.short.as_deref().unwrap_or(fallback_short);
    let scope = s.scope.as_deref().unwrap_or("");
    let long = s.long.as_deref().unwrap_or("");
    format_commit_message(commit_type, false, scope, short, long)
}

impl Command for GroupCommitCommand {
    fn execute(&self, _non_interactive: bool) -> Result<(), CliError> {
        match self.mode.as_str() {
            "plan" => {
                if self.push {
                    return Err(CliError::InputError(
                        "--push is only valid in apply mode".to_string(),
                    ));
                }
                let files = list_changed_files_from(&self.repo_path, self.include_unstaged)?;
                let repo = discover_repository_from(&self.repo_path)?;
                let repo_path = repo.workdir().ok_or_else(|| {
                    CliError::GitError(git2::Error::from_str("No working directory"))
                })?;
                let _git_command_config =
                    crate::git::resolve_git_command_config(repo_path, &self.git_config)?;
                let mut groups = build_groups(files);

                let mut errors: Vec<String> = Vec::new();

                if self.ai {
                    // Build system prompt
                    let mut system_prompt = self
                        .ai_system_prompt
                        .clone()
                        .unwrap_or_else(|| {
                            "You are a commit message assistant. Generate conventional commit messages. Prefer JSON if requested.\nReturn an object {\"commit_type\", \"short\", \"scope\", \"long\", \"message\"}. If 'message' is present, it should be a full commit message with the first line formatted as '<type>(<scope>): <short>' (scope optional).".to_string()
                        });
                    if let Some(path) = &self.ai_system_prompt_file {
                        if let Ok(fp) = fs::read_to_string(path) {
                            system_prompt = fp;
                        } else {
                            errors.push(format!("failed to read ai_system_prompt_file: {path}"));
                        }
                    }

                    // Determine provider and client
                    let json_mode = !self.no_ai_json_mode;
                    let max_tokens = self.ai_max_tokens;
                    let temperature = self.ai_temperature;
                    let timeout_ms = self.ai_timeout_ms;

                    let provider = self.ai_provider.as_str();
                    let runtime = tokio::runtime::Runtime::new()
                        .map_err(|e| CliError::Generic(e.to_string()))?;

                    for g in groups.iter_mut() {
                        let user_prompt = ai_user_prompt(
                            &g.name,
                            &g.commit_type,
                            &g.files,
                            self.ai_allow_sensitive,
                            self.ai_file_limit,
                        );

                        let result: Result<String, LlmError> = match provider {
                            "openrouter" => {
                                let base = self
                                    .ai_base_url
                                    .clone()
                                    .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
                                let model = self
                                    .ai_model
                                    .clone()
                                    .unwrap_or_else(|| "openrouter/auto".to_string());
                                let key_env = self.ai_api_key_env.clone();
                                let api_key = env::var(key_env).ok();
                                let client = OpenRouterClient {
                                    base_url: base,
                                    api_key,
                                    model,
                                };
                                runtime.block_on(client.suggest_commit(
                                    &system_prompt,
                                    &user_prompt,
                                    json_mode,
                                    max_tokens,
                                    temperature,
                                    timeout_ms,
                                ))
                            }
                            "ollama" => {
                                let base = self
                                    .ai_base_url
                                    .clone()
                                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                                let model = self
                                    .ai_model
                                    .clone()
                                    .unwrap_or_else(|| "llama3.2".to_string());
                                let client = OllamaClient {
                                    base_url: base,
                                    model,
                                };
                                runtime.block_on(client.suggest_commit(
                                    &system_prompt,
                                    &user_prompt,
                                    json_mode,
                                    max_tokens,
                                    temperature,
                                    timeout_ms,
                                ))
                            }
                            other => {
                                errors.push(format!("unknown ai provider: {other}"));
                                continue;
                            }
                        };

                        match result {
                            Ok(text) => {
                                let candidate = if json_mode {
                                    match serde_json::from_str::<AiCommitSuggestion>(text.trim()) {
                                        Ok(sug) => build_message_from_suggestion(
                                            &sug,
                                            &g.commit_type,
                                            default_short_for(&g.name),
                                        ),
                                        Err(e) => {
                                            errors.push(format!("AI JSON parse failed: {e}"));
                                            g.suggested_message.clone()
                                        }
                                    }
                                } else {
                                    // Treat the first non-empty line as the commit header
                                    text.lines()
                                        .find(|l| !l.trim().is_empty())
                                        .unwrap_or(g.suggested_message.as_str())
                                        .trim()
                                        .to_string()
                                };
                                // Lint and fallback
                                let issues = check_message_format_for_repo(repo_path, &candidate)
                                    .map_err(|e| CliError::Generic(e.to_string()))?;
                                if issues.is_empty() {
                                    g.suggested_message = candidate;
                                } else {
                                    errors.push(format!("AI suggestion failed lint: {issues:?}"));
                                }
                            }
                            Err(e) => {
                                errors.push(format!("AI error: {e}"));
                            }
                        }
                    }
                }

                validate_group_messages(repo_path, &mut groups, &mut errors)
                    .map_err(|e| CliError::Generic(e.to_string()))?;

                let res = GroupCommitPlanResult {
                    api_version: API_VERSION,
                    command: "group-commit".into(),
                    mode: "plan".into(),
                    ok: errors.is_empty(),
                    dry_run: true,
                    groups,
                    errors: if errors.is_empty() {
                        None
                    } else {
                        Some(errors)
                    },
                };
                if self.output == "json" {
                    println!("{}", serde_json::to_string(&res).unwrap());
                } else {
                    println!("Planned {} group(s)", res.groups.len());
                }
                Ok(())
            }
            "apply" => {
                if self.push && !self.confirm_push {
                    return Err(CliError::InputError(
                        "Remote push requires --confirm-push".to_string(),
                    ));
                }
                let repo = discover_repository_from(&self.repo_path)?;
                let repo_path = repo.workdir().ok_or_else(|| {
                    CliError::GitError(git2::Error::from_str("No working directory"))
                })?;
                let git_command_config =
                    crate::git::resolve_git_command_config(repo_path, &self.git_config)?;
                // Build groups as in plan
                let files = list_changed_files_from(&self.repo_path, self.include_unstaged)?;
                let mut groups = build_groups(files);

                let mut errors: Vec<String> = Vec::new();

                // Optionally enhance messages via AI
                if self.ai {
                    // Build system prompt
                    let mut system_prompt = self
                        .ai_system_prompt
                        .clone()
                        .unwrap_or_else(|| {
                            "You are a commit message assistant. Generate conventional commit messages. Prefer JSON if requested.\nReturn an object {\"commit_type\", \"short\", \"scope\", \"long\", \"message\"}. If 'message' is present, it should be a full commit message with the first line formatted as '<type>(<scope>): <short>' (scope optional).".to_string()
                        });
                    if let Some(path) = &self.ai_system_prompt_file {
                        if let Ok(fp) = fs::read_to_string(path) {
                            system_prompt = fp;
                        } else {
                            errors.push(format!("failed to read ai_system_prompt_file: {path}"));
                        }
                    }

                    let json_mode = !self.no_ai_json_mode;
                    let max_tokens = self.ai_max_tokens;
                    let temperature = self.ai_temperature;
                    let timeout_ms = self.ai_timeout_ms;
                    let provider = self.ai_provider.as_str();
                    let runtime = tokio::runtime::Runtime::new()
                        .map_err(|e| CliError::Generic(e.to_string()))?;

                    for g in groups.iter_mut() {
                        let user_prompt = ai_user_prompt(
                            &g.name,
                            &g.commit_type,
                            &g.files,
                            self.ai_allow_sensitive,
                            self.ai_file_limit,
                        );

                        let result: Result<String, LlmError> = match provider {
                            "openrouter" => {
                                let base = self
                                    .ai_base_url
                                    .clone()
                                    .unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
                                let model = self
                                    .ai_model
                                    .clone()
                                    .unwrap_or_else(|| "openrouter/auto".to_string());
                                let key_env = self.ai_api_key_env.clone();
                                let api_key = env::var(key_env).ok();
                                let client = OpenRouterClient {
                                    base_url: base,
                                    api_key,
                                    model,
                                };
                                runtime.block_on(client.suggest_commit(
                                    &system_prompt,
                                    &user_prompt,
                                    json_mode,
                                    max_tokens,
                                    temperature,
                                    timeout_ms,
                                ))
                            }
                            "ollama" => {
                                let base = self
                                    .ai_base_url
                                    .clone()
                                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                                let model = self
                                    .ai_model
                                    .clone()
                                    .unwrap_or_else(|| "llama3.2".to_string());
                                let client = OllamaClient {
                                    base_url: base,
                                    model,
                                };
                                runtime.block_on(client.suggest_commit(
                                    &system_prompt,
                                    &user_prompt,
                                    json_mode,
                                    max_tokens,
                                    temperature,
                                    timeout_ms,
                                ))
                            }
                            other => {
                                errors.push(format!("unknown ai provider: {other}"));
                                continue;
                            }
                        };

                        match result {
                            Ok(text) => {
                                let candidate = if json_mode {
                                    match serde_json::from_str::<AiCommitSuggestion>(text.trim()) {
                                        Ok(sug) => build_message_from_suggestion(
                                            &sug,
                                            &g.commit_type,
                                            default_short_for(&g.name),
                                        ),
                                        Err(e) => {
                                            errors.push(format!("AI JSON parse failed: {e}"));
                                            g.suggested_message.clone()
                                        }
                                    }
                                } else {
                                    text.lines()
                                        .find(|l| !l.trim().is_empty())
                                        .unwrap_or(g.suggested_message.as_str())
                                        .trim()
                                        .to_string()
                                };
                                let issues = check_message_format_for_repo(repo_path, &candidate)
                                    .map_err(|e| CliError::Generic(e.to_string()))?;
                                if issues.is_empty() {
                                    g.suggested_message = candidate;
                                } else {
                                    errors.push(format!("AI suggestion failed lint: {issues:?}"));
                                }
                            }
                            Err(e) => {
                                errors.push(format!("AI error: {e}"));
                            }
                        }
                    }
                }

                fn last_commit_sha(repo_path: &Path) -> Option<String> {
                    if let Ok(repo) = Repository::discover(repo_path) {
                        if let Ok(head) = repo.head() {
                            if let Ok(commit) = head.peel_to_commit() {
                                return Some(commit.id().to_string());
                            }
                        }
                    }
                    None
                }

                let mut commits: Vec<CommitRecord> = Vec::new();

                // Commit per group
                for g in &mut groups {
                    let final_msg = g.suggested_message.trim().to_string();
                    let issues = check_message_format_for_repo(repo_path, &final_msg)
                        .map_err(|e| CliError::Generic(e.to_string()))?;
                    if !issues.is_empty() {
                        g.issues = Some(issues.clone());
                        let error_message = format!(
                            "group {} message failed commit rules: {}",
                            group_name_str(g.name),
                            issues.join("; ")
                        );
                        errors.push(error_message.clone());
                        commits.push(CommitRecord {
                            group: g.name,
                            message: final_msg,
                            ok: false,
                            sha: None,
                            error: Some(error_message),
                        });
                        continue;
                    }
                    g.issues = None;

                    // Stage only this group's files if requested
                    if self.auto_stage {
                        // Unstage everything back to HEAD, then stage only the group's files
                        if let Err(e) = crate::git::run_git(
                            repo_path,
                            &["reset", "-q", "HEAD", "--"],
                            "reset staged changes",
                            &git_command_config,
                        ) {
                            errors.push(format!(
                                "git reset failed before staging {}: {}",
                                group_name_str(g.name),
                                e
                            ));
                        }
                        // Add files
                        let mut args: Vec<&str> = vec!["add", "--"];
                        for f in &g.files {
                            args.push(f.as_str());
                        }
                        if let Err(e) = crate::git::run_git(
                            repo_path,
                            &args,
                            "stage group files",
                            &git_command_config,
                        ) {
                            errors.push(format!(
                                "git add failed for group {}: {}",
                                group_name_str(g.name),
                                e
                            ));
                            commits.push(CommitRecord {
                                group: g.name,
                                message: final_msg.clone(),
                                ok: false,
                                sha: None,
                                error: Some("failed to stage files".into()),
                            });
                            continue;
                        }
                    }

                    // Create commit
                    match crate::git::commit_changes_in_with_config(
                        repo_path,
                        &final_msg,
                        false,
                        &git_command_config,
                    ) {
                        Ok(_) => {
                            let sha = last_commit_sha(repo_path);
                            commits.push(CommitRecord {
                                group: g.name,
                                message: final_msg.clone(),
                                ok: true,
                                sha,
                                error: None,
                            });
                        }
                        Err(e) => {
                            errors.push(format!(
                                "commit failed for group {}: {}",
                                group_name_str(g.name),
                                e
                            ));
                            commits.push(CommitRecord {
                                group: g.name,
                                message: final_msg.clone(),
                                ok: false,
                                sha: None,
                                error: Some(e.to_string()),
                            });
                        }
                    }
                }

                // Optional push
                let mut pushed: Option<bool> = None;
                if self.push {
                    let push_ok = crate::git::run_git(
                        repo_path,
                        &["push"],
                        "push grouped commits",
                        &git_command_config,
                    )
                    .is_ok();
                    if !push_ok {
                        errors.push("git push failed".to_string());
                    }
                    pushed = Some(push_ok);
                }

                let ok = commits.iter().all(|c| c.ok) && errors.is_empty();
                let res = GroupCommitApplyResult {
                    api_version: API_VERSION,
                    command: "group-commit".into(),
                    mode: "apply".into(),
                    ok,
                    dry_run: false,
                    groups: groups.clone(),
                    commits,
                    pushed,
                    errors: if errors.is_empty() {
                        None
                    } else {
                        Some(errors)
                    },
                };
                if self.output == "json" {
                    println!("{}", serde_json::to_string(&res).unwrap());
                } else {
                    println!("Applied group commits");
                }
                Ok(())
            }
            _ => Err(CliError::Generic("invalid mode".into())),
        }
    }

    fn machine_context(&self) -> Option<MachineContext> {
        (self.output == "json").then_some(MachineContext {
            command: "group-commit",
            dry_run: self.mode == "plan",
        })
    }
}

/// Summarise a change set without naming any file.
///
/// The default (non-sensitive) AI path used to send only the group name, which
/// gave the model nothing to improve on. This sends *shape* — how many files,
/// which extensions, which top-level areas — and never a filename, a path
/// below the first segment, or any file content.
fn redacted_shape_summary(files: &[String]) -> String {
    use std::collections::BTreeMap;

    let mut extensions: BTreeMap<String, usize> = BTreeMap::new();
    let mut areas: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    for file in files {
        let trimmed = file.trim_start_matches("./");
        let extension = std::path::Path::new(trimmed)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_else(|| "(no extension)".to_string());
        *extensions.entry(extension).or_insert(0) += 1;

        // First path segment only: "src", "docs", "tests". Anything deeper can
        // carry product or customer names.
        let area = trimmed
            .split('/')
            .next()
            .filter(|segment| !segment.is_empty() && trimmed.contains('/'))
            .unwrap_or("(repository root)");
        areas.insert(area.to_string());
    }

    let extension_list = extensions
        .iter()
        .map(|(extension, count)| format!("{extension} x{count}"))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "{} file(s); types: {}; areas: {}",
        files.len(),
        extension_list,
        areas.into_iter().collect::<Vec<_>>().join(", ")
    )
}

/// The per-group user prompt sent to the model.
///
/// `allow_sensitive` is the only switch that lets real paths leave the machine.
fn ai_user_prompt(
    name: &GroupName,
    commit_type: &str,
    files: &[String],
    allow_sensitive: bool,
    file_limit: usize,
) -> String {
    let header = format!(
        "Group: {}\nDefault type: {}\nDefault short: {}",
        group_name_str(*name),
        commit_type,
        default_short_for(name)
    );

    if allow_sensitive {
        let mut listed: Vec<String> = files.iter().take(file_limit).cloned().collect();
        if files.len() > file_limit {
            listed.push("...".to_string());
        }
        format!(
            "{header}\nFiles (truncated):\n- {}\nReturn a JSON object with fields: \
             commit_type, short, scope, long, message.",
            listed.join("\n- ")
        )
    } else {
        format!(
            "{header}\nChange shape (no filenames or content): {}\nSuggest a better short \
             description for this group. Return a JSON object with fields: commit_type, \
             short, scope, long, message.",
            redacted_shape_summary(files)
        )
    }
}

/// Classify a change set into commit groups.
///
/// Shared by `plan` and `apply`; they previously carried byte-identical copies.
fn build_groups(files: Vec<String>) -> Vec<PlanGroup> {
    let mut by_group: std::collections::BTreeMap<GroupName, Vec<String>> = [
        (GroupName::Docs, vec![]),
        (GroupName::Tests, vec![]),
        (GroupName::Ci, vec![]),
        (GroupName::Deps, vec![]),
        (GroupName::Build, vec![]),
        (GroupName::Chore, vec![]),
        (GroupName::Code, vec![]),
    ]
    .into_iter()
    .collect();

    for file in files {
        let group = classify_file(&file);
        if let Some(bucket) = by_group.get_mut(&group) {
            bucket.push(file);
        }
    }

    by_group
        .into_iter()
        .filter(|(_, files)| !files.is_empty())
        .map(|(name, files)| {
            let commit_type = default_type_for(&name).to_string();
            let short = default_short_for(&name).to_string();
            let suggested_message = format_commit_message(&commit_type, false, "", &short, "");
            PlanGroup {
                name,
                commit_type,
                files,
                suggested_message,
                issues: None,
            }
        })
        .collect()
}

fn validate_group_messages(
    repo_path: &Path,
    groups: &mut [PlanGroup],
    errors: &mut Vec<String>,
) -> Result<(), CliError> {
    for group in groups.iter_mut() {
        let issues = check_message_format_for_repo(repo_path, &group.suggested_message)
            .map_err(|e| CliError::Generic(e.to_string()))?;
        if issues.is_empty() {
            group.issues = None;
            continue;
        }

        group.issues = Some(issues.clone());
        errors.push(format!(
            "group {} message failed commit rules: {}",
            group_name_str(group.name),
            issues.join("; ")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> Vec<String> {
        vec![
            "src/cli/commands/secret_feature.rs".to_string(),
            "src/cli/commands/other.rs".to_string(),
            "docs/internal/roadmap.md".to_string(),
        ]
    }

    #[test]
    fn shape_summary_describes_without_naming() {
        let summary = redacted_shape_summary(&files());

        assert!(
            summary.contains('3'),
            "must report the file count: {summary}"
        );
        assert!(summary.contains(".rs"), "must report extensions: {summary}");
        assert!(summary.contains(".md"), "must report extensions: {summary}");
        assert!(
            summary.contains("src"),
            "must report top-level areas: {summary}"
        );
        assert!(
            summary.contains("docs"),
            "must report top-level areas: {summary}"
        );

        for leaked in ["secret_feature", "other.rs", "roadmap.md", "internal"] {
            assert!(
                !summary.contains(leaked),
                "shape summary leaked `{leaked}`: {summary}"
            );
        }
    }

    #[test]
    fn default_prompt_carries_shape_but_no_paths() {
        let prompt = ai_user_prompt(&GroupName::Code, "feat", &files(), false, 20);

        assert!(
            prompt.contains(&redacted_shape_summary(&files())),
            "the safe prompt must include the redacted shape: {prompt}"
        );
        for leaked in ["secret_feature", "roadmap.md", "src/cli/commands"] {
            assert!(!prompt.contains(leaked), "safe prompt leaked `{leaked}`");
        }
    }

    #[test]
    fn sensitive_prompt_lists_paths_and_respects_the_limit() {
        let prompt = ai_user_prompt(&GroupName::Code, "feat", &files(), true, 2);

        assert!(prompt.contains("src/cli/commands/secret_feature.rs"));
        assert!(
            prompt.contains("..."),
            "over-limit file lists must be elided: {prompt}"
        );
        assert!(
            !prompt.contains("docs/internal/roadmap.md"),
            "third path exceeds the limit of 2"
        );
    }

    #[test]
    fn build_groups_is_shared_between_plan_and_apply() {
        let groups = build_groups(files());

        let names: Vec<GroupName> = groups.iter().map(|g| g.name).collect();
        assert!(names.contains(&GroupName::Code));
        assert!(names.contains(&GroupName::Docs));
        assert!(
            groups.iter().all(|g| !g.files.is_empty()),
            "empty groups must be dropped"
        );
        assert!(groups.iter().all(|g| !g.suggested_message.is_empty()));
    }
}
