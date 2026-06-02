// Lightweight database browser backend (phase 1: MySQL + SQLite).
//
// Conductor stays secret-free: connection details are read from the project's
// existing `.env`, or supplied per-session by the user. Nothing new is persisted.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DbConfig {
    /// "mysql" | "sqlite"
    pub engine: String,
    #[serde(default)]
    pub host: String,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    /// For sqlite: absolute path to the .db file.
    #[serde(default)]
    pub path: String,
    /// Friendly label (e.g. project name).
    #[serde(default)]
    pub label: String,
}

enum Conn {
    Mysql(mysql::Pool),
    Sqlite(String), // file path; opened per query (cheap, avoids !Sync issues)
}

#[derive(Default)]
pub struct DbManager {
    conns: Mutex<HashMap<String, Conn>>,
    seq: Mutex<u64>,
}

#[derive(Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub rows_affected: Option<u64>,
    pub elapsed_ms: u64,
    pub is_select: bool,
    pub truncated: bool,
}

// ── .env reading ───────────────────────────────────────────────

fn parse_env(path: &Path) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Ok(txt) = std::fs::read_to_string(path) else {
        return out;
    };
    for line in txt.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let mut v = v.trim().to_string();
        // Strip surrounding quotes.
        if (v.starts_with('"') && v.ends_with('"') && v.len() >= 2)
            || (v.starts_with('\'') && v.ends_with('\'') && v.len() >= 2)
        {
            v = v[1..v.len() - 1].to_string();
        }
        out.insert(k.trim().to_string(), v);
    }
    out
}

/// Build a connection config from a project's `.env` (Laravel conventions).
/// Returns None if the file has no usable DB_CONNECTION we support.
#[tauri::command]
pub fn db_from_env(project: String) -> Option<DbConfig> {
    let env = parse_env(&Path::new(&project).join(".env"));
    let engine = env
        .get("DB_CONNECTION")
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    let label = Path::new(&project)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();

    match engine.as_str() {
        "mysql" | "mariadb" => Some(DbConfig {
            engine: "mysql".into(),
            host: env.get("DB_HOST").cloned().unwrap_or_else(|| "127.0.0.1".into()),
            port: env
                .get("DB_PORT")
                .and_then(|p| p.parse().ok())
                .unwrap_or(3306),
            database: env.get("DB_DATABASE").cloned().unwrap_or_default(),
            username: env.get("DB_USERNAME").cloned().unwrap_or_default(),
            password: env.get("DB_PASSWORD").cloned().unwrap_or_default(),
            path: String::new(),
            label,
        }),
        "sqlite" => {
            let raw = env.get("DB_DATABASE").cloned().unwrap_or_default();
            let path = if raw.is_empty() {
                Path::new(&project)
                    .join("database/database.sqlite")
                    .to_string_lossy()
                    .to_string()
            } else if Path::new(&raw).is_absolute() {
                raw
            } else {
                Path::new(&project).join(&raw).to_string_lossy().to_string()
            };
            Some(DbConfig {
                engine: "sqlite".into(),
                path,
                label,
                ..Default::default()
            })
        }
        _ => None,
    }
}

// ── connect / disconnect ───────────────────────────────────────

#[tauri::command]
pub fn db_connect(state: State<DbManager>, config: DbConfig) -> Result<String, String> {
    let conn = match config.engine.as_str() {
        "mysql" => {
            let opts = mysql::OptsBuilder::new()
                .ip_or_hostname(Some(config.host.clone()))
                .tcp_port(config.port)
                .user(Some(config.username.clone()))
                .pass(Some(config.password.clone()))
                .db_name(if config.database.is_empty() {
                    None
                } else {
                    Some(config.database.clone())
                });
            let pool = mysql::Pool::new(opts).map_err(|e| e.to_string())?;
            // Validate the connection eagerly so errors surface now.
            let _ = pool.get_conn().map_err(|e| e.to_string())?;
            Conn::Mysql(pool)
        }
        "sqlite" => {
            if !Path::new(&config.path).exists() {
                return Err(format!("SQLite file not found: {}", config.path));
            }
            // Validate it opens.
            rusqlite::Connection::open(&config.path).map_err(|e| e.to_string())?;
            Conn::Sqlite(config.path.clone())
        }
        other => return Err(format!("Unsupported engine: {other}")),
    };

    let mut seq = state.seq.lock().unwrap();
    *seq += 1;
    let id = format!("db-{}", *seq);
    drop(seq);
    state.conns.lock().unwrap().insert(id.clone(), conn);
    Ok(id)
}

#[tauri::command]
pub fn db_disconnect(state: State<DbManager>, id: String) {
    state.conns.lock().unwrap().remove(&id);
}

// ── schema ─────────────────────────────────────────────────────

#[tauri::command]
pub fn db_tables(state: State<DbManager>, id: String) -> Result<Vec<String>, String> {
    let conns = state.conns.lock().unwrap();
    let conn = conns.get(&id).ok_or("Connection not found")?;
    match conn {
        Conn::Mysql(pool) => {
            let mut c = pool.get_conn().map_err(|e| e.to_string())?;
            use mysql::prelude::Queryable;
            let tables: Vec<String> = c.query("SHOW TABLES").map_err(|e| e.to_string())?;
            Ok(tables)
        }
        Conn::Sqlite(path) => {
            let c = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
            let mut stmt = c
                .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    }
}

#[derive(Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub key: String,
}

