//! Local routing metadata. No task prompt, source file, or tool output is stored here.

use rusqlite::{params, Connection, OpenFlags};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: i64 = 1;
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

pub struct DecisionRecord<'a> {
    pub task_type: &'a str,
    pub repository: &'a str,
    pub priority: &'a str,
    pub profile: &'a str,
    pub provider: &'a str,
    pub effort: &'a str,
    pub source: &'a str,
    pub task_bytes: usize,
    pub candidates: Vec<&'a str>,
    pub rejected: Vec<&'a str>,
}

fn millis_now() -> Result<i64, String> {
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("clock error: {e}"))?
        .as_millis();
    i64::try_from(n).map_err(|_| "timestamp exceeds SQLite integer range".into())
}

fn task_id(now: i64) -> String {
    format!(
        "{now:x}-{:x}-{:x}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    )
}

fn db_path(root: &Path) -> PathBuf {
    root.join(".sentrith").join("sentrith.db")
}

fn inspect_regular_or_missing(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() => Ok(true),
        Ok(_) => Err(format!("{} is not a regular file", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("cannot inspect {}: {e}", path.display())),
    }
}

fn open(root: &Path) -> Result<Connection, String> {
    let dir = root.join(".sentrith");
    crate::create_real_directory_tree(&dir)?;
    let path = db_path(root);
    if !inspect_regular_or_missing(&path)? {
        // Restrict new local metadata to the owner. SQLite reopens this path.
        drop(crate::create_secure_file(&path)?);
    }
    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|e| format!("cannot open {}: {e}", path.display()))?;
    conn.busy_timeout(Duration::from_secs(3))
        .map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

fn version(conn: &Connection) -> Result<i64, String> {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|e| e.to_string())
}

fn migrate_connection(conn: &mut Connection) -> Result<i64, String> {
    let current = version(conn)?;
    if current > SCHEMA_VERSION {
        return Err(format!(
            "database schema {current} is newer than supported {SCHEMA_VERSION}"
        ));
    }
    if current == SCHEMA_VERSION {
        return Ok(current);
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute_batch(
        r#"
        CREATE TABLE routes (
            task_id TEXT PRIMARY KEY,
            created_at_unix_ms INTEGER NOT NULL,
            task_type TEXT NOT NULL,
            repository TEXT NOT NULL,
            priority TEXT NOT NULL,
            provider TEXT NOT NULL,
            execution_profile TEXT NOT NULL,
            effort TEXT NOT NULL,
            decision_source TEXT NOT NULL,
            context_size_before_bytes INTEGER NOT NULL,
            context_size_after_bytes INTEGER NOT NULL,
            duration_ms INTEGER,
            success TEXT NOT NULL DEFAULT 'unknown',
            retry_count INTEGER NOT NULL DEFAULT 0,
            validation_result TEXT NOT NULL DEFAULT 'not_run',
            review_result TEXT NOT NULL DEFAULT 'not_run',
            fallback_count INTEGER NOT NULL DEFAULT 0,
            quota_state TEXT NOT NULL DEFAULT 'unknown'
        );
        CREATE TABLE route_options (
            task_id TEXT NOT NULL REFERENCES routes(task_id) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL,
            profile TEXT NOT NULL,
            disposition TEXT NOT NULL,
            reason TEXT,
            PRIMARY KEY (task_id, ordinal)
        );
        CREATE TABLE events (
            seq INTEGER PRIMARY KEY,
            timestamp_unix_ms INTEGER NOT NULL,
            task_id TEXT NOT NULL REFERENCES routes(task_id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            detail TEXT
        );
        CREATE INDEX events_task_id ON events(task_id);
        PRAGMA user_version = 1;
    "#,
    )
    .map_err(|e| format!("schema migration failed: {e}"))?;
    tx.commit()
        .map_err(|e| format!("schema migration commit failed: {e}"))?;
    Ok(SCHEMA_VERSION)
}

pub fn migrate(root: &Path) -> Result<i64, String> {
    let mut conn = open(root)?;
    migrate_connection(&mut conn)
}

pub fn db_command(args: &[String]) -> Result<(), String> {
    match args {
        [command] if command == "migrate" => {
            let root = crate::config::current_root()?;
            println!("Database schema: {}", migrate(&root)?);
            Ok(())
        }
        _ => Err("usage: sentrith db migrate".into()),
    }
}

pub fn database_status(root: &Path) -> Result<Option<(i64, i64)>, String> {
    let dir = root.join(".sentrith");
    match fs::symlink_metadata(&dir) {
        Ok(meta) if !meta.is_dir() => return Err(".sentrith is not a real directory".into()),
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("cannot inspect .sentrith: {e}")),
    }
    let path = db_path(root);
    if !inspect_regular_or_missing(&path)? {
        return Ok(None);
    }
    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|e| format!("cannot read database: {e}"))?;
    let schema = version(&conn)?;
    if schema > SCHEMA_VERSION {
        return Err(format!(
            "database schema {schema} is newer than supported {SCHEMA_VERSION}"
        ));
    }
    let count = if schema == 0 {
        0
    } else {
        conn.query_row("SELECT count(*) FROM routes", [], |row| row.get(0))
            .map_err(|e| format!("cannot count routes: {e}"))?
    };
    Ok(Some((schema, count)))
}

