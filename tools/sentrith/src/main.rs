use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const USAGE_HEADER: &str = "timestamp,agent,model,phase,task,input_tokens,cached_input_tokens,output_tokens,credits,cost_usd,tool_calls,duration_seconds,success,rework_count,source,session_id,notes\n";

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

Provider measurement:
  GitHub Copilot snapshot uses `gh api` only when explicitly requested.
  Other commands are local/deterministic and make no model calls.
  Raw prompts, source code, repository names, transcripts, and session IDs
  are never included in community contribution files.
"#);
}

fn repo_file(path: &str) -> PathBuf {
    Path::new(path).to_path_buf()
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

fn usage_command(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("usage requires record, report, or note".into());
    }
    match args[0].as_str() {
        "record" => usage_record(&args[1..]),
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

    let phase = opts.get("phase").map(String::as_str).unwrap_or("standard");
    if !["baseline", "standard", "other"].contains(&phase) {
        return Err("--phase must be baseline, standard, or other".into());
    }

    let file = opts
        .get("file")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));

    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let exists = file.exists();

    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file)
        .map_err(|e| e.to_string())?;

    if !exists {
        f.write_all(USAGE_HEADER.as_bytes()).map_err(|e| e.to_string())?;
    }

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
    ];
    let row = values.iter().map(|x| csv_escape(x)).collect::<Vec<_>>().join(",");
    writeln!(f, "{row}").map_err(|e| e.to_string())?;

    println!("SENTRITH-USAGE: recorded {agent} / {phase} / {task} -> {}", file.display());
    Ok(())
}

#[derive(Default, Clone)]
struct UsageRow {
    agent: String,
    phase: String,
    success: String,
    nums: BTreeMap<&'static str, Option<f64>>,
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

fn parse_num(s: Option<&String>) -> Option<f64> {
    let s = s?;
    if s.is_empty() { None } else { s.parse().ok() }
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

    let text = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let mut lines = text.lines();
    let header = lines.next().ok_or("usage file is empty")?;
    let headers = parse_csv_line(header);
    let idx: BTreeMap<String, usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.clone(), i))
        .collect();

    let numeric = [
        "input_tokens", "cached_input_tokens", "output_tokens", "credits", "cost_usd",
        "tool_calls", "duration_seconds", "rework_count",
    ];

    let mut rows = Vec::new();
    for line in lines {
        if line.trim().is_empty() { continue; }
        let cols = parse_csv_line(line);
        let get = |name: &str| -> Option<String> {
            idx.get(name).and_then(|i| cols.get(*i)).cloned()
        };
        let agent = get("agent").unwrap_or_default();
        if let Some(filter) = opts.get("agent") {
            if &agent != filter { continue; }
        }

        let mut row = UsageRow {
            agent,
            phase: get("phase").unwrap_or_default(),
            success: get("success").unwrap_or_default(),
            nums: BTreeMap::new(),
        };
        for name in numeric {
            row.nums.insert(name, parse_num(get(name).as_ref()));
        }
        rows.push(row);
    }

    if rows.is_empty() {
        println!("SENTRITH-USAGE: no matching rows");
        return Ok(());
    }

    if opts.contains_key("compare") {
        let base: Vec<_> = rows.iter().filter(|r| r.phase == "baseline").cloned().collect();
        let std: Vec<_> = rows.iter().filter(|r| r.phase == "standard").cloned().collect();
        let b = summarize(&base);
        let s = summarize(&std);
        print_summary("baseline", &b);
        print_summary("standard", &s);
        println!("\n[standard vs baseline]");
        for name in numeric {
            println!("{name}: {}", pct_text(b.get(name).copied().flatten(), s.get(name).copied().flatten()));
        }
        let bs = b.get("success_rate").copied().flatten();
        let ss = s.get("success_rate").copied().flatten();
        if let (Some(a), Some(bv)) = (bs, ss) {
            println!("success_rate: {:+.1} percentage points", bv - a);
        }
    } else {
        let mut groups: BTreeMap<(String, String), Vec<UsageRow>> = BTreeMap::new();
        for r in rows {
            groups.entry((r.agent.clone(), r.phase.clone())).or_default().push(r);
        }
        for ((agent, phase), rs) in groups {
            let s = summarize(&rs);
            print_summary(&format!("{agent} / {phase}"), &s);
        }
    }
    Ok(())
}

