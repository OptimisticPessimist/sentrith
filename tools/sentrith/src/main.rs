use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const USAGE_HEADER_V1: &str = "timestamp,agent,model,phase,task,input_tokens,cached_input_tokens,output_tokens,credits,cost_usd,tool_calls,duration_seconds,success,rework_count,source,session_id,notes\n";
const USAGE_HEADER: &str = "timestamp,agent,model,phase,task,input_tokens,cached_input_tokens,output_tokens,credits,cost_usd,tool_calls,duration_seconds,success,rework_count,source,session_id,notes,head_sha,verification\n";

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        print_help();
        return;
    }

    let result = match args[0].as_str() {
        "preflight" => preflight(),
        "closeout-check" => closeout_check(),
        "guard" => guard_check(),
        "review-hint" => review_hint(),
        "diff-budget" => diff_budget(),
        "hooks" => hooks_command(&args[1..]),
        "usage" => usage_command(&args[1..]),
        "version" | "--version" | "-V" => {
            println!("sentrith {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown command: {other}")),
    };

    if let Err(msg) = result {
        eprintln!("SENTRITH-ERROR: {msg}");
        std::process::exit(2);
    }
}

fn print_help() {
    println!(
r#"sentrith — local AI-development guard and usage CLI

Commands:
  sentrith preflight
  sentrith closeout-check
  sentrith guard
  sentrith review-hint
  sentrith diff-budget

  sentrith hooks install [--agent claude|codex|all] [--dry-run]
  sentrith hooks status [--agent claude|codex|all]

  sentrith usage status [--min-samples 5] [--agent <name>]
  sentrith usage baseline start|stop|status

  sentrith usage record --agent <codex|claude|copilot|gemini|other> --task <name> [options]
  sentrith usage run codex --task <name> [--phase standard] -- <codex exec args...>
  sentrith usage run copilot --task <name> [--phase standard] -- <copilot args...>
  sentrith usage hook codex [--phase standard]
  sentrith usage hook claude [--phase standard]
  sentrith usage claude-status
  sentrith usage snapshot copilot --github-user <user> [--org <org>]
  sentrith usage task start --agent <name> --task <label> [options]
  sentrith usage task stop --success <yes|no|partial> [options]
  sentrith usage contribute --agent <name> [--metric auto|credits|cost_usd|tokens] [options]
  sentrith usage aggregate [--dir docs/metrics/contributions] [--publish]
  sentrith usage report [--compare] [--agent <name>] [--file <path>]
  sentrith usage report --tasks [--agent <name>] [--file <path>]
  sentrith usage report --churn [--days 14] [--agent <name>] [--file <path>]
  sentrith usage publish [options]
  sentrith usage note <text> [--file <path>]

  sentrith version

Usage record options:
  --model <name>
  --phase <baseline|standard|other>
  --input <tokens>
  --cached-input <tokens>
  --output <tokens>
  --credits <number>
  --tool-calls <number>
  --duration <seconds>
  --success <yes|no|partial>
  --rework <number>
  --notes <text>
  --file <path>

Phase resolution (highest first):
  --phase            explicit flag
  .ai-usage/phase    marker written by `usage baseline start`
  SENTRITH_PHASE     environment variable
  standard           default

Provider measurement:
  GitHub Copilot snapshot uses `gh api` only when explicitly requested.
  Other commands are local/deterministic and make no model calls.
  Raw prompts, source code, repository names, transcripts, and session IDs
  are never included in community contribution files.

Success semantics:
  Hook-captured rows derive success from repository evidence only:
  commit reached + last recorded test outcome. Undecidable rows are `unknown`
  and are excluded from success-rate denominators.
"#);
}

fn repo_file(path: &str) -> PathBuf {
    Path::new(path).to_path_buf()
}

// ---------------------------------------------------------------------------
// Minimal JSON model
//
// Editing a user's settings file by hand is the main friction point in enabling
// measurement, but corrupting that file is worse than the friction. This keeps
// the zero-dependency rule while allowing a parse -> edit -> serialize cycle.
// Object keys are stored as an ordered Vec so round-tripping preserves order,
// and numbers stay as their original text so no precision is invented.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: Json) {
        if let Json::Obj(entries) = self {
            if let Some(slot) = entries.iter_mut().find(|(k, _)| k == key) {
                slot.1 = value;
            } else {
                entries.push((key.to_string(), value));
            }
        }
    }

    fn remove(&mut self, key: &str) {
        if let Json::Obj(entries) = self {
            entries.retain(|(k, _)| k != key);
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
}

struct JsonParser<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> JsonParser<'a> {
    fn new(s: &'a str) -> Self {
        JsonParser { b: s.as_bytes(), i: 0 }
    }

    fn ws(&mut self) {
        while self.i < self.b.len() && (self.b[self.i] as char).is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn expect(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!("expected '{}' at byte {}", c as char, self.i))
        }
    }

    fn value(&mut self) -> Result<Json, String> {
        self.ws();
        match self.peek().ok_or("unexpected end of JSON")? {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Ok(Json::Str(self.string()?)),
            b't' => self.literal("true", Json::Bool(true)),
            b'f' => self.literal("false", Json::Bool(false)),
            b'n' => self.literal("null", Json::Null),
            _ => self.number(),
        }
    }

    fn literal(&mut self, text: &str, value: Json) -> Result<Json, String> {
        if self.b[self.i..].starts_with(text.as_bytes()) {
            self.i += text.len();
            Ok(value)
        } else {
            Err(format!("invalid literal at byte {}", self.i))
        }
    }

    fn number(&mut self) -> Result<Json, String> {
        let start = self.i;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                self.i += 1;
            } else {
                break;
            }
        }
        if start == self.i {
            return Err(format!("invalid value at byte {}", self.i));
        }
        Ok(Json::Num(String::from_utf8_lossy(&self.b[start..self.i]).to_string()))
    }

    /// Read exactly four hex digits of a `\u` escape.
    fn hex4(&mut self) -> Result<u32, String> {
        let hex = self.b.get(self.i..self.i + 4).ok_or("truncated \\u escape")?;
        if !hex.iter().all(|c| c.is_ascii_hexdigit()) {
            return Err("invalid \\u escape".into());
        }
        let code = u32::from_str_radix(&String::from_utf8_lossy(hex), 16)
            .map_err(|_| "invalid \\u escape".to_string())?;
        self.i += 4;
        Ok(code)
    }

    fn string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut out = String::new();
        loop {
            let c = self.peek().ok_or("unterminated string")?;
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = self.peek().ok_or("unterminated escape")?;
                    self.i += 1;
                    match e {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let code = self.hex4()?;
                            let ch = if (0xD800..=0xDBFF).contains(&code) {
                                // High surrogate: a non-BMP character such as an
                                // emoji is encoded as a pair. Decoding the halves
                                // separately would rewrite the user's setting as
                                // two replacement characters.
                                if !self.b[self.i..].starts_with(b"\\u") {
                                    return Err("unpaired high surrogate in JSON string".into());
                                }
                                self.i += 2;
                                let low = self.hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&low) {
                                    return Err("invalid low surrogate in JSON string".into());
                                }
                                let combined =
                                    0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                                char::from_u32(combined)
                                    .ok_or("invalid surrogate pair in JSON string")?
                            } else if (0xDC00..=0xDFFF).contains(&code) {
                                return Err("unpaired low surrogate in JSON string".into());
                            } else {
                                char::from_u32(code).ok_or("invalid \\u escape")?
                            };
                            out.push(ch);
                        }
                        _ => return Err("invalid escape".into()),
                    }
                }
                _ => {
                    // Copy the whole UTF-8 sequence, not just this byte.
                    let len = utf8_len(c);
                    let end = (self.i - 1 + len).min(self.b.len());
                    out.push_str(&String::from_utf8_lossy(&self.b[self.i - 1..end]));
                    self.i = end;
                }
            }
        }
    }

    fn array(&mut self) -> Result<Json, String> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Json::Arr(items));
        }
        loop {
            items.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b']') => {
                    self.i += 1;
                    return Ok(Json::Arr(items));
                }
                _ => return Err(format!("expected ',' or ']' at byte {}", self.i)),
            }
        }
    }

    fn object(&mut self) -> Result<Json, String> {
        self.expect(b'{')?;
        let mut entries = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Json::Obj(entries));
        }
        loop {
            self.ws();
            let key = self.string()?;
            self.ws();
            self.expect(b':')?;
            let value = self.value()?;
            entries.push((key, value));
            self.ws();
            match self.peek() {
                Some(b',') => {
                    self.i += 1;
                }
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Json::Obj(entries));
                }
                _ => return Err(format!("expected ',' or '}}' at byte {}", self.i)),
            }
        }
    }
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else if first >> 3 == 0b11110 {
        4
    } else {
        1
    }
}

fn json_parse(text: &str) -> Result<Json, String> {
    let mut p = JsonParser::new(text);
    let v = p.value()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(format!("trailing data at byte {}", p.i));
    }
    Ok(v)
}

fn json_write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn json_render(v: &Json, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    let pad_inner = "  ".repeat(indent + 1);
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Num(n) => out.push_str(n),
        Json::Str(s) => json_write_string(out, s),
        Json::Arr(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                out.push_str(&pad_inner);
                json_render(item, indent + 1, out);
                if i + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&pad);
            out.push(']');
        }
        Json::Obj(entries) => {
            if entries.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            for (i, (k, val)) in entries.iter().enumerate() {
                out.push_str(&pad_inner);
                json_write_string(out, k);
                out.push_str(": ");
                json_render(val, indent + 1, out);
                if i + 1 < entries.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&pad);
            out.push('}');
        }
    }
}

fn json_to_string(v: &Json) -> String {
    let mut out = String::new();
    json_render(v, 0, &mut out);
    out.push('\n');
    out
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn git(args: &[&str]) -> String {
    match Command::new("git").args(args).output() {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => String::new(),
    }
}

fn changed_files() -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for output in [git(&["diff", "--name-only"]), git(&["diff", "--cached", "--name-only"])] {
        for line in output.lines() {
            let s = line.trim();
            if !s.is_empty() {
                set.insert(s.to_string());
            }
        }
    }
    set
}

fn preflight() -> Result<(), String> {
    let project = repo_file("docs/ai/PROJECT.md");
    let state = repo_file("docs/ai/STATE.md");
    let mut warnings = Vec::new();

    if !project.exists() || !state.exists() {
        warnings.push("AI memory files missing; run project-bootstrap once.".to_string());
    } else {
        let ptxt = read_text(&project);
        let stxt = read_text(&state);

        if stxt.contains("Repository understanding: not initialized") {
            warnings.push("AI project memory is not bootstrapped.".to_string());
        }

        let plines = ptxt.lines().count().max(1);
        let slines = stxt.lines().count().max(1);

        if plines > 400 {
            warnings.push(format!("PROJECT.md is {plines} lines; consider memory-audit."));
        }
        if slines > 120 {
            warnings.push(format!("STATE.md is {slines} lines; consider memory-audit."));
        }
    }

    let profile = repo_file("docs/ai/PROFILE.md");
    if profile.exists() {
        let ftxt = read_text(&profile);
        if ftxt.contains("Status: not initialized") {
            warnings.push("Engineering profile not selected; run project-bootstrap profile questions.".to_string());
        }
        let flines = ftxt.lines().count();
        if flines > 100 {
            warnings.push(format!(
                "PROFILE.md is {flines} lines; it loads on ordinary tasks, keep it an index."
            ));
        }
    }

    if warnings.is_empty() {
        println!("SENTRITH-PREFLIGHT: ok");
    } else {
        println!("SENTRITH-PREFLIGHT: {}", warnings.join(" "));
    }
    Ok(())
}

fn closeout_check() -> Result<(), String> {
    let files = changed_files();
    if files.is_empty() {
        println!("SENTRITH-CLOSEOUT: no tracked diff detected");
        return Ok(());
    }

    let high_risk_prefixes = [
        "migrations/", "migration/", "db/", "database/", "auth/", "security/", "infra/",
        ".github/workflows/",
    ];
    let high_risk_names = [
        "openapi.yaml", "openapi.yml", "schema.prisma", "Dockerfile",
        "docker-compose.yml", "docker-compose.yaml",
    ];

    let mut flags = Vec::new();
    for f in &files {
        let low = f.to_lowercase();
        let name = Path::new(f).file_name().and_then(|x| x.to_str()).unwrap_or("");
        if high_risk_prefixes.iter().any(|p| low.starts_with(p))
            || high_risk_names.iter().any(|n| name.eq_ignore_ascii_case(n))
        {
            flags.push(f.clone());
        }
    }

    if flags.is_empty() {
        println!(
            "SENTRITH-CLOSEOUT: {} changed file(s); run normal verification and memory gate.",
            files.len()
        );
    } else {
        let shown = flags.iter().take(4).cloned().collect::<Vec<_>>().join(", ");
        let suffix = if flags.len() > 4 { " ..." } else { "" };
        println!(
            "SENTRITH-CLOSEOUT: high-risk paths changed ({shown}{suffix}); ensure significant-work verification/ADR check was considered."
        );
    }
    Ok(())
}

fn combined_diff() -> String {
    format!(
        "{}\n{}",
        git(&["diff", "--unified=0"]),
        git(&["diff", "--cached", "--unified=0"])
    )
}

fn guard_check() -> Result<(), String> {
    let diff = combined_diff();
    let patterns = [
        "skip(", "@skip", "pytest.mark.skip", "xfail", "eslint-disable", "type: ignore",
        "noinspection", "verify=false", "insecure", "ssl_verify=false",
    ];
    let mut hit = false;

    for line in diff.lines() {
        if !(line.starts_with('+') || line.starts_with('-')) || line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        let low = line.to_lowercase();
        if patterns.iter().any(|p| low.contains(p)) {
            hit = true;
            break;
        }
    }

    if hit {
        println!("SENTRITH-GUARD: potential verification/security bypass changed; confirm repository evidence before accepting.");
    } else {
        println!("SENTRITH-GUARD: no obvious bypass-pattern changes detected.");
    }
    Ok(())
}

fn review_hint() -> Result<(), String> {
    let files = changed_files();
    let diff = combined_diff().to_lowercase();

    let mut recommended = false;
    for f in &files {
        let low = f.to_lowercase();
        if ["migration", "schema", "auth", "security", "permission", "billing", "payment"]
            .iter()
            .any(|x| low.contains(x))
        {
            recommended = true;
            break;
        }
    }

    let destructive_terms = [
        "drop table", "drop column", "truncate ", "delete from ", "allow_anonymous",
        "disable_auth", "verify=false", "ssl_verify=false", "check_hostname = false",
    ];
    let breaking_terms = ["breaking change", "remove endpoint", "deprecated=false"];

    let required = destructive_terms.iter().any(|t| diff.contains(t));
    if breaking_terms.iter().any(|t| diff.contains(t)) {
        recommended = true;
    }

    if required {
        println!("SENTRITH-REVIEW-HINT: REVIEW-REQUIRED candidate; inspect independent authorization.");
    } else if recommended {
        println!("SENTRITH-REVIEW-HINT: REVIEW-RECOMMENDED candidate; inspect high-impact scope.");
    } else {
        println!("SENTRITH-REVIEW-HINT: no obvious high-impact pattern detected.");
    }
    Ok(())
}

fn diff_budget() -> Result<(), String> {
    let files = changed_files();
    let mut added: u64 = 0;
    let mut deleted: u64 = 0;

    let numstat = format!(
        "{}\n{}",
        git(&["diff", "--numstat"]),
        git(&["diff", "--cached", "--numstat"])
    );

    for line in numstat.lines() {
        let mut parts = line.split('\t');
        let a = parts.next().unwrap_or("");
        let d = parts.next().unwrap_or("");
        let _path = parts.next().unwrap_or("");
        if let (Ok(aa), Ok(dd)) = (a.parse::<u64>(), d.parse::<u64>()) {
            added += aa;
            deleted += dd;
        }
    }

    let changed_lines = added + deleted;
    let mut flags = Vec::new();
    if files.len() > 50 {
        flags.push(format!("{} files", files.len()));
    }
    if changed_lines > 1500 {
        flags.push(format!("{changed_lines} changed lines"));
    }

    if flags.is_empty() {
        println!(
            "SENTRITH-DIFF-BUDGET: {} files, {} changed lines.",
            files.len(),
            changed_lines
        );
    } else {
        println!(
            "SENTRITH-DIFF-BUDGET: large diff ({}); consider splitting unrelated work and broaden verification.",
            flags.join(", ")
        );
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// hooks install
// ---------------------------------------------------------------------------

/// Sentrith owns hooks whose command starts with one of the executable forms it
/// installs. Matching only the command token keeps an unrelated path such as
/// /workspace/sentrith/scripts/run-linter in the user's settings.
fn first_shell_token(command: &str) -> Option<String> {
    let mut token = String::new();
    let mut quote = None;

    for ch in command.chars() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                // Keep backslashes inside quotes. This matters for Windows
                // paths such as "C:\\Program Files\\sentrith.exe".
                token.push(ch);
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            c if c.is_whitespace() || matches!(c, ';' | '&' | '|') => break,
            _ => token.push(ch),
        }
    }
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

fn is_sentrith_command(cmd: &str) -> bool {
    let Some(token) = first_shell_token(cmd) else {
        return false;
    };
    let normalized = token.replace('\\', "/").to_ascii_lowercase();
    normalized == "sentrith"
        || normalized == "sentrith.exe"
        || normalized == "bin/sentrith"
        || normalized == "bin/sentrith.exe"
        || normalized.ends_with("/bin/sentrith")
        || normalized.ends_with("/bin/sentrith.exe")
}

fn count_sentrith_hooks(value: &Json) -> usize {
    match value {
        Json::Obj(entries) => entries
            .iter()
            .map(|(key, value)| {
                let own = usize::from(
                    key == "command"
                        && value
                            .as_str()
                            .map(is_sentrith_command)
                            .unwrap_or(false),
                );
                own + count_sentrith_hooks(value)
            })
            .sum(),
        Json::Arr(items) => items.iter().map(count_sentrith_hooks).sum(),
        _ => 0,
    }
}

fn is_usage_hook_command(cmd: &str, agent: &str) -> bool {
    let segments = shell_command_segments(cmd);
    let Some((tokens, _)) = segments.first() else {
        return false;
    };
    segments.len() == 1
        && tokens.first().map(|token| is_sentrith_command(token)).unwrap_or(false)
        && tokens.get(1).map(String::as_str) == Some("usage")
        && tokens.get(2).map(String::as_str) == Some("hook")
        && tokens.get(3).map(String::as_str) == Some(agent)
}

fn count_usage_hooks(value: &Json, agent: &str) -> usize {
    match value {
        Json::Obj(entries) => entries
            .iter()
            .map(|(key, value)| {
                let own = usize::from(
                    key == "command"
                        && value
                            .as_str()
                            .map(|command| is_usage_hook_command(command, agent))
                            .unwrap_or(false),
                );
                own + count_usage_hooks(value, agent)
            })
            .sum(),
        Json::Arr(items) => items.iter().map(|item| count_usage_hooks(item, agent)).sum(),
        _ => 0,
    }
}

fn sentrith_hook_count(text: &str) -> usize {
    let Ok(settings) = json_parse(text) else {
        return 0;
    };
    settings
        .get("hooks")
        .map(count_sentrith_hooks)
        .unwrap_or(0)
}

fn sentrith_usage_hook_count(text: &str, agent: &str) -> usize {
    let Ok(settings) = json_parse(text) else {
        return 0;
    };
    settings
        .get("hooks")
        .map(|hooks| count_usage_hooks(hooks, agent))
        .unwrap_or(0)
}

fn hook_target_matches_agent(target_agent: &str, requested_agent: Option<&str>) -> bool {
    requested_agent.map_or(true, |requested| requested == target_agent)
}

/// Rewrite the example's ./bin/sentrith invocation for the current platform.
/// On native Windows, hook commands run through cmd.exe, where ./bin/...
/// does not resolve.
fn platform_command(cmd: &str) -> String {
    if cfg!(windows) {
        cmd.replace("./bin/sentrith", "bin\\sentrith.exe")
    } else {
        cmd.to_string()
    }
}

fn map_commands(v: &mut Json) {
    match v {
        Json::Obj(entries) => {
            for (k, val) in entries.iter_mut() {
                if k == "command" {
                    if let Json::Str(s) = val {
                        *s = platform_command(s);
                    }
                } else {
                    map_commands(val);
                }
            }
        }
        Json::Arr(items) => {
            for item in items {
                map_commands(item);
            }
        }
        _ => {}
    }
}

/// Drop Sentrith-owned entries from a hooks object, leaving other tools' hooks
/// in place. Empty matcher groups and empty events are removed so repeated
/// installs do not accumulate husks.
/// Remove hook entries whose `command` matches `predicate`, then drop any
/// matcher group and any event left with no hooks. Shared by
/// `strip_sentrith_hooks` (drop all Sentrith entries, for idempotent
/// reinstall) and `is_workflow_check_command` (drop only the advisory
/// checks, keeping usage capture, for baseline measurement).
fn strip_hooks_matching(hooks: &mut Json, predicate: impl Fn(&str) -> bool) {
    let Json::Obj(events) = hooks else { return };
    for (_, groups) in events.iter_mut() {
        if let Json::Arr(group_list) = groups {
            for group in group_list.iter_mut() {
                if let Some(Json::Arr(inner)) = group.get("hooks").cloned() {
                    let kept: Vec<Json> = inner
                        .into_iter()
                        .filter(|h| {
                            !h.get("command")
                                .and_then(|c| c.as_str())
                                .map(&predicate)
                                .unwrap_or(false)
                        })
                        .collect();
                    group.set("hooks", Json::Arr(kept));
                }
            }
            group_list.retain(|g| !matches!(g.get("hooks"), Some(Json::Arr(v)) if v.is_empty()));
        }
    }
    events.retain(|(_, groups)| !matches!(groups, Json::Arr(v) if v.is_empty()));
}

fn strip_sentrith_hooks(hooks: &mut Json) {
    strip_hooks_matching(hooks, is_sentrith_command);
}

/// Sentrith's SessionStart/Stop advisory checks (preflight, closeout-check,
/// guard, review-hint, diff-budget). They print Sentrith-flavored text into
/// the agent's context on every turn, which would contaminate a baseline
/// session that is supposed to measure work without Sentrith active. This is
/// deliberately narrower than `is_sentrith_command`: usage capture
/// (`usage hook ...`) must keep running during a baseline, since it is what
/// records the baseline turns at all.
const WORKFLOW_CHECK_SUBCOMMANDS: &[&str] =
    &["preflight", "closeout-check", "guard", "review-hint", "diff-budget"];

fn is_workflow_check_command(cmd: &str) -> bool {
    if !is_sentrith_command(cmd) {
        return false;
    }
    let Some((tokens, _)) = shell_command_segments(cmd).into_iter().next() else {
        return false;
    };
    tokens
        .get(1)
        .map(|sub| WORKFLOW_CHECK_SUBCOMMANDS.contains(&sub.as_str()))
        .unwrap_or(false)
}

/// Append the example's Sentrith groups into the target hooks object.
fn merge_sentrith_hooks(target: &mut Json, source: &Json) -> usize {
    let Json::Obj(src_events) = source else { return 0 };
    let mut added = 0;
    for (event, src_groups) in src_events {
        let Json::Arr(src_list) = src_groups else { continue };
        let mut owned: Vec<Json> = Vec::new();
        for group in src_list {
            if let Some(Json::Arr(inner)) = group.get("hooks") {
                let sentrith_only: Vec<Json> = inner
                    .iter()
                    .filter(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map(is_sentrith_command)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();
                if !sentrith_only.is_empty() {
                    added += sentrith_only.len();
                    let mut g = group.clone();
                    g.set("hooks", Json::Arr(sentrith_only));
                    owned.push(g);
                }
            }
        }
        if owned.is_empty() {
            continue;
        }
        match target.get(event).cloned() {
            Some(Json::Arr(mut existing)) => {
                existing.extend(owned);
                target.set(event, Json::Arr(existing));
            }
            _ => target.set(event, Json::Arr(owned)),
        }
    }
    added
}

struct HookTarget {
    agent: &'static str,
    example: &'static str,
    settings: &'static str,
    /// Claude keeps hooks inside settings.json alongside unrelated settings and
    /// also supports statusLine; Codex uses a dedicated hooks file.
    with_status_line: bool,
}

const HOOK_TARGETS: &[HookTarget] = &[
    HookTarget {
        agent: "claude",
        example: ".claude/settings.hooks.example.json",
        settings: ".claude/settings.json",
        with_status_line: true,
    },
    HookTarget {
        agent: "codex",
        example: ".codex/hooks.example.json",
        settings: ".codex/hooks.json",
        with_status_line: false,
    },
];

fn hooks_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("hooks requires install or status".into());
    }
    let (opts, _) = parse_options(&args[1..])?;
    match args[0].as_str() {
        "install" => hooks_install(&opts),
        "status" => hooks_status(&opts),
        x => Err(format!("unknown hooks subcommand: {x}")),
    }
}

fn selected_targets(opts: &BTreeMap<String, String>) -> Vec<&'static HookTarget> {
    let want = opts.get("agent").map(String::as_str).unwrap_or("all");
    HOOK_TARGETS
        .iter()
        .filter(|t| want == "all" || want == t.agent)
        .collect()
}

fn hooks_status(opts: &BTreeMap<String, String>) -> Result<(), String> {
    for t in selected_targets(opts) {
        let path = repo_file(t.settings);
        if !path.exists() {
            println!("SENTRITH-HOOKS [{}]: not installed ({} missing)", t.agent, t.settings);
            continue;
        }
        let n = sentrith_usage_hook_count(&read_text(&path), t.agent);
        if n == 0 {
            println!("SENTRITH-HOOKS [{}]: {} exists but has no capture hook", t.agent, t.settings);
        } else {
            println!("SENTRITH-HOOKS [{}]: installed ({} capture hooks in {})", t.agent, n, t.settings);
        }
    }
    Ok(())
}

#[cfg(not(windows))]
fn copy_file_permissions(original: &Path, replacement: &Path) -> Result<(), String> {
    let permissions = fs::metadata(original)
        .map_err(|e| e.to_string())?
        .permissions();
    fs::set_permissions(replacement, permissions).map_err(|e| e.to_string())
}

/// Replace `destination` with `replacement`'s content. When `backup_destination`
/// is given, a copy of `destination`'s pre-replacement content is written
/// there first, carrying as much of `destination`'s security metadata as
/// this platform's tooling supports -- on Windows that copy is made by the OS
/// as part of the same atomic `ReplaceFileW` call and is documented to carry
/// the original's full security descriptor, which this project has no other
/// way to reproduce for a brand-new file without a new dependency; elsewhere
/// it is a plain copy plus the mode bits `copy_file_permissions` already
/// knows how to carry over.
#[cfg(not(windows))]
fn replace_file_preserving_security(
    replacement: &Path,
    destination: &Path,
    backup_destination: Option<&Path>,
) -> Result<(), String> {
    if let Some(backup) = backup_destination {
        fs::copy(destination, backup).map_err(|e| e.to_string())?;
        copy_file_permissions(destination, backup)?;
    }
    fs::rename(replacement, destination).map_err(|e| e.to_string())
}

/// `ReplaceFileW` has already committed (the swap itself succeeded) by the
/// time a caller might see this Err -- it's only reached when a step *after*
/// that succeeded call fails. Callers of `replace_file_preserving_security`
/// rely on `Err` meaning nothing changed (e.g. `baseline_start`'s rollback
/// only restores hook edits it recorded as `Ok(true)`; a path that returns
/// `Err` here despite the replacement having actually happened would be
/// skipped by that rollback, which could then delete the only backup while
/// leaving the live file reduced). Best-effort restores `destination`'s
/// content from `backup` before surfacing the original error, so the
/// function's contract holds even when this specific step fails. Without a
/// backup to roll back from, the error is annotated instead: there is
/// nothing this function can do to undo the swap.
#[cfg(windows)]
fn roll_back_committed_replacement(
    destination: &Path,
    backup: Option<&Path>,
    original_error: String,
) -> String {
    let Some(backup) = backup else {
        return format!(
            "{original_error} (the replacement of {} already committed and could not be rolled back: no backup was requested for this call)",
            destination.display()
        );
    };
    match fs::copy(backup, destination) {
        Ok(_) => format!(
            "{original_error} (the replacement was rolled back from {}; {} is unchanged)",
            backup.display(),
            destination.display()
        ),
        Err(rollback_error) => format!(
            "{original_error} (the replacement already committed and rolling it back from {} also failed: {rollback_error}; {} may not match its backup -- resolve manually)",
            backup.display(),
            destination.display()
        ),
    }
}

