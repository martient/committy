use structopt::StructOpt;
use serde::Serialize;
use crate::cli::Command;
use crate::error::CliError;
use crate::git::format_commit_message;
use crate::git::list_changed_files;
use crate::linter::check_message_format;
use crate::ai::{LlmClient, OpenRouterClient, OllamaClient, AiCommitSuggestion, LlmError};
use std::env;
use std::fs;
use std::process::Command as ProcCommand;
use git2::Repository;

#[derive(Debug, Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum GroupName { Docs, Tests, Ci, Deps, Build, Chore, Code }

#[derive(Debug, Serialize, Clone)]
pub struct PlanGroup {
    pub name: GroupName,
    pub commit_type: String,
    pub files: Vec<String>,
    pub suggested_message: String,
}

#[derive(Debug, Serialize)]
pub struct GroupCommitPlanResult {
    pub command: String,
    pub mode: String,
    pub ok: bool,
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
    pub command: String,
    pub mode: String,
    pub ok: bool,
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

    /// Diff lines per file sent to AI
    #[structopt(long = "ai-diff-lines-per-file", default_value = "80")]
    ai_diff_lines_per_file: usize,

    /// Allow sending sensitive content to external AI providers
    #[structopt(long = "ai-allow-sensitive")]
    ai_allow_sensitive: bool,
}

impl Default for GroupCommitCommand {
    fn default() -> Self {
        GroupCommitCommand {
            mode: "plan".into(),
            include_unstaged: false,
            auto_stage: false,
            push: false,
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
            ai_diff_lines_per_file: 80,
            ai_allow_sensitive: false,
        }
    }
}

