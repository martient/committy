use std::collections::HashMap;

use crate::cli::output::{MachineContext, API_VERSION};
use crate::cli::Command;
use crate::config::hierarchy::MergedConfig;
use crate::config::repository::BranchRulesConfig;
use crate::convention::Convention;
use crate::error::CliError;
use crate::git;
use crate::input;
use crate::input::validation::validate_section;
use crate::telemetry;
use log::debug;
use log::info;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct BranchCommand {
    #[structopt(short, long, help = "Name of the branch to create")]
    name: Option<String>,

    #[structopt(long = "type", help = "Structured branch type (e.g., feat, fix, docs)")]
    branch_type: Option<String>,

    #[structopt(long, help = "Structured ticket identifier")]
    ticket: Option<String>,

    #[structopt(long, help = "Structured branch subject")]
    subject: Option<String>,

    #[structopt(short, long, help = "Force create branch")]
    force: bool,

    #[structopt(short, long, help = "Validate branch name")]
    validate: bool,

    #[structopt(long, help = "Preview the branch operation without creating it")]
    dry_run: bool,

    #[structopt(long, default_value = "text", possible_values = &["text", "json"])]
    output: String,

    #[structopt(long, default_value = ".", parse(from_os_str))]
    repo_path: PathBuf,
}

#[derive(Debug, Serialize)]
struct BranchCommandOutput {
    api_version: u8,
    command: String,
    ok: bool,
    dry_run: bool,
    branch_name: String,
    branch_type: String,
    ticket: String,
    subject: String,
    would_create: bool,
    would_checkout: bool,
    errors: Option<Vec<String>>,
}

impl Default for BranchCommand {
    fn default() -> Self {
        Self {
            name: None,
            branch_type: None,
            ticket: None,
            subject: None,
            force: false,
            validate: false,
            dry_run: false,
            output: "text".to_string(),
            repo_path: PathBuf::from("."),
        }
    }
}

struct BranchPlan {
    branch_name: String,
    branch_type: String,
    ticket: String,
    subject: String,
    would_checkout: bool,
}

impl Command for BranchCommand {
    fn execute(&self, non_interactive: bool) -> Result<(), CliError> {
        let repo = git::discover_repository_from(&self.repo_path)?;
        let repo_path = repo
            .workdir()
            .ok_or_else(|| CliError::GitError(git2::Error::from_str("No working directory")))?;
        let convention = Convention::load(repo_path)?;
        let mut allowed_types = convention.allowed_branch_types();
        if allowed_types.is_empty() {
            allowed_types = Convention::from_config(Default::default())?.allowed_branch_types();
        }
        let merged = MergedConfig::load(repo_path).map_err(|e| CliError::Generic(e.to_string()))?;
        let rules = merged
            .repository_config()
            .map(|config| config.branch_rules.clone())
            .unwrap_or_default();

        let Some(plan) = self.build_plan(non_interactive, &allowed_types, &rules)? else {
            return Ok(());
        };
        git::validate_branch_name(&plan.branch_name)?;

        let mut errors = Vec::new();
        if git::branch_exists_in(&self.repo_path, &plan.branch_name)? && !self.force {
            errors.push(format!(
                "Branch '{}' already exists. Use --force to recreate it.",
                plan.branch_name
            ));
        }

        let output = BranchCommandOutput {
            api_version: API_VERSION,
            command: "branch".into(),
            ok: errors.is_empty(),
            dry_run: self.dry_run,
            branch_name: plan.branch_name.clone(),
            branch_type: plan.branch_type.clone(),
            ticket: plan.ticket.clone(),
            subject: plan.subject.clone(),
            would_create: errors.is_empty(),
            would_checkout: errors.is_empty() && plan.would_checkout,
            errors: if errors.is_empty() {
                None
            } else {
                Some(errors.clone())
            },
        };

        if !errors.is_empty() {
            return Err(CliError::InputError(errors.join(" ")));
        }

        if self.dry_run {
            self.print_output(&output);
            return Ok(());
        }

        git::create_branch_in(&self.repo_path, &plan.branch_name, self.force)?;
        if plan.would_checkout {
            git::checkout_branch_in(&self.repo_path, &plan.branch_name)?;
        }

        self.print_output(&output);

        if self.name.is_none() {
            if let Err(e) =
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(telemetry::posthog::publish_event(
                        "branch_created",
                        HashMap::from([
                            ("branch_type", Value::from(plan.branch_type.as_str())),
                            (
                                "as_ticket",
                                Value::from((!plan.ticket.is_empty()).to_string()),
                            ),
                            ("len_ticket", Value::from(plan.ticket.len())),
                            (
                                "as_subject",
                                Value::from((!plan.subject.is_empty()).to_string()),
                            ),
                            ("len_subject", Value::from(plan.subject.len())),
                        ]),
                    ))
            {
                debug!("Telemetry error: {e:?}");
            }
        }

        Ok(())
    }