pub fn record_route(root: &Path, record: DecisionRecord<'_>) -> Result<String, String> {
    let mut conn = open(root)?;
    migrate_connection(&mut conn)?;
    let now = millis_now()?;
    let id = task_id(now);
    let task_bytes = i64::try_from(record.task_bytes).map_err(|_| "task too large".to_string())?;
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO routes (task_id,created_at_unix_ms,task_type,repository,priority,provider,execution_profile,effort,decision_source,context_size_before_bytes,context_size_after_bytes) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)",
        params![id, now, record.task_type, record.repository, record.priority, record.provider, record.profile, record.effort, record.source, task_bytes],
    ).map_err(|e| format!("cannot record route: {e}"))?;
    for (ordinal, profile) in record.candidates.iter().enumerate() {
        tx.execute("INSERT INTO route_options (task_id,ordinal,profile,disposition) VALUES (?1,?2,?3,'allowed')",
            params![id, ordinal as i64, profile]).map_err(|e| e.to_string())?;
    }
    let offset = record.candidates.len();
    for (i, reason) in record.rejected.iter().enumerate() {
        tx.execute("INSERT INTO route_options (task_id,ordinal,profile,disposition,reason) VALUES (?1,?2,'','rejected',?3)",
            params![id, (offset + i) as i64, reason]).map_err(|e| e.to_string())?;
    }
    tx.execute(
        "INSERT INTO events (timestamp_unix_ms,task_id,name) VALUES (?1,?2,'task.created')",
        params![now, id],
    )
    .map_err(|e| e.to_string())?;
    let event = if record.profile == "HUMAN_GATE" {
        "route.rejected"
    } else {
        "route.selected"
    };
    tx.execute(
        "INSERT INTO events (timestamp_unix_ms,task_id,name,detail) VALUES (?1,?2,?3,?4)",
        params![now, id, event, record.profile],
    )
    .map_err(|e| e.to_string())?;
    tx.commit()
        .map_err(|e| format!("cannot commit route: {e}"))?;
    Ok(id)
}

pub fn explain(root: &Path, id: &str) -> Result<(), String> {
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return Err("invalid task ID".into());
    }
    let path = db_path(root);
    let dir = root.join(".sentrith");
    if !fs::symlink_metadata(&dir)
        .map_err(|e| format!("cannot inspect .sentrith: {e}"))?
        .is_dir()
    {
        return Err(".sentrith is not a real directory".into());
    }
    if !inspect_regular_or_missing(&path)? {
        return Err("routing database is not initialized".into());
    }
    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW,
    )
    .map_err(|e| format!("cannot read database: {e}"))?;
    let row = conn.query_row(
        "SELECT task_type,priority,execution_profile,decision_source,quota_state,validation_result FROM routes WHERE task_id=?1",
        [id], |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?,r.get::<_, String>(2)?,r.get::<_, String>(3)?,r.get::<_, String>(4)?,r.get::<_, String>(5)?))
    ).map_err(|e| format!("task not found: {e}"))?;
    println!("Task: {id}");
    println!("Type: {}; priority: {}", row.0, row.1);
    println!("Selected: {}", row.2);
    println!("Decision: {}", row.3);
    println!("Quota: {}; validation: {}", row.4, row.5);
    let mut stmt = conn.prepare("SELECT profile,disposition,reason FROM route_options WHERE task_id=?1 ORDER BY ordinal")
        .map_err(|e| e.to_string())?;
    let options = stmt
        .query_map([id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    for option in options {
        let (profile, disposition, reason) = option.map_err(|e| e.to_string())?;
        if disposition == "allowed" && profile != row.2 {
            println!("Alternative: {profile}");
        }
        if disposition == "rejected" {
            println!("Rejected: {}", reason.unwrap_or_default());
        }
    }
    Ok(())
}

pub fn explain_command(args: &[String]) -> Result<(), String> {
    match args {
        [id] => explain(&crate::config::current_root()?, id),
        _ => Err("usage: sentrith explain <task-id>".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "sentrith-storage-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn migration_is_idempotent_and_status_read_only() {
        let root = temp_root();
        assert!(database_status(&root).unwrap().is_none());
        assert!(!root.join(".sentrith").exists());
        assert_eq!(migrate(&root).unwrap(), 1);
        assert_eq!(migrate(&root).unwrap(), 1);
        assert_eq!(database_status(&root).unwrap(), Some((1, 0)));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn records_metadata_without_prompt() {
        let root = temp_root();
        let id = record_route(
            &root,
            DecisionRecord {
                task_type: "bugfix",
                repository: "test",
                priority: "P1",
                profile: "CODEX_MEDIUM",
                provider: "codex",
                effort: "medium",
                source: "rule",
                task_bytes: 19,
                candidates: vec!["CODEX_MEDIUM"],
                rejected: vec!["CLAUDE_NORMAL: CLI unavailable"],
            },
        )
        .unwrap();
        assert_eq!(database_status(&root).unwrap(), Some((1, 1)));
        let conn = Connection::open(db_path(&root)).unwrap();
        let names: Vec<String> = conn
            .prepare("SELECT name FROM events WHERE task_id=?1 ORDER BY seq")
            .unwrap()
            .query_map([id.as_str()], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        assert_eq!(names, ["task.created", "route.selected"]);
        drop(conn);
        assert!(!fs::read(db_path(&root))
            .unwrap()
            .windows(11)
            .any(|s| s == b"secret task"));
        explain(&root, &id).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn refuses_symlinked_store_directory() {
        let root = temp_root();
        fs::write(root.join(".sentrith"), "not a dir").unwrap();
        assert!(migrate(&root).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_newer_schema() {
        let root = temp_root();
        migrate(&root).unwrap();
        let conn = Connection::open(db_path(&root)).unwrap();
        conn.execute_batch("PRAGMA user_version = 2;").unwrap();
        drop(conn);
        assert!(migrate(&root).unwrap_err().contains("newer"));
        fs::remove_dir_all(root).unwrap();
    }
}
