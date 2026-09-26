//! Initial Suggest route with no agent execution. Policy is kept separate from ranking so a
//! future local Decision Engine cannot introduce a forbidden profile.

use std::env;
use std::path::Path;

use crate::config::{Config, Engine};
use crate::storage::{self, DecisionRecord};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Profile {
    CodexMedium,
    CodexHigh,
    ClaudeNormal,
    ClaudeDeep,
    HumanGate,
}

impl Profile {
    fn name(self) -> &'static str {
        match self {
            Self::CodexMedium => "CODEX_MEDIUM",
            Self::CodexHigh => "CODEX_HIGH",
            Self::ClaudeNormal => "CLAUDE_NORMAL",
            Self::ClaudeDeep => "CLAUDE_DEEP",
            Self::HumanGate => "HUMAN_GATE",
        }
    }

    fn provider(self) -> &'static str {
        match self {
            Self::CodexMedium | Self::CodexHigh => "codex",
            Self::ClaudeNormal | Self::ClaudeDeep => "claude",
            Self::HumanGate => "human",
        }
    }

    fn effort(self) -> &'static str {
        match self {
            Self::CodexMedium => "medium",
            Self::CodexHigh => "high",
            Self::ClaudeNormal => "normal",
            Self::ClaudeDeep => "deep",
            Self::HumanGate => "none",
        }
    }
}

const PROFILES: [Profile; 4] = [
    Profile::CodexMedium,
    Profile::CodexHigh,
    Profile::ClaudeNormal,
    Profile::ClaudeDeep,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Priority {
    P0,
    P1,
    P2,
    P3,
}

impl Priority {
    fn name(self) -> &'static str {
        match self {
            Self::P0 => "P0",
            Self::P1 => "P1",
            Self::P2 => "P2",
            Self::P3 => "P3",
        }
    }
}

struct Request {
    task: String,
    priority: Priority,
    agent: Option<String>,
    effort: Option<String>,
}

#[derive(Clone, Copy)]
struct ProviderStatus {
    cli_present: bool,
    enabled: bool,
    known_exhausted: bool,
}

struct PolicyInput {
    codex: ProviderStatus,
    claude: ProviderStatus,
    configured_profiles: Vec<String>,
}

struct AllowedRoutes {
    candidates: Vec<Profile>,
    rejected: Vec<String>,
    gate_reason: Option<&'static str>,
}

fn risky_task(task: &str) -> bool {
    let text = task.to_lowercase();
    // Conservative tripwires, not a complete security scanner. Match whole words
    // across intervening qualifiers and punctuation. Even negated/quoted risky
    // requests require human review; these heuristics must not authorize execution.
    let words: Vec<_> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect();
    let has = |terms: &[&str]| words.iter().any(|word| terms.contains(word));
    let destructive = has(&[
        "drop", "delete", "remove", "truncate", "wipe", "erase", "purge",
    ]) && has(&[
        "table",
        "tables",
        "database",
        "databases",
        "schema",
        "production",
        "data",
    ]);
    let security = (has(&["disable", "remove", "bypass", "weaken"])
        || words.windows(2).any(|pair| pair == ["turn", "off"]))
        && has(&["authentication", "authorization", "auth", "security"]);
    destructive
        || security
        || [
            "drop table",
            "delete production",
            "delete all data",
            "disable authentication",
            "disable security",
            "remove authorization",
            "本番データを削除",
            "認証を無効",
            "全データ削除",
        ]
        .iter()
        .any(|term| text.contains(term))
}

