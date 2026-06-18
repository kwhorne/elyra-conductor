// App-internal command history (the "flight recorder"), persisted to SQLite in
// the app data dir. This is Conductor's own store — distinct from the database
// browser in `db.rs`, which connects to the *user's* databases. Persisting the
// timeline across restarts is what unlocks cross-session recall ("how did I fix
// this last time?"), flow metrics, and runbook suggestions.

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};

pub struct HistoryStore(pub Mutex<Connection>);

impl HistoryStore {
    pub fn new(app: &AppHandle) -> Result<Self, String> {
        let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let conn = Connection::open(dir.join("history.db")).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS commands (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts INTEGER NOT NULL,
                project_path TEXT,
                label TEXT,
                proc TEXT,
                command TEXT,
                exit_code INTEGER,
                duration INTEGER,
                output TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_commands_ts ON commands(ts);
            CREATE INDEX IF NOT EXISTS idx_commands_project ON commands(project_path);",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self(Mutex::new(conn)))
    }
}

#[derive(serde::Deserialize)]
pub struct HistoryEntry {
    pub ts: i64,
    pub project_path: Option<String>,
    pub label: Option<String>,
    pub proc: Option<String>,
    pub command: Option<String>,
    pub exit_code: Option<i64>,
    pub duration: Option<i64>,
    pub output: Option<String>,
}

#[derive(serde::Serialize)]
pub struct HistoryRow {
    id: i64,
    ts: i64,
    project_path: Option<String>,
    label: Option<String>,
    proc: Option<String>,
    command: Option<String>,
    exit_code: Option<i64>,
    duration: Option<i64>,
    output: Option<String>,
}

#[tauri::command]
pub fn history_add(store: State<HistoryStore>, entry: HistoryEntry) -> Result<(), String> {
    let conn = store.0.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO commands (ts, project_path, label, proc, command, exit_code, duration, output)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        rusqlite::params![
            entry.ts,
            entry.project_path,
            entry.label,
            entry.proc,
            entry.command,
            entry.exit_code,
            entry.duration,
            entry.output,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Query history newest-first. With a non-empty `query`, does a substring match
/// over the command and its captured output (the "how did I fix this last time"
/// search). Optionally scoped to one project.
#[tauri::command]
pub fn history_query(
    store: State<HistoryStore>,
    query: Option<String>,
    project: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<HistoryRow>, String> {
    let conn = store.0.lock().map_err(|e| e.to_string())?;
    let limit = limit.unwrap_or(200).min(1000) as i64;
    let q = query.unwrap_or_default();
    let q = q.trim();
    let proj = project.unwrap_or_default();

    let mut sql = String::from(
        "SELECT id, ts, project_path, label, proc, command, exit_code, duration, output FROM commands WHERE 1=1",
    );
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if !q.is_empty() {
        sql.push_str(" AND (command LIKE ?1 OR output LIKE ?1)");
        params.push(Box::new(format!("%{q}%")));
    }
    if !proj.is_empty() {
        let idx = params.len() + 1;
        sql.push_str(&format!(" AND project_path = ?{idx}"));
        params.push(Box::new(proj));
    }
    let idx = params.len() + 1;
    sql.push_str(&format!(" ORDER BY ts DESC LIMIT ?{idx}"));
    params.push(Box::new(limit));

    let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
    let rows = stmt
        .query_map(param_refs.as_slice(), |r| {
            Ok(HistoryRow {
                id: r.get(0)?,
                ts: r.get(1)?,
                project_path: r.get(2)?,
                label: r.get(3)?,
                proc: r.get(4)?,
                command: r.get(5)?,
                exit_code: r.get(6)?,
                duration: r.get(7)?,
                output: r.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(rows)
}

/// Delete history, optionally only for one project.
#[tauri::command]
pub fn history_clear(store: State<HistoryStore>, project: Option<String>) -> Result<(), String> {
    let conn = store.0.lock().map_err(|e| e.to_string())?;
    match project.filter(|p| !p.is_empty()) {
        Some(p) => conn.execute("DELETE FROM commands WHERE project_path = ?1", [p]),
        None => conn.execute("DELETE FROM commands", []),
    }
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_store() -> HistoryStore {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE commands (id INTEGER PRIMARY KEY AUTOINCREMENT, ts INTEGER NOT NULL, project_path TEXT, label TEXT, proc TEXT, command TEXT, exit_code INTEGER, duration INTEGER, output TEXT);",
        )
        .unwrap();
        HistoryStore(Mutex::new(conn))
    }

    fn add(store: &HistoryStore, ts: i64, project: &str, command: &str, output: &str) {
        let conn = store.0.lock().unwrap();
        conn.execute(
            "INSERT INTO commands (ts, project_path, command, output) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![ts, project, command, output],
        )
        .unwrap();
    }

    fn query(store: &HistoryStore, q: Option<&str>, project: Option<&str>) -> Vec<HistoryRow> {
        // Mirror history_query without the State wrapper.
        let conn = store.0.lock().unwrap();
        let qs = q.unwrap_or_default();
        let proj = project.unwrap_or_default();
        let mut sql = String::from("SELECT id, ts, project_path, label, proc, command, exit_code, duration, output FROM commands WHERE 1=1");
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if !qs.is_empty() {
            sql.push_str(" AND (command LIKE ?1 OR output LIKE ?1)");
            params.push(Box::new(format!("%{qs}%")));
        }
        if !proj.is_empty() {
            let idx = params.len() + 1;
            sql.push_str(&format!(" AND project_path = ?{idx}"));
            params.push(Box::new(proj.to_string()));
        }
        sql.push_str(" ORDER BY ts DESC");
        let mut stmt = conn.prepare(&sql).unwrap();
        let refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| b.as_ref()).collect();
        stmt.query_map(refs.as_slice(), |r| {
            Ok(HistoryRow {
                id: r.get(0)?, ts: r.get(1)?, project_path: r.get(2)?, label: r.get(3)?,
                proc: r.get(4)?, command: r.get(5)?, exit_code: r.get(6)?, duration: r.get(7)?, output: r.get(8)?,
            })
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    }

    #[test]
    fn search_matches_command_and_output_newest_first() {
        let s = mem_store();
        add(&s, 100, "/a", "pnpm build", "compiled ok");
        add(&s, 200, "/a", "cargo test", "notarization hung waiting");
        add(&s, 150, "/b", "git push", "everything up-to-date");

        // newest-first
        let all = query(&s, None, None);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].ts, 200);

        // matches output text
        let hung = query(&s, Some("notarization"), None);
        assert_eq!(hung.len(), 1);
        assert_eq!(hung[0].command.as_deref(), Some("cargo test"));

        // matches command text
        let build = query(&s, Some("build"), None);
        assert_eq!(build.len(), 1);

        // project scope
        let proj_a = query(&s, None, Some("/a"));
        assert_eq!(proj_a.len(), 2);
    }
}