#[cfg(windows)]
fn replace_file_preserving_security(
    replacement: &Path,
    destination: &Path,
    backup_destination: Option<&Path>,
) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;

    const FILE_ATTRIBUTE_READONLY: u32 = 0x1;
    const INVALID_FILE_ATTRIBUTES: u32 = u32::MAX;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileAttributesW(file_name: *const u16) -> u32;
        fn GetLastError() -> u32;
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
        fn SetFileAttributesW(file_name: *const u16, attributes: u32) -> i32;
    }

    let replaced: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let replacement: Vec<u16> = replacement
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let backup_wide: Vec<u16> = backup_destination
        .map(|p| p.as_os_str().encode_wide().chain(std::iter::once(0)).collect())
        .unwrap_or_default();
    let backup_ptr = if backup_destination.is_some() {
        backup_wide.as_ptr()
    } else {
        std::ptr::null()
    };
    let original_attributes = unsafe { GetFileAttributesW(replaced.as_ptr()) };
    if original_attributes == INVALID_FILE_ATTRIBUTES {
        return Err(format!(
            "GetFileAttributesW failed with OS error {}",
            unsafe { GetLastError() }
        ));
    }
    let was_readonly = original_attributes & FILE_ATTRIBUTE_READONLY != 0;
    if was_readonly
        && unsafe {
            SetFileAttributesW(
                replaced.as_ptr(),
                original_attributes & !FILE_ATTRIBUTE_READONLY,
            )
        } == 0
    {
        return Err(format!(
            "SetFileAttributesW failed with OS error {}",
            unsafe { GetLastError() }
        ));
    }

    let ok = unsafe {
        ReplaceFileW(
            replaced.as_ptr(),
            replacement.as_ptr(),
            backup_ptr,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if ok == 0 {
        let error = unsafe { GetLastError() };
        if was_readonly {
            let _ = unsafe { SetFileAttributesW(replaced.as_ptr(), original_attributes) };
        }
        return Err(format!("ReplaceFileW failed with OS error {error}"));
    }

    if was_readonly {
        let new_attributes = unsafe { GetFileAttributesW(replaced.as_ptr()) };
        if new_attributes == INVALID_FILE_ATTRIBUTES {
            let error = format!(
                "GetFileAttributesW failed after replacement with OS error {}",
                unsafe { GetLastError() }
            );
            return Err(roll_back_committed_replacement(destination, backup_destination, error));
        }
        if unsafe {
            SetFileAttributesW(replaced.as_ptr(), new_attributes | FILE_ATTRIBUTE_READONLY)
        } == 0
        {
            let error = format!(
                "SetFileAttributesW failed after replacement with OS error {}",
                unsafe { GetLastError() }
            );
            return Err(roll_back_committed_replacement(destination, backup_destination, error));
        }
    }

    Ok(())
}

/// Create `path` fresh, exclusively, with a DACL granting access only to its
/// owner, SYSTEM, and Administrators -- never the parent directory's
/// inherited ACL, which may be broader than whatever restriction the file
/// this is standing in for (e.g. `live`) actually has. Used for temp files
/// that briefly hold sensitive content before a `ReplaceFileW` swap:
/// `ReplaceFileW` is documented to retain the *destination's* own security
/// descriptor after the swap, so this file's DACL only needs to hold for
/// that window, not to replicate `live`'s exact (possibly complex,
/// inherited) ACL long-term -- a fixed, deliberately narrow ACL is simpler
/// and just as effective for that purpose.
#[cfg(windows)]
fn create_file_owner_only(path: &Path) -> Result<fs::File, String> {
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::FromRawHandle;

    const GENERIC_WRITE: u32 = 0x4000_0000;
    const CREATE_NEW: u32 = 1;
    const FILE_ATTRIBUTE_NORMAL: u32 = 0x80;
    const INVALID_HANDLE_VALUE: isize = -1;

    #[repr(C)]
    struct SecurityAttributes {
        n_length: u32,
        lp_security_descriptor: *mut std::ffi::c_void,
        b_inherit_handle: i32,
    }

    #[link(name = "advapi32")]
    extern "system" {
        fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
            string_security_descriptor: *const u16,
            string_sd_revision: u32,
            security_descriptor: *mut *mut std::ffi::c_void,
            security_descriptor_size: *mut u32,
        ) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn LocalFree(mem: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *const SecurityAttributes,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
    }

    // D:P protects the DACL from inheriting anything from the parent;
    // (A;;FA;;;OW)(A;;FA;;;SY)(A;;FA;;;BA) grants full access to Owner,
    // SYSTEM, and Administrators only -- nobody else.
    let sddl: Vec<u16> = "D:P(A;;FA;;;OW)(A;;FA;;;SY)(A;;FA;;;BA)\0".encode_utf16().collect();
    let mut sd: *mut std::ffi::c_void = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(), 1, &mut sd, std::ptr::null_mut())
    } == 0
    {
        return Err(format!(
            "ConvertStringSecurityDescriptorToSecurityDescriptorW failed with OS error {}",
            unsafe { GetLastError() }
        ));
    }

    let sa = SecurityAttributes {
        n_length: std::mem::size_of::<SecurityAttributes>() as u32,
        lp_security_descriptor: sd,
        b_inherit_handle: 0,
    };
    let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let handle = unsafe {
        CreateFileW(
            path_wide.as_ptr(),
            GENERIC_WRITE,
            0,
            &sa,
            CREATE_NEW,
            FILE_ATTRIBUTE_NORMAL,
            std::ptr::null_mut(),
        )
    };
    let create_error = unsafe { GetLastError() };
    unsafe { LocalFree(sd) };
    if handle as isize == INVALID_HANDLE_VALUE {
        return Err(format!("CreateFileW failed for {} with OS error {create_error}", path.display()));
    }
    Ok(unsafe { fs::File::from_raw_handle(handle as *mut std::ffi::c_void) })
}

/// Create `path` fresh and exclusively, never exposing whatever gets written
/// to it more broadly than necessary: on Unix, at mode 0600 from the moment
/// `open()` creates it; on Windows, via `create_file_owner_only`'s narrow
/// DACL. Either way, `create_new`/`CREATE_NEW` refuses to reuse an existing
/// file or follow a symlink at this (often predictable) path -- a stale
/// leftover from an interrupted older run, or one placed there in a shared
/// writable repository or pointing somewhere unexpected, is removed first
/// rather than written through. Shared by every caller that stages
/// potentially sensitive content (a full settings file, a backup of one)
/// before either swapping it into place or leaving it as a standing copy.
fn create_secure_file(path: &Path) -> Result<fs::File, String> {
    let _ = fs::remove_file(path);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| format!("failed to prepare {}: {e}", path.display()))
    }
    #[cfg(windows)]
    {
        create_file_owner_only(path)
    }
}

/// Write `content` to a securely created `tmp` (see `create_secure_file`).
fn write_secure_temp_file(tmp: &Path, content: &str) -> Result<(), String> {
    use std::io::Write;
    let mut file = create_secure_file(tmp)?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("failed to prepare {}: {e}", tmp.display()))
}

fn hooks_install(opts: &BTreeMap<String, String>) -> Result<(), String> {
    let dry_run = opts.contains_key("dry-run");
    let mut touched = 0;

    for t in selected_targets(opts) {
        let example_path = repo_file(t.example);
        if !example_path.exists() {
            println!("SENTRITH-HOOKS [{}]: skipped, {} not found", t.agent, t.example);
            continue;
        }
        let mut example = json_parse(&read_text(&example_path))
            .map_err(|e| format!("{}: {e}", t.example))?;
        map_commands(&mut example);

        let settings_path = repo_file(t.settings);
        if settings_path.is_symlink() {
            // Replacing a symlinked settings file (common under dotfile
            // managers) would remove the link and install a plain file at
            // the repository path, permanently disconnecting it from
            // whatever it pointed at -- installation would report success
            // while silently breaking the managed link. Skipped rather than
            // resolved-and-written-through: matches how baseline reduction
            // already refuses a symlinked settings file, and avoids writing
            // through a link to a target this code has no way to vet.
            println!(
                "SENTRITH-HOOKS [{}]: skipped, {} is a symlink (dotfile-managed settings aren't supported by hooks install; edit the link's target directly, or replace the link with a regular file first)",
                t.agent, t.settings
            );
            continue;
        }
        let existed = settings_path.exists();
        let mut settings = if existed {
            // Read fallibly rather than through `read_text`, which folds a
            // read failure (invalid UTF-8, a transient permission error)
            // into the same empty string as a genuinely empty file --
            // silently treating the whole existing configuration as
            // nonexistent and about to be replaced with Sentrith-only
            // settings, discarding whatever it actually held.
            let raw = fs::read_to_string(&settings_path).map_err(|e| {
                format!(
                    "failed to read {}: {e}; fix or move it before running hooks install",
                    t.settings
                )
            })?;
            if raw.trim().is_empty() {
                Json::Obj(Vec::new())
            } else {
                json_parse(&raw).map_err(|e| {
                    format!(
                        "{} is not valid JSON ({e}); fix or move it before running hooks install",
                        t.settings
                    )
                })?
            }
        } else {
            Json::Obj(Vec::new())
        };
        if !matches!(settings, Json::Obj(_)) {
            return Err(format!("{} must contain a JSON object", t.settings));
        }

        let mut hooks = settings.get("hooks").cloned().unwrap_or(Json::Obj(Vec::new()));
        if !matches!(hooks, Json::Obj(_)) {
            return Err(format!("{}: \"hooks\" must be an object", t.settings));
        }
        // Removing our own entries first makes install idempotent and lets an
        // upgrade replace commands that changed between versions.
        strip_sentrith_hooks(&mut hooks);
        let added = match example.get("hooks") {
            Some(src) => merge_sentrith_hooks(&mut hooks, src),
            None => 0,
        };
        if matches!(&hooks, Json::Obj(e) if e.is_empty()) {
            settings.remove("hooks");
        } else {
            settings.set("hooks", hooks);
        }

        let mut status_note = String::new();
        if t.with_status_line {
            if let Some(sl) = example.get("statusLine").cloned() {
                let current = settings.get("statusLine").cloned();
                let ours = current
                    .as_ref()
                    .and_then(|c| c.get("command"))
                    .and_then(|c| c.as_str())
                    .map(is_sentrith_command)
                    .unwrap_or(false);
                match current {
                    None => {
                        settings.set("statusLine", sl);
                        status_note = "; statusLine set".into();
                    }
                    Some(_) if ours => {
                        settings.set("statusLine", sl);
                        status_note = "; statusLine updated".into();
                    }
                    Some(_) => {
                        status_note =
                            "; kept your existing statusLine (cost capture falls back to transcript-only)"
                                .into();
                    }
                }
            }
        }

        let rendered = json_to_string(&settings);
        // Never write output we cannot read back.
        json_parse(&rendered).map_err(|e| format!("internal: produced invalid JSON ({e})"))?;

        if dry_run {
            println!("--- {} (dry run) ---\n{}", t.settings, rendered);
            continue;
        }

        if let Some(parent) = settings_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if existed {
            let backup = settings_path.with_extension("json.sentrith-bak");
            // `fs::copy` opens its destination without refusing to follow an
            // existing symlink there: a `*.json.sentrith-bak` symlink left
            // at this predictable path (or pointing anywhere else) would
            // otherwise get silently overwritten with the settings content
            // instead of the backup itself. `create_secure_file` refuses to
            // reuse or follow anything already at the path; streaming
            // through the handle it returns (rather than a second, separate
            // open via `fs::copy`) means the destination is never reopened
            // by path after that check.
            let mut dest = create_secure_file(&backup).map_err(|e| e.to_string())?;
            let mut src = fs::File::open(&settings_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut src, &mut dest).map_err(|e| e.to_string())?;
            drop(dest);
            #[cfg(not(windows))]
            copy_file_permissions(&settings_path, &backup)?;
        }
        let tmp = settings_path.with_extension("json.sentrith-tmp");
        // `rendered` is the full settings file, including whatever the user
        // already had beyond Sentrith's own hooks; write_secure_temp_file
        // keeps it from sitting exposed under default/inherited permissions
        // even briefly, and never reuses or follows a stale file or symlink
        // left at this predictable path.
        write_secure_temp_file(&tmp, &rendered)?;
        #[cfg(not(windows))]
        if existed {
            copy_file_permissions(&settings_path, &tmp)?;
        }
        if existed {
            replace_file_preserving_security(&tmp, &settings_path, None)?;
        } else {
            fs::rename(&tmp, &settings_path).map_err(|e| e.to_string())?;
        }
        touched += 1;

        println!(
            "SENTRITH-HOOKS [{}]: {} hook(s) installed into {}{}{}",
            t.agent,
            added,
            t.settings,
            status_note,
            if existed { " (backup: *.json.sentrith-bak)" } else { " (created)" }
        );
    }

    if !dry_run && touched > 0 {
        println!("Restart the agent session so it re-reads the settings.");
    }
    Ok(())
}

fn usage_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage requires record, report, or note".into());
    }
    match args[0].as_str() {
        "record" => usage_record(&args[1..]),
        "status" => usage_status(&args[1..]),
        "baseline" => usage_baseline(&args[1..]),
        "run" => usage_run(&args[1..]),
        "hook" => usage_hook(&args[1..]),
        "claude-status" => usage_claude_status(&args[1..]),
        "snapshot" => usage_snapshot(&args[1..]),
        "task" => usage_task(&args[1..]),
        "contribute" => usage_contribute(&args[1..]),
        "aggregate" => usage_aggregate(&args[1..]),
        "report" => usage_report(&args[1..]),
        "publish" => usage_publish(&args[1..]),
        "note" => usage_note(&args[1..]),
        x => Err(format!("unknown usage subcommand: {x}")),
    }
}

fn parse_options(args: &[String]) -> Result<(BTreeMap<String, String>, Vec<String>), String> {
    let mut opts = BTreeMap::new();
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if let Some(key) = args[i].strip_prefix("--") {
            if i + 1 >= args.len() || args[i + 1].starts_with("--") {
                opts.insert(key.to_string(), "true".to_string());
                i += 1;
            } else {
                opts.insert(key.to_string(), args[i + 1].clone());
                i += 2;
            }
        } else {
            positional.push(args[i].clone());
            i += 1;
        }
    }
    Ok((opts, positional))
}

fn require<'a>(opts: &'a BTreeMap<String, String>, key: &str) -> Result<&'a str, String> {
    opts.get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| format!("missing --{key}"))
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn now_unix() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn usage_record(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let agent = require(&opts, "agent")?;
    let task = require(&opts, "task")?;
    if !["codex", "claude", "copilot", "gemini", "other"].contains(&agent) {
        return Err("--agent must be codex, claude, copilot, gemini, or other".into());
    }

    let phase = resolve_phase(opts.get("phase").map(String::as_str));
    let phase = phase.as_str();
    if !["baseline", "standard", "other"].contains(&phase) {
        return Err("--phase (or SENTRITH_PHASE) must be baseline, standard, or other".into());
    }

    let file = opts
        .get("file")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));

    let values = [
        now_unix(),
        agent.to_string(),
        opts.get("model").cloned().unwrap_or_default(),
        phase.to_string(),
        task.to_string(),
        opts.get("input").cloned().unwrap_or_default(),
        opts.get("cached-input").cloned().unwrap_or_default(),
        opts.get("output").cloned().unwrap_or_default(),
        opts.get("credits").cloned().unwrap_or_default(),
        opts.get("cost-usd").cloned().unwrap_or_default(),
        opts.get("tool-calls").cloned().unwrap_or_default(),
        opts.get("duration").cloned().unwrap_or_default(),
        opts.get("success").cloned().unwrap_or_default(),
        opts.get("rework").cloned().unwrap_or_default(),
        opts.get("source").cloned().unwrap_or_else(|| "manual".to_string()),
        opts.get("session-id").cloned().unwrap_or_default(),
        opts.get("notes").cloned().unwrap_or_default(),
        opts.get("head-sha").cloned().unwrap_or_default(),
        opts.get("verification").cloned().unwrap_or_default(),
    ];
    append_usage_row(&file, &values)?;

    println!("SENTRITH-USAGE: recorded {agent} / {phase} / {task} -> {}", file.display());
    Ok(())
}


fn parse_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                out.push(cur);
                cur = String::new();
            }
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out
}

/// Split CSV text into logical records. A quoted field may contain newlines
/// (`csv_escape` produces them), so splitting on physical lines would cut one
/// record in half and corrupt it on the next rewrite.
fn split_csv_records(text: &str) -> Vec<String> {
    let mut records = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                current.push(c);
                if quoted && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    quoted = !quoted;
                }
            }
            '\r' if !quoted => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                records.push(std::mem::take(&mut current));
            }
            '\n' if !quoted => records.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        records.push(current);
    }
    records
}

fn usage_report(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let file = opts
        .get("file")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));
    if !file.exists() {
        println!("SENTRITH-USAGE: no data file: {}", file.display());
        return Ok(());
    }
    if opts.contains_key("churn") {
        return usage_report_churn(&file, &opts);
    }
    if opts.contains_key("tasks") {
        return usage_report_tasks(&file, &opts);
    }

    // Every view is per task. Hook capture writes one row per turn, so
    // summarizing raw rows divides each metric by however many turns happened
    // to precede a commit, which makes phases with different turn counts
    // incomparable.
    let rows = load_usage_rows(&file, opts.get("agent").map(String::as_str))?;
    if rows.is_empty() {
        println!("SENTRITH-USAGE: no matching rows");
        return Ok(());
    }

    let refs: Vec<&BTreeMap<String, String>> = rows.iter().collect();
    let by_phase = tasks_by_phase(&refs);
    if opts.contains_key("compare") {
        let b = phase_summary(by_phase.get("baseline").map(Vec::as_slice).unwrap_or(&[]));
        let s = phase_summary(by_phase.get("standard").map(Vec::as_slice).unwrap_or(&[]));
        print_summary("baseline", &b);
        print_summary("standard", &s);
        println!("\n[standard vs baseline]");
        for name in NUMERIC_COLUMNS {
            println!(
                "{name}: {}",
                pct_text(b.get(name).copied().flatten(), s.get(name).copied().flatten())
            );
        }
        if let (Some(a), Some(bv)) = (
            b.get("success_rate").copied().flatten(),
            s.get("success_rate").copied().flatten(),
        ) {
            println!("success_rate: {:+.1} percentage points", bv - a);
        }
        println!("\nValues are per task; turns of the same task are summed first.");
    } else {
        let mut groups: BTreeMap<(String, String), Vec<Vec<&BTreeMap<String, String>>>> = BTreeMap::new();
        for task in group_tasks(&refs) {
            let r = task.last().copied();
            let key = (
                r.and_then(|r| r.get("agent")).cloned().unwrap_or_default(),
                r.and_then(|r| r.get("phase")).cloned().unwrap_or_default(),
            );
            groups.entry(key).or_default().push(task);
        }
        for ((agent, phase), tasks) in groups {
            let summary = phase_summary(&tasks);
            print_summary(&format!("{agent} / {phase}"), &summary);
        }
        println!("\nValues are per task. Success breakdown: sentrith usage report --tasks");
        println!("Progress and next step: sentrith usage status");
    }
    Ok(())
}

const NUMERIC_COLUMNS: &[&str] = &[
    "input_tokens", "cached_input_tokens", "output_tokens", "credits", "cost_usd",
    "tool_calls", "duration_seconds", "rework_count",
];

/// Per-task averages for one phase or agent group.
fn phase_summary(tasks: &[Vec<&BTreeMap<String, String>>]) -> BTreeMap<&'static str, Option<f64>> {
    let mut out = BTreeMap::new();
    for name in NUMERIC_COLUMNS {
        out.insert(*name, mean(&per_task_values(tasks, name)));
    }
    out.insert("success_rate", decided_success_rate(tasks));
    out.insert("tasks", Some(tasks.len() as f64));
    out
}

/// Group all rows into tasks before assigning a task to its closing phase.
/// A baseline task may close after baseline stop, so filtering rows by phase
/// before calling group_tasks would split one task into two partial tasks.
fn tasks_by_phase<'a>(
    rows: &[&'a BTreeMap<String, String>],
) -> BTreeMap<String, Vec<Vec<&'a BTreeMap<String, String>>>> {
    tasks_by_phase_from_tasks(group_tasks(rows))
}

fn tasks_by_phase_from_tasks<'a>(
    tasks: Vec<Vec<&'a BTreeMap<String, String>>>,
) -> BTreeMap<String, Vec<Vec<&'a BTreeMap<String, String>>>> {
    let mut by_phase: BTreeMap<String, Vec<Vec<&'a BTreeMap<String, String>>>> = BTreeMap::new();
    for task in tasks {
        let phase = task
            .last()
            .and_then(|r| r.get("phase"))
            .cloned()
            .unwrap_or_default();
        by_phase.entry(phase).or_default().push(task);
    }
    by_phase
}

/// Apply a model filter after task grouping so a task that changes models is
/// never split into multiple partial tasks. Mixed-model tasks are excluded
/// from model-specific metrics because assigning their totals to one model
/// would be misleading.
fn filter_tasks_by_model<'a>(
    tasks: Vec<Vec<&'a BTreeMap<String, String>>>,
    model: Option<&str>,
) -> Vec<Vec<&'a BTreeMap<String, String>>> {
    let Some(wanted) = model else {
        return tasks;
    };

    tasks
        .into_iter()
        .filter(|task| {
            let models: BTreeSet<String> = task
                .iter()
                .filter_map(|row| row.get("model"))
                .filter(|value| !value.is_empty())
                .cloned()
                .collect();
            models.len() == 1 && models.contains(wanted)
        })
        .collect()
}

/// Group turn rows into tasks: rows without a session id stand alone;
/// within a session, a change of `head_sha` closes the current task at the
/// row that observed the new commit.
fn group_tasks<'a>(rows: &[&'a BTreeMap<String, String>]) -> Vec<Vec<&'a BTreeMap<String, String>>> {
    let mut tasks: Vec<Vec<&'a BTreeMap<String, String>>> = Vec::new();
    let mut open: BTreeMap<String, Vec<&'a BTreeMap<String, String>>> = BTreeMap::new();
    for row in rows.iter().copied() {
        let sid = row.get("session_id").cloned().unwrap_or_default();
        if sid.is_empty() {
            tasks.push(vec![row]);
            continue;
        }
        open.entry(sid.clone()).or_default().push(row);

        // `head_sha` is written only for a turn that produced a commit, so any
        // value closes the task. Comparing against a previous SHA instead would
        // miss a session whose first captured turn commits — including the case
        // where no test ran and the outcome is `unknown`, which carries no other
        // signal that a task ended.
        let committed = !row.get("head_sha").map(String::as_str).unwrap_or("").is_empty();
        // Manual ledger rows may carry an outcome without a SHA.
        let decided = matches!(
            row.get("success").map(String::as_str).unwrap_or(""),
            "yes" | "no"
        );

        if committed || decided {
            if let Some(g) = open.remove(&sid) {
                tasks.push(g);
            }
        }
    }
    for (_, g) in open {
        if !g.is_empty() {
            tasks.push(g);
        }
    }
    tasks
}

fn usage_report_tasks(file: &Path, opts: &BTreeMap<String, String>) -> Result<(), String> {
    let rows = load_usage_rows(file, opts.get("agent").map(String::as_str))?;
    if rows.is_empty() {
        println!("SENTRITH-USAGE: no matching rows");
        return Ok(());
    }
    let refs: Vec<&BTreeMap<String, String>> = rows.iter().collect();
    let by_phase = tasks_by_phase(&refs);
    for (phase, group) in by_phase {
        let mut yes = 0usize;
        let mut no = 0usize;
        let mut unknown = 0usize;
        let mut cost = 0.0;
        let mut cost_n = 0usize;
        let mut tokens = 0.0;
        let mut tokens_n = 0usize;
        let mut turns = 0usize;
        for t in &group {
            turns += t.len();
            match t.last().copied().and_then(|r| r.get("success")).map(String::as_str).unwrap_or("") {
                "yes" => yes += 1,
                "no" => no += 1,
                _ => unknown += 1,
            }
            let refs: Vec<&BTreeMap<String, String>> = t.iter().copied().collect();
            if let Some(c) = sum_field(&refs, "cost_usd") {
                cost += c;
                cost_n += 1;
            }
            let i = sum_field(&refs, "input_tokens").unwrap_or(0.0);
            let o = sum_field(&refs, "output_tokens").unwrap_or(0.0);
            if i > 0.0 || o > 0.0 {
                tokens += i + o;
                tokens_n += 1;
            }
        }
        let n = group.len();
        println!("\n[tasks: {phase}]");
        println!("tasks: {n} (turns: {turns})");
        println!("success: yes={yes} no={no} unknown={unknown}");
        if yes + no > 0 {
            println!(
                "success rate (yes/(yes+no)): {:.1}%",
                yes as f64 / (yes + no) as f64 * 100.0
            );
        } else {
            println!("success rate: - (no decided tasks)");
        }
        if cost_n > 0 {
            println!("avg cost USD / task: {:.4}", cost / cost_n as f64);
            if yes > 0 {
                println!("cost USD / successful task: {:.4}", cost / yes as f64);
            }
        }
        if tokens_n > 0 {
            println!("avg tokens / task: {:.0}", tokens / tokens_n as f64);
        }
    }
    println!("\nSuccess uses objective proxies (commit reached + last recorded test outcome); unknown tasks are excluded from the rate.");
    Ok(())
}

/// One-screen answer to "am I measuring, and how far am I from a number?".
fn usage_status(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let min: usize = opts.get("min-samples").and_then(|x| x.parse().ok()).unwrap_or(5);
    let file = opts
        .get("file")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));

    println!("== capture ==");
    let mut hooks_ready = false;
    let requested_agent = opts.get("agent").map(String::as_str);
    for t in HOOK_TARGETS {
        let path = repo_file(t.settings);
        let installed =
            path.exists() && sentrith_usage_hook_count(&read_text(&path), t.agent) > 0;
        if installed && hook_target_matches_agent(t.agent, requested_agent) {
            hooks_ready = true;
        }
        println!(
            "{:<7} {}",
            t.agent,
            if installed { "hooks installed" } else { "hooks NOT installed" }
        );
    }

    println!("\n== phase ==");
    if baseline_active() {
        println!("baseline mode ACTIVE (contract stashed); turns record phase=baseline");
    } else {
        println!("recording phase: {}", resolve_phase(None));
    }

    println!("\n== data ==");
    if !file.exists() {
        println!("no usage file yet: {}", file.display());
        println!("\n== next ==");
        if !hooks_ready {
            println!("1. sentrith hooks install");
            println!("2. sentrith usage baseline start   (measure without the contract first)");
        } else {
            println!("1. sentrith usage baseline start   (measure without the contract first)");
        }
        return Ok(());
    }

    let rows = load_usage_rows(&file, opts.get("agent").map(String::as_str))?;
    let refs: Vec<&BTreeMap<String, String>> = rows.iter().collect();
    let by_phase = tasks_by_phase(&refs);
    let mut counts: BTreeMap<String, (usize, usize, usize, usize)> = BTreeMap::new();
    for (phase, tasks) in by_phase {
        let mut e = (tasks.len(), 0, 0, 0);
        for t in tasks {
            match t.last().and_then(|r| r.get("success")).map(String::as_str).unwrap_or("") {
                "yes" => e.1 += 1,
                "no" => e.2 += 1,
                _ => e.3 += 1,
            }
        }
        counts.insert(phase, e);
    }
    println!("turns recorded: {}", rows.len());
    for (phase, (n, yes, no, unknown)) in &counts {
        println!("{phase:<9} tasks: {n:<4} (yes {yes}, no {no}, unknown {unknown})");
    }

    let base_n = counts.get("baseline").map(|c| c.0).unwrap_or(0);
    let std_n = counts.get("standard").map(|c| c.0).unwrap_or(0);

    println!("\n== next ==");
    if base_n < min {
        println!(
            "collect {} more baseline task(s) ({}/{}).",
            min - base_n,
            base_n,
            min
        );
        if !baseline_active() {
            println!("  sentrith usage baseline start");
        }
    } else if std_n < min {
        if baseline_active() {
            println!("baseline is complete ({base_n}/{min}). Restore the contract:");
            println!("  sentrith usage baseline stop");
        } else {
            println!(
                "collect {} more standard task(s) ({}/{}); just keep working normally.",
                min - std_n,
                std_n,
                min
            );
        }
    } else {
        println!("comparable ({base_n} baseline / {std_n} standard). Compare with:");
        println!("  sentrith usage report --compare");
        println!("  sentrith usage report --tasks");
        println!("  sentrith usage report --churn");
    }

    let decided: usize = counts.values().map(|c| c.1 + c.2).sum();
    let unknown: usize = counts.values().map(|c| c.3).sum();
    if unknown > decided && unknown > 0 {
        println!(
            "\nNote: {unknown} task(s) are `unknown` (no commit, or no test run observed)."
        );
        println!("Success rate uses decided tasks only; commit your work so tasks can be scored.");
    }
    Ok(())
}

