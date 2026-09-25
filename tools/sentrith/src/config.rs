//! Strict local TOML settings. Missing config means conservative defaults.

use serde::Deserialize;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const TEMPLATE: &str = r#"# Local settings; Sentrith never falls back to paid API keys.
mode = "suggest"
execution_profiles = ["CODEX_MEDIUM", "CODEX_HIGH", "CLAUDE_NORMAL", "CLAUDE_DEEP"]

[decision]
engine = "rule"
lm_studio_endpoint = "http://127.0.0.1:1234"
# model = "APUS-OpenJev-v1-4B-Q8_0"

[providers.codex]
enabled = true
reserve_level = 0.20
max_concurrent = 1

[providers.claude]
enabled = true
reserve_level = 0.20
max_concurrent = 1

[providers.antigravity]
enabled = false
reserve_level = 0.15
max_concurrent = 1

[providers.local]
enabled = false
reserve_level = 0.0
max_concurrent = 1

[billing]
allow_api_keys = false

[retention]
important_days = 30
ordinary_days = 7
disposable_hours = 24

[retry]
max_attempts = 2
max_provider_switches = 1
max_total_runtime_seconds = 1800

[validation]
commands = []
"#;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Suggest,
    Assist,
    Auto,
}

impl Mode {
    pub fn name(self) -> &'static str {
        match self {
            Self::Suggest => "suggest",
            Self::Assist => "assist",
            Self::Auto => "auto",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Rule,
    Apus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DecisionConfig {
    pub engine: Engine,
    pub lm_studio_endpoint: String,
    pub model: Option<String>,
}

impl Default for DecisionConfig {
    fn default() -> Self {
        Self {
            engine: Engine::Rule,
            lm_studio_endpoint: "http://127.0.0.1:1234".into(),
            model: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProviderConfig {
    pub enabled: bool,
    pub reserve_level: f64,
    pub max_concurrent: u32,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            reserve_level: 0.0,
            max_concurrent: 1,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Providers {
    pub codex: ProviderConfig,
    pub claude: ProviderConfig,
    pub antigravity: ProviderConfig,
    pub local: ProviderConfig,
}

impl Default for Providers {
    fn default() -> Self {
        Self {
            codex: ProviderConfig {
                enabled: true,
                reserve_level: 0.20,
                ..ProviderConfig::default()
            },
            claude: ProviderConfig {
                enabled: true,
                reserve_level: 0.20,
                ..ProviderConfig::default()
            },
            antigravity: ProviderConfig {
                reserve_level: 0.15,
                ..ProviderConfig::default()
            },
            local: ProviderConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct BillingConfig {
    pub allow_api_keys: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetentionConfig {
    pub important_days: u32,
    pub ordinary_days: u32,
    pub disposable_hours: u32,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            important_days: 30,
            ordinary_days: 7,
            disposable_hours: 24,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub max_provider_switches: u32,
    pub max_total_runtime_seconds: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 2,
            max_provider_switches: 1,
            max_total_runtime_seconds: 1800,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ValidationConfig {
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub mode: Mode,
    pub execution_profiles: Vec<String>,
    pub decision: DecisionConfig,
    pub providers: Providers,
    pub billing: BillingConfig,
    pub retention: RetentionConfig,
    pub retry: RetryConfig,
    pub validation: ValidationConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Suggest,
            execution_profiles: ["CODEX_MEDIUM", "CODEX_HIGH", "CLAUDE_NORMAL", "CLAUDE_DEEP"]
                .iter()
                .map(|s| (*s).into())
                .collect(),
            decision: DecisionConfig::default(),
            providers: Providers::default(),
            billing: BillingConfig::default(),
            retention: RetentionConfig::default(),
            retry: RetryConfig::default(),
            validation: ValidationConfig::default(),
        }
    }
}

impl Config {
    pub fn parse(raw: &str) -> Result<Self, String> {
        let parsed: Self = toml::from_str(raw).map_err(|e| format!("invalid config TOML: {e}"))?;
        parsed.validate()?;
        Ok(parsed)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.billing.allow_api_keys {
            return Err(
                "billing.allow_api_keys=true is unsupported; no paid API fallback exists".into(),
            );
        }
        let endpoint = self.decision.lm_studio_endpoint.trim_end_matches('/');
        let port = ["http://127.0.0.1:", "http://localhost:", "http://[::1]:"]
            .iter()
            .find_map(|prefix| endpoint.strip_prefix(prefix));
        if port
            .and_then(|p| p.parse::<u16>().ok())
            .filter(|p| *p > 0)
            .is_none()
        {
            return Err(
                "decision.lm_studio_endpoint must be loopback HTTP with an explicit port".into(),
            );
        }
        if self.decision.engine == Engine::Apus
            && self
                .decision
                .model
                .as_deref()
                .is_none_or(|m| m.trim().is_empty())
        {
            return Err("decision.model is required when engine=apus".into());
        }
        for (name, provider) in [
            ("codex", &self.providers.codex),
            ("claude", &self.providers.claude),
            ("antigravity", &self.providers.antigravity),
            ("local", &self.providers.local),
        ] {
            if !(0.0..=1.0).contains(&provider.reserve_level) || !provider.reserve_level.is_finite()
            {
                return Err(format!(
                    "providers.{name}.reserve_level must be between 0 and 1"
                ));
            }
            if provider.max_concurrent == 0 {
                return Err(format!("providers.{name}.max_concurrent must be positive"));
            }
        }
        if self.execution_profiles.is_empty() {
            return Err("execution_profiles must not be empty".into());
        }
        let mut seen = std::collections::BTreeSet::new();
        for name in &self.execution_profiles {
            if !matches!(
                name.as_str(),
                "CODEX_MEDIUM" | "CODEX_HIGH" | "CLAUDE_NORMAL" | "CLAUDE_DEEP"
            ) {
                return Err(format!("unsupported execution profile: {name}"));
            }
            if !seen.insert(name) {
                return Err(format!("duplicate execution profile: {name}"));
            }
        }
        if self.retention.important_days == 0
            || self.retention.ordinary_days == 0
            || self.retention.disposable_hours == 0
        {
            return Err("retention periods must be positive".into());
        }
        if self.retry.max_attempts == 0 || self.retry.max_total_runtime_seconds == 0 {
            return Err("retry attempts and runtime must be positive".into());
        }
        if self.retry.max_provider_switches >= self.retry.max_attempts {
            return Err("retry.max_provider_switches must be less than max_attempts".into());
        }
        if self
            .validation
            .commands
            .iter()
            .any(|command| command.trim().is_empty() || command.contains(['\n', '\r', '\0']))
        {
            return Err("validation commands must be nonempty single lines".into());
        }
        Ok(())
    }

    pub fn load(root: &Path) -> Result<(Self, bool), String> {
        let dir = root.join(".sentrith");
        match fs::symlink_metadata(&dir) {
            Ok(meta) if !meta.is_dir() => return Err(".sentrith is not a real directory".into()),
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok((Self::default(), false))
            }
            Err(e) => return Err(format!("cannot inspect .sentrith: {e}")),
        }
        let path = dir.join("config.toml");
        match fs::symlink_metadata(&path) {
            Ok(meta) if !meta.is_file() => return Err("config.toml is not a regular file".into()),
            Ok(_) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok((Self::default(), false))
            }
            Err(e) => return Err(format!("cannot inspect config.toml: {e}")),
        }
        let mut file = crate::open_regular_file_no_follow(&path)
            .map_err(|e| format!("cannot open config safely: {e}"))?;
        let mut raw = String::new();
        file.read_to_string(&mut raw)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        Ok((Self::parse(&raw)?, true))
    }
}

pub fn current_root() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|e| format!("cannot determine working directory: {e}"))
}

fn init_at(root: &Path) -> Result<(), String> {
    let dir = root.join(".sentrith");
    crate::create_real_directory_tree(&dir)?;
    let path = dir.join("config.toml");
    let mut file = crate::create_secure_file(&path)?;
    file.write_all(TEMPLATE.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    println!("Created {}", path.display());
    Ok(())
}

pub fn config_command(args: &[String]) -> Result<(), String> {
    let root = current_root()?;
    match args {
        [command] if command == "init" => init_at(&root),
        [command] if command == "check" => {
            let (config, from_file) = Config::load(&root)?;
            config.validate()?;
            println!(
                "Config: valid ({})",
                if from_file {
                    ".sentrith/config.toml"
                } else {
                    "built-in defaults"
                }
            );
            println!("Mode: {}", config.mode.name());
            println!("Decision engine: {:?}", config.decision.engine);
            if config.mode != Mode::Suggest {
                println!("Warning: Assist/Auto execution is not implemented yet");
            }
            if config.decision.engine == Engine::Apus {
                println!("Warning: APUS adapter requires an on-device contract probe and is not connected yet");
            }
            Ok(())
        }
        _ => Err("usage: sentrith config init|check".into()),
    }
}

pub fn status_command(args: &[String]) -> Result<(), String> {
    if !args.is_empty() {
        return Err("usage: sentrith status".into());
    }
    let root = current_root()?;
    let (config, from_file) = Config::load(&root)?;
    println!(
        "Config: {}",
        if from_file {
            ".sentrith/config.toml"
        } else {
            "built-in defaults"
        }
    );
    println!(
        "Mode configured: {} (route is Suggest only)",
        config.mode.name()
    );
    println!("Decision engine: {:?}", config.decision.engine);
    for (name, provider) in [
        ("codex", &config.providers.codex),
        ("claude", &config.providers.claude),
    ] {
        println!(
            "{name}: enabled={}, cli_present={}, quota=UNKNOWN, authentication=UNKNOWN, reserve_configured={}, reserve_enforcement=PENDING",
            provider.enabled,
            crate::routing::command_on_path(name),
            provider.reserve_level,
        );
    }
    println!(
        "antigravity: enabled={}, adapter=UNAVAILABLE",
        config.providers.antigravity.enabled
    );
    println!(
        "local: enabled={}, adapter=UNAVAILABLE",
        config.providers.local.enabled
    );
    match crate::storage::database_status(&root)? {
        Some((version, count)) => println!("Database: schema={version}, routes={count}"),
        None => println!("Database: not initialized"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_is_valid() {
        let config = Config::parse(TEMPLATE).unwrap();
        assert_eq!(config.mode, Mode::Suggest);
    }

    #[test]
    fn rejects_unknown_fields_and_paid_api_mode() {
        assert!(Config::parse("unexpected = true").is_err());
        assert!(Config::parse("[billing]\nallow_api_keys = true").is_err());
    }

    #[test]
    fn rejects_nonlocal_endpoint_and_invalid_limits() {
        assert!(
            Config::parse("[decision]\nlm_studio_endpoint = 'https://example.com:443'").is_err()
        );
        assert!(Config::parse("[retry]\nmax_attempts = 0").is_err());
        assert!(Config::parse("[providers.codex]\nreserve_level = 1.5").is_err());
    }

    #[test]
    fn rejects_apus_without_model_and_unknown_profile() {
        assert!(Config::parse("[decision]\nengine = 'apus'").is_err());
        assert!(Config::parse("execution_profiles = ['CODEX_UNKNOWN']").is_err());
    }
}
