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

fn sentrith_hook_count(text: &str) -> usize {
    let Ok(settings) = json_parse(text) else {
        return 0;
    };
    settings
        .get("hooks")
        .map(count_sentrith_hooks)
        .unwrap_or(0)
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
fn strip_sentrith_hooks(hooks: &mut Json) {
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
                                .map(is_sentrith_command)
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
        let n = sentrith_hook_count(&read_text(&path));
        if n == 0 {
            println!("SENTRITH-HOOKS [{}]: {} exists but has no Sentrith hooks", t.agent, t.settings);
        } else {
            println!("SENTRITH-HOOKS [{}]: installed ({} Sentrith hooks in {})", t.agent, n, t.settings);
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

#[cfg(not(windows))]
fn replace_file_preserving_security(replacement: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(replacement, destination).map_err(|e| e.to_string())
}

#[cfg(windows)]
fn replace_file_preserving_security(replacement: &Path, destination: &Path) -> Result<(), String> {
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
            std::ptr::null(),
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
            return Err(format!(
                "GetFileAttributesW failed after replacement with OS error {}",
                unsafe { GetLastError() }
            ));
        }
        if unsafe {
            SetFileAttributesW(replaced.as_ptr(), new_attributes | FILE_ATTRIBUTE_READONLY)
        } == 0
        {
            return Err(format!(
                "SetFileAttributesW failed after replacement with OS error {}",
                unsafe { GetLastError() }
            ));
        }
    }

    Ok(())
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
        let existed = settings_path.exists();
        let mut settings = if existed {
            let raw = read_text(&settings_path);
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
            fs::copy(&settings_path, &backup).map_err(|e| e.to_string())?;
        }
        let tmp = settings_path.with_extension("json.sentrith-tmp");
        fs::write(&tmp, &rendered).map_err(|e| e.to_string())?;
        #[cfg(not(windows))]
        if existed {
            copy_file_permissions(&settings_path, &tmp)?;
        }
        if existed {
            replace_file_preserving_security(&tmp, &settings_path)?;
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
    for t in HOOK_TARGETS {
        let path = repo_file(t.settings);
        let installed = path.exists() && sentrith_hook_count(&read_text(&path)) > 0;
        if installed {
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

/// File paths from `git … --numstat` output. Binary files are reported as
/// `-\t-\tpath` and are skipped: line counts are the churn signal here.
fn parse_numstat_files(text: &str) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for line in text.lines() {
        let mut parts = line.split('\t');
        if let (Some(a), Some(_d), Some(path)) = (parts.next(), parts.next(), parts.next()) {
            if a.trim().parse::<u64>().is_ok() && !path.trim().is_empty() {
                set.insert(path.trim().to_string());
            }
        }
    }
    set
}

/// File-level churn: how many files of `sha` were modified again by later
/// commits within `days`. A rework proxy computable retroactively from git.
fn churn_for_commit(sha: &str, days: f64) -> Option<(usize, usize)> {
    let files = parse_numstat_files(&git(&["show", "--numstat", "--format=", sha]));
    if files.is_empty() {
        return None;
    }
    let t0: f64 = git(&["show", "-s", "--format=%ct", sha]).trim().parse().ok()?;
    let range = format!("{sha}..HEAD");
    let log = git(&["log", "--numstat", "--format=COMMIT %ct", range.as_str()]);
    let mut touched = BTreeSet::new();
    let mut in_window = false;
    for line in log.lines() {
        if let Some(rest) = line.strip_prefix("COMMIT ") {
            in_window = rest
                .trim()
                .parse::<f64>()
                .map(|t| t <= t0 + days * 86400.0)
                .unwrap_or(false);
        } else if in_window {
            for path in parse_numstat_files(line) {
                touched.insert(path);
            }
        }
    }
    let changed = files.iter().filter(|f| touched.contains(*f)).count();
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
    let rest = rest[colon + 1..].trim_start();
    if !rest.starts_with('"') { return None; }
    let mut out = String::new();
    let mut escaped = false;
    for c in rest[1..].chars() {
        if escaped {
            out.push(c);
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            return Some(out);
        } else {
            out.push(c);
        }
    }
    None
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
    replace_file_preserving_security(&tmp, path)?;
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

fn baseline_stash_dir() -> PathBuf {
    PathBuf::from(".sentrith-private/baseline-stash")
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
        let mut rollback_failed = Vec::new();
        for path in moved.iter().rev() {
            let src = stash.join(path);
            let dst = repo_file(path);
            if src.exists() && !dst.exists() {
                if let Err(e) = fs::rename(&src, &dst) {
                    rollback_failed.push(format!("{path} ({e})"));
                }
            }
        }
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

    println!("SENTRITH-BASELINE: started. Stashed {} path(s):", moved.len());
    for m in &moved {
        println!("  {m}");
    }
    println!("Measurement hooks and .ai-usage/ were left active; new turns record phase=baseline.");
    println!("Git will show these paths as deleted until you run `sentrith usage baseline stop`.");
    println!("Start a NEW agent session so the stashed instructions are not still in its context.");
    println!("When you have enough baseline tasks: sentrith usage baseline stop");
    Ok(())
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
        .filter(|n| n != "STASHED.txt")
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
) -> Result<(), String> {
    remove_empty_stash_parents(stash, entries)?;
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
    let entries = match inspect_stash(&stash)? {
        StashState::Listed(paths) => paths,
        StashState::Empty => {
            finish_baseline_stop_cleanup(&stash, &manifest, &phase_marker_path(), &[])?;
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

    finish_baseline_stop_cleanup(&stash, &manifest, &phase_marker_path(), &entries)?;
    println!("SENTRITH-BASELINE: stopped. Restored {restored} path(s); phase is standard again.");
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

fn shell_command_segments(command: &str) -> Vec<Vec<String>> {
    let mut segments = vec![Vec::new()];
    let mut token = String::new();
    let mut quote = None;

    let flush_token = |segments: &mut Vec<Vec<String>>, token: &mut String| {
        if !token.is_empty() {
            segments.last_mut().unwrap().push(std::mem::take(token));
        }
    };

    for ch in command.chars() {
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
            c if c.is_whitespace() => flush_token(&mut segments, &mut token),
            ';' | '|' | '&' => {
                flush_token(&mut segments, &mut token);
                if !segments.last().unwrap().is_empty() {
                    segments.push(Vec::new());
                }
            }
            _ => token.push(ch),
        }
    }
    flush_token(&mut segments, &mut token);
    segments.retain(|segment| !segment.is_empty());
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

/// Recognize an actual test-runner executable in a shell command. Text that
/// merely mentions a test command, such as `echo "cargo test"` or `rg`,
/// must not mark verification as successful.
fn is_test_command(cmd: &str) -> bool {
    shell_command_segments(cmd)
        .iter()
        .any(|segment| is_test_invocation(segment))
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
/// Codex rollout entries carry `command` and, in exec-result entries,
/// `exit_code`; the two may or may not share a line.
fn scan_codex_window_for_tests(text: &str, skip_lines: usize) -> Option<bool> {
    let mut result = None;
    let mut awaiting = false;
    for line in text.lines().skip(skip_lines) {
        if line.contains("\"command\"") {
            if let Some(cmd) = json_string_field(line, "command") {
                if is_test_command(&cmd) {
                    if let Some(c) = json_number_field(line, "exit_code") {
                        result = Some(c == 0.0);
                        awaiting = false;
                    } else {
                        awaiting = true;
                    }
                    continue;
                }
            }
        }
        if awaiting && line.contains("\"exit_code\"") {
            if let Some(c) = json_number_field(line, "exit_code") {
                result = Some(c == 0.0);
            }
            awaiting = false;
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
        assert!(!is_test_command("cargo test --no-run"));
        assert!(!is_test_command("cargo test -- --list"));
        assert!(!is_test_command("pytest --collect-only"));
        assert!(!is_test_command("python -m pytest --collect-only"));
        assert!(!is_test_command("go test -list ."));
        assert!(!is_test_command("dotnet test --list-tests"));
    }

    #[test]
    fn codex_window_reads_exit_code_on_following_line() {
        let same_line = r#"{"command":"cargo test","exit_code":0}"#;
        assert_eq!(scan_codex_window_for_tests(same_line, 0), Some(true));

        let split = [
            r#"{"type":"exec","command":"pytest -q"}"#,
            r#"{"type":"exec_result","exit_code":1}"#,
        ]
        .join("\n");
        assert_eq!(scan_codex_window_for_tests(&split, 0), Some(false));

        let unrelated = r#"{"command":"ls","exit_code":1}"#;
        assert_eq!(scan_codex_window_for_tests(unrelated, 0), None);
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

        replace_file_preserving_security(&replacement, &original).unwrap();

        assert!(fs::metadata(&original).unwrap().permissions().readonly());
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
    fn numstat_parsing_collects_changed_paths() {
        let text = "3\t1\tsrc/a.rs\n0\t0\tsrc/b.rs\n-\t-\tbin/blob\n";
        let files = parse_numstat_files(text);
        assert!(files.contains("src/a.rs"));
        assert!(files.contains("src/b.rs"));
        assert!(!files.contains("bin/blob"), "binary rows have no numeric counts");
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
        finish_baseline_stop_cleanup(&stash, &manifest, &marker, &entries).unwrap();

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

        let error = finish_baseline_stop_cleanup(&stash, &manifest, &marker, &[]).unwrap_err();
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

        let error = finish_baseline_stop_cleanup(&stash, &manifest, &marker, &[]).unwrap_err();
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