/// One item recovered from `git ... --numstat -z` output: either a commit
/// header's timestamp (only present when the header format is
/// `"COMMIT <unix time>"`) or a file path that commit touched.
///
/// `-z` matters for correctness, not just convenience: without it, a rename
/// with a common path prefix is rendered as a single human-readable field
/// like `old.txt => new.txt` (verified against a real repo), which
/// line-oriented tab-splitting stores as one literal, unmatchable "path" -
/// silently breaking churn tracking across the rename. With `-z`, a rename is
/// added/deleted counts followed by an *empty* NUL-terminated field, then the
/// old and new paths as two further NUL-terminated fields; a non-renamed
/// entry is counts plus a single NUL-terminated path. The destination path is
/// recorded, since that is what a later edit touches.
enum NumstatZItem {
    Commit(f64),
    Path(String),
    /// A rename record. Kept separate from `Path` because the two callers of
    /// `parse_numstat_z` need different halves of it: the measured commit's
    /// file set uses only `new` (that is the file's identity going forward,
    /// and counting both names would double-count one logical file in the
    /// churn denominator); scanning later history for touches must check
    /// both `old` and `new`, since a later commit may rename the measured
    /// file again before ever touching it under the newer name.
    Rename { old: String, new: String },
}

/// Parse `-z`-delimited numstat output, optionally interleaved with
/// `COMMIT <unix time>` header lines from a custom `--format`. Git also
/// inserts a bare newline right after each NUL-terminated header when diff
/// data follows it; that is a formatting artifact of `-z`, not content, and
/// is stripped rather than treated as part of the next field.
fn parse_numstat_z(text: &str) -> Vec<NumstatZItem> {
    let mut items = Vec::new();
    let mut fields = text.split('\0').peekable();
    while let Some(field) = fields.next() {
        let field = field.trim_start_matches('\n');
        if field.is_empty() {
            continue;
        }
        if let Some(rest) = field.strip_prefix("COMMIT ") {
            if let Ok(t) = rest.trim().parse::<f64>() {
                items.push(NumstatZItem::Commit(t));
            }
            continue;
        }
        let mut parts = field.splitn(3, '\t');
        let (Some(_added), Some(_deleted), Some(path)) = (parts.next(), parts.next(), parts.next()) else {
            continue;
        };
        // Binary files report `-\t-\t<path>` instead of numeric counts, but
        // both callers of this parser (the measured commit's file set, and
        // later history's touched-path set) use only the path, never the
        // counts -- so a binary file is exactly as real a churn entry as a
        // text one. Excluding it here would drop it from both sets
        // uniformly: an all-binary commit would show no files at all, and a
        // mixed commit would undercount its denominator, inflating the
        // reported share of files re-modified.
        if path.is_empty() {
            // Rename: the next two NUL-terminated fields are old and new
            // paths. Git does not report renames for binary files as `-`/`-`
            // rows, so this branch is reached only for text renames.
            let Some(old_path) = fields.next() else { continue };
            let Some(new_path) = fields.next() else { continue };
            if !old_path.is_empty() && !new_path.is_empty() {
                items.push(NumstatZItem::Rename {
                    old: old_path.to_string(),
                    new: new_path.to_string(),
                });
            }
        } else {
            items.push(NumstatZItem::Path(path.to_string()));
        }
    }
    items
}

/// Whether a commit timestamp `t` counts as later rework of a commit made at
/// `t0`, within `days`. Both bounds matter: `sha..HEAD` selects commits by
/// ancestry, not time, so a side-branch commit merged in later but authored
/// before `t0` must not be treated as rework that happened after it.
fn within_churn_window(t: f64, t0: f64, days: f64) -> bool {
    t >= t0 && t <= t0 + days * 86400.0
}

/// File-level churn: how many files of `sha` were modified again by later
/// commits within `days`. A rework proxy computable retroactively from git.
fn churn_for_commit(sha: &str, days: f64) -> Option<(usize, usize)> {
    // A rename in the measured commit records only its destination: that is
    // the file's identity going forward, and counting the old name too would
    // count one logical file twice in the denominator below.
    let files: BTreeSet<String> = parse_numstat_z(&git(&["show", "--numstat", "-z", "--format=", sha]))
        .into_iter()
        .filter_map(|item| match item {
            NumstatZItem::Path(p) => Some(p),
            NumstatZItem::Rename { new, .. } => Some(new),
            NumstatZItem::Commit(_) => None,
        })
        .collect();
    if files.is_empty() {
        return None;
    }
    let t0: f64 = git(&["show", "-s", "--format=%ct", sha]).trim().parse().ok()?;
    let range = format!("{sha}..HEAD");
    let log = git(&["log", "--numstat", "-z", "--format=COMMIT %ct", range.as_str()]);
    let mut touched = BTreeSet::new();
    let mut in_window = false;
    for item in parse_numstat_z(&log) {
        match item {
            NumstatZItem::Commit(t) => in_window = within_churn_window(t, t0, days),
            NumstatZItem::Path(p) => {
                if in_window {
                    touched.insert(p);
                }
            }
            // A later rename touches the file under both names: the measured
            // commit's file set may hold either one, depending on whether the
            // measured commit itself renamed the file.
            NumstatZItem::Rename { old, new } => {
                if in_window {
                    touched.insert(old);
                    touched.insert(new);
                }
            }
        }
    }
    let changed = files.iter().filter(|f| touched.contains(f.as_str())).count();
    Some((changed, files.len()))
}

fn usage_report_churn(file: &Path, opts: &BTreeMap<String, String>) -> Result<(), String> {
    let days: f64 = opts.get("days").and_then(|x| x.parse().ok()).unwrap_or(14.0);
    let rows = load_usage_rows(file, opts.get("agent").map(String::as_str))?;
    // A recorded `head_sha` is by construction a commit observed during that
    // turn, so every one counts. Requiring a transition from a previous SHA
    // would drop each session's first commit, including sessions that contain
    // exactly one.
    let mut done: BTreeSet<String> = BTreeSet::new();
    let mut per_phase: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for r in &rows {
        let sha = r.get("head_sha").cloned().unwrap_or_default();
        if sha.is_empty() || !done.insert(sha.clone()) {
            continue;
        }
        if let Some((changed, total)) = churn_for_commit(&sha, days) {
            let phase = r.get("phase").cloned().unwrap_or_default();
            per_phase
                .entry(phase)
                .or_default()
                .push(changed as f64 / total as f64 * 100.0);
        }
    }
    if per_phase.is_empty() {
        println!("SENTRITH-CHURN: no recorded commits found in usage data");
        return Ok(());
    }
    for (phase, rates) in per_phase {
        let avg = rates.iter().sum::<f64>() / rates.len() as f64;
        println!(
            "SENTRITH-CHURN [{phase}]: commits={} avg share of files re-modified within {:.0} days: {:.1}%",
            rates.len(),
            days,
            avg
        );
    }
    println!("File-level churn is a rework proxy computed retroactively from git history.");
    Ok(())
}

fn print_summary(label: &str, s: &BTreeMap<&'static str, Option<f64>>) {
    println!("\n[{label}]");
    println!("tasks: {}", fmt(s.get("tasks").copied().flatten()));
    println!("success rate: {}%", fmt(s.get("success_rate").copied().flatten()));
    println!("avg input tokens: {}", fmt(s.get("input_tokens").copied().flatten()));
    println!("avg cached input: {}", fmt(s.get("cached_input_tokens").copied().flatten()));
    println!("avg output tokens: {}", fmt(s.get("output_tokens").copied().flatten()));
    println!("avg credits: {}", fmt(s.get("credits").copied().flatten()));
    println!("avg cost USD: {}", fmt(s.get("cost_usd").copied().flatten()));
    println!("avg tool calls: {}", fmt(s.get("tool_calls").copied().flatten()));
    println!("avg duration sec: {}", fmt(s.get("duration_seconds").copied().flatten()));
    println!("avg rework count: {}", fmt(s.get("rework_count").copied().flatten()));
}

fn fmt(v: Option<f64>) -> String {
    match v {
        None => "-".to_string(),
        Some(x) if x.abs() >= 1000.0 => format!("{x:.1}"),
        Some(x) => format!("{x:.2}"),
    }
}

fn pct_text(base: Option<f64>, standard: Option<f64>) -> String {
    match (base, standard) {
        (Some(a), Some(b)) if a != 0.0 => format!("{:+.1}%", (b - a) / a * 100.0),
        _ => "-".to_string(),
    }
}


#[derive(Default, Clone)]
struct PublishStats {
    tasks: usize,
    successes: usize,
    success_rate: Option<f64>,
    credits_avg: Option<f64>,
    tool_calls_avg: Option<f64>,
    rework_avg: Option<f64>,
    input_avg: Option<f64>,
    cached_input_avg: Option<f64>,
    output_avg: Option<f64>,
    duration_avg: Option<f64>,
    total_credits: Option<f64>,
    credits_per_success: Option<f64>,
}

fn load_usage_rows(path: &Path, agent_filter: Option<&str>) -> Result<Vec<BTreeMap<String, String>>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let records = split_csv_records(&text);
    let mut records = records.iter();
    let header = records.next().ok_or("usage file is empty")?;
    let headers = parse_csv_line(header);
    let mut rows = Vec::new();

    for line in records {
        if line.trim().is_empty() { continue; }
        let cols = parse_csv_line(line);
        let mut row = BTreeMap::new();
        for (i, h) in headers.iter().enumerate() {
            row.insert(h.clone(), cols.get(i).cloned().unwrap_or_default());
        }
        if let Some(a) = agent_filter {
            if row.get("agent").map(String::as_str).unwrap_or("") != a { continue; }
        }
        rows.push(row);
    }
    Ok(rows)
}

fn avg_field(rows: &[&BTreeMap<String, String>], key: &str) -> Option<f64> {
    let vals: Vec<f64> = rows.iter()
        .filter_map(|r| r.get(key))
        .filter_map(|s| if s.is_empty() { None } else { s.parse::<f64>().ok() })
        .collect();
    if vals.is_empty() { None } else { Some(vals.iter().sum::<f64>() / vals.len() as f64) }
}

fn sum_field(rows: &[&BTreeMap<String, String>], key: &str) -> Option<f64> {
    let vals: Vec<f64> = rows.iter()
        .filter_map(|r| r.get(key))
        .filter_map(|s| if s.is_empty() { None } else { s.parse::<f64>().ok() })
        .collect();
    if vals.is_empty() { None } else { Some(vals.iter().sum::<f64>()) }
}

/// A task's outcome is the outcome of the turn that closed it.
fn task_success(task: &[&BTreeMap<String, String>]) -> &'static str {
    match task.last().and_then(|r| r.get("success")).map(String::as_str) {
        Some("yes") => "yes",
        Some("no") => "no",
        _ => "unknown",
    }
}

/// Success rate over decided tasks only (`yes` / (`yes` + `no`)).
/// `unknown`/blank tasks are undecidable evidence, not failures.
fn decided_success_rate(tasks: &[Vec<&BTreeMap<String, String>>]) -> Option<f64> {
    let yes = tasks.iter().filter(|t| task_success(t) == "yes").count();
    let no = tasks.iter().filter(|t| task_success(t) == "no").count();
    if yes + no == 0 { None } else { Some(yes as f64 / (yes + no) as f64 * 100.0) }
}

/// Per-task totals for one column: turns of the same task are summed first, so
/// averaging afterwards yields a value per task rather than per turn.
/// A manual record has no session id and forms its own task, so single-row
/// behavior is unchanged.
fn per_task_values(tasks: &[Vec<&BTreeMap<String, String>>], key: &str) -> Vec<f64> {
    tasks.iter().filter_map(|t| sum_field(t, key)).collect()
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn publish_stats_for_tasks(tasks: &[Vec<&BTreeMap<String, String>>]) -> PublishStats {
    let successes = tasks.iter().filter(|t| task_success(t) == "yes").count();
    let success_rate = decided_success_rate(tasks);
    let all_rows: Vec<&BTreeMap<String, String>> =
        tasks.iter().flat_map(|task| task.iter().copied()).collect();
    let total_credits = sum_field(&all_rows, "credits");
    let credits_per_success = match (total_credits, successes) {
        (Some(c), n) if n > 0 => Some(c / n as f64),
        _ => None,
    };

    PublishStats {
        tasks: tasks.len(),
        successes,
        success_rate,
        credits_avg: mean(&per_task_values(tasks, "credits")),
        tool_calls_avg: mean(&per_task_values(tasks, "tool_calls")),
        rework_avg: mean(&per_task_values(tasks, "rework_count")),
        input_avg: mean(&per_task_values(tasks, "input_tokens")),
        cached_input_avg: mean(&per_task_values(tasks, "cached_input_tokens")),
        output_avg: mean(&per_task_values(tasks, "output_tokens")),
        duration_avg: mean(&per_task_values(tasks, "duration_seconds")),
        total_credits,
        credits_per_success,
    }
}

fn publish_stats(rows: &[&BTreeMap<String, String>]) -> PublishStats {
    // Hook capture writes one row per turn; task-level statistics must be
    // computed over tasks, not turns, or every metric is divided by the number
    // of turns that happened to precede the commit.
    publish_stats_for_tasks(&group_tasks(rows))
}

fn change_text(base: Option<f64>, standard: Option<f64>, suffix: &str) -> String {
    match (base, standard) {
        (Some(a), Some(b)) if a != 0.0 => format!("{:+.1}%{}", (b-a)/a*100.0, suffix),
        _ => "-".to_string(),
    }
}

fn value_or_dash(v: Option<f64>, digits: usize) -> String {
    match v {
        Some(x) => format!("{:.*}", digits, x),
        None => "-".to_string(),
    }
}

fn replace_marked_section(path: &Path, begin: &str, end: &str, body: &str) -> Result<(), String> {
    let current = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let start = current.find(begin).ok_or_else(|| format!("marker missing in {}: {begin}", path.display()))?;
    let after_start = start + begin.len();
    let rel_end = current[after_start..].find(end).ok_or_else(|| format!("marker missing in {}: {end}", path.display()))?;
    let finish = after_start + rel_end;
    let mut updated = String::new();
    updated.push_str(&current[..after_start]);
    updated.push('\n');
    updated.push_str(body.trim());
    updated.push('\n');
    updated.push_str(&current[finish..]);
    fs::write(path, updated).map_err(|e| e.to_string())
}

fn usage_publish(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let file = opts.get("file").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));
    if !file.exists() {
        return Err(format!("usage file not found: {}", file.display()));
    }

    let agent = require(&opts, "agent")?;
    let model = opts.get("model").map(String::as_str);
    let date = opts.get("date").cloned().unwrap_or_else(now_unix);
    let task_mix = opts.get("task-mix").cloned().unwrap_or_else(|| "mixed engineering tasks".to_string());
    let min_samples: usize = opts.get("min-samples")
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    let force = opts.contains_key("force");
    let dry_run = opts.contains_key("dry-run");

    let rows = load_usage_rows(&file, Some(agent))?;
    let refs: Vec<&BTreeMap<String, String>> = rows.iter().collect();
    let tasks = filter_tasks_by_model(group_tasks(&refs), model);
    let by_phase = tasks_by_phase_from_tasks(tasks);
    let baseline_tasks = by_phase.get("baseline").map(Vec::as_slice).unwrap_or(&[]);
    let standard_tasks = by_phase.get("standard").map(Vec::as_slice).unwrap_or(&[]);

    // Qualify on the same task counts the table publishes. Counting raw turns
    // would let five turns of one task pass a five-sample threshold and publish
    // a one-task benchmark with no small-sample warning.
    let b = publish_stats_for_tasks(baseline_tasks);
    let s = publish_stats_for_tasks(standard_tasks);

    if !force && (b.tasks < min_samples || s.tasks < min_samples) {
        return Err(format!(
            "refusing README publication: baseline={} standard={} task(s) but min-samples={}. Add more data or pass --force.",
            b.tasks, s.tasks, min_samples
        ));
    }

    let model_name = model.unwrap_or("mixed/unspecified");
    let measured_note_ja = if force && (b.tasks < min_samples || s.tasks < min_samples) {
        "⚠️ `--force` で掲載された小規模サンプルです。"
    } else {
        "この表は `.ai-usage/usage.csv` から自動集計した実測値です。"
    };
    let measured_note_en = if force && (b.tasks < min_samples || s.tasks < min_samples) {
        "⚠️ Published from a small sample using `--force`."
    } else {
        "This table was generated automatically from `.ai-usage/usage.csv`."
    };

    let ja_body = format!(r#"
### 実測結果

{measured_note_ja}

測定条件:

- Agent: `{agent}`
- Model: `{model_name}`
- Baseline: {bn} tasks
- Standard: {sn} tasks
- Task mix: {task_mix}
- Measured: {date}

| Metric | Baseline | Standard | Change |
|---|---:|---:|---:|
| Credits / task | {bcred} | {scred} | {ccred} |
| Credits / successful task | {bcps} | {scps} | {ccps} |
| Input tokens / task | {binp} | {sinp} | {cinp} |
| Cached input / task | {bcache} | {scache} | {ccache} |
| Tool calls / task | {btool} | {stool} | {ctool} |
| Rework / task | {brew} | {srew} | {crew} |
| Success rate | {bsucc}% | {ssucc}% | {succ_delta}pp |

詳細: [`docs/metrics/BENCHMARK_GUIDE.ja.md`](docs/metrics/BENCHMARK_GUIDE.ja.md)
"#,
        bn=b.tasks, sn=s.tasks,
        bcred=value_or_dash(b.credits_avg,2), scred=value_or_dash(s.credits_avg,2), ccred=change_text(b.credits_avg,s.credits_avg,""),
        bcps=value_or_dash(b.credits_per_success,2), scps=value_or_dash(s.credits_per_success,2), ccps=change_text(b.credits_per_success,s.credits_per_success,""),
        binp=value_or_dash(b.input_avg,1), sinp=value_or_dash(s.input_avg,1), cinp=change_text(b.input_avg,s.input_avg,""),
        bcache=value_or_dash(b.cached_input_avg,1), scache=value_or_dash(s.cached_input_avg,1), ccache=change_text(b.cached_input_avg,s.cached_input_avg,""),
        btool=value_or_dash(b.tool_calls_avg,2), stool=value_or_dash(s.tool_calls_avg,2), ctool=change_text(b.tool_calls_avg,s.tool_calls_avg,""),
        brew=value_or_dash(b.rework_avg,2), srew=value_or_dash(s.rework_avg,2), crew=change_text(b.rework_avg,s.rework_avg,""),
        bsucc=value_or_dash(b.success_rate,1), ssucc=value_or_dash(s.success_rate,1),
        succ_delta=match (b.success_rate,s.success_rate){(Some(a),Some(bb))=>format!("{:+.1}",bb-a),_=>"-".to_string()},
    );

    let en_body = format!(r#"
### Measured benchmark

{measured_note_en}

Measured with:

- Agent: `{agent}`
- Model: `{model_name}`
- Baseline: {bn} tasks
- Standard: {sn} tasks
- Task mix: {task_mix}
- Date: {date}

| Metric | Baseline | Standard | Change |
|---|---:|---:|---:|
| Credits / task | {bcred} | {scred} | {ccred} |
| Credits / successful task | {bcps} | {scps} | {ccps} |
| Input tokens / task | {binp} | {sinp} | {cinp} |
| Cached input / task | {bcache} | {scache} | {ccache} |
| Tool calls / task | {btool} | {stool} | {ctool} |
| Rework / task | {brew} | {srew} | {crew} |
| Success rate | {bsucc}% | {ssucc}% | {succ_delta}pp |

Details: [`docs/metrics/BENCHMARK_GUIDE.en.md`](docs/metrics/BENCHMARK_GUIDE.en.md)
"#,
        bn=b.tasks, sn=s.tasks,
        bcred=value_or_dash(b.credits_avg,2), scred=value_or_dash(s.credits_avg,2), ccred=change_text(b.credits_avg,s.credits_avg,""),
        bcps=value_or_dash(b.credits_per_success,2), scps=value_or_dash(s.credits_per_success,2), ccps=change_text(b.credits_per_success,s.credits_per_success,""),
        binp=value_or_dash(b.input_avg,1), sinp=value_or_dash(s.input_avg,1), cinp=change_text(b.input_avg,s.input_avg,""),
        bcache=value_or_dash(b.cached_input_avg,1), scache=value_or_dash(s.cached_input_avg,1), ccache=change_text(b.cached_input_avg,s.cached_input_avg,""),
        btool=value_or_dash(b.tool_calls_avg,2), stool=value_or_dash(s.tool_calls_avg,2), ctool=change_text(b.tool_calls_avg,s.tool_calls_avg,""),
        brew=value_or_dash(b.rework_avg,2), srew=value_or_dash(s.rework_avg,2), crew=change_text(b.rework_avg,s.rework_avg,""),
        bsucc=value_or_dash(b.success_rate,1), ssucc=value_or_dash(s.success_rate,1),
        succ_delta=match (b.success_rate,s.success_rate){(Some(a),Some(bb))=>format!("{:+.1}",bb-a),_=>"-".to_string()},
    );

    if dry_run {
        println!("--- README.ja.md benchmark section ---\n{}", ja_body.trim());
        println!("\n--- README.md benchmark section ---\n{}", en_body.trim());
        return Ok(());
    }

    replace_marked_section(
        Path::new("README.ja.md"),
        "<!-- SENTRITH-USAGE-BENCHMARK:BEGIN -->",
        "<!-- SENTRITH-USAGE-BENCHMARK:END -->",
        &ja_body,
    )?;
    replace_marked_section(
        Path::new("README.md"),
        "<!-- SENTRITH-USAGE-BENCHMARK:BEGIN -->",
        "<!-- SENTRITH-USAGE-BENCHMARK:END -->",
        &en_body,
    )?;

    println!(
        "SENTRITH-USAGE: published benchmark to README.md and README.ja.md (baseline={}, standard={})",
        b.tasks, s.tasks
    );
    Ok(())
}



fn json_string_field(s: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let pos = s.find(&needle)?;
    let rest = &s[pos + needle.len()..];
    let colon = rest.find(':')?;
    let mut parser = JsonParser::new(rest[colon + 1..].trim_start());
    match parser.value().ok()? {
        Json::Str(value) => Some(value),
        _ => None,
    }
}