fn classify_file(file: &str) -> GroupName {
    let f = file.trim_start_matches("./");
    // CI
    if f.starts_with(".github/") { return GroupName::Ci; }
    // Docs
    if f.starts_with("docs/") || f.ends_with("README.md") || f.ends_with("README.MD") || f.ends_with(".md") || f.ends_with(".MD") || f.ends_with(".mdx") || f.ends_with(".MDX") {
        return GroupName::Docs;
    }
    // Tests
    if f.starts_with("tests/") || f.ends_with("_test.rs") || f.ends_with(".test.js") || f.ends_with(".test.ts") || f.ends_with(".spec.js") || f.ends_with(".spec.ts") {
        return GroupName::Tests;
    }
    // Deps (lockfiles)
    if f.ends_with("package-lock.json") || f.ends_with("npm-shrinkwrap.json") || f.ends_with("pnpm-lock.yaml") || f.ends_with("yarn.lock") || f.ends_with("Cargo.lock") {
        return GroupName::Deps;
    }
    // Build/config
    if f.ends_with("Cargo.toml") || f.ends_with("build.rs") || f.ends_with("package.json") || f.ends_with("tsconfig.json") || f.contains("eslint.") || f.ends_with(".eslintrc") || f.contains("vite.config") || f.ends_with("rollup.config.js") || f.ends_with("rollup.config.cjs") || f.ends_with("rollup.config.mjs") {
        return GroupName::Build;
    }
    // Chore (editor/config meta)
    if f.starts_with(".vscode/") || f.ends_with(".editorconfig") || f.ends_with(".gitignore") || f.ends_with(".npmrc") {
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

fn build_message_from_suggestion(s: &AiCommitSuggestion, fallback_type: &str, fallback_short: &str) -> String {
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
                let files = list_changed_files(self.include_unstaged)?;
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

                for f in files {
                    let g = classify_file(&f);
                    if let Some(v) = by_group.get_mut(&g) {
                        v.push(f);
                    }
                }

                let mut groups: Vec<PlanGroup> = Vec::new();
                for (name, files) in by_group.into_iter() {
                    if files.is_empty() {
                        continue;
                    }
                    let commit_type = default_type_for(&name).to_string();
                    let short = default_short_for(&name).to_string();
                    let message = format_commit_message(&commit_type, false, "", &short, "");
                    groups.push(PlanGroup {
                        name,
                        commit_type,
                        files,
                        suggested_message: message,
                    });
                }

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
                            errors.push(format!("failed to read ai_system_prompt_file: {}", path));
                        }
                    }

                    // Determine provider and client
                    let json_mode = !self.no_ai_json_mode;
                    let max_tokens = self.ai_max_tokens;
                    let temperature = self.ai_temperature;
                    let timeout_ms = self.ai_timeout_ms;

                    let provider = self.ai_provider.as_str();
                    let runtime = tokio::runtime::Runtime::new().map_err(|e| CliError::Generic(e.to_string()))?;

                    for g in groups.iter_mut() {
                        let files_preview: Vec<String> = if self.ai_allow_sensitive {
                            let mut v = g.files.iter().take(self.ai_file_limit).cloned().collect::<Vec<_>>();
                            if g.files.len() > self.ai_file_limit { v.push("...".to_string()); }
                            v
                        } else {
                            vec![]
                        };
                        let user_prompt = if self.ai_allow_sensitive {
                            format!(
                                "Group: {}\nDefault type: {}\nDefault short: {}\nFiles (truncated):\n- {}\nReturn a JSON object with fields: commit_type, short, scope, long, message.",
                                group_name_str(g.name),
                                g.commit_type,
                                default_short_for(&g.name),
                                files_preview.join("\n- ")
                            )
                        } else {
                            format!(
                                "Group: {}\nDefault type: {}\nDefault short: {}\nWithout revealing code or filenames, suggest a better short description if needed. Return JSON.",
                                group_name_str(g.name),
                                g.commit_type,
                                default_short_for(&g.name)
                            )
                        };

                        // Prepare client per provider
                        let result: Result<String, LlmError> = match provider {
                            "openrouter" => {
                                let base = self.ai_base_url.clone().unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
                                let model = self.ai_model.clone().unwrap_or_else(|| "openrouter/auto".to_string());
                                let key_env = self.ai_api_key_env.clone();
                                let api_key = env::var(key_env).ok();
                                let client = OpenRouterClient { base_url: base, api_key, model };
                                runtime.block_on(client.suggest_commit(&system_prompt, &user_prompt, json_mode, max_tokens, temperature, timeout_ms))
                            }
                            "ollama" => {
                                let base = self.ai_base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
                                let model = self.ai_model.clone().unwrap_or_else(|| "llama3.2".to_string());
                                let client = OllamaClient { base_url: base, model };
                                runtime.block_on(client.suggest_commit(&system_prompt, &user_prompt, json_mode, max_tokens, temperature, timeout_ms))
                            }
                            other => {
                                errors.push(format!("unknown ai provider: {}", other));
                                continue;
                            }
                        };

                        match result {
                            Ok(text) => {
                                let candidate = if json_mode {
                                    match serde_json::from_str::<AiCommitSuggestion>(text.trim()) {
                                        Ok(sug) => build_message_from_suggestion(&sug, &g.commit_type, default_short_for(&g.name)),
                                        Err(e) => {
                                            errors.push(format!("AI JSON parse failed: {}", e));
                                            g.suggested_message.clone()
                                        }
                                    }
                                } else {
                                    // Treat the first non-empty line as the commit header
                                    text.lines().find(|l| !l.trim().is_empty()).unwrap_or(g.suggested_message.as_str()).trim().to_string()
                                };
                                // Lint and fallback
                                let issues = check_message_format(&candidate);
                                if issues.is_empty() {
                                    g.suggested_message = candidate;
                                } else {
                                    errors.push(format!("AI suggestion failed lint: {:?}", issues));
                                }
                            }
                            Err(e) => {
                                errors.push(format!("AI error: {}", e));
                            }
                        }
                    }
                }

                let res = GroupCommitPlanResult { command: "group-commit".into(), mode: "plan".into(), ok: true, groups, errors: if errors.is_empty() { None } else { Some(errors) } };
                if self.output == "json" {
                    println!("{}", serde_json::to_string(&res).unwrap());
                } else {
                    println!("Planned {} group(s)", res.groups.len());
                }
                Ok(())
            }
            "apply" => {
                // Build groups as in plan
                let files = list_changed_files(self.include_unstaged)?;
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

                for f in files {
                    let g = classify_file(&f);
                    if let Some(v) = by_group.get_mut(&g) { v.push(f); }
                }

                let mut groups: Vec<PlanGroup> = Vec::new();
                for (name, files) in by_group.into_iter() {
                    if files.is_empty() { continue; }
                    let commit_type = default_type_for(&name).to_string();
                    let short = default_short_for(&name).to_string();
                    let message = format_commit_message(&commit_type, false, "", &short, "");
                    groups.push(PlanGroup { name, commit_type, files, suggested_message: message });
                }

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
                        if let Ok(fp) = fs::read_to_string(path) { system_prompt = fp; } else { errors.push(format!("failed to read ai_system_prompt_file: {}", path)); }
                    }

                    let json_mode = !self.no_ai_json_mode;
                    let max_tokens = self.ai_max_tokens;
                    let temperature = self.ai_temperature;
                    let timeout_ms = self.ai_timeout_ms;
                    let provider = self.ai_provider.as_str();
                    let runtime = tokio::runtime::Runtime::new().map_err(|e| CliError::Generic(e.to_string()))?;

                    for g in groups.iter_mut() {
                        let files_preview: Vec<String> = if self.ai_allow_sensitive {
                            let mut v = g.files.iter().take(self.ai_file_limit).cloned().collect::<Vec<_>>();
                            if g.files.len() > self.ai_file_limit { v.push("...".to_string()); }
                            v
                        } else { vec![] };

                        let user_prompt = if self.ai_allow_sensitive {
                            format!(
                                "Group: {}\nDefault type: {}\nDefault short: {}\nFiles (truncated):\n- {}\nReturn a JSON object with fields: commit_type, short, scope, long, message.",
                                group_name_str(g.name), g.commit_type, default_short_for(&g.name), files_preview.join("\n- ")
                            )
                        } else {
                            format!(
                                "Group: {}\nDefault type: {}\nDefault short: {}\nWithout revealing code or filenames, suggest a better short description if needed. Return JSON.",
                                group_name_str(g.name), g.commit_type, default_short_for(&g.name)
                            )
                        };

                        let result: Result<String, LlmError> = match provider {
                            "openrouter" => {
                                let base = self.ai_base_url.clone().unwrap_or_else(|| "https://openrouter.ai/api/v1".to_string());
                                let model = self.ai_model.clone().unwrap_or_else(|| "openrouter/auto".to_string());
                                let key_env = self.ai_api_key_env.clone();
                                let api_key = env::var(key_env).ok();
                                let client = OpenRouterClient { base_url: base, api_key, model };
                                runtime.block_on(client.suggest_commit(&system_prompt, &user_prompt, json_mode, max_tokens, temperature, timeout_ms))
                            }
                            "ollama" => {
                                let base = self.ai_base_url.clone().unwrap_or_else(|| "http://localhost:11434".to_string());
                                let model = self.ai_model.clone().unwrap_or_else(|| "llama3.2".to_string());
                                let client = OllamaClient { base_url: base, model };
                                runtime.block_on(client.suggest_commit(&system_prompt, &user_prompt, json_mode, max_tokens, temperature, timeout_ms))
                            }
                            other => { errors.push(format!("unknown ai provider: {}", other)); continue; }
                        };

                        match result {
                            Ok(text) => {
                                let candidate = if json_mode {
                                    match serde_json::from_str::<AiCommitSuggestion>(text.trim()) {
                                        Ok(sug) => build_message_from_suggestion(&sug, &g.commit_type, default_short_for(&g.name)),
                                        Err(e) => { errors.push(format!("AI JSON parse failed: {}", e)); g.suggested_message.clone() }
                                    }
                                } else {
                                    text.lines().find(|l| !l.trim().is_empty()).unwrap_or(g.suggested_message.as_str()).trim().to_string()
                                };
                                let issues = check_message_format(&candidate);
                                if issues.is_empty() { g.suggested_message = candidate; } else { errors.push(format!("AI suggestion failed lint: {:?}", issues)); }
                            }
                            Err(e) => { errors.push(format!("AI error: {}", e)); }
                        }
                    }
                }

                // Helper: quietly run `git` command
                fn run_git(args: &[&str]) -> Result<(), CliError> {
                    let mut cmd = ProcCommand::new("git");
                    cmd.args(args);
                    let status = cmd.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().map_err(|e| CliError::Generic(e.to_string()))?;
                    if status.success() { Ok(()) } else { Err(CliError::Generic(format!("git {:?} failed with status {:?}", args, status))) }
                }

                fn last_commit_sha() -> Option<String> {
                    if let Ok(repo) = Repository::discover(std::env::current_dir().ok()?) {
                        if let Ok(head) = repo.head() {
                            if let Ok(commit) = head.peel_to_commit() { return Some(commit.id().to_string()); }
                        }
                    }
                    None
                }

                let mut commits: Vec<CommitRecord> = Vec::new();

                // Commit per group
                for g in &groups {
                    // Validate message again and fallback to default formatting
                    let candidate = g.suggested_message.trim().to_string();
                    let final_msg = if check_message_format(&candidate).is_empty() {
                        candidate
                    } else {
                        // Rebuild from defaults
                        let short = default_short_for(&g.name);
                        format_commit_message(&g.commit_type, false, "", short, "")
                    };

                    // Stage only this group's files if requested
                    if self.auto_stage {
                        // Unstage everything back to HEAD, then stage only the group's files
                        if let Err(e) = run_git(&["reset", "-q", "HEAD", "--"]) { errors.push(format!("git reset failed before staging {}: {}", group_name_str(g.name), e)); }
                        // Add files
                        let mut args: Vec<&str> = vec!["add", "--"];
                        for f in &g.files { args.push(f.as_str()); }
                        if let Err(e) = run_git(&args) { errors.push(format!("git add failed for group {}: {}", group_name_str(g.name), e)); commits.push(CommitRecord { group: g.name, message: final_msg.clone(), ok: false, sha: None, error: Some("failed to stage files".into()) }); continue; }
                    }

                    // Create commit
                    match crate::git::commit_changes(&final_msg, false) {
                        Ok(_) => {
                            let sha = last_commit_sha();
                            commits.push(CommitRecord { group: g.name, message: final_msg.clone(), ok: true, sha, error: None });
                        }
                        Err(e) => {
                            errors.push(format!("commit failed for group {}: {}", group_name_str(g.name), e));
                            commits.push(CommitRecord { group: g.name, message: final_msg.clone(), ok: false, sha: None, error: Some(e.to_string()) });
                        }
                    }
                }

                // Optional push
                let mut pushed: Option<bool> = None;
                if self.push {
                    pushed = Some(run_git(&["push"]).is_ok());
                }

                let ok = commits.iter().all(|c| c.ok);
                let res = GroupCommitApplyResult { command: "group-commit".into(), mode: "apply".into(), ok, groups: groups.clone(), commits, pushed, errors: if errors.is_empty() { None } else { Some(errors) } };
                if self.output == "json" { println!("{}", serde_json::to_string(&res).unwrap()); } else { println!("Applied group commits"); }
                Ok(())
            }
            _ => Err(CliError::Generic("invalid mode".into())),
        }
    }
}