    fn machine_context(&self) -> Option<MachineContext> {
        (self.output == "json").then_some(MachineContext {
            command: "branch",
            dry_run: self.dry_run,
        })
    }
}

impl BranchCommand {
    fn build_plan(
        &self,
        non_interactive: bool,
        allowed_types: &[String],
        rules: &BranchRulesConfig,
    ) -> Result<Option<BranchPlan>, CliError> {
        if self.name.is_some()
            && (self.branch_type.is_some() || self.ticket.is_some() || self.subject.is_some())
        {
            return Err(CliError::InputError(
                "Use either --name or structured branch flags (--type/--ticket/--subject), not both"
                    .to_string(),
            ));
        }

        if let Some(name) = &self.name {
            let (branch_type, ticket, subject) = parse_branch_name(name, rules)?;
            if rules.enforce_explicit_names {
                validate_branch_type(&branch_type, allowed_types)?;
                validate_ticket(&ticket, rules)?;
                if subject.is_empty() {
                    return Err(CliError::InputError(
                        "Explicit branch name must include a subject".to_string(),
                    ));
                }
            }
            return Ok(Some(BranchPlan {
                branch_name: name.clone(),
                branch_type,
                ticket,
                subject,
                would_checkout: false,
            }));
        }

        if self.branch_type.is_some() || self.ticket.is_some() || self.subject.is_some() {
            return self.build_structured_plan(non_interactive, allowed_types, rules);
        }

        if non_interactive {
            return Err(CliError::InputError(
                "Branch name is required in non-interactive mode. Use --name or --type with --subject."
                    .to_string(),
            ));
        }

        let branch_type = input::select_branch_type_from(allowed_types)?;
        let ticket = input::input_ticket()?;
        validate_ticket(&ticket, rules)?;
        let subject = input::input_subject()?;

        let branch_name = if ticket.is_empty() {
            format!("{branch_type}-{subject}")
        } else {
            format!("{branch_type}-{ticket}-{subject}")
        };

        let validate = if !self.validate && !self.dry_run {
            input::ask_want_create_new_branch(&branch_name)?
        } else {
            true
        };
        if !validate {
            info!("Abort");
            return Ok(None);
        }

        Ok(Some(BranchPlan {
            branch_name,
            branch_type,
            ticket,
            subject,
            would_checkout: true,
        }))
    }