/// The last string element of a JSON array field. Codex's rollout `command`
/// field is `["/bin/bash", "-lc", "<the actual shell text>"]`; the shell text
/// to check against test-command patterns is always the final element.
fn json_string_array_last(s: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let pos = s.find(&needle)?;
    let rest = &s[pos + needle.len()..];
    let colon = rest.find(':')?;
    let mut parser = JsonParser::new(rest[colon + 1..].trim_start());
    match parser.value().ok()? {
        Json::Arr(items) => match items.last()? {
            Json::Str(value) => Some(value.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn json_number_field(s: &str, key: &str) -> Option<f64> {
    let needle = format!("\"{}\"", key);
    let pos = s.find(&needle)?;
    let rest = &s[pos + needle.len()..];
    let colon = rest.find(':')?;
    let rest = rest[colon + 1..].trim_start();
    let token: String = rest.chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
        .collect();
    if token.is_empty() { None } else { token.parse().ok() }
}

fn read_stdin_all() -> Result<String, String> {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s).map_err(|e| e.to_string())?;
    Ok(s)
}

fn with_usage_file_lock<T, F>(path: &Path, operation: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let lock_path = path.with_extension("csv.lock");
    let mut guard: Option<File> = None;
    for _ in 0..500 {
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock_path)
        {
            Ok(mut file) => match file.try_lock() {
                Ok(()) => {
                    let owner = format!("pid={}\ncreated={}\n", std::process::id(), now_unix());
                    if let Err(e) = file.set_len(0).and_then(|_| file.seek(SeekFrom::Start(0))) {
                        return Err(e.to_string());
                    }
                    if let Err(e) = file.write_all(owner.as_bytes()) {
                        return Err(e.to_string());
                    }
                    guard = Some(file);
                    break;
                }
                Err(std::fs::TryLockError::WouldBlock) => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(std::fs::TryLockError::Error(e)) => return Err(e.to_string()),
            },
            Err(e) => return Err(e.to_string()),
        }
    }
    let Some(guard) = guard else {
        return Err(format!("timed out waiting for usage lock: {}", lock_path.display()));
    };

    let result = operation();
    // Keep the lock file in place. Removing it after unlocking reintroduces a
    // race where another process can acquire a newly created file and the
    // cleanup from this process deletes that new owner's lock. The OS-managed
    // lock is released when this handle is dropped, including after a crash.
    drop(guard);
    result
}

fn ensure_usage_file(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if !path.exists() {
        fs::write(path, USAGE_HEADER).map_err(|e| e.to_string())?;
    } else {
        migrate_usage_file_if_needed(path)?;
    }
    Ok(())
}

/// Upgrade a schema-v1 usage file in place by rewriting the header and padding
/// old rows with empty `head_sha`/`verification` columns. Unknown headers are
/// left untouched.
fn migrate_usage_file_if_needed(path: &Path) -> Result<(), String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    // Split on logical records: a quoted field may span physical lines, and
    // appending columns per physical line would inject commas into it.
    let records = split_csv_records(&text);
    let mut records = records.iter();
    let Some(header) = records.next() else { return Ok(()); };
    if header.trim_end() == USAGE_HEADER.trim_end() {
        return Ok(());
    }
    if header.trim_end() != USAGE_HEADER_V1.trim_end() {
        return Ok(());
    }
    let mut out = String::from(USAGE_HEADER);
    for record in records {
        if record.trim().is_empty() {
            continue;
        }
        out.push_str(record);
        out.push_str(",,");
        out.push('\n');
    }
    let tmp = path.with_extension(format!("csv.migrate.{}.tmp", std::process::id()));
    fs::write(&tmp, out).map_err(|e| e.to_string())?;
    #[cfg(not(windows))]
    copy_file_permissions(path, &tmp)?;
    replace_file_preserving_security(&tmp, path, None)?;
    Ok(())
}

fn phase_marker_path() -> PathBuf {
    PathBuf::from(".ai-usage/phase")
}

/// Phase precedence: explicit flag > `.ai-usage/phase` marker > SENTRITH_PHASE
/// > "standard".
///
/// The marker outranks the environment variable because hooks are spawned by
/// the agent process: a variable exported after the agent started never reaches
/// them, while `usage baseline start` writes a marker every hook can read.
fn resolve_phase_value(
    explicit: Option<&str>,
    marker: Option<&str>,
    env_value: Option<&str>,
) -> String {
    for candidate in [explicit, marker, env_value] {
        if let Some(p) = candidate {
            if !p.trim().is_empty() {
                return p.trim().to_string();
            }
        }
    }
    "standard".to_string()
}

fn resolve_phase(explicit: Option<&str>) -> String {
    let marker = fs::read_to_string(phase_marker_path()).ok();
    resolve_phase_value(
        explicit,
        marker.as_deref(),
        env::var("SENTRITH_PHASE").ok().as_deref(),
    )
}

// ---------------------------------------------------------------------------
// usage baseline
//
// A baseline must be measured with the Sentrith contract inactive, which is
// otherwise a manual and error-prone step. These commands stash the agent
// instruction files and restore them, so the measurement is reversible.
// Hook configuration and `.ai-usage/` are deliberately left in place: they are
// what performs the measurement.
// ---------------------------------------------------------------------------

const BASELINE_STASH_PATHS: &[&str] = &[
    "AGENTS.md",
    "CLAUDE.md",
    ".github/copilot-instructions.md",
    ".github/prompts",
    ".agents",
    ".claude/skills",
];

/// Hook-settings files whose advisory-check entries (see
/// `WORKFLOW_CHECK_SUBCOMMANDS`) are edited out, not moved, for the duration
/// of a baseline. Unlike `BASELINE_STASH_PATHS`, the live file keeps
/// existing throughout: usage capture still needs it.
const BASELINE_HOOK_SETTINGS_PATHS: &[&str] = &[".claude/settings.json", ".codex/hooks.json"];

fn baseline_stash_dir() -> PathBuf {
    PathBuf::from(".sentrith-private/baseline-stash")
}

/// A cheap, non-cryptographic content digest used purely to detect whether a
/// file changed between two points in time on this machine -- not a security
/// boundary, so `std::hash`'s built-in hasher (no dependency needed) is
/// entirely adequate; nothing here defends against a motivated adversary
/// crafting a collision, only against accidentally comparing stale bytes.
fn content_digest(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// Strip a hook-settings file's advisory-check entries for the duration of a
/// baseline, backing up the original so `baseline stop` can restore it
/// verbatim rather than reconstructing it from the shipped example (which
/// may not match what the user actually has). Returns `Ok(false)` when there
/// is nothing to do: no file, empty file, no `hooks` field, or no entry
/// actually matched.
/// `rel_path` is only a naming key for the backup under `stash`; `live` is the
/// file actually read and written. Kept separate (rather than deriving `live`
/// from `rel_path` via `repo_file` internally) so this is testable with an
/// arbitrary temp-directory path instead of depending on the process's
/// current directory, the same reasoning that shaped `inspect_stash`.
fn reduce_hook_settings_for_baseline(
    rel_path: &str,
    live: &Path,
    stash: &Path,
) -> Result<bool, String> {
    // A symlinked settings file (common under dotfile managers) must not be
    // touched: `fs::write` follows the link and would reduce whatever it
    // points at -- possibly shared across other projects -- and restoring by
    // rename would later replace the symlink itself with a plain file,
    // permanently breaking the link. The caller aborts and rolls back the
    // whole baseline start on this Err, rather than proceeding with this
    // path's hooks silently left active.
    if live.is_symlink() {
        return Err(format!(
            "{} is a symlink; leaving it untouched for baseline rather than rewriting or breaking whatever it points at",
            live.display()
        ));
    }
    if !live.exists() {
        return Ok(false);
    }
    // Read fallibly rather than through `read_text`, which folds a read
    // failure (invalid UTF-8, a transient permission error) into the same
    // empty string as a genuinely empty file. That would make this return
    // `Ok(false)` -- "nothing to reduce" -- for a file that actually still
    // holds live Sentrith hooks; `baseline_start` would then report success
    // while this path's advisory hooks stay active for the whole baseline,
    // contaminating the very sample the baseline exists to keep contract-free.
    let original = fs::read_to_string(live).map_err(|e| {
        format!("failed to read {}: {e}; left untouched for baseline", live.display())
    })?;
    if original.trim().is_empty() {
        return Ok(false);
    }
    let mut settings = json_parse(&original).map_err(|e| {
        format!(
            "{} is not valid JSON ({e}); left untouched for baseline",
            live.display()
        )
    })?;
    let Some(mut hooks) = settings.get("hooks").cloned() else {
        return Ok(false);
    };
    let before = json_to_string(&hooks);
    strip_hooks_matching(&mut hooks, is_workflow_check_command);
    if json_to_string(&hooks) == before {
        return Ok(false);
    }
    if matches!(&hooks, Json::Obj(e) if e.is_empty()) {
        settings.remove("hooks");
    } else {
        settings.set("hooks", hooks);
    }
    let reduced = json_to_string(&settings);
    // Never write output we cannot read back.
    json_parse(&reduced).map_err(|e| {
        format!(
            "internal: reduced {} would be invalid JSON ({e})",
            live.display()
        )
    })?;

    let backup_dir = stash.join("hook-settings-backup");
    let backup = backup_dir.join(rel_path);
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    // Everything up to here can fail without having touched `live` at all, so
    // returning `Err` is always safe: the caller skips this path rather than
    // getting stuck with it silently reduced and never restored (a disk
    // filling up mid-write, or the process exiting, must not be able to leave
    // `live` reduced with nothing recording that it needs restoring).
    //
    // The digest -- not the full reduced content -- is what `restore` later
    // compares against; storing only a digest means there is no second copy
    // of settings content (which may itself hold something sensitive) sitting
    // at whatever permissions a fresh file gets by default.
    let digest_path = backup_dir.join(format!("{rel_path}.reduced-digest"));
    fs::write(&digest_path, content_digest(&reduced).to_string())
        .map_err(|e| format!("failed to record the baseline snapshot for {}: {e}", live.display()))?;

    let tmp = live.with_extension("sentrith-baseline-tmp");
    // `write_secure_temp_file` never lets the full reduced settings --
    // unrelated secrets included -- sit exposed under default/inherited
    // permissions even briefly; `copy_file_permissions` below still narrows
    // (or widens) the result to match `live`'s actual mode on Unix, since
    // the temp file's own fixed mode is only meant to hold for this window,
    // not to be the final permissions.
    write_secure_temp_file(&tmp, &reduced)?;
    #[cfg(not(windows))]
    copy_file_permissions(live, &tmp)?;
    // `backup` is created here, atomically with the live replacement on
    // Windows via `ReplaceFileW`'s own backup parameter (the only way this
    // project can carry over a restricted file's full security descriptor to
    // a brand-new path without a new dependency), and just before the rename
    // elsewhere -- either way, before `live` holds anything but the original.
    replace_file_preserving_security(&tmp, live, Some(&backup))?;
    Ok(true)
}

/// Restore a hook-settings file from its baseline backup, removing the
/// backup. A no-op if there is no backup for this path (nothing was reduced,
/// or it was already restored). Refuses to overwrite a file that was edited
/// since it was reduced, rather than silently discarding that edit.
fn restore_hook_settings_backup(rel_path: &str, live: &Path, stash: &Path) -> Result<bool, String> {
    let backup_dir = stash.join("hook-settings-backup");
    let backup = backup_dir.join(rel_path);
    let digest_path = backup_dir.join(format!("{rel_path}.reduced-digest"));
    if !backup.exists() {
        // A prior attempt may have removed the backup but been interrupted,
        // or failed, before removing its digest -- there is no restore work
        // left (the backup's absence is what signals `live` was already
        // fully restored), but the orphaned digest still needs clearing:
        // left behind, it keeps `hook-settings-backup` non-empty, which
        // later fails the stash directory's removal and strands the phase
        // marker at `baseline`, the same failure mode fixed for the backup
        // file itself in `finish_hook_restore_cleanup`.
        if digest_path.exists() {
            fs::remove_file(&digest_path).map_err(|e| {
                format!("could not remove orphaned digest {}: {e}", digest_path.display())
            })?;
        }
        return Ok(false);
    }

    // A retry after an earlier attempt: the live replacement below already
    // succeeded, but a later step (removing the backup or digest) failed and
    // the function returned Err. `live` now holds the *restored* original,
    // not the reduced content the digest describes, so comparing it against
    // the digest here would misread this state as a conflict and bury an
    // already-correct file's backup for no reason. `reduce_hook_settings_for_baseline`
    // only ever creates a backup when the reduction actually changed
    // something, so `live` cannot legitimately equal `backup`'s content
    // unless the replacement already happened -- that equality is what
    // distinguishes this case from a genuinely fresh restore, without
    // needing any separate persisted state. Once detected, skip straight to
    // cleanup rather than touching `live` or running divergence detection
    // again.
    if fs::metadata(live).is_ok() && read_text(live) == read_text(&backup) {
        return finish_hook_restore_cleanup(&backup, &digest_path, live);
    }

    // Fail closed: a missing, unreadable, or malformed digest means there is
    // no way to tell whether `live` was edited during baseline, and treating
    // that as "not diverged" would silently overwrite a genuine edit with
    // the stale backup -- the retry-cleanup check above already handles the
    // one case where a missing/mismatched digest is actually safe (`live`
    // already restored), so anything reaching this line with an unreadable
    // digest is a genuinely unexplained state, not a known-safe one.
    let diverged = read_text(&digest_path)
        .trim()
        .parse::<u64>()
        .map(|expected| expected != content_digest(&read_text(live)))
        .unwrap_or(true);
    if diverged {
        // Move the conflict out of the stash entirely (rather than leaving it
        // there and refusing to clean up) so restoring the contract files --
        // the safety-critical half of `baseline stop` -- is never held
        // hostage by an unrelated hook-settings edit made during baseline.
        // Derived from `stash`'s own parent rather than a fresh cwd-relative
        // path, so it lands next to the stash wherever that actually is
        // (matters for tests, and keeps the eventual rename on one volume).
        let conflict_root = stash.parent().unwrap_or(stash);
        let conflict_dir = conflict_root.join("baseline-hook-conflicts");
        // A conflict from an *earlier* baseline may still be sitting here
        // unresolved -- nothing blocks starting a new baseline while one is
        // pending. Never rename onto it: that would silently destroy the
        // only copy of the original the user was told to merge by hand.
        // Walk numbered variants until an unused path is found instead.
        let mut conflict = conflict_dir.join(rel_path);
        if conflict.exists() {
            let mut n = 1u32;
            loop {
                let candidate = conflict_dir.join(format!("{rel_path}.conflict-{n}"));
                if !candidate.exists() {
                    conflict = candidate;
                    break;
                }
                n += 1;
            }
        }
        if let Some(parent) = conflict.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if fs::rename(&backup, &conflict).is_ok() {
            // Only safe to drop the digest once the original is confirmed
            // preserved outside the stash: with the backup gone from its
            // known path, nothing else records that this path was ever
            // diverged.
            let _ = fs::remove_file(&digest_path);
            return Err(format!(
                "{} was edited while baseline was active; refusing to overwrite it. \
                 The pre-baseline original is kept at {}: merge the change by hand.",
                live.display(),
                conflict.display()
            ));
        }
        // Preserving the original at `conflict` failed too (e.g. a sharing
        // violation or an unwritable conflict directory). The backup is
        // still sitting at its original path -- deliberately NOT removing
        // the digest here: without it, a retry's divergence check reads a
        // missing digest as "not diverged" and silently overwrites the edit
        // with the stale backup, exactly the loss this whole check exists to
        // prevent. Keeping the digest means a retry re-detects the same
        // conflict and tries preservation again instead.
        return Err(format!(
            "{} was edited while baseline was active; refusing to overwrite it. \
             Preserving the pre-baseline original at {} also failed; it remains at {} -- \
             do not delete it. Resolve whatever is blocking {}, then retry `baseline stop`.",
            live.display(),
            conflict.display(),
            backup.display(),
            conflict.display()
        ));
    }

    // `backup` is left at its original, already-known path until the
    // replacement below actually commits: a copy here (not a rename) means an
    // interruption at any point up to and including a failed replace still
    // leaves `backup` exactly where a retried restore looks for it, instead
    // of orphaning the original content at a temp path nothing else knows to
    // check.
    let tmp = live.with_extension("sentrith-baseline-restore-tmp");
    // `fs::copy` opens its destination without refusing to follow an
    // existing symlink there -- the same class of attack just fixed for the
    // install-time backup, reachable here too since this predictable path
    // could already be a symlink: it would silently overwrite the symlink's
    // target with the backed-up settings, and the rename below would then
    // move that same symlink onto `live` itself. `create_secure_file`
    // refuses to reuse or follow anything already at the path; streaming
    // `backup`'s bytes into the already-open handle it returns (rather than
    // a second, separate open by path, which is what `fs::copy` does
    // internally) closes the window entirely. This also means `tmp` is
    // always a freshly created file with no attributes inherited from
    // `backup` -- unlike `fs::copy`, so the read-only-attribute workaround
    // this used to need on Windows no longer applies.
    let mut dest = create_secure_file(&tmp)
        .map_err(|e| format!("failed to prepare restore of {}: {e}", live.display()))?;
    let mut src = fs::File::open(&backup)
        .map_err(|e| format!("failed to prepare restore of {}: {e}", live.display()))?;
    std::io::copy(&mut src, &mut dest)
        .map_err(|e| format!("failed to prepare restore of {}: {e}", live.display()))?;
    drop(dest);
    #[cfg(not(windows))]
    copy_file_permissions(live, &tmp)?;
    // As in `reduce`: on Windows, replacing through `ReplaceFileW` (rather
    // than a raw rename) is what actually preserves whatever the reduced
    // file's security descriptor was, instead of silently adopting the
    // backup's.
    replace_file_preserving_security(&tmp, live, None)?;
    finish_hook_restore_cleanup(&backup, &digest_path, live)
}

/// Remove a hook-settings backup and its digest once `live` already holds
/// the restored content, whether that replacement just happened or was
/// confirmed on a retry (see the equality check at the top of
/// `restore_hook_settings_backup`). Split out so both paths share the same
/// cleanup and the same failure handling.
fn finish_hook_restore_cleanup(backup: &Path, digest_path: &Path, live: &Path) -> Result<bool, String> {
    // Defensive: on Windows, a read-only file refuses to delete. In this
    // codebase the backup created during reduce does not currently end up
    // read-only even from a read-only original -- `replace_file_preserving_security`
    // clears the destination's read-only attribute before ReplaceFileW runs,
    // so ReplaceFileW's backup is made from the already-cleared file -- but
    // clearing it here anyway (rather than assuming that stays true) costs
    // nothing and closes the failure mode for good if that ever changes. Not
    // ignoring the removal failure matters regardless of the cause: an
    // unremovable backup keeps `hook-settings-backup` non-empty, which later
    // fails the stash directory's removal and strands the phase marker at
    // `baseline`.
    if let Ok(metadata) = fs::metadata(backup) {
        let mut perms = metadata.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            let _ = fs::set_permissions(backup, perms);
        }
    }
    fs::remove_file(backup).map_err(|e| {
        format!("restored {} but could not remove its backup {}: {e}", live.display(), backup.display())
    })?;
    // Only removed once the backup is confirmed gone. If this step or the one
    // above fails, the digest may still describe the pre-restore reduced
    // content while `live` already holds the restored original -- exactly
    // the mismatch the equality check at the top of the caller exists to
    // recognize, so a retry still finds its way back here rather than
    // running divergence detection against a digest that no longer applies.
    fs::remove_file(digest_path).map_err(|e| {
        format!("restored {} but could not remove its digest {}: {e}", live.display(), digest_path.display())
    })?;
    Ok(true)
}

fn usage_baseline(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage baseline requires start, stop, or status".into());
    }
    match args[0].as_str() {
        "start" => baseline_start(),
        "stop" => baseline_stop(),
        "status" => baseline_status(),
        x => Err(format!("unknown usage baseline command: {x}")),
    }
}

fn baseline_active() -> bool {
    baseline_stash_dir().exists()
}

fn baseline_status() -> Result<(), String> {
    let phase = fs::read_to_string(phase_marker_path())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "standard (no marker)".into());
    if baseline_active() {
        println!("SENTRITH-BASELINE: active; contract stashed in {}", baseline_stash_dir().display());
        let edited = read_text(&baseline_stash_dir().join("HOOK_EDITS.txt"))
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count();
        if edited > 0 {
            println!("SENTRITH-BASELINE: advisory-check hooks reduced in {edited} path(s); usage capture unaffected");
        }
    } else {
        println!("SENTRITH-BASELINE: inactive");
    }
    println!("SENTRITH-BASELINE: recorded phase = {phase}");
    Ok(())
}

fn baseline_start() -> Result<(), String> {
    if baseline_active() {
        return Err(format!(
            "baseline already active ({} exists); run `sentrith usage baseline stop` first",
            baseline_stash_dir().display()
        ));
    }
    // Set the phase marker up front. If this fails, nothing has moved yet and
    // the working tree is untouched; doing it after the moves would leave the
    // contract stashed with no marker on an early return.
    fs::create_dir_all(".ai-usage")
        .map_err(|e| format!("failed to prepare .ai-usage: {e}"))?;
    fs::write(phase_marker_path(), "baseline\n")
        .map_err(|e| format!("failed to write the phase marker: {e}"))?;

    let stash = baseline_stash_dir();
    if let Err(e) = fs::create_dir_all(&stash) {
        let _ = fs::remove_file(phase_marker_path());
        return Err(e.to_string());
    }
    let manifest = stash.join("STASHED.txt");

    // The manifest is written before each move, so an interrupted run still
    // leaves a record of what was stashed. Losing the manifest would strand the
    // moved files, and `baseline stop` deletes the stash directory once it has
    // restored everything the manifest names.
    let mut moved: Vec<String> = Vec::new();
    let mut failure: Option<String> = None;
    for path in BASELINE_STASH_PATHS {
        let src = repo_file(path);
        if !src.exists() {
            continue;
        }
        let dst = stash.join(path);
        if let Some(parent) = dst.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                failure = Some(format!("failed to prepare stash for {path}: {e}"));
                break;
            }
        }
        let mut planned = moved.clone();
        planned.push((*path).to_string());
        if let Err(e) = fs::write(&manifest, planned.join("\n") + "\n") {
            failure = Some(format!("failed to record stash manifest: {e}"));
            break;
        }
        if let Err(e) = fs::rename(&src, &dst) {
            failure = Some(format!("failed to stash {path}: {e}"));
            break;
        }
        moved.push((*path).to_string());
    }

    if let Some(err) = failure {
        // Roll back so a partial stash never leaves the project without its
        // contract files, and never leaves files only inside the stash.
        let rollback_failed = rollback_moved_paths(&moved, &stash);
        if rollback_failed.is_empty() {
            let _ = fs::remove_file(&manifest);
            let _ = fs::remove_dir_all(&stash);
            let _ = fs::remove_file(phase_marker_path());
            return Err(format!("{err}; rolled back, the working tree is unchanged"));
        }
        return Err(format!(
            "{err}; rollback incomplete for: {}. Files are still in {} — restore them manually.",
            rollback_failed.join(", "),
            stash.display()
        ));
    }

    if moved.is_empty() {
        let _ = fs::remove_file(&manifest);
        let _ = fs::remove_dir_all(&stash);
        let _ = fs::remove_file(phase_marker_path());
        return Err("no Sentrith contract files found to stash; is this a Sentrith project?".into());
    }

    // Reduce advisory-check hooks (preflight/guard/etc.) so a baseline session
    // does not see Sentrith-flavored Stop/SessionStart output reacting to the
    // paths this just stashed; usage capture keeps running unmodified. A
    // settings file that cannot be safely reduced (malformed JSON, a symlink)
    // aborts the whole baseline start instead of merely warning: leaving its
    // hooks active would silently contaminate every task recorded as
    // "baseline" with Sentrith still running, which defeats the point of
    // measuring one. If recording which files were edited fails partway,
    // everything rolls back so baseline start stays all-or-nothing either way.
    let mut hook_edits: Vec<String> = Vec::new();
    let hook_edits_manifest = stash.join("HOOK_EDITS.txt");
    let mut hook_edit_failure: Option<String> = None;
    for path in BASELINE_HOOK_SETTINGS_PATHS {
        match reduce_hook_settings_for_baseline(path, &repo_file(path), &stash) {
            Ok(true) => {
                hook_edits.push((*path).to_string());
                if let Err(e) = fs::write(&hook_edits_manifest, hook_edits.join("\n") + "\n") {
                    hook_edit_failure = Some(format!("failed to record hook-edit manifest: {e}"));
                    break;
                }
            }
            Ok(false) => {}
            Err(e) => {
                hook_edit_failure = Some(e);
                break;
            }
        }
    }
    if let Some(err) = hook_edit_failure {
        // Failures here must block deleting the stash below just as much as a
        // failed contract-file rollback does: the backup this would discard
        // is the only copy of that path's original, unreduced hooks, and a
        // path whose restore failed is still sitting reduced in the working
        // tree -- deleting the stash on top of that would erase the only way
        // to recover it while claiming "the working tree is unchanged".
        let mut hook_restore_failed: Vec<String> = Vec::new();
        for path in hook_edits.iter().rev() {
            if let Err(e) = restore_hook_settings_backup(path, &repo_file(path), &stash) {
                hook_restore_failed.push(format!("{path} ({e})"));
            }
        }
        let mut rollback_failed = rollback_moved_paths(&moved, &stash);
        rollback_failed.extend(hook_restore_failed);
        if rollback_failed.is_empty() {
            let _ = fs::remove_file(&hook_edits_manifest);
            let _ = fs::remove_file(&manifest);
            let _ = fs::remove_dir_all(&stash);
            let _ = fs::remove_file(phase_marker_path());
            return Err(format!("{err}; rolled back, the working tree is unchanged"));
        }
        return Err(format!(
            "{err}; rollback incomplete for: {}. Files are still in {} — restore them manually.",
            rollback_failed.join(", "),
            stash.display()
        ));
    }

    println!("SENTRITH-BASELINE: started. Stashed {} path(s):", moved.len());
    for m in &moved {
        println!("  {m}");
    }
    if !hook_edits.is_empty() {
        println!(
            "Reduced advisory-check hooks in {} path(s) (usage capture unaffected):",
            hook_edits.len()
        );
        for h in &hook_edits {
            println!("  {h}");
        }
    }
    println!("Measurement hooks and .ai-usage/ were left active; new turns record phase=baseline.");
    println!("Git will show these paths as deleted until you run `sentrith usage baseline stop`.");
    println!("Start a NEW agent session so the stashed instructions are not still in its context.");
    println!("When you have enough baseline tasks: sentrith usage baseline stop");
    Ok(())
}

/// Move stashed paths back to their live locations for as many `moved`
/// entries as possible, in reverse order. Returns the entries that could not
/// be restored, so the caller can decide whether the stash is safe to
/// delete. Shared by `baseline_start`'s two rollback points.
fn rollback_moved_paths(moved: &[String], stash: &Path) -> Vec<String> {
    let mut failed = Vec::new();
    for path in moved.iter().rev() {
        let src = stash.join(path);
        let dst = repo_file(path);
        if src.exists() && !dst.exists() {
            if let Err(e) = fs::rename(&src, &dst) {
                failed.push(format!("{path} ({e})"));
            }
        }
    }
    failed
}

enum StashState {
    /// Nothing was ever stashed; the directory is safe to remove.
    Empty,
    /// The manifest names these paths, in stash order.
    Listed(Vec<String>),
    /// Files exist but no manifest explains them. They may be the only copy, so
    /// they must not be deleted.
    Unattributable(Vec<String>),
}

fn inspect_stash(stash: &Path) -> Result<StashState, String> {
    let listed: Vec<String> = read_text(&stash.join("STASHED.txt"))
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    if !listed.is_empty() {
        return Ok(StashState::Listed(listed));
    }
    let leftovers: Vec<String> = fs::read_dir(stash)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        // Tracked by HOOK_EDITS.txt, a separate manifest for a separate
        // restore mechanism (edit-in-place, not move); not a sign of an
        // unattributable stash on its own.
        .filter(|n| n != "STASHED.txt" && n != "HOOK_EDITS.txt" && n != "hook-settings-backup")
        .collect();
    if leftovers.is_empty() {
        Ok(StashState::Empty)
    } else {
        Ok(StashState::Unattributable(leftovers))
    }
}