#[tauri::command]
pub fn db_columns(
    state: State<DbManager>,
    id: String,
    table: String,
) -> Result<Vec<ColumnInfo>, String> {
    let conns = state.conns.lock().unwrap();
    let conn = conns.get(&id).ok_or("Connection not found")?;
    match conn {
        Conn::Mysql(pool) => {
            let mut c = pool.get_conn().map_err(|e| e.to_string())?;
            use mysql::prelude::Queryable;
            let q = format!("SHOW COLUMNS FROM `{}`", table.replace('`', "``"));
            let rows: Vec<(String, String, String, String, Option<String>, String)> =
                c.query(q).map_err(|e| e.to_string())?;
            Ok(rows
                .into_iter()
                .map(|(field, ty, null, key, _default, _extra)| ColumnInfo {
                    name: field,
                    data_type: ty,
                    nullable: null.eq_ignore_ascii_case("YES"),
                    key,
                })
                .collect())
        }
        Conn::Sqlite(path) => {
            let c = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
            let q = format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\""));
            let mut stmt = c.prepare(&q).map_err(|e| e.to_string())?;
            // cid, name, type, notnull, dflt_value, pk
            let rows = stmt
                .query_map([], |r| {
                    Ok(ColumnInfo {
                        name: r.get::<_, String>(1)?,
                        data_type: r.get::<_, String>(2).unwrap_or_default(),
                        nullable: r.get::<_, i64>(3).unwrap_or(0) == 0,
                        key: if r.get::<_, i64>(5).unwrap_or(0) > 0 {
                            "PRI".into()
                        } else {
                            String::new()
                        },
                    })
                })
                .map_err(|e| e.to_string())?;
            Ok(rows.filter_map(|r| r.ok()).collect())
        }
    }
}

// ── query ──────────────────────────────────────────────────────

fn mysql_value_to_string(v: &mysql::Value) -> Option<String> {
    use mysql::Value::*;
    match v {
        NULL => None,
        Bytes(b) => Some(String::from_utf8_lossy(b).to_string()),
        Int(i) => Some(i.to_string()),
        UInt(u) => Some(u.to_string()),
        Float(f) => Some(f.to_string()),
        Double(d) => Some(d.to_string()),
        Date(y, mo, d, h, mi, s, _us) => {
            Some(format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}"))
        }
        Time(neg, days, h, mi, s, _us) => Some(format!(
            "{}{}:{mi:02}:{s:02}",
            if *neg { "-" } else { "" },
            *h as u32 + days * 24
        )),
    }
}

fn is_select(sql: &str) -> bool {
    let s = sql.trim_start().to_lowercase();
    s.starts_with("select") || s.starts_with("show") || s.starts_with("pragma") || s.starts_with("explain") || s.starts_with("describe") || s.starts_with("desc ") || s.starts_with("with")
}

const MAX_ROWS: usize = 1000;

#[tauri::command]
pub fn db_query(state: State<DbManager>, id: String, sql: String) -> Result<QueryResult, String> {
    let conns = state.conns.lock().unwrap();
    let conn = conns.get(&id).ok_or("Connection not found")?;
    let select = is_select(&sql);
    let start = Instant::now();

    match conn {
        Conn::Mysql(pool) => {
            let mut c = pool.get_conn().map_err(|e| e.to_string())?;
            use mysql::prelude::Queryable;
            if select {
                let result = c.query_iter(&sql).map_err(|e| e.to_string())?;
                let columns: Vec<String> =
                    result.columns().as_ref().iter().map(|c| c.name_str().to_string()).collect();
                let mut rows = Vec::new();
                let mut truncated = false;
                for row in result {
                    let row = row.map_err(|e| e.to_string())?;
                    if rows.len() >= MAX_ROWS {
                        truncated = true;
                        break;
                    }
                    let vals: Vec<Option<String>> = (0..columns.len())
                        .map(|i| row.as_ref(i).and_then(mysql_value_to_string))
                        .collect();
                    rows.push(vals);
                }
                Ok(QueryResult {
                    columns,
                    rows,
                    rows_affected: None,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    is_select: true,
                    truncated,
                })
            } else {
                c.query_drop(&sql).map_err(|e| e.to_string())?;
                Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected: Some(c.affected_rows()),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    is_select: false,
                    truncated: false,
                })
            }
        }
        Conn::Sqlite(path) => {
            let c = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
            if select {
                let mut stmt = c.prepare(&sql).map_err(|e| e.to_string())?;
                let columns: Vec<String> =
                    stmt.column_names().iter().map(|s| s.to_string()).collect();
                let ncols = columns.len();
                let mut rows = Vec::new();
                let mut truncated = false;
                let mut q = stmt.query([]).map_err(|e| e.to_string())?;
                while let Some(row) = q.next().map_err(|e| e.to_string())? {
                    if rows.len() >= MAX_ROWS {
                        truncated = true;
                        break;
                    }
                    let mut vals = Vec::with_capacity(ncols);
                    for i in 0..ncols {
                        use rusqlite::types::ValueRef::*;
                        let v = match row.get_ref(i).map_err(|e| e.to_string())? {
                            Null => None,
                            Integer(n) => Some(n.to_string()),
                            Real(f) => Some(f.to_string()),
                            Text(t) => Some(String::from_utf8_lossy(t).to_string()),
                            Blob(b) => Some(format!("<{} bytes>", b.len())),
                        };
                        vals.push(v);
                    }
                    rows.push(vals);
                }
                Ok(QueryResult {
                    columns,
                    rows,
                    rows_affected: None,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    is_select: true,
                    truncated,
                })
            } else {
                let affected = c.execute(&sql, []).map_err(|e| e.to_string())?;
                Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected: Some(affected as u64),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    is_select: false,
                    truncated: false,
                })
            }
        }
    }
}