fn policy(request: &Request, input: &PolicyInput) -> AllowedRoutes {
    if risky_task(&request.task) {
        return AllowedRoutes {
            candidates: Vec::new(),
            rejected: vec!["destructive or security-sensitive instruction".into()],
            gate_reason: Some("requires human safety review"),
        };
    }

    let mut result = AllowedRoutes {
        candidates: Vec::new(),
        rejected: Vec::new(),
        gate_reason: None,
    };
    for profile in PROFILES {
        let status = match profile.provider() {
            "codex" => input.codex,
            _ => input.claude,
        };
        let reject = if request
            .agent
            .as_deref()
            .is_some_and(|a| a != profile.provider())
        {
            Some("excluded by explicit agent override")
        } else if request
            .effort
            .as_deref()
            .is_some_and(|e| e != profile.effort())
        {
            Some("excluded by explicit effort override")
        } else if !input
            .configured_profiles
            .iter()
            .any(|name| name == profile.name())
        {
            Some("profile disabled in config")
        } else if !status.enabled {
            Some("provider disabled")
        } else if status.known_exhausted {
            Some("provider exhausted")
        } else if !status.cli_present {
            Some("CLI unavailable")
        } else {
            None
        };
        if let Some(reason) = reject {
            result
                .rejected
                .push(format!("{}: {reason}", profile.name()));
        } else {
            result.candidates.push(profile);
        }
    }
    if result.candidates.is_empty() {
        result.gate_reason = Some("no policy-allowed execution profile");
    }
    result
}

trait DecisionEngine {
    fn choose(&self, request: &Request, candidates: &[Profile]) -> Option<Profile>;
    fn provenance(&self) -> &'static str;
}

struct RuleEngine;

impl DecisionEngine for RuleEngine {
    fn choose(&self, request: &Request, candidates: &[Profile]) -> Option<Profile> {
        let preference = match request.priority {
            Priority::P0 | Priority::P1 => [
                Profile::CodexMedium,
                Profile::ClaudeNormal,
                Profile::CodexHigh,
                Profile::ClaudeDeep,
            ],
            Priority::P2 | Priority::P3 => [
                Profile::CodexHigh,
                Profile::ClaudeDeep,
                Profile::CodexMedium,
                Profile::ClaudeNormal,
            ],
        };
        preference
            .into_iter()
            .find(|profile| candidates.contains(profile))
    }

    fn provenance(&self) -> &'static str {
        "rule-based provisional decision (no model call)"
    }
}

fn select<E: DecisionEngine>(
    engine: &E,
    request: &Request,
    allowed: &AllowedRoutes,
) -> (Profile, &'static str) {
    if allowed.gate_reason.is_some() {
        return (Profile::HumanGate, "hard policy");
    }
    match engine.choose(request, &allowed.candidates) {
        Some(profile) if allowed.candidates.contains(&profile) => (profile, engine.provenance()),
        _ => (Profile::HumanGate, "invalid decision; human gate"),
    }
}

fn parse_request(args: &[String]) -> Result<Request, String> {
    let mut priority = Priority::P1;
    let mut agent = None;
    let mut effort = None;
    let mut task = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--priority" | "--agent" | "--effort" => {
                let key = args[i].as_str();
                i += 1;
                let value = args.get(i).ok_or_else(|| format!("{key} needs a value"))?;
                match key {
                    "--priority" => {
                        priority = match value.to_ascii_lowercase().as_str() {
                            "p0" => Priority::P0,
                            "p1" => Priority::P1,
                            "p2" => Priority::P2,
                            "p3" => Priority::P3,
                            _ => return Err("priority must be p0, p1, p2, or p3".into()),
                        }
                    }
                    "--agent" => {
                        if !matches!(value.as_str(), "codex" | "claude") {
                            return Err(
                                "agent must be codex or claude in this initial slice".into()
                            );
                        }
                        agent = Some(value.clone());
                    }
                    _ => {
                        if !matches!(value.as_str(), "normal" | "medium" | "high" | "deep") {
                            return Err("effort must be normal, medium, high, or deep".into());
                        }
                        effort = Some(value.clone());
                    }
                }
            }
            value if value.starts_with('-') => {
                return Err(format!("unknown route option: {value}"))
            }
            value => {
                if task.is_some() {
                    return Err("route expects one quoted task argument".into());
                }
                if value.trim().is_empty() {
                    return Err("task must not be empty".into());
                }
                task = Some(value.to_owned());
            }
        }
        i += 1;
    }
    Ok(Request {
        task: task.ok_or("route needs a task")?,
        priority,
        agent,
        effort,
    })
}