fn remove_empty_stash_parents(stash: &Path, entries: &[String]) -> Result<(), String> {
    let mut parents = BTreeSet::new();
    for entry in entries {
        let mut parent = Path::new(entry).parent();
        while let Some(relative) = parent {
            if relative.as_os_str().is_empty() || relative == Path::new(".") {
                break;
            }
            parents.insert(relative.to_path_buf());
            parent = relative.parent();
        }
    }

    let mut parents: Vec<PathBuf> = parents.into_iter().collect();
    parents.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for relative in parents {
        let path = stash.join(relative);
        match fs::remove_dir(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            // An unexpected file keeps the directory in place. The root
            // removal below will fail too, so the marker and stash remain.
            Err(e) if e.kind() == io::ErrorKind::DirectoryNotEmpty => {}
            Err(e) => {
                return Err(format!(
                    "could not remove empty baseline stash parent {}: {e}",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn finish_baseline_stop_cleanup(
    stash: &Path,
    manifest: &Path,
    marker: &Path,
    entries: &[String],
    hook_edits: &[String],
) -> Result<(), String> {
    remove_empty_stash_parents(stash, entries)?;
    let hook_backup_entries: Vec<String> = hook_edits
        .iter()
        .map(|p| format!("hook-settings-backup/{p}"))
        .collect();
    remove_empty_stash_parents(stash, &hook_backup_entries)?;
    let _ = fs::remove_file(stash.join("HOOK_EDITS.txt"));
    match fs::remove_file(manifest) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(format!(
                "restored files, but could not remove the baseline manifest {}: {e}; the marker and stash were kept",
                manifest.display()
            ));
        }
    }

    match fs::remove_dir(stash) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(format!(
                "restored files, but could not remove the baseline stash {}: {e}; the phase marker was kept",
                stash.display()
            ));
        }
    }

    match fs::remove_file(marker) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(marker_error) => {
            match fs::create_dir_all(stash) {
                Ok(()) => Err(format!(
                    "restored files, but could not remove the phase marker {}: {marker_error}; an empty stash was kept so baseline remains active",
                    marker.display()
                )),
                Err(stash_error) => Err(format!(
                    "baseline cleanup is incomplete: could not remove phase marker {} ({marker_error}) or recreate the stash ({stash_error})",
                    marker.display()
                )),
            }
        }
    }
}

fn baseline_stop() -> Result<(), String> {
    let stash = baseline_stash_dir();
    if !stash.exists() {
        return Err("no active baseline to stop".into());
    }
    let manifest = stash.join("STASHED.txt");
    // Restore by scanning the fixed candidate list -- the same one `start`
    // reduces from -- rather than trusting HOOK_EDITS.txt. That manifest is
    // written only after a path's live file has already been reduced, so an
    // interruption in between would otherwise leave a reduced file with
    // nothing telling `stop` to restore it. `restore_hook_settings_backup`
    // already no-ops when a path has no backup, so scanning every candidate
    // is safe regardless of whether it was ever actually reduced.
    let hook_edits: Vec<String> = BASELINE_HOOK_SETTINGS_PATHS.iter().map(|s| s.to_string()).collect();
    let mut hook_edits_restored = 0;
    for path in &hook_edits {
        match restore_hook_settings_backup(path, &repo_file(path), &stash) {
            Ok(true) => hook_edits_restored += 1,
            Ok(false) => {}
            Err(e) => println!("SENTRITH-BASELINE: warning: {e}"),
        }
    }

    let entries = match inspect_stash(&stash)? {
        StashState::Listed(paths) => paths,
        StashState::Empty => {
            finish_baseline_stop_cleanup(&stash, &manifest, &phase_marker_path(), &[], &hook_edits)?;
            println!("SENTRITH-BASELINE: stopped; the stash was empty. Phase is standard again.");
            return Ok(());
        }
        StashState::Unattributable(leftovers) => {
            return Err(format!(
                "{} has no manifest but still contains: {}. Move them back manually; nothing was deleted.",
                stash.display(),
                leftovers.join(", ")
            ));
        }
    };

    let mut restored = 0;
    let mut failed = Vec::new();
    for path in entries.iter().map(String::as_str) {
        let src = stash.join(path);
        let dst = repo_file(path);
        if !src.exists() {
            continue;
        }
        if dst.exists() {
            failed.push(format!("{path} (already exists in the working tree)"));
            continue;
        }
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        match fs::rename(&src, &dst) {
            Ok(_) => restored += 1,
            Err(e) => failed.push(format!("{path} ({e})")),
        }
    }

    if !failed.is_empty() {
        // The marker stays: part of the contract is still stashed, so the
        // project is not back in `standard`. Clearing it here would silently
        // label the following turns `standard` while the agent is still running
        // without its instructions, contaminating the comparison.
        let mut msg = format!(
            "restored {restored} path(s), but the baseline is still active because these need manual attention: {}",
            failed.join(", ")
        );
        msg.push_str(&format!(
            ". The stash is kept at {} and the phase marker still reads `baseline`. Resolve the conflicts and run `sentrith usage baseline stop` again.",
            stash.display()
        ));
        return Err(msg);
    }

    finish_baseline_stop_cleanup(&stash, &manifest, &phase_marker_path(), &entries, &hook_edits)?;
    println!("SENTRITH-BASELINE: stopped. Restored {restored} path(s); phase is standard again.");
    if hook_edits_restored > 0 {
        println!("Restored advisory-check hooks in {hook_edits_restored} path(s).");
    }
    println!("Start a NEW agent session so the contract is loaded before your next task.");
    Ok(())
}

#[derive(Default, Clone)]
struct AutoUsage {
    agent: String,
    model: String,
    phase: String,
    task: String,
    input_tokens: Option<f64>,
    cached_input_tokens: Option<f64>,
    output_tokens: Option<f64>,
    credits: Option<f64>,
    cost_usd: Option<f64>,
    tool_calls: Option<f64>,
    duration_seconds: Option<f64>,
    success: String,
    rework_count: Option<f64>,
    source: String,
    session_id: String,
    notes: String,
    head_sha: String,
    verification: String,
}

fn num_cell(v: Option<f64>) -> String {
    v.map(|x| {
        if (x.fract()).abs() < 0.0000001 { format!("{:.0}", x) } else { format!("{}", x) }
    }).unwrap_or_default()
}

fn append_usage_row(path: &Path, values: &[String]) -> Result<(), String> {
    with_usage_file_lock(path, || {
        ensure_usage_file(path)?;
        let row = values.iter().map(|x| csv_escape(x)).collect::<Vec<_>>().join(",");
        let mut f = OpenOptions::new().append(true).open(path).map_err(|e| e.to_string())?;
        writeln!(f, "{row}").map_err(|e| e.to_string())
    })
}

fn append_auto_usage(path: &Path, u: &AutoUsage) -> Result<(), String> {
    let values = [
        now_unix(),
        u.agent.clone(),
        u.model.clone(),
        u.phase.clone(),
        u.task.clone(),
        num_cell(u.input_tokens),
        num_cell(u.cached_input_tokens),
        num_cell(u.output_tokens),
        num_cell(u.credits),
        num_cell(u.cost_usd),
        num_cell(u.tool_calls),
        num_cell(u.duration_seconds),
        u.success.clone(),
        num_cell(u.rework_count),
        u.source.clone(),
        u.session_id.clone(),
        u.notes.clone(),
        u.head_sha.clone(),
        u.verification.clone(),
    ];
    append_usage_row(path, &values)
}

fn split_double_dash(args: &[String]) -> (Vec<String>, Vec<String>) {
    if let Some(i) = args.iter().position(|x| x == "--") {
        (args[..i].to_vec(), args[i+1..].to_vec())
    } else {
        (args.to_vec(), Vec::new())
    }
}

fn usage_run(args: &[String]) -> Result<(), String> {
    if args.is_empty() { return Err("usage run requires codex or copilot".into()); }
    match args[0].as_str() {
        "codex" => usage_run_codex(&args[1..]),
        "copilot" => usage_run_copilot(&args[1..]),
        x => Err(format!("unsupported usage run adapter: {x}")),
    }
}

fn usage_run_codex(args: &[String]) -> Result<(), String> {
    let (front, passthrough) = split_double_dash(args);
    let (opts, positional) = parse_options(&front)?;
    let task = opts.get("task").cloned().or_else(|| positional.first().cloned()).ok_or("missing --task")?;
    let phase = resolve_phase(opts.get("phase").map(String::as_str));
    let file = opts.get("file").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));

    let mut cmd = Command::new("codex");
    cmd.arg("exec").arg("--json");
    for a in passthrough { cmd.arg(a); }

    let out = cmd.output().map_err(|e| format!("failed to launch codex: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    print!("{stdout}");
    eprint!("{stderr}");

    let mut usage = AutoUsage {
        agent: "codex".into(),
        phase,
        task,
        source: "codex-exec-json".into(),
        success: if out.status.success() { "yes".into() } else { "no".into() },
        ..Default::default()
    };

    for line in stdout.lines() {
        if line.contains("\"type\":\"turn.completed\"") || line.contains("\"type\": \"turn.completed\"") {
            usage.input_tokens = json_number_field(line, "input_tokens");
            usage.cached_input_tokens = json_number_field(line, "cached_input_tokens");
            usage.output_tokens = json_number_field(line, "output_tokens");
        }
        if usage.session_id.is_empty() && (line.contains("thread.started") || line.contains("thread_id")) {
            usage.session_id = json_string_field(line, "thread_id").unwrap_or_default();
        }
    }
    usage.model = opts.get("model").cloned().unwrap_or_default();
    append_auto_usage(&file, &usage)?;
    if !out.status.success() { return Err(format!("codex exited with {}", out.status)); }
    Ok(())
}

fn parse_number_near_label(text: &str, labels: &[&str]) -> Option<f64> {
    let lower = text.to_lowercase();
    for label in labels {
        if let Some(pos) = lower.find(&label.to_lowercase()) {
            let rest = &text[pos + label.len()..];
            let mut started = false;
            let token: String = rest.chars().filter_map(|c| {
                if c.is_ascii_digit() || c == '.' {
                    started = true;
                    Some(c)
                } else if started {
                    None
                } else {
                    Some(' ')
                }
            }).collect();
            for part in token.split_whitespace() {
                if let Ok(v) = part.parse::<f64>() { return Some(v); }
            }
        }
    }
    None
}

fn usage_run_copilot(args: &[String]) -> Result<(), String> {
    let (front, passthrough) = split_double_dash(args);
    let (opts, positional) = parse_options(&front)?;
    let task = opts.get("task").cloned().or_else(|| positional.first().cloned()).ok_or("missing --task")?;
    let phase = resolve_phase(opts.get("phase").map(String::as_str));
    let file = opts.get("file").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));

    let mut cmd = Command::new("copilot");
    for a in passthrough { cmd.arg(a); }
    let out = cmd.output().map_err(|e| format!("failed to launch copilot: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    print!("{stdout}");
    eprint!("{stderr}");

    let combined = format!("{stdout}\n{stderr}");
    let usage = AutoUsage {
        agent: "copilot".into(),
        model: opts.get("model").cloned().unwrap_or_default(),
        phase,
        task,
        credits: parse_number_near_label(&combined, &["AI Credits", "AI credits", "credits used"]),
        duration_seconds: parse_number_near_label(&combined, &["duration", "session duration"]),
        success: if out.status.success() { "yes".into() } else { "no".into() },
        source: "copilot-cli-output".into(),
        notes: if combined.to_lowercase().contains("credit") {
            "Parsed Copilot CLI usage footer; output format is vendor-controlled.".into()
        } else {
            "Copilot CLI usage footer was not machine-parseable; credits left blank.".into()
        },
        ..Default::default()
    };
    append_auto_usage(&file, &usage)?;
    if !out.status.success() { return Err(format!("copilot exited with {}", out.status)); }
    Ok(())
}

fn live_dir() -> PathBuf {
    PathBuf::from(".ai-usage/live")
}

fn snapshot_path(agent: &str, session: &str) -> PathBuf {
    live_dir().join(format!("{agent}-{session}.snapshot"))
}

fn task_path(agent: &str, session: &str) -> PathBuf {
    live_dir().join(format!("{agent}-{session}.task"))
}

fn write_kv(path: &Path, pairs: &[(&str, String)]) -> Result<(), String> {
    if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| e.to_string())?; }
    let mut s = String::new();
    for (k,v) in pairs {
        s.push_str(k); s.push('\t'); s.push_str(&v.replace('\n'," ")); s.push('\n');
    }
    fs::write(path, s).map_err(|e| e.to_string())
}

fn read_kv(path: &Path) -> BTreeMap<String,String> {
    let mut m = BTreeMap::new();
    if let Ok(s) = fs::read_to_string(path) {
        for line in s.lines() {
            if let Some((k,v)) = line.split_once('\t') { m.insert(k.to_string(),v.to_string()); }
        }
    }
    m
}

const UNBORN_HEAD: &str = "<unborn>";

fn git_head() -> String {
    let head = git(&["rev-parse", "HEAD"]);
    if !head.trim().is_empty() {
        return head.trim().to_string();
    }
    if git(&["rev-parse", "--is-inside-work-tree"]).trim() == "true" {
        UNBORN_HEAD.to_string()
    } else {
        String::new()
    }
}

fn commit_reached(start_head: &str, head: &str) -> bool {
    !start_head.is_empty() && !head.is_empty() && head != start_head
}

fn verif_path(agent: &str, session: &str) -> PathBuf {
    live_dir().join(format!("{agent}-{session}.verif"))
}

/// Aggregated usage and verification signals from one turn's slice of a
/// provider transcript. Transcript formats are not stable vendor contracts;
/// all parsing here is best-effort and must degrade to `seen == false`.
#[derive(Default)]
struct TranscriptWindow {
    input_tokens: f64,
    cache_creation_tokens: f64,
    cached_input_tokens: f64,
    output_tokens: f64,
    model: String,
    seen: bool,
    /// Some(true)=last test command in the window passed, Some(false)=failed.
    verification: Option<bool>,
}

/// Return the characters following `marker` up to the next `"`.
fn extract_id_after(line: &str, marker: &str) -> Option<String> {
    let pos = line.find(marker)?;
    let rest = &line[pos + marker.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn finish_shell_segment(
    segments: &mut Vec<(Vec<String>, String)>,
    token: &mut String,
    operator: String,
) {
    if !token.is_empty() {
        segments.last_mut().unwrap().0.push(std::mem::take(token));
    }
    if !segments.last().unwrap().0.is_empty() {
        segments.push((Vec::new(), operator));
    }
}

fn shell_command_segments(command: &str) -> Vec<(Vec<String>, String)> {
    let mut segments = vec![(Vec::new(), String::new())];
    let mut token = String::new();
    let mut quote = None;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                token.push(ch);
            }
            continue;
        }
        match ch {
            '\'' | '"' => quote = Some(ch),
            '\n' => finish_shell_segment(&mut segments, &mut token, "\n".into()),
            c if c.is_whitespace() => {
                if !token.is_empty() {
                    segments.last_mut().unwrap().0.push(std::mem::take(&mut token));
                }
            }
            ';' => finish_shell_segment(&mut segments, &mut token, ";".into()),
            '&' => {
                let operator = if chars.peek() == Some(&'&') {
                    chars.next();
                    "&&"
                } else {
                    "&"
                };
                finish_shell_segment(&mut segments, &mut token, operator.into());
            }
            '|' => {
                let operator = if chars.peek() == Some(&'|') {
                    chars.next();
                    "||"
                } else {
                    "|"
                };
                finish_shell_segment(&mut segments, &mut token, operator.into());
            }
            _ => token.push(ch),
        }
    }
    if !token.is_empty() {
        segments.last_mut().unwrap().0.push(token);
    }
    segments.retain(|(tokens, _)| !tokens.is_empty());
    segments
}

fn executable_name(token: &str) -> String {
    token
        .replace('\\', "/")
        .rsplit('/')
        .next()
        .unwrap_or(token)
        .trim_end_matches(".exe")
        .to_ascii_lowercase()
}

fn first_non_option_arg(args: &[String]) -> Option<&str> {
    args.iter()
        .map(String::as_str)
        .find(|arg| !arg.starts_with('-'))
}

fn no_test_execution_requested(name: &str, args: &[String]) -> bool {
    let has = |flag: &str| args.iter().any(|arg| arg == flag || arg.starts_with(&format!("{flag}=")));
    match name {
        "cargo" => has("--no-run") || has("--list"),
        "pytest" | "python" | "python3" | "py" => has("--collect-only") || has("--co"),
        "go" => has("-list"),
        "dotnet" => has("--list-tests"),
        "mvn" => args.iter().any(|arg| {
            matches!(
                arg.as_str(),
                "-DskipTests" | "-DskipTests=true" | "-Dmaven.test.skip" | "-Dmaven.test.skip=true"
            )
        }),
        "gradle" | "gradlew" => {
            has("--dry-run") || args.windows(2).any(|w| {
                matches!(w[0].as_str(), "-x" | "--exclude-task") && w[1] == "test"
            })
        }
        "ctest" => has("-N") || has("--show-only"),
        "jest" => has("--listTests"),
        "vitest" => has("--list"),
        _ => false,
    }
}

fn is_test_invocation(tokens: &[String]) -> bool {
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        if token == "env" || token == "command" {
            index += 1;
            continue;
        }
        if token
            .split_once('=')
            .map(|(name, _)| {
                !name.is_empty()
                    && name
                        .chars()
                        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            })
            .unwrap_or(false)
        {
            index += 1;
            continue;
        }
        break;
    }

    let Some(executable) = tokens.get(index) else {
        return false;
    };
    let args = &tokens[index + 1..];
    let name = executable_name(executable);
    if no_test_execution_requested(&name, args) {
        return false;
    }
    // `cargo +nightly test` / `cargo +1.75 test`: cargo's own `--help`
    // documents `Usage: cargo [+toolchain] [OPTIONS] [COMMAND]`. The selector
    // does not start with `-`, so `first_non_option_arg` would otherwise treat
    // it as the subcommand and miss the following `test`. This is cargo-only
    // syntax; `+` has no such meaning for the other runners here.
    let args: &[String] = if name == "cargo" {
        match args.first() {
            Some(a) if a.starts_with('+') => &args[1..],
            _ => args,
        }
    } else {
        args
    };
    let first = first_non_option_arg(args);

    match name.as_str() {
        "cargo" | "go" | "dotnet" | "mvn" | "gradle" | "gradlew" | "make" | "rake" | "mix" => {
            first == Some("test")
        }
        "npm" | "yarn" | "pnpm" => {
            first == Some("test") || (first == Some("run") && args.iter().any(|arg| arg == "test"))
        }
        "uv" => {
            let Some(run) = args.iter().position(|arg| arg == "run") else {
                return false;
            };
            is_test_invocation(&args[run + 1..])
        }
        "python" | "python3" | "py" => args
            .windows(2)
            .any(|window| window[0] == "-m" && matches!(window[1].as_str(), "pytest" | "unittest")),
        "pytest" | "rspec" | "phpunit" | "vitest" | "jest" | "ctest" | "tox" | "unittest" => true,
        _ => false,
    }
}

/// Recognize a test invocation only when its status controls the shell result.
/// Text that merely mentions a test command, masked failures, and pipelines
/// are excluded so a successful wrapper cannot turn a failed test into a pass.
fn is_test_command(cmd: &str) -> bool {
    let segments = shell_command_segments(cmd);
    let matches: Vec<usize> = segments
        .iter()
        .enumerate()
        .filter_map(|(index, (tokens, _))| is_test_invocation(tokens).then_some(index))
        .collect();
    if matches.len() != 1 {
        return false;
    }
    let test_index = matches[0];
    if test_index != 0 {
        return false;
    }
    segments[test_index + 1..].is_empty()
}

/// Parse the Claude Code transcript JSONL lines after `skip_lines`.
///
/// - Token usage is summed once per distinct assistant message id (one API
///   message is serialized as one transcript line per content block, each
///   repeating the same usage object).
/// - `input_tokens` and `cache_creation_input_tokens` are collected separately;
///   the caller decides how to combine them.
/// - Test-like Bash `tool_use` blocks are matched to their `tool_result` by
///   tool id; failure is only observable through `"is_error":true`.
fn parse_claude_transcript_window(text: &str, skip_lines: usize) -> TranscriptWindow {
    let mut w = TranscriptWindow::default();
    let mut counted: BTreeSet<String> = BTreeSet::new();
    let mut pending_tests: BTreeSet<String> = BTreeSet::new();

    for line in text.lines().skip(skip_lines) {
        if line.contains("\"type\":\"assistant\"") {
            if w.model.is_empty() {
                if let Some(m) = json_string_field(line, "model") {
                    w.model = m;
                }
            }
            if let Some(id) = extract_id_after(line, "\"id\":\"msg_") {
                if counted.insert(id) {
                    // Prefer the structural usage object (assistant content can
                    // legitimately contain the literal string "usage").
                    let upos = line
                        .rfind("\"usage\":{\"input_tokens\"")
                        .or_else(|| line.rfind("\"usage\""));
                    if let Some(p) = upos {
                        let u = &line[p..];
                        w.seen = true;
                        w.input_tokens += json_number_field(u, "input_tokens").unwrap_or(0.0);
                        w.cache_creation_tokens +=
                            json_number_field(u, "cache_creation_input_tokens").unwrap_or(0.0);
                        w.cached_input_tokens +=
                            json_number_field(u, "cache_read_input_tokens").unwrap_or(0.0);
                        w.output_tokens += json_number_field(u, "output_tokens").unwrap_or(0.0);
                    }
                }
            }
            if line.contains("\"name\":\"Bash\"") {
                if let Some(cmd) = json_string_field(line, "command") {
                    if is_test_command(&cmd) {
                        if let Some(tid) = extract_id_after(line, "\"id\":\"toolu_") {
                            pending_tests.insert(format!("toolu_{tid}"));
                        }
                    }
                }
            }
        } else if line.contains("\"tool_use_id\":\"") {
            if let Some(tid) = extract_id_after(line, "\"tool_use_id\":\"") {
                if pending_tests.contains(&tid) {
                    let failed = line.contains("\"is_error\":true")
                        || line.contains("\"is_error\": true");
                    w.verification = Some(!failed);
                }
            }
        }
    }
    w
}

/// Best-effort scan of a Codex transcript window for test-command outcomes.
///
/// Verified against a real `~/.codex/sessions/**/rollout-*.jsonl` file: a
/// completed shell command is a single `event_msg` / `item_completed` line
/// whose `payload.item.type` is `"CommandExecution"`, carrying `command` (an
/// argv array — `["/bin/bash", "-lc", "<shell text>"]`, not a plain string),
/// `exit_code`, and `id`, all on that one line. Command and result are never
/// split across lines in this format, so matching only within one line — never
/// via a flag carried over from an earlier line — cannot misattribute one
/// command's result to another, even when Codex logs commands in parallel.
/// `codex exec --json`'s flat `{"command":"...","exit_code":N}` shape is also
/// accepted, as a fallback for older or differently-shaped output, under the
/// same same-line-only rule. Codex documents that this format is not a stable
/// interface, so an unrecognized line is simply skipped rather than guessed at.
fn scan_codex_window_for_tests(text: &str, skip_lines: usize) -> Option<bool> {
    let mut result = None;
    for line in text.lines().skip(skip_lines) {
        let cmd = json_string_array_last(line, "command").or_else(|| json_string_field(line, "command"));
        let Some(cmd) = cmd else { continue };
        if !is_test_command(&cmd) {
            continue;
        }
        if let Some(code) = json_number_field(line, "exit_code") {
            result = Some(code == 0.0);
        }
    }
    result
}

/// Carry the latest verification outcome across turns of a session, then
/// derive the objective success value for a row.
///
/// Success semantics (documented in docs/metrics/MEASUREMENT_ARCHITECTURE.*):
/// commit reached + last verification pass => "yes";
/// commit reached + last verification fail => "no";
/// otherwise => "unknown".
fn update_and_resolve_success(
    agent: &str,
    session: &str,
    window_verification: Option<bool>,
    committed: bool,
) -> String {
    let vpath = verif_path(agent, session);
    if let Some(pass) = window_verification {
        if let Some(parent) = vpath.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&vpath, if pass { "pass" } else { "fail" });
    }
    let carried = fs::read_to_string(&vpath)
        .ok()
        .map(|s| s.trim().to_string());
    let success = if committed {
        match carried.as_deref() {
            Some("pass") => "yes",
            Some("fail") => "no",
            _ => "unknown",
        }
    } else {
        "unknown"
    };
    if committed {
        let _ = fs::remove_file(&vpath);
    }
    success.to_string()
}

fn usage_claude_status(_args: &[String]) -> Result<(), String> {
    let input = read_stdin_all()?;
    let session = json_string_field(&input, "session_id").unwrap_or_else(|| "unknown".into());
    let model = json_string_field(&input, "display_name").unwrap_or_default();
    let cost = json_number_field(&input, "total_cost_usd").unwrap_or(0.0);
    let duration_ms = json_number_field(&input, "total_duration_ms").unwrap_or(0.0);
    let input_tokens = json_number_field(&input, "total_input_tokens").unwrap_or(0.0);
    let output_tokens = json_number_field(&input, "total_output_tokens").unwrap_or(0.0);
    let cache_read = json_number_field(&input, "cache_read_input_tokens").unwrap_or(0.0);

    let path = snapshot_path("claude", &session);
    write_kv(&path, &[
        ("timestamp", now_unix()),
        ("model", model.clone()),
        ("cost_usd", cost.to_string()),
        ("duration_ms", duration_ms.to_string()),
        ("input_tokens_context", input_tokens.to_string()),
        ("output_tokens_current", output_tokens.to_string()),
        ("cache_read_current", cache_read.to_string()),
    ])?;

    // Remain a valid statusLine command: print compact useful output.
    println!("[{}] ${:.4}", if model.is_empty() {"Claude"} else {&model}, cost);
    Ok(())
}

fn usage_hook(args: &[String]) -> Result<(), String> {
    if args.is_empty() { return Err("usage hook requires codex or claude".into()); }
    match args[0].as_str() {
        "claude" => usage_hook_claude(&args[1..]),
        "codex" => usage_hook_codex(&args[1..]),
        x => Err(format!("unsupported hook adapter: {x}")),
    }
}

fn usage_hook_claude(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let phase = resolve_phase(opts.get("phase").map(String::as_str));
    let file = opts.get("file").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));
    let input = read_stdin_all()?;
    let event = json_string_field(&input, "hook_event_name").unwrap_or_default();
    let session = json_string_field(&input, "session_id").unwrap_or_else(|| "unknown".into());
    let transcript = json_string_field(&input, "transcript_path").unwrap_or_default();

    if event == "UserPromptSubmit" {
        let prompt = json_string_field(&input, "prompt").unwrap_or_else(|| "Claude turn".into());
        let snap = read_kv(&snapshot_path("claude", &session));
        let transcript_lines = if transcript.is_empty() {
            0
        } else {
            fs::read_to_string(&transcript).map(|s| s.lines().count()).unwrap_or(0)
        };
        write_kv(&task_path("claude", &session), &[
            ("phase", phase),
            ("task", prompt.chars().take(160).collect()),
            ("start_cost", snap.get("cost_usd").cloned().unwrap_or_else(|| "0".into())),
            ("start_duration_ms", snap.get("duration_ms").cloned().unwrap_or_else(|| "0".into())),
            ("model", snap.get("model").cloned().unwrap_or_default()),
            ("transcript", transcript),
            ("transcript_lines", transcript_lines.to_string()),
            ("start_head", git_head()),
        ])?;
    } else if event == "Stop" {
        let task = read_kv(&task_path("claude", &session));
        if task.is_empty() {
            return Ok(());
        }
        let snap = read_kv(&snapshot_path("claude", &session));

        let tp = task
            .get("transcript")
            .cloned()
            .filter(|x| !x.is_empty())
            .unwrap_or(transcript);
        let skip: usize = task
            .get("transcript_lines")
            .and_then(|x| x.parse().ok())
            .unwrap_or(0);
        let window = if tp.is_empty() {
            TranscriptWindow::default()
        } else {
            fs::read_to_string(&tp)
                .map(|s| parse_claude_transcript_window(&s, skip))
                .unwrap_or_default()
        };

        let start_head = task.get("start_head").cloned().unwrap_or_default();
        let head = git_head();
        let committed = commit_reached(&start_head, &head);
        let success = update_and_resolve_success("claude", &session, window.verification, committed);

        // statusLine snapshot stays as the cost/duration fallback; it may lag
        // the turn end, which is why tokens now come from the transcript.
        let cost = if snap.is_empty() {
            None
        } else {
            let sc: f64 = task.get("start_cost").and_then(|x| x.parse().ok()).unwrap_or(0.0);
            let ec: f64 = snap.get("cost_usd").and_then(|x| x.parse().ok()).unwrap_or(sc);
            Some((ec - sc).max(0.0))
        };
        let duration = if snap.is_empty() {
            None
        } else {
            let sd: f64 = task.get("start_duration_ms").and_then(|x| x.parse().ok()).unwrap_or(0.0);
            let ed: f64 = snap.get("duration_ms").and_then(|x| x.parse().ok()).unwrap_or(sd);
            Some(((ed - sd) / 1000.0).max(0.0))
        };

        let u = AutoUsage {
            agent: "claude".into(),
            model: if !window.model.is_empty() {
                window.model.clone()
            } else {
                task.get("model").cloned().unwrap_or_default()
            },
            phase: task.get("phase").cloned().unwrap_or_else(|| "standard".into()),
            task: task.get("task").cloned().unwrap_or_else(|| "Claude turn".into()),
            // input includes cache-writes; cache reads are reported separately.
            input_tokens: if window.seen {
                Some(window.input_tokens + window.cache_creation_tokens)
            } else {
                None
            },
            cached_input_tokens: if window.seen { Some(window.cached_input_tokens) } else { None },
            output_tokens: if window.seen { Some(window.output_tokens) } else { None },
            cost_usd: cost,
            duration_seconds: duration,
            success,
            source: if window.seen {
                "claude-transcript-hooks".into()
            } else {
                "claude-statusline-hooks".into()
            },
            session_id: session.clone(),
            notes: if window.seen {
                "Tokens summed from transcript window (input includes cache creation); cost is statusLine session delta when available. Transcript format is not a stable vendor contract.".into()
            } else {
                "Transcript unavailable; estimated session-cost delta from statusLine JSON only.".into()
            },
            // Recorded only when this turn actually produced a commit, so a
            // non-empty value means "this turn closed a task". Storing HEAD
            // unconditionally could not distinguish a first turn that
            // committed from one that merely inherited an existing HEAD.
            head_sha: if committed { head } else { String::new() },
            verification: match window.verification {
                Some(true) => "pass".into(),
                Some(false) => "fail".into(),
                None => String::new(),
            },
            ..Default::default()
        };
        append_auto_usage(&file, &u)?;
        let _ = fs::remove_file(task_path("claude", &session));
    }
    Ok(())
}

fn latest_token_usage_from_codex_transcript(path: &Path) -> (Option<f64>,Option<f64>,Option<f64>) {
    let Ok(s) = fs::read_to_string(path) else { return (None,None,None); };
    let mut last = (None,None,None);
    for line in s.lines() {
        if line.contains("token_count") || line.contains("total_token_usage") || line.contains("\"usage\"") {
            let i = json_number_field(line, "input_tokens");
            let c = json_number_field(line, "cached_input_tokens");
            let o = json_number_field(line, "output_tokens");
            if i.is_some() || c.is_some() || o.is_some() { last=(i,c,o); }
        }
    }
    last
}

fn usage_hook_codex(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let phase = resolve_phase(opts.get("phase").map(String::as_str));
    let file = opts.get("file").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));
    let input = read_stdin_all()?;
    let event = json_string_field(&input, "hook_event_name").unwrap_or_default();
    let session = json_string_field(&input, "session_id").unwrap_or_else(|| "unknown".into());
    let model = json_string_field(&input, "model").unwrap_or_default();
    let transcript = json_string_field(&input, "transcript_path").unwrap_or_default();

    if event == "UserPromptSubmit" {
        let prompt = json_string_field(&input, "prompt").unwrap_or_else(|| "Codex turn".into());
        let (i,c,o) = if transcript.is_empty() {(None,None,None)} else {latest_token_usage_from_codex_transcript(Path::new(&transcript))};
        let transcript_lines = if transcript.is_empty() {
            0
        } else {
            fs::read_to_string(&transcript).map(|s| s.lines().count()).unwrap_or(0)
        };
        write_kv(&task_path("codex", &session), &[
            ("phase", phase),
            ("task", prompt.chars().take(160).collect()),
            ("model", model),
            ("transcript", transcript),
            ("transcript_lines", transcript_lines.to_string()),
            ("start_head", git_head()),
            ("start_input", num_cell(i)),
            ("start_cached", num_cell(c)),
            ("start_output", num_cell(o)),
        ])?;
    } else if event == "Stop" {
        let task = read_kv(&task_path("codex", &session));
        if !task.is_empty() {
            let tp = task
                .get("transcript")
                .cloned()
                .filter(|x| !x.is_empty())
                .unwrap_or(transcript);
            let (ei,ec,eo) = if tp.is_empty() {(None,None,None)} else {latest_token_usage_from_codex_transcript(Path::new(&tp))};
            let parse = |k:&str| task.get(k).and_then(|x| x.parse::<f64>().ok());
            let delta = |end:Option<f64>, start:Option<f64>| match (end,start) {(Some(e),Some(s))=>Some((e-s).max(0.0)),(Some(e),None)=>Some(e),_=>None};

            let skip: usize = task.get("transcript_lines").and_then(|x| x.parse().ok()).unwrap_or(0);
            let verification = if tp.is_empty() {
                None
            } else {
                fs::read_to_string(&tp)
                    .ok()
                    .and_then(|s| scan_codex_window_for_tests(&s, skip))
            };
            let start_head = task.get("start_head").cloned().unwrap_or_default();
            let head = git_head();
            let committed = commit_reached(&start_head, &head);
            let success = update_and_resolve_success("codex", &session, verification, committed);

            let u = AutoUsage {
                agent: "codex".into(),
                model: task.get("model").cloned().unwrap_or_default(),
                phase: task.get("phase").cloned().unwrap_or_else(|| "standard".into()),
                task: task.get("task").cloned().unwrap_or_else(|| "Codex turn".into()),
                input_tokens: delta(ei, parse("start_input")),
                cached_input_tokens: delta(ec, parse("start_cached")),
                output_tokens: delta(eo, parse("start_output")),
                success,
                source: "codex-hook-transcript-best-effort".into(),
                session_id: session.clone(),
                notes: "Best-effort interactive capture: Codex documents transcript_path but warns transcript format is not a stable hook interface. Prefer usage run codex for stable JSON usage.".into(),
                head_sha: if committed { head } else { String::new() },
                verification: match verification {
                    Some(true) => "pass".into(),
                    Some(false) => "fail".into(),
                    None => String::new(),
                },
                ..Default::default()
            };
            append_auto_usage(&file, &u)?;
            let _ = fs::remove_file(task_path("codex", &session));
        }
    }
    Ok(())
}