    fn build_structured_plan(
        &self,
        non_interactive: bool,
        allowed_types: &[String],
        rules: &BranchRulesConfig,
    ) -> Result<Option<BranchPlan>, CliError> {
        let branch_type = match &self.branch_type {
            Some(branch_type) => validate_branch_type(branch_type, allowed_types)?,
            None if !non_interactive => input::select_branch_type_from(allowed_types)?,
            None => {
                return Err(CliError::InputError(
                    "Branch type is required when using structured branch flags in non-interactive mode"
                        .to_string(),
                ))
            }
        };

        let ticket = match &self.ticket {
            Some(ticket) => validate_structured_section(ticket, "ticket")?,
            None if !non_interactive => input::input_ticket()?,
            None => String::new(),
        };
        validate_ticket(&ticket, rules)?;

        let subject = match &self.subject {
            Some(subject) => validate_structured_section(subject, "subject")?,
            None if !non_interactive => input::input_subject()?,
            None => return Err(CliError::InputError(
                "Subject is required when using structured branch flags in non-interactive mode"
                    .to_string(),
            )),
        };

        let branch_name = build_branch_name(&branch_type, &ticket, &subject);
        let would_checkout = !non_interactive;

        if !non_interactive && !self.validate && !self.dry_run {
            let validate = input::ask_want_create_new_branch(&branch_name)?;
            if !validate {
                info!("Abort");
                return Ok(None);
            }
        }

        Ok(Some(BranchPlan {
            branch_name,
            branch_type,
            ticket,
            subject,
            would_checkout,
        }))
    }

    fn print_output(&self, output: &BranchCommandOutput) {
        if self.output == "json" {
            println!("{}", serde_json::to_string(output).unwrap());
            return;
        }

        if output.dry_run {
            if output.would_checkout {
                println!("Would create and switch to branch {}", output.branch_name);
            } else {
                println!("Would create branch {}", output.branch_name);
            }
            return;
        }

        println!("Branch {} created successfully!", output.branch_name);
        if output.would_checkout {
            println!("Switched to branch {}", output.branch_name);
        }
    }
}

fn build_branch_name(branch_type: &str, ticket: &str, subject: &str) -> String {
    if ticket.is_empty() {
        format!("{branch_type}-{subject}")
    } else {
        format!("{branch_type}-{ticket}-{subject}")
    }
}

fn validate_branch_type(branch_type: &str, allowed_types: &[String]) -> Result<String, CliError> {
    if allowed_types.iter().any(|known| known == branch_type) {
        Ok(branch_type.to_string())
    } else {
        Err(CliError::InputError(format!(
            "Invalid branch type '{}'. Valid branch types are: {}",
            branch_type,
            allowed_types.join(", ")
        )))
    }
}

fn validate_ticket(ticket: &str, rules: &BranchRulesConfig) -> Result<(), CliError> {
    if ticket.is_empty() {
        return if rules.require_ticket {
            Err(CliError::InputError(
                "A ticket is required by branch_rules".to_string(),
            ))
        } else {
            Ok(())
        };
    }

    let pattern = regex::Regex::new(&rules.ticket_pattern)
        .map_err(|error| CliError::RegexError(error.to_string()))?;
    if pattern.is_match(ticket) {
        Ok(())
    } else {
        Err(CliError::InputError(format!(
            "Ticket '{}' does not match branch_rules.ticket_pattern '{}'",
            ticket, rules.ticket_pattern
        )))
    }
}

fn validate_structured_section(value: &str, field_name: &str) -> Result<String, CliError> {
    validate_section(value).map_err(|error| {
        CliError::InputError(format!("Invalid {field_name} value '{}': {error}", value))
    })
}

fn parse_branch_name(
    name: &str,
    rules: &BranchRulesConfig,
) -> Result<(String, String, String), CliError> {
    let mut branch_parts = name.splitn(3, '-');
    let branch_type = branch_parts.next().unwrap_or("unknown").to_string();
    let second = branch_parts.next().unwrap_or("");
    let third = branch_parts.next();
    let ticket_pattern = regex::Regex::new(&rules.ticket_pattern)
        .map_err(|error| CliError::RegexError(error.to_string()))?;
    let (ticket, subject) = if let Some(subject) = third {
        if rules.require_ticket || ticket_pattern.is_match(second) {
            (second.to_string(), subject.to_string())
        } else {
            (String::new(), format!("{second}-{subject}"))
        }
    } else {
        (String::new(), second.to_string())
    };
    Ok((branch_type, ticket, subject))
}