pub(crate) fn command_on_path(name: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    command_on_search_path(name, &path)
}

fn command_on_search_path(name: &str, path: &std::ffi::OsStr) -> bool {
    #[cfg(windows)]
    let names = [format!("{name}.exe"), format!("{name}.cmd")];
    #[cfg(not(windows))]
    let names = [name.to_owned(), name.to_owned()];
    env::split_paths(path).any(|dir| {
        names
            .iter()
            .any(|file| is_executable_file(&Path::new(&dir).join(file)))
    })
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = path.metadata() else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn route_command(args: &[String]) -> Result<(), String> {
    let request = parse_request(args)?;
    let root = crate::config::current_root()?;
    let (config, _) = Config::load(&root)?;
    if config.decision.engine == Engine::Apus {
        return Err("APUS Decision Engine is not connected yet; set decision.engine='rule' or complete the local contract probe".into());
    }
    let input = PolicyInput {
        codex: ProviderStatus {
            cli_present: command_on_path("codex"),
            enabled: config.providers.codex.enabled,
            known_exhausted: false,
        },
        claude: ProviderStatus {
            cli_present: command_on_path("claude"),
            enabled: config.providers.claude.enabled,
            known_exhausted: false,
        },
        configured_profiles: config.execution_profiles.clone(),
    };
    let allowed = policy(&request, &input);
    let (selected, provenance) = select(&RuleEngine, &request, &allowed);
    let task_type = task_type(&request.task);
    let candidate_names = allowed.candidates.iter().map(|p| p.name()).collect();
    let rejected = allowed.rejected.iter().map(String::as_str).collect();
    let id = storage::record_route(
        &root,
        DecisionRecord {
            task_type,
            repository: &root.to_string_lossy(),
            priority: request.priority.name(),
            profile: selected.name(),
            provider: selected.provider(),
            effort: selected.effort(),
            source: provenance,
            task_bytes: request.task.len(),
            candidates: candidate_names,
            rejected,
        },
    )?;
    println!("Mode: Suggest (no agent execution)");
    println!("Task ID: {id}");
    println!("Selected: {}", selected.name());
    println!("Decision: {provenance}");
    println!(
        "Why: priority {:?}; CLI availability and explicit override applied",
        request.priority
    );
    if let Some(reason) = allowed.gate_reason {
        println!("Gate: {reason}");
    }
    println!(
        "Alternatives: {}",
        allowed
            .candidates
            .iter()
            .filter(|p| **p != selected)
            .map(|p| p.name())
            .collect::<Vec<_>>()
            .join(", ")
    );
    for reason in &allowed.rejected {
        println!("Rejected: {reason}");
    }
    println!("Quota: UNKNOWN; authentication: UNKNOWN; no agent executed");
    Ok(())
}

fn task_type(task: &str) -> &'static str {
    let text = task.to_lowercase();
    if ["fix", "bug", "修正", "バグ"]
        .iter()
        .any(|word| text.contains(word))
    {
        "bugfix"
    } else if ["review", "レビュー"]
        .iter()
        .any(|word| text.contains(word))
    {
        "review"
    } else {
        "general"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(task: &str) -> Request {
        Request {
            task: task.into(),
            priority: Priority::P1,
            agent: None,
            effort: None,
        }
    }

    fn input() -> PolicyInput {
        PolicyInput {
            codex: ProviderStatus {
                cli_present: true,
                enabled: true,
                known_exhausted: false,
            },
            claude: ProviderStatus {
                cli_present: true,
                enabled: true,
                known_exhausted: false,
            },
            configured_profiles: PROFILES.iter().map(|p| p.name().into()).collect(),
        }
    }

    #[test]
    fn override_cannot_bypass_exhaustion() {
        let mut req = request("fix bug");
        req.agent = Some("codex".into());
        req.effort = Some("high".into());
        let mut status = input();
        status.codex.known_exhausted = true;
        let allowed = policy(&req, &status);
        assert!(allowed.candidates.is_empty());
        assert_eq!(select(&RuleEngine, &req, &allowed).0, Profile::HumanGate);
    }

    #[test]
    fn destructive_request_gates_even_with_override() {
        let mut req = request("drop table users");
        req.agent = Some("codex".into());
        assert_eq!(
            select(&RuleEngine, &req, &policy(&req, &input())).0,
            Profile::HumanGate
        );
    }

    #[test]
    fn destructive_variants_gate_before_overrides_or_ranking() {
        for task in [
            "drop the users table",
            "DROP\nTABLE users",
            "delete the production database",
            "delete all customer data",
            "truncate the audit tables",
            "disable login authentication",
            "remove the authorization checks",
            "turn off the security checks",
            "bypass the auth check",
            "本番データを削除して",
        ] {
            let mut req = request(task);
            req.agent = Some("codex".into());
            req.effort = Some("high".into());
            let allowed = policy(&req, &input());
            assert!(allowed.candidates.is_empty(), "{task}");
            assert_eq!(
                select(&RuleEngine, &req, &allowed).0,
                Profile::HumanGate,
                "{task}"
            );
        }
        for task in [
            "fix the authentication bug",
            "add a dropdown to the table",
            "remove the unused helper",
            "review architecture",
        ] {
            let req = request(task);
            assert!(!policy(&req, &input()).candidates.is_empty(), "{task}");
        }
    }

    #[test]
    fn unavailable_cli_is_excluded() {
        let req = request("fix bug");
        let mut status = input();
        status.codex.cli_present = false;
        let allowed = policy(&req, &status);
        assert_eq!(select(&RuleEngine, &req, &allowed).0, Profile::ClaudeNormal);
    }

    #[cfg(unix)]
    #[test]
    fn command_discovery_requires_executable_regular_files() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let command = crate::tests::temp_path("codex");
        let root = command.parent().unwrap();
        std::fs::write(&command, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(!command_on_search_path("codex", root.as_os_str()));
        std::fs::set_permissions(&command, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(command_on_search_path("codex", root.as_os_str()));
        symlink(&command, root.join("claude")).unwrap();
        assert!(command_on_search_path("claude", root.as_os_str()));
        std::fs::remove_file(&command).unwrap();
        assert!(!command_on_search_path("claude", root.as_os_str()));
        std::fs::create_dir(&command).unwrap();
        assert!(!command_on_search_path("codex", root.as_os_str()));
        std::fs::remove_dir_all(root).unwrap();
    }

    struct InvalidEngine;

    impl DecisionEngine for InvalidEngine {
        fn choose(&self, _: &Request, _: &[Profile]) -> Option<Profile> {
            Some(Profile::ClaudeDeep)
        }
        fn provenance(&self) -> &'static str {
            "invalid"
        }
    }

    #[test]
    fn model_cannot_select_outside_allowed_set() {
        let req = request("fix bug");
        let mut status = input();
        status.claude.enabled = false;
        let allowed = policy(&req, &status);
        assert_eq!(select(&InvalidEngine, &req, &allowed).0, Profile::HumanGate);
    }

    #[test]
    fn priority_and_effort_control_rule_choice() {
        let mut req = request("review architecture");
        req.priority = Priority::P2;
        assert_eq!(
            select(&RuleEngine, &req, &policy(&req, &input())).0,
            Profile::CodexHigh
        );
        req.agent = Some("claude".into());
        req.effort = Some("normal".into());
        assert_eq!(
            select(&RuleEngine, &req, &policy(&req, &input())).0,
            Profile::ClaudeNormal
        );
    }
}