fn sum_json_numbers_after_key(s: &str, key: &str) -> f64 {
    let needle = format!("\"{}\"", key);
    let mut rest = s;
    let mut total = 0.0;
    while let Some(pos) = rest.find(&needle) {
        rest = &rest[pos + needle.len()..];
        if let Some(colon) = rest.find(':') {
            let after = rest[colon + 1..].trim_start();
            let token: String = after.chars()
                .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+' | 'e' | 'E'))
                .collect();
            if let Ok(v) = token.parse::<f64>() { total += v; }
            rest = after;
        } else {
            break;
        }
    }
    total
}

fn github_copilot_snapshot(opts: &BTreeMap<String, String>) -> Result<f64, String> {
    let user = require(opts, "github-user")?;
    let endpoint = if let Some(org) = opts.get("org") {
        format!("organizations/{org}/settings/billing/ai_credit/usage?user={user}")
    } else {
        format!("users/{user}/settings/billing/ai_credit/usage")
    };

    let out = Command::new("gh")
        .args([
            "api",
            "-H", "Accept: application/vnd.github+json",
            "-H", "X-GitHub-Api-Version: 2026-03-10",
            endpoint.as_str(),
        ])
        .output()
        .map_err(|e| format!("failed to run `gh api`: {e}. Install/authenticate GitHub CLI or use --snapshot-credits."))?;

    if !out.status.success() {
        return Err(format!(
            "GitHub AI Credits API failed ({}): {}. Personal plans use the user endpoint; organization-billed Copilot may require organization/enterprise permissions. You can use --snapshot-credits as a fallback.",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let body = String::from_utf8_lossy(&out.stdout);
    let credits = sum_json_numbers_after_key(&body, "netQuantity");
    Ok(credits)
}

fn usage_snapshot(args: &[String]) -> Result<(), String> {
    if args.is_empty() { return Err("usage snapshot requires a provider (currently: copilot)".into()); }
    let (opts, _) = parse_options(&args[1..])?;
    match args[0].as_str() {
        "copilot" => {
            let credits = github_copilot_snapshot(&opts)?;
            println!("SENTRITH-USAGE-SNAPSHOT: copilot_ai_credits={:.6}", credits);
            Ok(())
        }
        other => Err(format!("unsupported snapshot provider: {other}")),
    }
}

fn active_task_file() -> PathBuf {
    PathBuf::from(".ai-usage/active-task.tsv")
}

fn usage_task(args: &[String]) -> Result<(), String> {
    if args.is_empty() { return Err("usage task requires start or stop".into()); }
    match args[0].as_str() {
        "start" => usage_task_start(&args[1..]),
        "stop" => usage_task_stop(&args[1..]),
        x => Err(format!("unknown usage task command: {x}")),
    }
}

fn usage_task_start(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let agent = require(&opts, "agent")?;
    let task = require(&opts, "task")?;
    let phase = resolve_phase(opts.get("phase").map(String::as_str));
    let model = opts.get("model").cloned().unwrap_or_default();
    let category = opts.get("category").cloned().unwrap_or_else(|| "unspecified".into());

    let start_credits = if let Some(v) = opts.get("snapshot-credits") {
        v.parse::<f64>().map_err(|_| "invalid --snapshot-credits")?
    } else if agent == "copilot" && opts.contains_key("github-user") {
        github_copilot_snapshot(&opts)?
    } else {
        0.0
    };
    let has_credits = opts.contains_key("snapshot-credits") || (agent == "copilot" && opts.contains_key("github-user"));

    write_kv(&active_task_file(), &[
        ("started_at", now_unix()),
        ("agent", agent.to_string()),
        ("model", model),
        ("phase", phase),
        ("task", task.to_string()),
        ("category", category),
        ("start_credits", start_credits.to_string()),
        ("has_credits", has_credits.to_string()),
        ("github_user", opts.get("github-user").cloned().unwrap_or_default()),
        ("org", opts.get("org").cloned().unwrap_or_default()),
    ])?;

    println!("SENTRITH-TASK: started `{task}` ({agent})");
    Ok(())
}

fn usage_task_stop(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let active = read_kv(&active_task_file());
    if active.is_empty() { return Err("no active Sentrith task".into()); }

    let agent = active.get("agent").cloned().unwrap_or_else(|| "other".into());
    let success = opts.get("success").cloned().unwrap_or_default();
    let rework = opts.get("rework").and_then(|x| x.parse::<f64>().ok());
    let started: f64 = active.get("started_at").and_then(|x| x.parse().ok()).unwrap_or(0.0);
    let ended: f64 = now_unix().parse().unwrap_or(started);
    let duration = if started > 0.0 { Some((ended-started).max(0.0)) } else { None };

    let mut credits = None;
    if active.get("has_credits").map(String::as_str) == Some("true") {
        let start: f64 = active.get("start_credits").and_then(|x| x.parse().ok()).unwrap_or(0.0);
        let end = if let Some(v) = opts.get("snapshot-credits") {
            v.parse::<f64>().map_err(|_| "invalid --snapshot-credits")?
        } else if agent == "copilot" && !active.get("github_user").cloned().unwrap_or_default().is_empty() {
            let mut snap_opts = BTreeMap::new();
            snap_opts.insert("github-user".to_string(), active.get("github_user").cloned().unwrap_or_default());
            if let Some(org) = active.get("org").filter(|x| !x.is_empty()) {
                snap_opts.insert("org".to_string(), org.clone());
            }
            github_copilot_snapshot(&snap_opts)?
        } else {
            start
        };
        credits = Some((end-start).max(0.0));
    }

    let u = AutoUsage {
        agent: agent.clone(),
        model: active.get("model").cloned().unwrap_or_default(),
        phase: active.get("phase").cloned().unwrap_or_else(|| "standard".into()),
        task: active.get("task").cloned().unwrap_or_else(|| "task".into()),
        credits,
        cost_usd: opts.get("cost-usd").and_then(|x| x.parse().ok()),
        input_tokens: opts.get("input").and_then(|x| x.parse().ok()),
        cached_input_tokens: opts.get("cached-input").and_then(|x| x.parse().ok()),
        output_tokens: opts.get("output").and_then(|x| x.parse().ok()),
        duration_seconds: duration,
        success,
        rework_count: rework,
        source: if credits.is_some() && agent == "copilot" {
            "github-ai-credits-snapshot-delta".into()
        } else {
            "sentrith-task-ledger".into()
        },
        notes: format!(
            "category={}; {}",
            active.get("category").cloned().unwrap_or_else(|| "unspecified".into()),
            opts.get("notes").cloned().unwrap_or_default()
        ),
        ..Default::default()
    };
    append_auto_usage(Path::new(".ai-usage/usage.csv"), &u)?;
    let _ = fs::remove_file(active_task_file());
    println!("SENTRITH-TASK: stopped; usage recorded");
    Ok(())
}

fn metric_sum(rows: &[&BTreeMap<String,String>], metric: &str) -> Option<f64> {
    if metric == "tokens" {
        let mut found = false;
        let mut total = 0.0;
        for r in rows {
            let i = r.get("input_tokens").and_then(|x| x.parse::<f64>().ok());
            let o = r.get("output_tokens").and_then(|x| x.parse::<f64>().ok());
            if i.is_some() || o.is_some() {
                found = true;
                total += i.unwrap_or(0.0) + o.unwrap_or(0.0);
            }
        }
        if found { Some(total) } else { None }
    } else {
        sum_field(rows, metric)
    }
}

/// Usage per successful *task*. Counting successful rows would divide by the
/// number of turns, not the number of tasks.
fn metric_per_success_for_tasks(
    tasks: &[Vec<&BTreeMap<String, String>>],
    metric: &str,
) -> Option<f64> {
    let successes = tasks.iter().filter(|t| task_success(t) == "yes").count();
    if successes == 0 {
        return None;
    }
    let rows: Vec<&BTreeMap<String, String>> =
        tasks.iter().flat_map(|task| task.iter().copied()).collect();
    metric_sum(&rows, metric).map(|v| v / successes as f64)
}

fn metric_per_success(rows: &[&BTreeMap<String,String>], metric: &str) -> Option<f64> {
    metric_per_success_for_tasks(&group_tasks(rows), metric)
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
}

fn usage_contribute(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let agent = require(&opts, "agent")?;
    let model = opts.get("model").map(String::as_str);
    let file = opts.get("file").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));
    let rows = load_usage_rows(&file, Some(agent))?;
    let refs: Vec<&BTreeMap<String, String>> = rows.iter().collect();
    let tasks = filter_tasks_by_model(group_tasks(&refs), model);
    let by_phase = tasks_by_phase_from_tasks(tasks);
    let baseline_tasks = by_phase.get("baseline").map(Vec::as_slice).unwrap_or(&[]);
    let standard_tasks = by_phase.get("standard").map(Vec::as_slice).unwrap_or(&[]);

    // Qualification counts tasks, not captured turns.
    let baseline_task_count = baseline_tasks.len();
    let standard_task_count = standard_tasks.len();

    let min_samples: usize = opts.get("min-samples").and_then(|x| x.parse().ok()).unwrap_or(10);
    if !opts.contains_key("force") && (baseline_task_count < min_samples || standard_task_count < min_samples) {
        return Err(format!("contribution needs at least {min_samples}+{min_samples} baseline/standard tasks; got {baseline_task_count}+{standard_task_count}. Use --force only for experimental data."));
    }

    let requested = opts.get("metric").map(String::as_str).unwrap_or("auto");
    let candidates: Vec<&str> = if requested == "auto" {
        vec!["credits", "cost_usd", "tokens"]
    } else {
        vec![requested]
    };
    let mut chosen = None;
    for m in candidates {
        if metric_per_success_for_tasks(baseline_tasks, m).is_some()
            && metric_per_success_for_tasks(standard_tasks, m).is_some()
        {
            chosen = Some(m);
            break;
        }
    }
    let metric = chosen.ok_or("no comparable usage metric found in both baseline and standard")?;
    let bps = metric_per_success_for_tasks(baseline_tasks, metric).unwrap();
    let sps = metric_per_success_for_tasks(standard_tasks, metric).unwrap();
    let change = if bps != 0.0 { (sps-bps)/bps*100.0 } else { 0.0 };
    let bsr = decided_success_rate(baseline_tasks).unwrap_or(0.0);
    let ssr = decided_success_rate(standard_tasks).unwrap_or(0.0);
    let quality = if baseline_task_count >= 10 && standard_task_count >= 10 { "qualified" } else { "experimental" };
    let model_name = model.unwrap_or("mixed/unspecified");
    let id = format!("{}-{}-{}", agent, now_unix(), std::process::id());
    let out = opts.get("out").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(format!("docs/metrics/contributions/{id}.json")));
    if let Some(parent)=out.parent(){ fs::create_dir_all(parent).map_err(|e|e.to_string())?; }

    let body = format!(
r#"{{
  "schema_version": 1,
  "sentrith_version": "{}",
  "contribution_id": "{}",
  "agent": "{}",
  "model": "{}",
  "quality": "{}",
  "metric": "{}",
  "baseline_tasks": {},
  "standard_tasks": {},
  "baseline_success_rate": {:.3},
  "standard_success_rate": {:.3},
  "baseline_usage_per_success": {:.6},
  "standard_usage_per_success": {:.6},
  "normalized_usage_change_pct": {:.6},
  "created_at_unix": {}
}}
"#,
        env!("CARGO_PKG_VERSION"),
        json_escape(&id),
        json_escape(agent),
        json_escape(model_name),
        quality,
        metric,
        baseline_task_count,
        standard_task_count,
        bsr,
        ssr,
        bps,
        sps,
        change,
        now_unix()
    );
    fs::write(&out, body).map_err(|e| e.to_string())?;
    println!("SENTRITH-CONTRIBUTE: {}", out.display());
    println!("SENTRITH-CONTRIBUTE: raw prompts/repository/source/transcript/session data were not exported");
    Ok(())
}

fn median(mut vals: Vec<f64>) -> Option<f64> {
    if vals.is_empty() { return None; }
    vals.sort_by(|a,b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n=vals.len();
    if n%2==1 { Some(vals[n/2]) } else { Some((vals[n/2-1]+vals[n/2])/2.0) }
}

fn usage_aggregate(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let dir = opts.get("dir").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("docs/metrics/contributions"));
    if !dir.exists() {
        println!("SENTRITH-COMMUNITY: no contribution directory");
        return Ok(());
    }
    let mut changes=Vec::new();
    let mut baseline_tasks=0usize;
    let mut standard_tasks=0usize;
    let mut files=0usize;
    for ent in fs::read_dir(&dir).map_err(|e|e.to_string())? {
        let p=ent.map_err(|e|e.to_string())?.path();
        if p.extension().and_then(|x|x.to_str()) != Some("json") { continue; }
        let s=fs::read_to_string(&p).map_err(|e|e.to_string())?;
        if json_number_field(&s,"schema_version") != Some(1.0) { continue; }
        let quality=json_string_field(&s,"quality").unwrap_or_default();
        if quality!="qualified" && !opts.contains_key("include-experimental") { continue; }
        if let Some(v)=json_number_field(&s,"normalized_usage_change_pct"){ changes.push(v); }
        baseline_tasks += json_number_field(&s,"baseline_tasks").unwrap_or(0.0) as usize;
        standard_tasks += json_number_field(&s,"standard_tasks").unwrap_or(0.0) as usize;
        files += 1;
    }
    let med=median(changes);
    println!("SENTRITH-COMMUNITY: contributions={files} baseline_tasks={baseline_tasks} standard_tasks={standard_tasks} median_change={}", med.map(|x|format!("{:+.1}%",x)).unwrap_or_else(||"-".into()));

    if opts.contains_key("publish") {
        let body_ja = if let Some(m)=med {
            format!("### Community Benchmark\n\n有志ユーザーの匿名化済み・qualified contributionを集計しています。\n\n- Contributions: **{files}**\n- Baseline tasks: **{baseline_tasks}**\n- Sentrith tasks: **{standard_tasks}**\n- Median normalized usage / successful task: **{m:+.1}%**\n\n> Community-reported benchmarkです。Providerごとのnative単位は混ぜず、各環境のbaseline比を集計します。")
        } else {
            "### Community Benchmark\n\nまだqualified contributionがありません。`sentrith usage contribute` で匿名化済み結果を投稿できます。".into()
        };
        let body_en = if let Some(m)=med {
            format!("### Community Benchmark\n\nAggregated from anonymized, qualified community contributions.\n\n- Contributions: **{files}**\n- Baseline tasks: **{baseline_tasks}**\n- Sentrith tasks: **{standard_tasks}**\n- Median normalized usage / successful task: **{m:+.1}%**\n\n> Community-reported benchmark. Provider-native units are kept separate; aggregation uses each environment's baseline-relative change.")
        } else {
            "### Community Benchmark\n\nNo qualified contributions yet. Generate an anonymized contribution with `sentrith usage contribute`.".into()
        };
        replace_marked_section(Path::new("README.ja.md"), "<!-- SENTRITH-COMMUNITY:BEGIN -->", "<!-- SENTRITH-COMMUNITY:END -->", &body_ja)?;
        replace_marked_section(Path::new("README.md"), "<!-- SENTRITH-COMMUNITY:BEGIN -->", "<!-- SENTRITH-COMMUNITY:END -->", &body_en)?;
        println!("SENTRITH-COMMUNITY: README community section updated");
    }
    Ok(())
}