fn summarize(rows: &[UsageRow]) -> BTreeMap<&'static str, Option<f64>> {
    let names = [
        "input_tokens", "cached_input_tokens", "output_tokens", "credits", "cost_usd",
        "tool_calls", "duration_seconds", "rework_count",
    ];
    let mut out = BTreeMap::new();
    for name in names {
        let vals: Vec<f64> = rows.iter().filter_map(|r| r.nums.get(name).copied().flatten()).collect();
        out.insert(name, if vals.is_empty() { None } else { Some(vals.iter().sum::<f64>() / vals.len() as f64) });
    }
    let success_rate = if rows.is_empty() {
        None
    } else {
        Some(rows.iter().filter(|r| r.success == "yes").count() as f64 / rows.len() as f64 * 100.0)
    };
    out.insert("success_rate", success_rate);
    out.insert("tasks", Some(rows.len() as f64));
    out
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

fn load_usage_rows(path: &Path, agent_filter: Option<&str>, model_filter: Option<&str>) -> Result<Vec<BTreeMap<String, String>>, String> {
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut lines = text.lines();
    let header = lines.next().ok_or("usage file is empty")?;
    let headers = parse_csv_line(header);
    let mut rows = Vec::new();

    for line in lines {
        if line.trim().is_empty() { continue; }
        let cols = parse_csv_line(line);
        let mut row = BTreeMap::new();
        for (i, h) in headers.iter().enumerate() {
            row.insert(h.clone(), cols.get(i).cloned().unwrap_or_default());
        }
        if let Some(a) = agent_filter {
            if row.get("agent").map(String::as_str).unwrap_or("") != a { continue; }
        }
        if let Some(m) = model_filter {
            if row.get("model").map(String::as_str).unwrap_or("") != m { continue; }
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

fn publish_stats(rows: &[&BTreeMap<String, String>]) -> PublishStats {
    let successes = rows.iter()
        .filter(|r| r.get("success").map(String::as_str) == Some("yes"))
        .count();
    let tasks = rows.len();
    let success_rate = if tasks == 0 { None } else { Some(successes as f64 / tasks as f64 * 100.0) };
    let total_credits = sum_field(rows, "credits");
    let credits_per_success = match (total_credits, successes) {
        (Some(c), n) if n > 0 => Some(c / n as f64),
        _ => None,
    };

    PublishStats {
        tasks,
        successes,
        success_rate,
        credits_avg: avg_field(rows, "credits"),
        tool_calls_avg: avg_field(rows, "tool_calls"),
        rework_avg: avg_field(rows, "rework_count"),
        input_avg: avg_field(rows, "input_tokens"),
        cached_input_avg: avg_field(rows, "cached_input_tokens"),
        output_avg: avg_field(rows, "output_tokens"),
        duration_avg: avg_field(rows, "duration_seconds"),
        total_credits,
        credits_per_success,
    }
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

    let rows = load_usage_rows(&file, Some(agent), model)?;
    let baseline_rows: Vec<_> = rows.iter().filter(|r| r.get("phase").map(String::as_str) == Some("baseline")).collect();
    let standard_rows: Vec<_> = rows.iter().filter(|r| r.get("phase").map(String::as_str) == Some("standard")).collect();

    if !force && (baseline_rows.len() < min_samples || standard_rows.len() < min_samples) {
        return Err(format!(
            "refusing README publication: baseline={} standard={} but min-samples={}. Add more data or pass --force.",
            baseline_rows.len(), standard_rows.len(), min_samples
        ));
    }

    let b = publish_stats(&baseline_rows);
    let s = publish_stats(&standard_rows);

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

fn ensure_usage_file(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    if !path.exists() {
        fs::write(path, USAGE_HEADER).map_err(|e| e.to_string())?;
    }
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
}

fn num_cell(v: Option<f64>) -> String {
    v.map(|x| {
        if (x.fract()).abs() < 0.0000001 { format!("{:.0}", x) } else { format!("{}", x) }
    }).unwrap_or_default()
}

fn append_auto_usage(path: &Path, u: &AutoUsage) -> Result<(), String> {
    ensure_usage_file(path)?;
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
    ];
    let row = values.iter().map(|x| csv_escape(x)).collect::<Vec<_>>().join(",");
    let mut f = OpenOptions::new().append(true).open(path).map_err(|e| e.to_string())?;
    writeln!(f, "{row}").map_err(|e| e.to_string())
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
    let phase = opts.get("phase").cloned().unwrap_or_else(|| "standard".into());
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
    let phase = opts.get("phase").cloned().unwrap_or_else(|| "standard".into());
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
    let phase = opts.get("phase").cloned().unwrap_or_else(|| "standard".into());
    let file = opts.get("file").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));
    let input = read_stdin_all()?;
    let event = json_string_field(&input, "hook_event_name").unwrap_or_default();
    let session = json_string_field(&input, "session_id").unwrap_or_else(|| "unknown".into());

    if event == "UserPromptSubmit" {
        let prompt = json_string_field(&input, "prompt").unwrap_or_else(|| "Claude turn".into());
        let snap = read_kv(&snapshot_path("claude", &session));
        write_kv(&task_path("claude", &session), &[
            ("phase", phase),
            ("task", prompt.chars().take(160).collect()),
            ("start_cost", snap.get("cost_usd").cloned().unwrap_or_else(|| "0".into())),
            ("start_duration_ms", snap.get("duration_ms").cloned().unwrap_or_else(|| "0".into())),
            ("model", snap.get("model").cloned().unwrap_or_default()),
        ])?;
    } else if event == "Stop" {
        let task = read_kv(&task_path("claude", &session));
        let snap = read_kv(&snapshot_path("claude", &session));
        if !task.is_empty() && !snap.is_empty() {
            let sc: f64 = task.get("start_cost").and_then(|x| x.parse().ok()).unwrap_or(0.0);
            let ec: f64 = snap.get("cost_usd").and_then(|x| x.parse().ok()).unwrap_or(sc);
            let sd: f64 = task.get("start_duration_ms").and_then(|x| x.parse().ok()).unwrap_or(0.0);
            let ed: f64 = snap.get("duration_ms").and_then(|x| x.parse().ok()).unwrap_or(sd);
            let u = AutoUsage {
                agent: "claude".into(),
                model: task.get("model").cloned().unwrap_or_default(),
                phase: task.get("phase").cloned().unwrap_or_else(|| "standard".into()),
                task: task.get("task").cloned().unwrap_or_else(|| "Claude turn".into()),
                cost_usd: Some((ec-sc).max(0.0)),
                duration_seconds: Some(((ed-sd)/1000.0).max(0.0)),
                source: "claude-statusline-hooks".into(),
                session_id: session.clone(),
                notes: "Estimated session-cost delta from official Claude Code statusLine JSON.".into(),
                ..Default::default()
            };
            append_auto_usage(&file, &u)?;
            let _ = fs::remove_file(task_path("claude", &session));
        }
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
    let phase = opts.get("phase").cloned().unwrap_or_else(|| "standard".into());
    let file = opts.get("file").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));
    let input = read_stdin_all()?;
    let event = json_string_field(&input, "hook_event_name").unwrap_or_default();
    let session = json_string_field(&input, "session_id").unwrap_or_else(|| "unknown".into());
    let model = json_string_field(&input, "model").unwrap_or_default();
    let transcript = json_string_field(&input, "transcript_path").unwrap_or_default();

    if event == "UserPromptSubmit" {
        let prompt = json_string_field(&input, "prompt").unwrap_or_else(|| "Codex turn".into());
        let (i,c,o) = if transcript.is_empty() {(None,None,None)} else {latest_token_usage_from_codex_transcript(Path::new(&transcript))};
        write_kv(&task_path("codex", &session), &[
            ("phase", phase),
            ("task", prompt.chars().take(160).collect()),
            ("model", model),
            ("transcript", transcript),
            ("start_input", num_cell(i)),
            ("start_cached", num_cell(c)),
            ("start_output", num_cell(o)),
        ])?;
    } else if event == "Stop" {
        let task = read_kv(&task_path("codex", &session));
        if !task.is_empty() {
            let tp = task.get("transcript").cloned().unwrap_or(transcript);
            let (ei,ec,eo) = if tp.is_empty() {(None,None,None)} else {latest_token_usage_from_codex_transcript(Path::new(&tp))};
            let parse = |k:&str| task.get(k).and_then(|x| x.parse::<f64>().ok());
            let delta = |end:Option<f64>, start:Option<f64>| match (end,start) {(Some(e),Some(s))=>Some((e-s).max(0.0)),(Some(e),None)=>Some(e),_=>None};
            let u = AutoUsage {
                agent: "codex".into(),
                model: task.get("model").cloned().unwrap_or_default(),
                phase: task.get("phase").cloned().unwrap_or_else(|| "standard".into()),
                task: task.get("task").cloned().unwrap_or_else(|| "Codex turn".into()),
                input_tokens: delta(ei, parse("start_input")),
                cached_input_tokens: delta(ec, parse("start_cached")),
                output_tokens: delta(eo, parse("start_output")),
                source: "codex-hook-transcript-best-effort".into(),
                session_id: session.clone(),
                notes: "Best-effort interactive capture: Codex documents transcript_path but warns transcript format is not a stable hook interface. Prefer usage run codex for stable JSON usage.".into(),
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
    let phase = opts.get("phase").cloned().unwrap_or_else(|| "standard".into());
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

fn metric_per_success(rows: &[&BTreeMap<String,String>], metric: &str) -> Option<f64> {
    let successes = rows.iter().filter(|r| r.get("success").map(String::as_str) == Some("yes")).count();
    if successes == 0 { return None; }
    metric_sum(rows, metric).map(|v| v / successes as f64)
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r")
}

fn usage_contribute(args: &[String]) -> Result<(), String> {
    let (opts, _) = parse_options(args)?;
    let agent = require(&opts, "agent")?;
    let model = opts.get("model").map(String::as_str);
    let file = opts.get("file").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(".ai-usage/usage.csv"));
    let rows = load_usage_rows(&file, Some(agent), model)?;
    let baseline: Vec<_> = rows.iter().filter(|r| r.get("phase").map(String::as_str)==Some("baseline")).collect();
    let standard: Vec<_> = rows.iter().filter(|r| r.get("phase").map(String::as_str)==Some("standard")).collect();

    let min_samples: usize = opts.get("min-samples").and_then(|x| x.parse().ok()).unwrap_or(10);
    if !opts.contains_key("force") && (baseline.len() < min_samples || standard.len() < min_samples) {
        return Err(format!("contribution needs at least {min_samples}+{min_samples} baseline/standard tasks; got {}+{}. Use --force only for experimental data.", baseline.len(), standard.len()));
    }

    let requested = opts.get("metric").map(String::as_str).unwrap_or("auto");
    let candidates: Vec<&str> = if requested == "auto" {
        vec!["credits", "cost_usd", "tokens"]
    } else {
        vec![requested]
    };
    let mut chosen = None;
    for m in candidates {
        if metric_per_success(&baseline, m).is_some() && metric_per_success(&standard, m).is_some() {
            chosen = Some(m);
            break;
        }
    }
    let metric = chosen.ok_or("no comparable usage metric found in both baseline and standard")?;
    let bps = metric_per_success(&baseline, metric).unwrap();
    let sps = metric_per_success(&standard, metric).unwrap();
    let change = if bps != 0.0 { (sps-bps)/bps*100.0 } else { 0.0 };
    let bs = baseline.iter().filter(|r| r.get("success").map(String::as_str)==Some("yes")).count();
    let ss = standard.iter().filter(|r| r.get("success").map(String::as_str)==Some("yes")).count();
    let bsr = if baseline.is_empty(){0.0}else{bs as f64/baseline.len() as f64*100.0};
    let ssr = if standard.is_empty(){0.0}else{ss as f64/standard.len() as f64*100.0};
    let quality = if baseline.len() >= 10 && standard.len() >= 10 { "qualified" } else { "experimental" };
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
        baseline.len(),
        standard.len(),
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
}