fn usage_note(args: &[String]) -> Result<(), String> {
    let (opts, positional) = parse_options(args)?;
    if positional.is_empty() {
        return Err("usage note requires text".into());
    }
    let text = positional.join(" ");
    let file = opts
        .get("file")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".ai-usage/status-notes.log"));
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .map_err(|e| e.to_string())?;
    writeln!(f, "{}\t{}", now_unix(), text).map_err(|e| e.to_string())?;
    println!("SENTRITH-USAGE: note appended -> {}", file.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escape_quotes_commas() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn csv_parser_handles_quotes() {
        let v = parse_csv_line("a,\"b,c\",\"d\"\"e\"");
        assert_eq!(v, vec!["a", "b,c", "d\"e"]);
    }

    #[test]
    fn pct_change_formats() {
        assert_eq!(pct_text(Some(100.0), Some(75.0)), "-25.0%");
    }

    fn temp_path(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let dir = env::temp_dir().join(format!(
            "sentrith-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn assistant_line(msg_id: &str, input: u64, cache_read: u64, output: u64) -> String {
        format!(
            r#"{{"type":"assistant","message":{{"model":"claude-fable-5","id":"{msg_id}","type":"message","role":"assistant","content":[{{"type":"text","text":"hi"}}],"usage":{{"input_tokens":{input},"cache_creation_input_tokens":0,"cache_read_input_tokens":{cache_read},"output_tokens":{output},"output_tokens_details":{{"thinking_tokens":1}}}}}}}}"#
        )
    }

    fn bash_line(msg_id: &str, tool_id: &str, command: &str) -> String {
        format!(
            r#"{{"type":"assistant","message":{{"model":"claude-fable-5","id":"{msg_id}","type":"message","role":"assistant","content":[{{"type":"tool_use","id":"{tool_id}","name":"Bash","input":{{"command":"{command}","description":"run"}}}}],"usage":{{"input_tokens":1,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":1}}}}}}"#
        )
    }

    fn tool_result_line(tool_id: &str, is_error: bool) -> String {
        format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"tool_use_id":"{tool_id}","type":"tool_result","content":"output","is_error":{is_error}}}]}}}}"#
        )
    }

    #[test]
    fn transcript_window_sums_tokens_once_per_message_id() {
        // One API message is serialized as several transcript lines that all
        // repeat the same usage object; it must be counted once.
        let text = [
            assistant_line("msg_old", 999, 999, 999),
            assistant_line("msg_a", 10, 100, 5),
            assistant_line("msg_a", 10, 100, 5),
            assistant_line("msg_b", 20, 200, 7),
        ]
        .join("\n");

        let w = parse_claude_transcript_window(&text, 1);
        assert!(w.seen);
        assert_eq!(w.input_tokens, 30.0);
        assert_eq!(w.cached_input_tokens, 300.0);
        assert_eq!(w.output_tokens, 12.0);
        assert_eq!(w.model, "claude-fable-5");
    }

    #[test]
    fn transcript_window_skips_lines_before_the_turn() {
        let text = [
            assistant_line("msg_before", 500, 500, 500),
            assistant_line("msg_after", 1, 2, 3),
        ]
        .join("\n");

        let w = parse_claude_transcript_window(&text, 1);
        assert_eq!(w.input_tokens, 1.0);
        assert_eq!(w.output_tokens, 3.0);
    }

    #[test]
    fn transcript_window_detects_test_failure_then_pass() {
        let text = [
            bash_line("msg_1", "toolu_fail", "cargo test --manifest-path x"),
            tool_result_line("toolu_fail", true),
            bash_line("msg_2", "toolu_pass", "cargo test --manifest-path x"),
            tool_result_line("toolu_pass", false),
        ]
        .join("\n");

        assert_eq!(parse_claude_transcript_window(&text, 0).verification, Some(true));

        let only_failure = [
            bash_line("msg_1", "toolu_fail", "pytest -q"),
            tool_result_line("toolu_fail", true),
        ]
        .join("\n");
        assert_eq!(
            parse_claude_transcript_window(&only_failure, 0).verification,
            Some(false)
        );
    }

    #[test]
    fn non_test_commands_do_not_set_verification() {
        let text = [
            bash_line("msg_1", "toolu_ls", "ls -la"),
            tool_result_line("toolu_ls", false),
        ]
        .join("\n");
        assert_eq!(parse_claude_transcript_window(&text, 0).verification, None);
    }

    #[test]
    fn test_command_matching_respects_word_boundaries() {
        assert!(is_test_command("cargo test --manifest-path tools/sentrith/Cargo.toml"));
        assert!(is_test_command("uv run pytest tests/"));
        assert!(is_test_command("npm test"));
        assert!(!is_test_command("git log --oneline"));
        // `contest` must not match `go test`, `pytest-cov` must not match bare `pytest`
        assert!(!is_test_command("echo pytestcov"));
        assert!(!is_test_command("cargo testbench"));
        assert!(!is_test_command(r#"echo "cargo test""#));
        assert!(!is_test_command(r#"rg "cargo test" docs"#));
        assert!(is_test_command("env CI=1 cargo test"));
        assert!(is_test_command(r#""/opt/project/bin/cargo" test"#));
        assert!(!is_test_command("cd tools && cargo test"));
        assert!(!is_test_command("cd tools\ncargo test"));
        assert!(!is_test_command("cargo fmt --check && cargo test"));
        assert!(!is_test_command("cargo test && echo done"));
        assert!(!is_test_command("cargo test || true"));
        assert!(!is_test_command("cargo test; echo done"));
        assert!(!is_test_command("cargo test | tee test.log"));
        assert!(!is_test_command("cargo test --no-run"));
        assert!(!is_test_command("cargo test --no-run"));
        assert!(!is_test_command("cargo test -- --list"));
        // cargo's `Usage: cargo [+toolchain] [OPTIONS] [COMMAND]`: `+nightly`
        // does not start with `-`, so it must be stripped explicitly or it is
        // mistaken for the subcommand and the following `test` is missed.
        assert!(is_test_command("cargo +nightly test"));
        assert!(is_test_command("cargo +stable test --workspace"));
        assert!(is_test_command("cargo +1.75.0 test"));
        assert!(!is_test_command("cargo +nightly test --no-run"));
        assert!(!is_test_command("cargo +nightly build"));
        // `+` has no such meaning for other runners; it must not be stripped
        // there, so this stays unrecognized rather than false-positive.
        assert!(!is_test_command("npm +nightly test"));
        assert!(!is_test_command("pytest --collect-only"));
        assert!(!is_test_command("python -m pytest --collect-only"));
        assert!(!is_test_command("go test -list ."));
        assert!(!is_test_command("dotnet test --list-tests"));
    }

    #[test]
    fn codex_window_matches_command_and_result_on_the_same_line_only() {
        let same_line = r#"{"command":"cargo test","exit_code":0}"#;
        assert_eq!(scan_codex_window_for_tests(same_line, 0), Some(true));

        // A result on a later line is intentionally no longer attributed to an
        // earlier command; see
        // `codex_command_result_is_never_borrowed_from_a_different_line` for
        // why cross-line matching was removed rather than merely narrowed.
        let split = [
            r#"{"type":"exec","command":"pytest -q"}"#,
            r#"{"type":"exec_result","exit_code":1}"#,
        ]
        .join("\n");
        assert_eq!(scan_codex_window_for_tests(&split, 0), None);

        let unrelated = r#"{"command":"ls","exit_code":1}"#;
        assert_eq!(scan_codex_window_for_tests(unrelated, 0), None);

        let multiline = r#"{"command":"cd tools\ncargo test","exit_code":0}"#;
        assert_eq!(scan_codex_window_for_tests(multiline, 0), None);
    }

    #[test]
    fn codex_window_reads_the_real_command_execution_shape() {
        // Structure verified against a real `~/.codex/sessions/**/rollout-
        // *.jsonl` file: `command` is an argv array, not a plain string, and
        // `exit_code` sits in the same object.
        let passing = r#"{"payload":{"type":"item_completed","item":{"type":"CommandExecution","id":"exec-1","command":["/bin/bash","-lc","cargo test"],"exit_code":0,"status":"completed"}}}"#;
        assert_eq!(scan_codex_window_for_tests(passing, 0), Some(true));

        let failing = r#"{"payload":{"item":{"type":"CommandExecution","command":["/bin/bash","-lc","pytest -q"],"exit_code":1}}}"#;
        assert_eq!(scan_codex_window_for_tests(failing, 0), Some(false));

        let non_test = r#"{"payload":{"item":{"type":"CommandExecution","command":["/bin/bash","-lc","ls -la"],"exit_code":0}}}"#;
        assert_eq!(scan_codex_window_for_tests(non_test, 0), None);
    }

    #[test]
    fn codex_command_result_is_never_borrowed_from_a_different_line() {
        // Codex can log commands in parallel, so a later line's exit_code may
        // belong to a different, unrelated command. Same-line-only matching
        // must not attribute it to an earlier test command that had none of
        // its own, rather than guessing via a flag carried over from that
        // earlier line.
        let interleaved = [
            r#"{"command":"cargo test"}"#,
            r#"{"command":"ls","exit_code":0}"#,
        ]
        .join("
");
        assert_eq!(
            scan_codex_window_for_tests(&interleaved, 0),
            None,
            "must not borrow ls's exit code for cargo test"
        );

        let interleaved_real_shape = [
            r#"{"payload":{"item":{"type":"CommandExecution","command":["/bin/bash","-lc","cargo test"]}}}"#,
            r#"{"payload":{"item":{"type":"CommandExecution","command":["/bin/bash","-lc","ls"],"exit_code":0}}}"#,
        ]
        .join("
");
        assert_eq!(scan_codex_window_for_tests(&interleaved_real_shape, 0), None);
    }

    #[test]
    fn json_string_array_last_reads_the_final_element() {
        assert_eq!(
            json_string_array_last(r#"{"command":["/bin/bash","-lc","cargo test"]}"#, "command"),
            Some("cargo test".to_string())
        );
        assert_eq!(json_string_array_last(r#"{"command":[]}"#, "command"), None);
        assert_eq!(json_string_array_last(r#"{"command":"cargo test"}"#, "command"), None);
    }

    #[test]
    fn usage_lock_waits_for_an_os_owned_lock() {
        let path = temp_path("usage.csv");
        let lock = path.with_extension("csv.lock");
        let holder = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock)
            .unwrap();
        holder.try_lock().unwrap();

        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let worker_path = path.clone();
        let worker = thread::spawn(move || {
            with_usage_file_lock(&worker_path, || {
                entered_tx.send(()).unwrap();
                Ok(())
            })
        });

        assert!(
            entered_rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "a held OS lock must keep another writer out"
        );
        drop(holder);
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("the waiter should enter after the lock is released");
        worker.join().unwrap().unwrap();
        let _ = fs::remove_file(lock);
    }

    #[test]
    fn v1_usage_file_migrates_to_v2_preserving_rows() {
        let path = temp_path("usage.csv");
        let v1_row = "1,claude,m,standard,\"task, with comma\",1,2,3,,,,,yes,0,manual,sess,note";
        fs::write(&path, format!("{}{}\n", USAGE_HEADER_V1, v1_row)).unwrap();

        ensure_usage_file(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let mut lines = text.lines();
        assert_eq!(lines.next().unwrap(), USAGE_HEADER.trim_end());
        let cols = parse_csv_line(lines.next().unwrap());
        assert_eq!(cols.len(), USAGE_HEADER.trim_end().split(',').count());
        assert_eq!(cols[4], "task, with comma");
        assert_eq!(cols[12], "yes");
        assert_eq!(cols[17], "");
        assert_eq!(cols[18], "");

        // Migration must be idempotent.
        ensure_usage_file(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), text);
    }

    #[test]
    fn preexisting_unlocked_usage_lock_file_is_reused() {
        let path = temp_path("usage.csv");
        let lock = path.with_extension("csv.lock");
        fs::write(&lock, "pid=999999\ncreated=0\n").unwrap();

        with_usage_file_lock(&path, || Ok(())).unwrap();

        assert!(lock.exists(), "the persistent lock file must remain for safe reuse");
        let _ = fs::remove_file(lock);
    }

    #[test]
    fn concurrent_first_appends_preserve_migrated_rows() {
        let path = temp_path("usage.csv");
        fs::write(&path, USAGE_HEADER_V1).unwrap();

        let first_path = path.clone();
        let first = thread::spawn(move || {
            let usage = AutoUsage {
                agent: "codex".into(),
                task: "first".into(),
                ..Default::default()
            };
            append_auto_usage(&first_path, &usage)
        });
        let second_path = path.clone();
        let second = thread::spawn(move || {
            let usage = AutoUsage {
                agent: "codex".into(),
                task: "second".into(),
                ..Default::default()
            };
            append_auto_usage(&second_path, &usage)
        });

        first.join().unwrap().unwrap();
        second.join().unwrap().unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 3, "v2 header plus both appended rows");
        assert!(text.lines().any(|line| line.contains(",first,")));
        assert!(text.lines().any(|line| line.contains(",second,")));
    }

    #[test]
    fn csv_records_survive_newlines_inside_quoted_fields() {
        let text = "a,b\n\"line one\nline two\",x\n\"has \"\"quote\"\" and\nnewline\",y\n";
        let records = split_csv_records(text);
        assert_eq!(records.len(), 3, "one header plus two logical records");
        assert_eq!(parse_csv_line(&records[1]), vec!["line one\nline two", "x"]);
        assert_eq!(
            parse_csv_line(&records[2]),
            vec!["has \"quote\" and\nnewline", "y"]
        );

        // CRLF files must split the same way.
        let crlf = "a,b\r\n\"one\r\ntwo\",x\r\n";
        assert_eq!(split_csv_records(crlf).len(), 2);
    }

    #[test]
    fn migration_does_not_corrupt_multiline_fields() {
        // `csv_escape` quotes embedded newlines, so a v1 row can span physical
        // lines. Appending columns per line would inject commas into the field.
        let path = temp_path("usage.csv");
        let v1_row = "1,claude,m,standard,\"first line\nsecond line\",1,2,3,,,,,yes,0,manual,sess,note";
        fs::write(&path, format!("{}{}\n", USAGE_HEADER_V1, v1_row)).unwrap();

        ensure_usage_file(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let records = split_csv_records(&text);
        assert_eq!(records[0], USAGE_HEADER.trim_end());
        let cols = parse_csv_line(&records[1]);
        assert_eq!(
            cols[4], "first line\nsecond line",
            "the multiline field must be unchanged"
        );
        assert_eq!(cols.len(), USAGE_HEADER.trim_end().split(',').count());
        assert_eq!(cols[17], "");
        assert_eq!(cols[18], "");
    }

    #[test]
    fn phase_comparison_is_per_task_not_per_turn() {
        // Codex's repro: a three-turn baseline task and a one-turn standard
        // task, each totaling 30 tokens. Per-turn averaging would report
        // 10 vs 30; per task they are equal.
        let baseline_owned = vec![
            turn("s1", "", "unknown", "10"),
            turn("s1", "", "unknown", "10"),
            turn("s1", "aaa", "yes", "10"),
        ];
        let standard_owned = vec![turn("s2", "bbb", "yes", "30")];

        let baseline: Vec<&BTreeMap<String, String>> = baseline_owned.iter().collect();
        let standard: Vec<&BTreeMap<String, String>> = standard_owned.iter().collect();

        let b = phase_summary(&group_tasks(&baseline));
        let s = phase_summary(&group_tasks(&standard));

        assert_eq!(b.get("tasks").copied().flatten(), Some(1.0));
        assert_eq!(s.get("tasks").copied().flatten(), Some(1.0));
        assert_eq!(b.get("input_tokens").copied().flatten(), Some(30.0));
        assert_eq!(s.get("input_tokens").copied().flatten(), Some(30.0));
        assert_eq!(
            pct_text(
                b.get("input_tokens").copied().flatten(),
                s.get("input_tokens").copied().flatten()
            ),
            "+0.0%"
        );
    }

    #[test]
    fn tasks_are_grouped_before_phase_partitioning() {
        let mut baseline_turn = turn("s1", "", "unknown", "10");
        baseline_turn.insert("phase".into(), "baseline".into());
        let standard_turn = turn("s1", "aaa", "yes", "20");
        let owned = vec![baseline_turn, standard_turn];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();

        let by_phase = tasks_by_phase(&rows);
        assert!(by_phase.get("baseline").is_none());
        let standard = by_phase.get("standard").unwrap();
        assert_eq!(standard.len(), 1);
        assert_eq!(standard[0].len(), 2);
        assert_eq!(
            publish_stats_for_tasks(standard).input_avg,
            Some(30.0),
            "a task crossing baseline stop stays one task"
        );
    }

    #[test]
    fn phase_precedence_flag_then_marker_then_env_then_default() {
        assert_eq!(
            resolve_phase_value(Some("other"), Some("baseline"), Some("standard")),
            "other"
        );
        // The marker outranks the environment: a variable exported after the
        // agent started never reaches the hook process.
        assert_eq!(
            resolve_phase_value(None, Some("baseline"), Some("standard")),
            "baseline"
        );
        assert_eq!(resolve_phase_value(None, None, Some("baseline")), "baseline");
        assert_eq!(resolve_phase_value(None, None, None), "standard");
        assert_eq!(
            resolve_phase_value(Some("  "), None, Some("baseline")),
            "baseline"
        );
        assert_eq!(
            resolve_phase_value(None, Some("baseline\n"), None),
            "baseline"
        );
    }

    #[test]
    fn json_round_trips_and_preserves_key_order() {
        let src = r#"{"b":1,"a":[true,false,null,"x\ny"],"n":-1.5e3,"o":{},"e":[]}"#;
        let v = json_parse(src).unwrap();
        let rendered = json_to_string(&v);
        let again = json_parse(&rendered).unwrap();
        assert_eq!(v, again);
        if let Json::Obj(entries) = &v {
            let keys: Vec<&str> = entries.iter().map(|(k, _)| k.as_str()).collect();
            assert_eq!(keys, vec!["b", "a", "n", "o", "e"]);
        } else {
            panic!("expected object");
        }
        assert_eq!(v.get("n").unwrap(), &Json::Num("-1.5e3".into()));
    }

    #[test]
    fn json_handles_unicode_and_escapes() {
        let v = json_parse(r#"{"k":"日本語 é \" \\ tab\there"}"#).unwrap();
        assert_eq!(v.get("k").unwrap().as_str().unwrap(), "日本語 é \" \\ tab\there");
        let again = json_parse(&json_to_string(&v)).unwrap();
        assert_eq!(v, again);
    }

    #[test]
    fn json_rejects_malformed_input() {
        assert!(json_parse("{").is_err());
        assert!(json_parse(r#"{"a":1}}"#).is_err());
        assert!(json_parse(r#"{"a" 1}"#).is_err());
        assert!(json_parse("").is_err());
        assert!(json_parse(r#"{"a":"\uZZZZ"}"#).is_err());
    }

    #[test]
    fn json_decodes_surrogate_pairs_without_corrupting_them() {
        // A user's settings may contain an emoji. Decoding each half separately
        // would rewrite it as two replacement characters.
        let v = json_parse(r#"{"k":"a😀b"}"#).unwrap();
        assert_eq!(v.get("k").unwrap().as_str().unwrap(), "a\u{1F600}b");

        let round = json_parse(&json_to_string(&v)).unwrap();
        assert_eq!(round, v);
        assert!(
            !json_to_string(&v).contains('\u{FFFD}'),
            "must not emit replacement characters"
        );

        // Unpaired surrogates are invalid JSON; refuse rather than corrupt.
        assert!(json_parse(r#"{"k":"\ud83d"}"#).is_err());
        assert!(json_parse(r#"{"k":"\ude00"}"#).is_err());
        assert!(json_parse(r#"{"k":"\ud83dx"}"#).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn usage_migration_preserves_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let path = temp_path("usage.csv");
        fs::write(&path, USAGE_HEADER_V1).unwrap();
        let mut permissions = fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&path, permissions).unwrap();

        ensure_usage_file(&path).unwrap();

        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(windows)]
    #[test]
    fn windows_replacement_preserves_readonly_attribute() {
        let original = temp_path("settings.json");
        let replacement = temp_path("settings.tmp");
        fs::write(&original, "{}").unwrap();
        let mut permissions = fs::metadata(&original).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&original, permissions).unwrap();
        fs::write(&replacement, "{}").unwrap();

        replace_file_preserving_security(&replacement, &original, None).unwrap();

        assert!(fs::metadata(&original).unwrap().permissions().readonly());
    }

    #[cfg(windows)]
    #[test]
    fn windows_replacement_backup_carries_the_original_security_descriptor() {
        let original = temp_path("settings.json");
        let replacement = temp_path("settings.tmp");
        let backup = temp_path("settings.bak");
        fs::write(&original, "original-secret").unwrap();
        let mut permissions = fs::metadata(&original).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&original, permissions).unwrap();
        fs::write(&replacement, "reduced").unwrap();

        replace_file_preserving_security(&replacement, &original, Some(&backup)).unwrap();

        assert_eq!(read_text(&original), "reduced");
        assert_eq!(read_text(&backup), "original-secret", "the backup gets the pre-replacement content");
    }

    #[cfg(windows)]
    #[test]
    fn create_file_owner_only_grants_no_broad_access() {
        let path = temp_path("owner-only.txt");
        let mut file = create_file_owner_only(&path).unwrap();
        use std::io::Write;
        file.write_all(b"sensitive").unwrap();
        drop(file);
        assert_eq!(read_text(&path), "sensitive");

        // Verified empirically (icacls) that this SDDL grants Full Control
        // to only Owner Rights, SYSTEM, and Administrators -- none of the
        // broad, commonly-inherited principals below.
        let out = std::process::Command::new("icacls").arg(&path).output().unwrap();
        let listing = String::from_utf8_lossy(&out.stdout);
        for broad in ["Everyone", "Authenticated Users", "BUILTIN\\Users", "\\Users:"] {
            assert!(
                !listing.contains(broad),
                "expected no broad-access principal {broad:?} in icacls output: {listing}"
            );
        }
        assert!(listing.contains("OWNER RIGHTS"), "expected an explicit owner grant in icacls output: {listing}");
    }

    #[cfg(windows)]
    #[test]
    fn create_file_owner_only_refuses_to_reuse_an_existing_file() {
        let path = temp_path("owner-only-existing.txt");
        fs::write(&path, "pre-existing").unwrap();
        let result = create_file_owner_only(&path);
        assert!(result.is_err(), "must not silently reuse a file that already exists at this path");
        assert_eq!(read_text(&path), "pre-existing", "the pre-existing file must be left untouched");
    }

    #[cfg(windows)]
    #[test]
    fn roll_back_committed_replacement_restores_content_from_the_backup() {
        // Callers rely on Err from replace_file_preserving_security meaning
        // nothing changed; when ReplaceFileW itself already succeeded and
        // only a later metadata step failed, this is what makes that
        // guarantee hold anyway by restoring destination from the backup
        // that was already created.
        let destination = temp_path("destination.json");
        let backup = temp_path("destination.bak");
        fs::write(&destination, "committed-replacement-content").unwrap();
        fs::write(&backup, "original-content").unwrap();

        let message = roll_back_committed_replacement(&destination, Some(&backup), "metadata step failed".into());

        assert_eq!(read_text(&destination), "original-content", "destination must be restored from the backup");
        assert!(message.contains("rolled back"), "the error message must say a rollback happened: {message}");
    }

    #[cfg(windows)]
    #[test]
    fn roll_back_committed_replacement_explains_when_no_backup_was_available() {
        let destination = temp_path("destination-no-backup.json");
        fs::write(&destination, "committed-replacement-content").unwrap();

        let message = roll_back_committed_replacement(&destination, None, "metadata step failed".into());

        assert_eq!(
            read_text(&destination),
            "committed-replacement-content",
            "with nothing to roll back from, destination is left as the swap left it"
        );
        assert!(message.contains("no backup was requested"), "the error message must explain rollback wasn't possible: {message}");
    }

    #[cfg(unix)]
    #[test]
    fn hook_replacement_preserves_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let original = temp_path("settings.json");
        let replacement = temp_path("settings.tmp");
        fs::write(&original, "{}").unwrap();
        let mut permissions = fs::metadata(&original).unwrap().permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&original, permissions).unwrap();
        fs::write(&replacement, "{}").unwrap();

        copy_file_permissions(&original, &replacement).unwrap();

        let mode = fs::metadata(&replacement).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn unix_replacement_backup_carries_content_and_mode() {
        use std::os::unix::fs::PermissionsExt;

        let original = temp_path("settings.json");
        let replacement = temp_path("settings.tmp");
        let backup = temp_path("settings.bak");
        fs::write(&original, "original-secret").unwrap();
        fs::set_permissions(&original, fs::Permissions::from_mode(0o600)).unwrap();
        fs::write(&replacement, "reduced").unwrap();

        replace_file_preserving_security(&replacement, &original, Some(&backup)).unwrap();

        assert_eq!(read_text(&original), "reduced");
        assert_eq!(read_text(&backup), "original-secret");
        assert_eq!(
            fs::metadata(&backup).unwrap().permissions().mode() & 0o777,
            0o600,
            "the backup must carry the original's mode, not the process default"
        );
    }

    #[test]
    fn hook_merge_is_idempotent_and_keeps_foreign_hooks() {
        let example = json_parse(
            r#"{"hooks":{"Stop":[{"matcher":"","hooks":[{"type":"command","command":"./bin/sentrith guard"}]}]}}"#,
        )
        .unwrap();
        let mut settings = json_parse(
            r#"{"hooks":{"Stop":[{"matcher":"","hooks":[{"type":"command","command":"my-linter"}]}]}}"#,
        )
        .unwrap();

        let mut first = String::new();
        for pass in 0..3 {
            let mut hooks = settings.get("hooks").cloned().unwrap();
            strip_sentrith_hooks(&mut hooks);
            let added = merge_sentrith_hooks(&mut hooks, example.get("hooks").unwrap());
            assert_eq!(added, 1, "pass {pass}");
            settings.set("hooks", hooks);
            let rendered = json_to_string(&settings);
            if pass == 0 {
                first = rendered;
            } else {
                assert_eq!(rendered, first, "install must be idempotent");
            }
        }
        assert_eq!(first.matches("my-linter").count(), 1);
        assert_eq!(first.matches("sentrith guard").count(), 1);
    }

    #[test]
    fn hook_status_requires_an_owned_command() {
        let foreign = json_parse(
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"/workspace/sentrith/scripts/run-linter"}]}]}}"#,
        )
        .unwrap();
        let owned = json_parse(
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"}]}]}}"#,
        )
        .unwrap();

        assert_eq!(count_sentrith_hooks(foreign.get("hooks").unwrap()), 0);
        assert_eq!(count_sentrith_hooks(owned.get("hooks").unwrap()), 1);
        assert_eq!(
            sentrith_hook_count(
                r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"echo sentrith"}]}]}}"#
            ),
            0
        );
    }

    #[test]
    fn hook_status_requires_a_capture_command() {
        let guard_only = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"}]}]}}"#;
        let claude_hook = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith usage hook claude --phase standard"}]}]}}"#;
        let codex_hook = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith usage hook codex"}]}]}}"#;

        assert_eq!(sentrith_usage_hook_count(guard_only, "claude"), 0);
        assert_eq!(sentrith_usage_hook_count(claude_hook, "claude"), 1);
        assert_eq!(sentrith_usage_hook_count(claude_hook, "codex"), 0);
        assert_eq!(sentrith_usage_hook_count(codex_hook, "codex"), 1);
    }

    #[test]
    fn usage_status_readiness_scopes_requested_agent() {
        assert!(hook_target_matches_agent("codex", None));
        assert!(hook_target_matches_agent("codex", Some("codex")));
        assert!(!hook_target_matches_agent("claude", Some("codex")));
    }

    #[test]
    fn hook_matching_does_not_remove_foreign_paths_containing_sentrith() {
        assert!(is_sentrith_command("./bin/sentrith guard"));
        assert!(is_sentrith_command("bin\\sentrith.exe guard"));
        assert!(is_sentrith_command(r#""C:\Program Files\project\bin\sentrith.exe" guard"#));
        assert!(!is_sentrith_command("/workspace/sentrith/scripts/run-linter"));
        assert!(!is_sentrith_command("echo sentrith"));

        let mut hooks = json_parse(
            r#"{"Stop":[{"matcher":"","hooks":[{"type":"command","command":"/workspace/sentrith/scripts/run-linter"},{"type":"command","command":"./bin/sentrith guard"}]}]}"#,
        )
        .unwrap();
        strip_sentrith_hooks(&mut hooks);
        let rendered = json_to_string(&hooks);
        assert!(rendered.contains("/workspace/sentrith/scripts/run-linter"));
        assert!(!rendered.contains("./bin/sentrith guard"));
    }

    #[test]
    fn stripping_removes_empty_groups_and_events() {
        let mut hooks = json_parse(
            r#"{"Stop":[{"matcher":"","hooks":[{"type":"command","command":"./bin/sentrith guard"}]}],"Other":[{"matcher":"","hooks":[{"type":"command","command":"keep-me"}]}]}"#,
        )
        .unwrap();
        strip_sentrith_hooks(&mut hooks);
        assert!(hooks.get("Stop").is_none(), "event with only Sentrith hooks is removed");
        assert!(hooks.get("Other").is_some(), "foreign event is kept");
    }

    #[test]
    fn workflow_check_detection_distinguishes_advisory_checks_from_capture() {
        assert!(is_workflow_check_command("./bin/sentrith preflight"));
        assert!(is_workflow_check_command("./bin/sentrith closeout-check"));
        assert!(is_workflow_check_command("bin\\sentrith.exe guard"));
        assert!(is_workflow_check_command("./bin/sentrith review-hint"));
        assert!(is_workflow_check_command("./bin/sentrith diff-budget"));
        // Usage capture must keep running during a baseline: it is what
        // records the baseline turns at all.
        assert!(!is_workflow_check_command("./bin/sentrith usage hook claude"));
        assert!(!is_workflow_check_command("./bin/sentrith usage hook codex"));
        assert!(!is_workflow_check_command("./bin/sentrith usage claude-status"));
        // A foreign command that happens to contain one of the words must not
        // match; the binary itself must be Sentrith's.
        assert!(!is_workflow_check_command("my-own-guard-script"));
        assert!(!is_workflow_check_command("echo preflight"));
    }

    #[test]
    fn baseline_hook_reduction_strips_advisory_checks_and_keeps_capture() {
        let stash = temp_path("stash-reduce");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        fs::write(&live, r#"{
  "hooks": {
    "SessionStart": [{"matcher":"","hooks":[{"type":"command","command":"./bin/sentrith preflight"}]}],
    "Stop": [
      {"matcher":"","hooks":[
        {"type":"command","command":"./bin/sentrith closeout-check"},
        {"type":"command","command":"./bin/sentrith guard"},
        {"type":"command","command":"./bin/sentrith review-hint"},
        {"type":"command","command":"./bin/sentrith diff-budget"}
      ]},
      {"matcher":"","hooks":[{"type":"command","command":"./bin/sentrith usage hook claude","timeout":5}]}
    ],
    "UserPromptSubmit": [{"matcher":"","hooks":[{"type":"command","command":"./bin/sentrith usage hook claude","timeout":5}]}]
  },
  "statusLine": {"type":"command","command":"./bin/sentrith usage claude-status","padding":1}
}"#).unwrap();

        let changed = reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap();
        assert!(changed);

        let reduced = read_text(&live);
        let parsed = json_parse(&reduced).unwrap();
        assert!(parsed.get("hooks").unwrap().get("SessionStart").is_none(), "preflight's only event is dropped entirely");
        assert!(reduced.contains("usage hook claude"), "capture hooks must survive");
        assert!(!reduced.contains("closeout-check"));
        assert!(!reduced.contains("\"guard\""));
        assert!(reduced.contains("statusLine"), "statusLine is a terminal display, not agent context; left alone");

        let backup = stash.join("hook-settings-backup").join(".claude/settings.json");
        assert!(backup.exists());
        let original = read_text(&backup);
        assert!(original.contains("preflight"), "the original is preserved verbatim for restore");
    }

    #[test]
    fn baseline_hook_reduction_round_trips_back_to_the_original() {
        let stash = temp_path("stash-roundtrip");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("hooks.json");
        let original = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"},{"type":"command","command":"./bin/sentrith usage hook codex"}]}]}}"#;
        fs::write(&live, original).unwrap();

        assert!(reduce_hook_settings_for_baseline(".codex/hooks.json", &live, &stash).unwrap());
        assert_ne!(read_text(&live), original, "the live file is reduced while active");

        assert!(
            restore_hook_settings_backup(".codex/hooks.json", &live, &stash).unwrap(),
            "an actual restore reports true, distinguishing it from a no-op"
        );
        assert_eq!(read_text(&live), original, "stop restores the exact original, not a reconstruction");
        assert!(
            !stash.join("hook-settings-backup").join(".codex/hooks.json").exists(),
            "the backup is consumed on restore"
        );
    }

    #[test]
    fn restore_hook_settings_backup_retries_cleanup_without_a_false_conflict() {
        // Simulates an earlier restore attempt whose live replacement
        // already succeeded, but whose cleanup (removing the backup or
        // digest) failed and left them behind -- e.g. a transient sharing
        // violation on Windows. A retry must recognize that `live` already
        // holds the restored original and finish cleanup, not compare it
        // against the reduced-content digest and misreport a conflict.
        let stash = temp_path("stash-retry-cleanup");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        let original = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"},{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#;
        fs::write(&live, original).unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());
        // The earlier attempt's live replacement already committed.
        fs::write(&live, original).unwrap();

        let result = restore_hook_settings_backup(".claude/settings.json", &live, &stash);
        assert!(result.is_ok(), "a retry after live was already restored must not report a conflict: {result:?}");
        assert_eq!(read_text(&live), original, "the already-restored content must be left as is");
        assert!(
            !stash.join("hook-settings-backup").join(".claude/settings.json").exists(),
            "the leftover backup must still be cleaned up"
        );
        assert!(
            !stash.parent().unwrap().join("baseline-hook-conflicts").exists(),
            "no conflict must be filed for a file that was already correctly restored"
        );
    }

    #[test]
    fn restore_hook_settings_backup_clears_an_orphaned_digest_with_no_backup() {
        // Simulates an even later interruption than the retry-cleanup test
        // above: the backup itself was already removed, but the digest
        // removal that follows it either failed or never ran. `!backup.exists()`
        // used to return early without touching the digest at all, leaving it
        // to keep `hook-settings-backup` non-empty forever -- which later
        // fails the stash directory's removal on every subsequent
        // `baseline stop` and strands the phase marker at `baseline`.
        let stash = temp_path("stash-orphaned-digest");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        let original = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"},{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#;
        fs::write(&live, original).unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());
        // Simulate a completed restore whose backup was removed but whose
        // digest removal did not happen.
        fs::write(&live, original).unwrap();
        let backup = stash.join("hook-settings-backup").join(".claude/settings.json");
        let digest = stash.join("hook-settings-backup").join(".claude/settings.json.reduced-digest");
        fs::remove_file(&backup).unwrap();
        assert!(digest.exists(), "the digest must still be there for this scenario to be meaningful");

        assert!(
            !restore_hook_settings_backup(".claude/settings.json", &live, &stash).unwrap(),
            "there is no backup, so this must report a no-op, not an actual restore"
        );
        assert!(!digest.exists(), "the orphaned digest must be cleaned up even with no backup present");
        assert_eq!(read_text(&live), original, "live must be left untouched");
    }

    #[test]
    fn restore_fails_closed_when_the_digest_is_unreadable() {
        // A missing, unreadable, or malformed digest while the backup still
        // exists is not the known-safe retry state (that's `live` already
        // equaling `backup`, handled separately above) -- it's genuinely
        // unexplained, and treating it as "not diverged" would silently
        // overwrite whatever edit `live` may hold with the stale backup.
        let stash = temp_path("stash-unreadable-digest");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"},{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#).unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());
        // A genuine edit made during baseline, distinct from both the
        // original and the reduced content.
        let edited = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith usage hook claude"}]}],"custom":true}}"#;
        fs::write(&live, edited).unwrap();
        // Corrupt the digest so it can't be parsed.
        let digest = stash.join("hook-settings-backup").join(".claude/settings.json.reduced-digest");
        fs::write(&digest, "not-a-number").unwrap();

        let result = restore_hook_settings_backup(".claude/settings.json", &live, &stash);
        assert!(result.is_err(), "an unreadable digest must be treated as diverged, not as safe to overwrite");
        assert_eq!(read_text(&live), edited, "the edit must survive when the digest can't be verified");
    }

    #[test]
    fn restore_hook_settings_backup_is_a_noop_when_the_path_was_never_reduced() {
        // `baseline_stop` now scans every candidate path unconditionally
        // (rather than trusting a manifest of what was actually reduced), so
        // restoring a path with no backup at all must stay a safe, silent
        // no-op rather than an error.
        let stash = temp_path("stash-restore-noop");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        let original = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#;
        fs::write(&live, original).unwrap();

        assert!(
            !restore_hook_settings_backup(".claude/settings.json", &live, &stash).unwrap(),
            "no backup exists for this path, so restore must report false, not error"
        );
        assert_eq!(read_text(&live), original, "a path that was never reduced must be left untouched");
    }

    #[test]
    fn baseline_hook_reduction_is_a_noop_when_nothing_matches() {
        let stash = temp_path("stash-noop");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("no-workflow-checks.json");
        let original = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#;
        fs::write(&live, original).unwrap();

        assert!(!reduce_hook_settings_for_baseline("x", &live, &stash).unwrap());
        assert_eq!(read_text(&live), original, "nothing to strip means the file is left untouched");

        let missing = temp_path("does-not-exist.json");
        assert!(!reduce_hook_settings_for_baseline("y", &missing, &stash).unwrap());

        let empty = temp_path("empty.json");
        fs::write(&empty, "").unwrap();
        assert!(!reduce_hook_settings_for_baseline("z", &empty, &stash).unwrap());
    }

    #[test]
    fn baseline_hook_reduction_reports_malformed_json_without_writing() {
        let stash = temp_path("stash-malformed");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("broken.json");
        fs::write(&live, "{not valid json").unwrap();

        let result = reduce_hook_settings_for_baseline("w", &live, &stash);
        assert!(result.is_err());
        assert_eq!(read_text(&live), "{not valid json", "a malformed file is left exactly as found");
    }

    #[test]
    fn baseline_hook_reduction_aborts_rather_than_skip_an_unreadable_settings_file() {
        // `read_text` folds a read failure (invalid UTF-8 here) into the same
        // empty string as a genuinely empty file, which used to make this
        // return `Ok(false)` -- "nothing to reduce" -- for a file that still
        // holds live Sentrith hooks. baseline_start would then report success
        // while this path's hooks stayed active for the whole baseline,
        // contaminating the sample. It must now return Err so the caller
        // aborts and rolls back instead.
        let stash = temp_path("stash-unreadable");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("invalid-utf8.json");
        fs::write(&live, [0x7b, 0xff, 0xfe, 0x7d]).unwrap();

        let result = reduce_hook_settings_for_baseline("w", &live, &stash);
        assert!(result.is_err(), "an unreadable settings file must abort, not silently report nothing-to-reduce");
    }

    #[cfg(unix)]
    #[test]
    fn baseline_hook_reduction_preserves_restrictive_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let stash = temp_path("stash-perms");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"}]}]}}"#).unwrap();
        fs::set_permissions(&live, fs::Permissions::from_mode(0o600)).unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());
        assert_eq!(
            fs::metadata(&live).unwrap().permissions().mode() & 0o777,
            0o600,
            "reducing the file must not loosen its mode"
        );

        // The backup holds the complete, unreduced original for as long as
        // the baseline runs; a default-mode copy would expose whatever the
        // original's permissions were restricting for that whole window.
        let backup = stash.join("hook-settings-backup").join(".claude/settings.json");
        assert_eq!(
            fs::metadata(&backup).unwrap().permissions().mode() & 0o777,
            0o600,
            "the backup must carry the original's restrictive mode, not the process default"
        );

        restore_hook_settings_backup(".claude/settings.json", &live, &stash).unwrap();
        assert_eq!(
            fs::metadata(&live).unwrap().permissions().mode() & 0o777,
            0o600,
            "restoring via rename must not adopt the backup's create-mode instead of the original's"
        );
    }

    #[cfg(unix)]
    #[test]
    fn reduce_widens_the_temp_file_to_a_less_restrictive_original_mode() {
        use std::os::unix::fs::PermissionsExt;

        // The temp file is now created at a tight 0600 up front (rather than
        // created loose and chmod'd after) to close a permission-exposure
        // window; this confirms that when `live` is actually less
        // restrictive than 0600, copy_file_permissions afterward still
        // widens the reduced file to match it instead of leaving it stuck
        // at the tighter creation default.
        let stash = temp_path("stash-widen-perms");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"}]}]}}"#).unwrap();
        fs::set_permissions(&live, fs::Permissions::from_mode(0o644)).unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());
        assert_eq!(
            fs::metadata(&live).unwrap().permissions().mode() & 0o777,
            0o644,
            "the reduced file must end up matching the original's actual mode"
        );
    }

    #[cfg(unix)]
    #[test]
    fn baseline_hook_reduction_refuses_a_symlinked_settings_file() {
        let stash = temp_path("stash-symlink");
        fs::create_dir_all(&stash).unwrap();
        let target = temp_path("settings-target.json");
        fs::write(&target, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"}]}]}}"#).unwrap();
        let live = temp_path("settings-link.json");
        std::os::unix::fs::symlink(&target, &live).unwrap();

        let result = reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash);
        assert!(result.is_err(), "must refuse a symlinked settings file rather than following it");
        assert_eq!(
            read_text(&target),
            r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"}]}]}}"#,
            "the symlink target must be left untouched"
        );
        assert!(live.is_symlink(), "the symlink itself must not be replaced");
    }

    #[cfg(unix)]
    #[test]
    fn reduce_does_not_write_through_a_stale_symlink_at_the_temp_path() {
        // A stale symlink left at the predictable temp path (an interrupted
        // older run, or placed there deliberately in a shared writable
        // repository) must never be followed: writing the reduced settings
        // through it would leak them to wherever the symlink points, not
        // the intended scratch location.
        let stash = temp_path("stash-stale-tmp-symlink");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"},{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#).unwrap();

        let elsewhere = temp_path("elsewhere.txt");
        fs::write(&elsewhere, "untouched").unwrap();
        let tmp = live.with_extension("sentrith-baseline-tmp");
        std::os::unix::fs::symlink(&elsewhere, &tmp).unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());
        assert!(!live.is_symlink(), "live must end up as a regular reduced file, not a symlink");
        assert!(!read_text(&live).contains("\"guard\""), "live must actually be reduced");
        assert_eq!(
            read_text(&elsewhere),
            "untouched",
            "the stale symlink's target must never receive the reduced content"
        );
    }

    #[cfg(unix)]
    #[test]
    fn write_secure_temp_file_creates_at_restrictive_mode_and_refuses_reuse() {
        use std::os::unix::fs::PermissionsExt;

        // The function `hooks_install` now shares with baseline reduction:
        // exercised directly here (rather than only indirectly, through
        // reduce's own tests) since it's the exact entry point hooks_install
        // depends on for the same secure-temp-file guarantee.
        let tmp = temp_path("secure-temp.json");
        write_secure_temp_file(&tmp, "content").unwrap();
        assert_eq!(read_text(&tmp), "content");
        assert_eq!(
            fs::metadata(&tmp).unwrap().permissions().mode() & 0o777,
            0o600,
            "must be created at a restrictive mode regardless of which caller uses it"
        );

        let elsewhere = temp_path("elsewhere.json");
        fs::write(&elsewhere, "untouched").unwrap();
        fs::remove_file(&tmp).unwrap();
        std::os::unix::fs::symlink(&elsewhere, &tmp).unwrap();

        write_secure_temp_file(&tmp, "new-content").unwrap();
        assert!(!tmp.is_symlink(), "a stale symlink at the temp path must be replaced, not followed");
        assert_eq!(read_text(&tmp), "new-content");
        assert_eq!(read_text(&elsewhere), "untouched", "the stale symlink's target must never receive the content");
    }

    #[cfg(unix)]
    #[test]
    fn backup_creation_does_not_follow_a_symlink_at_the_destination() {
        // Reproduces the exact vulnerability: a `*.json.sentrith-bak` symlink
        // pointing at an unrelated file must not have its target silently
        // overwritten with the backup content -- exercised at the same
        // create_secure_file + io::copy shape hooks_install now uses, since
        // hooks_install itself is cwd-coupled and not unit-testable directly.
        let source = temp_path("source.json");
        fs::write(&source, "settings-content").unwrap();

        let victim = temp_path("victim.txt");
        fs::write(&victim, "victim-must-not-be-touched").unwrap();
        let backup = temp_path("settings.json.sentrith-bak");
        std::os::unix::fs::symlink(&victim, &backup).unwrap();

        let mut dest = create_secure_file(&backup).unwrap();
        let mut src = fs::File::open(&source).unwrap();
        std::io::copy(&mut src, &mut dest).unwrap();
        drop(dest);

        assert_eq!(
            read_text(&victim),
            "victim-must-not-be-touched",
            "the symlink target must never receive the backup content"
        );
        assert!(!backup.is_symlink(), "the backup path must end up as a regular file, not the stale symlink");
        assert_eq!(read_text(&backup), "settings-content");
    }

    #[cfg(unix)]
    #[test]
    fn restore_does_not_follow_a_symlink_at_the_restore_temp_path() {
        // Reproduces the reported scenario directly against
        // restore_hook_settings_backup itself, matching how it was found:
        // link the restore temp path to an unrelated file after a baseline
        // reduction, then run restore. Before the fix, fs::copy followed the
        // symlink (clobbering the victim) and the subsequent rename moved
        // that same symlink onto `live`.
        let stash = temp_path("stash-restore-tmp-symlink");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        let original = r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"},{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#;
        fs::write(&live, original).unwrap();
        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());

        let victim = temp_path("victim.txt");
        fs::write(&victim, "victim-must-not-be-touched").unwrap();
        let restore_tmp = live.with_extension("sentrith-baseline-restore-tmp");
        std::os::unix::fs::symlink(&victim, &restore_tmp).unwrap();

        assert!(restore_hook_settings_backup(".claude/settings.json", &live, &stash).unwrap());

        assert_eq!(read_text(&victim), "victim-must-not-be-touched", "the symlink target must never receive the settings content");
        assert!(!live.is_symlink(), "live must end up as a regular file, not the stale symlink renamed onto it");
        assert_eq!(read_text(&live), original, "the restore must still succeed correctly despite the stale symlink");
    }

    #[test]
    fn baseline_hook_reduction_snapshot_does_not_store_full_settings_content() {
        let stash = temp_path("stash-digest");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"},{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#).unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());

        let digest_path = stash
            .join("hook-settings-backup")
            .join(".claude/settings.json.reduced-digest");
        let digest_text = read_text(&digest_path);
        assert!(
            digest_text.trim().parse::<u64>().is_ok(),
            "the on-disk snapshot must be a digest, not settings content: got {digest_text:?}"
        );
        assert!(
            !digest_text.contains("sentrith") && !digest_text.contains("hooks"),
            "the digest must not leak the reduced settings content"
        );
    }

    #[test]
    fn baseline_stop_refuses_to_discard_an_edit_made_during_baseline() {
        let stash = temp_path("stash-conflict");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"},{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#).unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());

        // Simulate a legitimate edit made while baseline was active.
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith usage hook claude"}]}],"custom":true}}"#).unwrap();
        let edited = read_text(&live);

        let result = restore_hook_settings_backup(".claude/settings.json", &live, &stash);
        assert!(result.is_err(), "must refuse rather than silently discard the edit");
        assert_eq!(read_text(&live), edited, "the edit made during baseline must survive");

        // The conflict must not block the stash from being cleaned up: its
        // backup is moved out entirely, not left sitting in the stash.
        let backup_dir = stash.join("hook-settings-backup").join(".claude");
        assert!(
            !backup_dir.exists() || fs::read_dir(&backup_dir).unwrap().next().is_none(),
            "the backup and its snapshot must be moved out of the stash on conflict"
        );
        let conflict = stash
            .parent()
            .unwrap()
            .join("baseline-hook-conflicts")
            .join(".claude/settings.json");
        assert!(conflict.exists(), "the pre-baseline original must be preserved at the conflict path");
    }

    #[test]
    fn baseline_hook_conflict_never_overwrites_an_earlier_unresolved_conflict() {
        let stash = temp_path("stash-conflict-unique");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"},{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#).unwrap();

        // An earlier baseline already left an unresolved conflict at the
        // fixed path this rel_path maps to; nothing blocks starting a new
        // baseline while it sits there unmerged.
        let conflict_root = stash.parent().unwrap();
        let earlier_conflict = conflict_root.join("baseline-hook-conflicts").join(".claude/settings.json");
        fs::create_dir_all(earlier_conflict.parent().unwrap()).unwrap();
        fs::write(&earlier_conflict, "EARLIER-UNRESOLVED-ORIGINAL").unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith usage hook claude"}]}],"custom":true}}"#).unwrap();

        let result = restore_hook_settings_backup(".claude/settings.json", &live, &stash);
        assert!(result.is_err(), "must refuse rather than silently discard the edit");

        assert_eq!(
            read_text(&earlier_conflict),
            "EARLIER-UNRESOLVED-ORIGINAL",
            "an earlier unresolved conflict must never be silently replaced"
        );
        let new_conflict = conflict_root.join("baseline-hook-conflicts").join(".claude/settings.json.conflict-1");
        assert!(new_conflict.exists(), "the new conflict must be preserved at a distinct path");
    }

    #[cfg(unix)]
    #[test]
    fn conflict_preservation_failure_keeps_the_digest_for_a_retry() {
        use std::os::unix::fs::PermissionsExt;
        extern "C" {
            fn geteuid() -> u32;
        }

        // Root bypasses Unix access checks entirely, so a read-only
        // directory does not actually block root from renaming into it --
        // the permission-based failure this test simulates cannot happen
        // under root, and asserting on it would fail for a reason unrelated
        // to the behavior under test. Skip rather than assert a premise that
        // does not hold for this user.
        if unsafe { geteuid() } == 0 {
            eprintln!("skipping conflict_preservation_failure_keeps_the_digest_for_a_retry: running as root, where the read-only-directory simulation cannot fail");
            return;
        }

        // If moving the diverged backup out to baseline-hook-conflicts fails
        // (simulated here with a read-only conflict root, standing in for a
        // sharing violation or an unwritable directory), the digest must
        // survive: without it, a retry's divergence check reads a missing
        // digest as "not diverged" and silently overwrites the edit with the
        // stale backup -- exactly the loss this whole mechanism exists to
        // prevent.
        let stash = temp_path("stash-conflict-preserve-fail");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"},{"type":"command","command":"./bin/sentrith usage hook claude"}]}]}}"#).unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith usage hook claude"}]}],"custom":true}}"#).unwrap();

        let conflict_root = stash.parent().unwrap();
        fs::set_permissions(conflict_root, fs::Permissions::from_mode(0o500)).unwrap();
        let result = restore_hook_settings_backup(".claude/settings.json", &live, &stash);
        fs::set_permissions(conflict_root, fs::Permissions::from_mode(0o700)).unwrap();

        assert!(result.is_err(), "must still refuse to overwrite the edit even when preservation fails");
        let digest = stash.join("hook-settings-backup").join(".claude/settings.json.reduced-digest");
        assert!(digest.exists(), "the digest must survive a failed preservation attempt so a retry can re-detect the conflict");
        let backup = stash.join("hook-settings-backup").join(".claude/settings.json");
        assert!(backup.exists(), "the backup must remain at its known path when preservation fails");
    }

    #[cfg(windows)]
    #[test]
    fn restore_removes_a_read_only_backup_file() {
        // Simulates a backup that ended up read-only for any reason (an
        // inherited directory ACL, a manual edit, or a future change to how
        // it's created): empirically, a backup created from a read-only
        // original via the current reduce path does NOT end up read-only
        // (replace_file_preserving_security clears the destination's
        // read-only attribute before ReplaceFileW runs, so ReplaceFileW's
        // own backup is made from the already-cleared file) -- but restore
        // must not depend on that happening to be true today.
        let stash = temp_path("stash-readonly-backup");
        fs::create_dir_all(&stash).unwrap();
        let live = temp_path("settings.json");
        fs::write(&live, r#"{"hooks":{"Stop":[{"hooks":[{"type":"command","command":"./bin/sentrith guard"}]}]}}"#).unwrap();

        assert!(reduce_hook_settings_for_baseline(".claude/settings.json", &live, &stash).unwrap());

        let backup = stash.join("hook-settings-backup").join(".claude/settings.json");
        let mut perms = fs::metadata(&backup).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&backup, perms).unwrap();

        assert!(restore_hook_settings_backup(".claude/settings.json", &live, &stash).unwrap());
        assert!(!backup.exists(), "a read-only backup must still be removed after a successful restore, not left behind");
    }

    fn row(session: &str, sha: &str, success: &str, phase: &str) -> BTreeMap<String, String> {
        let mut m = BTreeMap::new();
        m.insert("session_id".into(), session.into());
        m.insert("head_sha".into(), sha.into());
        m.insert("success".into(), success.into());
        m.insert("phase".into(), phase.into());
        m
    }

    #[test]
    fn first_commit_after_unborn_head_is_counted() {
        assert!(commit_reached(UNBORN_HEAD, "abc123"));
        assert!(!commit_reached("", "abc123"));
        assert!(!commit_reached("abc123", "abc123"));
    }

    #[test]
    fn tasks_split_on_head_sha_transitions() {
        let owned = vec![
            row("s1", "", "unknown", "standard"),
            row("s1", "", "unknown", "standard"),
            row("s1", "bbb", "yes", "standard"),
            row("s1", "", "unknown", "standard"),
        ];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();
        let tasks = group_tasks(&rows);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].len(), 3, "commit-closing turn ends the first task");
        assert_eq!(tasks[1].len(), 1);
    }

    #[test]
    fn success_rate_excludes_unknown_from_denominator() {
        let owned = vec![
            row("", "", "yes", "standard"),
            row("", "", "no", "standard"),
            row("", "", "unknown", "standard"),
            row("", "", "", "standard"),
        ];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();
        assert_eq!(decided_success_rate(&group_tasks(&rows)), Some(50.0));

        let undecided: Vec<&BTreeMap<String, String>> = owned[2..].iter().collect();
        assert_eq!(decided_success_rate(&group_tasks(&undecided)), None);
    }

    fn turn(session: &str, sha: &str, success: &str, input: &str) -> BTreeMap<String, String> {
        let mut m = row(session, sha, success, "standard");
        m.insert("input_tokens".into(), input.into());
        m.insert("credits".into(), "3".into());
        m
    }

    #[test]
    fn model_filter_groups_before_filtering() {
        let mut first = turn("s1", "", "unknown", "10");
        first.insert("model".into(), "model-a".into());
        let mut second = turn("s1", "sha", "yes", "20");
        second.insert("model".into(), "model-b".into());
        let owned = vec![first, second];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();

        let tasks = group_tasks(&rows);
        assert_eq!(tasks.len(), 1, "the model change is still one task");
        assert!(
            filter_tasks_by_model(tasks, Some("model-a")).is_empty(),
            "mixed-model tasks must not be attributed to either model"
        );
    }

    #[test]
    fn published_stats_are_per_task_not_per_turn() {
        // Three turns of one session, closed by a commit: one task worth
        // 30 input tokens, not three tasks of 10.
        let owned = vec![
            turn("s1", "", "unknown", "10"),
            turn("s1", "", "unknown", "10"),
            turn("s1", "bbb", "yes", "10"),
        ];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();
        let stats = publish_stats(&rows);

        assert_eq!(stats.tasks, 1, "turns before the commit are one task");
        assert_eq!(stats.successes, 1);
        assert_eq!(stats.input_avg, Some(30.0), "input is summed across the task");
        assert_eq!(stats.success_rate, Some(100.0));
        assert_eq!(stats.credits_per_success, Some(9.0), "total credits / successful task");
    }

    #[test]
    fn manual_single_row_records_are_unchanged_by_task_grouping() {
        // Manual records carry no session id, so each stays its own task and
        // averages keep their previous meaning.
        let owned = vec![
            turn("", "", "yes", "10"),
            turn("", "", "no", "20"),
        ];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();
        let stats = publish_stats(&rows);

        assert_eq!(stats.tasks, 2);
        assert_eq!(stats.input_avg, Some(15.0));
        assert_eq!(stats.success_rate, Some(50.0));
    }

    #[test]
    fn first_captured_turn_can_close_its_own_task() {
        // A session whose very first captured turn commits has no previous SHA
        // to transition from. Without the decided-outcome rule its task would
        // merge into the next one and its `yes` would be overwritten.
        let owned = vec![
            row("s1", "bbb", "yes", "standard"),
            row("s1", "ccc", "no", "standard"),
        ];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();
        let tasks = group_tasks(&rows);

        assert_eq!(tasks.len(), 2, "each committed turn closes its own task");
        assert_eq!(task_success(&tasks[0]), "yes");
        assert_eq!(task_success(&tasks[1]), "no");
        assert_eq!(decided_success_rate(&tasks), Some(50.0));
    }

    #[test]
    fn undecided_turns_stay_with_the_task_they_precede() {
        let owned = vec![
            row("s1", "", "unknown", "standard"),
            row("s1", "", "unknown", "standard"),
            row("s1", "bbb", "yes", "standard"),
            row("s1", "", "unknown", "standard"),
        ];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();
        let tasks = group_tasks(&rows);

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].len(), 3);
        assert_eq!(task_success(&tasks[0]), "yes");
        assert_eq!(task_success(&tasks[1]), "unknown");
    }

    #[test]
    fn a_commit_closes_its_task_even_without_a_test_run() {
        // A first turn that commits without running tests records
        // `success=unknown`, so the outcome carries no signal. The recorded
        // commit must still end the task, or its usage merges into the next one.
        let owned = vec![
            row("s1", "aaa", "unknown", "standard"),
            row("s1", "", "unknown", "standard"),
            row("s1", "bbb", "yes", "standard"),
        ];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();
        let tasks = group_tasks(&rows);

        assert_eq!(tasks.len(), 2, "the unverified first commit is its own task");
        assert_eq!(tasks[0].len(), 1);
        assert_eq!(task_success(&tasks[0]), "unknown");
        assert_eq!(tasks[1].len(), 2);
        assert_eq!(task_success(&tasks[1]), "yes");
    }

    #[test]
    fn churn_window_excludes_commits_before_the_measured_commit() {
        // `sha..HEAD` selects by ancestry, not time: a side-branch commit
        // merged in later can still carry a timestamp earlier than `t0`. Only
        // the upper bound would have wrongly counted it as later rework.
        let t0 = 1_000_000.0;
        let days = 14.0;

        assert!(
            !within_churn_window(t0 - 3600.0, t0, days),
            "a commit authored before t0 must not count as rework of it"
        );
        assert!(within_churn_window(t0, t0, days), "t0 itself is in-window");
        assert!(within_churn_window(t0 + 3600.0, t0, days));
        assert!(within_churn_window(t0 + days * 86400.0, t0, days), "the upper bound is inclusive");
        assert!(!within_churn_window(t0 + days * 86400.0 + 1.0, t0, days));
    }

    #[test]
    fn every_recorded_commit_is_eligible_for_churn() {
        // A session with exactly one commit must not be skipped: each recorded
        // head_sha is by construction a commit observed during that turn.
        let owned = vec![
            row("s1", "aaa", "unknown", "standard"),
            row("s2", "bbb", "yes", "standard"),
            row("s2", "bbb", "unknown", "standard"),
        ];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();
        let shas: Vec<String> = {
            let mut seen = BTreeSet::new();
            rows.iter()
                .filter_map(|r| r.get("head_sha"))
                .filter(|s| !s.is_empty())
                .filter(|s| seen.insert((*s).clone()))
                .cloned()
                .collect()
        };
        assert_eq!(shas, vec!["aaa".to_string(), "bbb".to_string()]);
    }

    #[test]
    fn usage_per_success_counts_tasks_not_successful_turns() {
        // Two working turns and a third that commits: capture only resolves a
        // `yes` on the turn where HEAD moved, so this is one successful task.
        let owned = vec![
            turn("s1", "", "unknown", "10"),
            turn("s1", "", "unknown", "10"),
            turn("s1", "bbb", "yes", "10"),
        ];
        let rows: Vec<&BTreeMap<String, String>> = owned.iter().collect();
        // 9 credits total over 1 successful task, not 3 credits over 3 rows.
        assert_eq!(metric_per_success(&rows, "credits"), Some(9.0));
    }

    #[test]
    fn numstat_z_collects_paths_including_binary() {
        // `-z`-delimited entries; a binary file reports `-\t-\t<path>` instead
        // of numeric counts, but the counts are never used by either caller
        // of this parser, so a binary path is just as real a churn entry as
        // a text one and must not be silently dropped.
        let text = "3\t1\tsrc/a.rs\00\t0\tsrc/b.rs\0-\t-\tbin/blob\0";
        let paths: Vec<String> = parse_numstat_z(text)
            .into_iter()
            .filter_map(|item| match item {
                NumstatZItem::Path(p) => Some(p),
                _ => None,
            })
            .collect();
        assert!(paths.contains(&"src/a.rs".to_string()));
        assert!(paths.contains(&"src/b.rs".to_string()));
        assert!(paths.contains(&"bin/blob".to_string()), "binary rows must still count as a touched path");
    }

    #[test]
    fn numstat_z_records_a_rename_with_both_names() {
        // Byte layout verified against a real repo: `git show --numstat -z`
        // for a rename is `added\tdeleted\t` + an empty NUL-terminated field
        // (the rename marker), then the old path, then the new path. Both
        // names are kept on the item; which one(s) a caller uses depends on
        // whether it is building the measured commit's file set (new only)
        // or scanning later history for touches (both).
        let text = "0\t0\t\0old.txt\0new.txt\0";
        let items = parse_numstat_z(text);
        assert_eq!(items.len(), 1);
        assert!(matches!(
            &items[0],
            NumstatZItem::Rename { old, new } if old == "old.txt" && new == "new.txt"
        ));
    }

    #[test]
    fn numstat_z_reads_commit_headers_across_the_artifact_newline() {
        // `git log -z --format="COMMIT %ct"` NUL-terminates each header, then
        // inserts a bare `\n` before the numstat block when one follows; that
        // newline is a formatting artifact, not part of the next path.
        let text = "COMMIT 100\0\n0\t0\tplain.txt\0COMMIT 90\0\n1\t0\tother.txt\0";
        let items = parse_numstat_z(text);
        assert_eq!(items.len(), 4);
        assert!(matches!(items[0], NumstatZItem::Commit(t) if t == 100.0));
        assert!(matches!(&items[1], NumstatZItem::Path(p) if p == "plain.txt"));
        assert!(matches!(items[2], NumstatZItem::Commit(t) if t == 90.0));
        assert!(matches!(&items[3], NumstatZItem::Path(p) if p == "other.txt"));
    }

    /// Mirrors how `churn_for_commit` itself flattens `parse_numstat_z`
    /// output, so these tests exercise the exact asymmetry the fix relies on
    /// rather than a simplified stand-in for it.
    fn measured_files(text: &str) -> BTreeSet<String> {
        parse_numstat_z(text)
            .into_iter()
            .filter_map(|item| match item {
                NumstatZItem::Path(p) => Some(p),
                NumstatZItem::Rename { new, .. } => Some(new),
                NumstatZItem::Commit(_) => None,
            })
            .collect()
    }

    fn touched_paths(text: &str) -> BTreeSet<String> {
        let mut touched = BTreeSet::new();
        for item in parse_numstat_z(text) {
            match item {
                NumstatZItem::Path(p) => {
                    touched.insert(p);
                }
                NumstatZItem::Rename { old, new } => {
                    touched.insert(old);
                    touched.insert(new);
                }
                NumstatZItem::Commit(_) => {}
            }
        }
        touched
    }

    #[test]
    fn churn_matches_a_renamed_file_by_its_destination_path() {
        // The bug this fixes: a rename with a common-prefix path renders as
        // `old.txt => new.txt` under plain (non-`-z`) `--numstat`, which
        // line-based tab-splitting stored as one literal, unmatchable path,
        // so a later edit of the file under its new name never intersected
        // it and churn was always undercounted across a rename.
        let files = measured_files("0\t0\t\0old.txt\0new.txt\0");
        let touched = touched_paths("COMMIT 200\01\t0\tnew.txt\0");
        assert_eq!(files.intersection(&touched).count(), 1, "new.txt must match on both sides of the rename");
    }

    #[test]
    fn churn_matches_a_file_later_renamed_by_its_original_name() {
        // The opposite direction: the measured commit is a plain edit (no
        // rename), and a later commit renames the file. The old name must
        // still be recognized as touched, or churn stays 0% across a rename
        // that happens after measurement instead of within the measured
        // commit itself.
        let files = measured_files("2\t1\ta.txt\0");
        let touched = touched_paths("COMMIT 200\00\t0\t\0a.txt\0b.txt\0");
        assert_eq!(files.intersection(&touched).count(), 1, "a.txt must match its own later rename");

        // The measured file set itself must not double-count a rename: only
        // the destination is a "file" from the measured commit's own
        // perspective, so a rename there keeps the denominator at 1.
        let rename_as_measured = measured_files("0\t0\t\0old.txt\0new.txt\0");
        assert_eq!(rename_as_measured.len(), 1);
    }

    #[test]
    fn baseline_cleanup_removes_empty_parent_directories() {
        let marker = temp_path("phase");
        fs::write(&marker, "baseline\n").unwrap();
        let stash = temp_path("stash");
        fs::create_dir_all(stash.join(".github")).unwrap();
        fs::create_dir_all(stash.join(".claude")).unwrap();
        let manifest = stash.join("STASHED.txt");
        fs::write(
            &manifest,
            ".github/copilot-instructions.md\n.claude/skills\n",
        )
        .unwrap();

        let entries = vec![
            ".github/copilot-instructions.md".to_string(),
            ".claude/skills".to_string(),
        ];
        finish_baseline_stop_cleanup(&stash, &manifest, &marker, &entries, &[]).unwrap();

        assert!(
            !stash.exists(),
            "empty parent directories must not block cleanup"
        );
        assert!(
            !marker.exists(),
            "successful cleanup returns to standard phase"
        );
    }

    #[test]
    fn baseline_cleanup_keeps_marker_when_stash_removal_fails() {
        let marker = temp_path("phase");
        fs::write(&marker, "baseline\n").unwrap();
        let stash = temp_path("stash");
        fs::create_dir(&stash).unwrap();
        let manifest = stash.join("STASHED.txt");
        fs::write(&manifest, "AGENTS.md\n").unwrap();
        fs::write(stash.join("unexpected"), "keep me").unwrap();

        let error = finish_baseline_stop_cleanup(&stash, &manifest, &marker, &[], &[]).unwrap_err();
        assert!(error.contains("baseline stash"));
        assert!(marker.exists(), "the phase marker must remain active");
        assert!(stash.exists(), "the stash must remain recoverable");
        assert!(!manifest.exists());
    }

    #[test]
    fn baseline_cleanup_keeps_active_state_when_marker_removal_fails() {
        let marker = temp_path("phase-dir");
        fs::create_dir(&marker).unwrap();
        let stash = temp_path("stash");
        fs::create_dir(&stash).unwrap();
        let manifest = stash.join("STASHED.txt");
        fs::write(&manifest, "AGENTS.md\n").unwrap();

        let error = finish_baseline_stop_cleanup(&stash, &manifest, &marker, &[], &[]).unwrap_err();
        assert!(error.contains("phase marker"));
        assert!(marker.is_dir());
        assert!(stash.exists(), "an empty stash keeps baseline_active true");
    }

    #[test]
    fn stash_without_a_manifest_is_never_treated_as_deletable() {
        // An interrupted start can leave files with no manifest. They may be
        // the only copy of a contract file, so `stop` must refuse rather than
        // remove the directory.
        let stash = temp_path("stash");
        fs::create_dir_all(&stash).unwrap();
        fs::write(stash.join("AGENTS.md"), "contract").unwrap();

        match inspect_stash(&stash).unwrap() {
            StashState::Unattributable(found) => assert_eq!(found, vec!["AGENTS.md"]),
            _ => panic!("a stash with contents and no manifest must be unattributable"),
        }

        // With a manifest, the same directory is restorable.
        fs::write(stash.join("STASHED.txt"), "AGENTS.md\n\n").unwrap();
        match inspect_stash(&stash).unwrap() {
            StashState::Listed(paths) => assert_eq!(paths, vec!["AGENTS.md"]),
            _ => panic!("a manifest must be honored"),
        }

        // A genuinely empty stash is safe to drop.
        let empty = temp_path("empty-stash");
        fs::create_dir_all(&empty).unwrap();
        assert!(matches!(inspect_stash(&empty).unwrap(), StashState::Empty));
    }

    #[test]
    fn success_resolution_requires_commit_and_verification() {
        let session = format!("t{}", std::process::id());
        let dir = live_dir();
        let _ = fs::create_dir_all(&dir);

        // No commit yet: undecidable even though tests passed.
        assert_eq!(
            update_and_resolve_success("testagent", &session, Some(true), false),
            "unknown"
        );
        // Commit arrives in a later turn; the earlier pass is carried forward.
        assert_eq!(
            update_and_resolve_success("testagent", &session, None, true),
            "yes"
        );
        // State is cleared after the commit closes the task.
        assert_eq!(
            update_and_resolve_success("testagent", &session, None, true),
            "unknown"
        );
        assert_eq!(
            update_and_resolve_success("testagent", &session, Some(false), true),
            "no"
        );
        let _ = fs::remove_file(verif_path("testagent", &session));
    }
}
