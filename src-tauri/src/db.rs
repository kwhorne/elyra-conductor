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
    /// Connect over TLS (Postgres / ClickHouse).
    #[serde(default)]
    pub tls: bool,
    /// Skip certificate verification (self-signed / internal hosts).
    #[serde(default)]
    pub tls_insecure: bool,
}

enum Conn {
    Mysql(mysql::Pool),
    Sqlite(String), // file path; opened per query (cheap, avoids !Sync issues)
    Postgres(Mutex<postgres::Client>),
    Clickhouse(klickhouse::Client), // native TCP protocol (port 9000)
}

// A dynamic ClickHouse row: column name + value, no compile-time struct needed.
// klickhouse's Row trait hands us (name, type, value) per cell.
struct ChRow {
    cells: Vec<(String, klickhouse::Value)>,
}
impl klickhouse::Row for ChRow {
    const COLUMN_COUNT: Option<usize> = None;
    fn column_names() -> Option<Vec<std::borrow::Cow<'static, str>>> {
        None
    }
    fn deserialize_row(
        map: Vec<(&str, &klickhouse::Type, klickhouse::Value)>,
    ) -> klickhouse::Result<Self> {
        Ok(ChRow {
            cells: map.into_iter().map(|(n, _t, v)| (n.to_string(), v)).collect(),
        })
    }
    fn serialize_row(
        self,
        _hints: &klickhouse::IndexMap<String, klickhouse::Type>,
    ) -> klickhouse::Result<Vec<(std::borrow::Cow<'static, str>, klickhouse::Value)>> {
        Ok(vec![])
    }
}
// klickhouse's Value implements Display with ClickHouse-style formatting, so any
// value (incl. Decimal/Date/Array/Tuple/UUID) renders to text trivially.
fn ch_value_to_string(v: &klickhouse::Value) -> Option<String> {
    match v {
        klickhouse::Value::Null => None,
        other => Some(other.to_string()),
    }
}

// Run a query via Postgres' simple protocol, which returns every value as text
// (Option<&str>) — ideal for a generic browser (no per-type decoding needed).
fn pg_query(
    client: &mut postgres::Client,
    sql: &str,
) -> Result<(Vec<String>, Vec<Vec<Option<String>>>, Option<u64>, bool), String> {
    use postgres::SimpleQueryMessage::*;
    let msgs = client.simple_query(sql).map_err(|e| e.to_string())?;
    let mut columns: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Option<String>>> = Vec::new();
    let mut affected = None;
    let mut truncated = false;
    for m in msgs {
        match m {
            RowDescription(cols) => {
                columns = cols.iter().map(|c| c.name().to_string()).collect();
            }
            Row(row) => {
                if columns.is_empty() {
                    columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                }
                if rows.len() >= MAX_ROWS {
                    truncated = true;
                    continue;
                }
                let vals = (0..row.len()).map(|i| row.get(i).map(|s| s.to_string())).collect();
                rows.push(vals);
            }
            CommandComplete(n) => affected = Some(n),
            _ => {}
        }
    }
    Ok((columns, rows, affected, truncated))
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
            tls: false,
            tls_insecure: false,
            database: env.get("DB_DATABASE").cloned().unwrap_or_default(),
            username: env.get("DB_USERNAME").cloned().unwrap_or_default(),
            password: env.get("DB_PASSWORD").cloned().unwrap_or_default(),
            path: String::new(),
            label,
        }),
        "pgsql" | "postgres" | "postgresql" => Some(DbConfig {
            engine: "postgres".into(),
            host: env.get("DB_HOST").cloned().unwrap_or_else(|| "127.0.0.1".into()),
            port: env.get("DB_PORT").and_then(|p| p.parse().ok()).unwrap_or(5432),
            tls: false,
            tls_insecure: false,
            database: env.get("DB_DATABASE").cloned().unwrap_or_default(),
            username: env.get("DB_USERNAME").cloned().unwrap_or_default(),
            password: env.get("DB_PASSWORD").cloned().unwrap_or_default(),
            path: String::new(),
            label,
        }),
        "clickhouse" => Some(DbConfig {
            engine: "clickhouse".into(),
            host: env.get("DB_HOST").cloned().unwrap_or_else(|| "127.0.0.1".into()),
            port: env.get("DB_PORT").and_then(|p| p.parse().ok()).unwrap_or(9000),
            tls: false,
            tls_insecure: false,
            database: env.get("DB_DATABASE").cloned().unwrap_or_default(),
            username: env.get("DB_USERNAME").cloned().unwrap_or_else(|| "default".into()),
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

// ── saved connections (per project, in the OS keychain) ──
// A project can have several databases (e.g. MySQL for app data + ClickHouse for
// BI). The whole connection list (including passwords) is stored in the OS
// keychain (macOS Keychain), keyed by project path — nothing is written into the
// project folder, so nothing can be committed, and secrets live in secure storage.
const KEYCHAIN_SERVICE: &str = "com.elyra.conductor.db-connections";

#[tauri::command]
pub fn list_connections(project: String) -> Result<Vec<DbConfig>, String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &project).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(s) => Ok(serde_json::from_str(&s).unwrap_or_default()),
        Err(keyring::Error::NoEntry) => Ok(Vec::new()),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn save_connections(project: String, connections: Vec<DbConfig>) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, &project).map_err(|e| e.to_string())?;
    if connections.is_empty() {
        let _ = entry.delete_credential();
        return Ok(());
    }
    let json = serde_json::to_string(&connections).map_err(|e| e.to_string())?;
    entry.set_password(&json).map_err(|e| e.to_string())
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
        "postgres" | "postgresql" | "pgsql" => {
            let mut pg = postgres::Config::new();
            pg.host(if config.host.is_empty() { "127.0.0.1" } else { &config.host })
                .port(if config.port == 0 { 5432 } else { config.port })
                .user(&config.username);
            if !config.password.is_empty() {
                pg.password(&config.password);
            }
            if !config.database.is_empty() {
                pg.dbname(&config.database);
            }
            if config.tls {
                let connector = native_tls::TlsConnector::builder()
                    .danger_accept_invalid_certs(config.tls_insecure)
                    .danger_accept_invalid_hostnames(config.tls_insecure)
                    .build()
                    .map_err(|e| e.to_string())?;
                let tls = postgres_native_tls::MakeTlsConnector::new(connector);
                let client = pg.connect(tls).map_err(|e| e.to_string())?;
                Conn::Postgres(Mutex::new(client))
            } else {
                let client = pg.connect(postgres::NoTls).map_err(|e| e.to_string())?;
                Conn::Postgres(Mutex::new(client))
            }
        }
        "clickhouse" | "ch" => {
            let host = if config.host.is_empty() { "127.0.0.1".to_string() } else { config.host.clone() };
            let port = if config.port == 0 { 9000 } else { config.port };
            let opts = klickhouse::ClientOptions {
                username: if config.username.is_empty() { "default".into() } else { config.username.clone() },
                password: config.password.clone(),
                default_database: config.database.clone(),
                ..Default::default()
            };
            let addr = format!("{host}:{port}");
            let client = if config.tls {
                // Build the TLS stream ourselves with native-tls (macOS Secure
                // Transport) and hand the split halves to connect_stream — avoids
                // pulling rustls/aws-lc-rs (which needs cmake).
                let insecure = config.tls_insecure;
                let host_cloned = host.clone();
                tauri::async_runtime::block_on(async move {
                    use tokio::io::AsyncWriteExt;
                    let connector = native_tls::TlsConnector::builder()
                        .danger_accept_invalid_certs(insecure)
                        .danger_accept_invalid_hostnames(insecure)
                        .build()
                        .map_err(|e| e.to_string())?;
                    let connector = tokio_native_tls::TlsConnector::from(connector);
                    let tcp = tokio::net::TcpStream::connect(&addr).await.map_err(|e| e.to_string())?;
                    let _ = tcp.set_nodelay(true);
                    let tls = connector.connect(&host_cloned, tcp).await.map_err(|e| e.to_string())?;
                    let (read, mut writer) = tokio::io::split(tls);
                    let _ = writer.flush().await;
                    klickhouse::Client::connect_stream(read, writer, opts)
                        .await
                        .map_err(|e| e.to_string())
                })?
            } else {
                tauri::async_runtime::block_on(klickhouse::Client::connect(addr, opts))
                    .map_err(|e| e.to_string())?
            };
            // Validate eagerly.
            tauri::async_runtime::block_on(client.execute("SELECT 1")).map_err(|e| e.to_string())?;
            Conn::Clickhouse(client)
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
        Conn::Postgres(m) => {
            let mut client = m.lock().unwrap();
            let (_c, rows, _a, _t) = pg_query(
                &mut client,
                "SELECT tablename FROM pg_catalog.pg_tables WHERE schemaname NOT IN ('pg_catalog','information_schema') ORDER BY tablename",
            )?;
            Ok(rows.into_iter().filter_map(|r| r.into_iter().next().flatten()).collect())
        }
        Conn::Clickhouse(client) => {
            let rows: Vec<ChRow> = tauri::async_runtime::block_on(client.query_collect::<ChRow>("SHOW TABLES"))
                .map_err(|e| e.to_string())?;
            Ok(rows
                .into_iter()
                .filter_map(|r| r.cells.into_iter().next().and_then(|(_, v)| ch_value_to_string(&v)))
                .collect())
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
        Conn::Postgres(m) => {
            let mut client = m.lock().unwrap();
            let t = table.replace('\'', "''");
            // Primary-key columns for this table.
            let (_pc, pk_rows, _a, _tr) = pg_query(
                &mut client,
                &format!(
                    "SELECT kcu.column_name FROM information_schema.table_constraints tc \
                     JOIN information_schema.key_column_usage kcu \
                       ON kcu.constraint_name = tc.constraint_name AND kcu.table_schema = tc.table_schema \
                     WHERE tc.table_name = '{t}' AND tc.constraint_type = 'PRIMARY KEY'"
                ),
            )?;
            let pks: std::collections::HashSet<String> = pk_rows
                .into_iter()
                .filter_map(|r| r.into_iter().next().flatten())
                .collect();
            let (_cc, rows, _a2, _t2) = pg_query(
                &mut client,
                &format!(
                    "SELECT column_name, data_type, is_nullable FROM information_schema.columns \
                     WHERE table_name = '{t}' ORDER BY ordinal_position"
                ),
            )?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let name = r.first().cloned().flatten().unwrap_or_default();
                    let data_type = r.get(1).cloned().flatten().unwrap_or_default();
                    let nullable = r.get(2).cloned().flatten().unwrap_or_default().eq_ignore_ascii_case("YES");
                    let key = if pks.contains(&name) { "PRI".to_string() } else { String::new() };
                    ColumnInfo { name, data_type, nullable, key }
                })
                .collect())
        }
        Conn::Clickhouse(client) => {
            let q = format!("DESCRIBE TABLE `{}`", table.replace('`', "``"));
            let rows: Vec<ChRow> = tauri::async_runtime::block_on(client.query_collect::<ChRow>(q))
                .map_err(|e| e.to_string())?;
            // DESCRIBE columns: name, type, default_type, default_expression, ...
            Ok(rows
                .into_iter()
                .map(|r| {
                    let v: Vec<Option<String>> = r.cells.iter().map(|(_, x)| ch_value_to_string(x)).collect();
                    let name = v.first().cloned().flatten().unwrap_or_default();
                    let data_type = v.get(1).cloned().flatten().unwrap_or_default();
                    let nullable = data_type.starts_with("Nullable(");
                    ColumnInfo { name, data_type, nullable, key: String::new() }
                })
                .collect())
        }
    }
}

// ── table metadata (approx row count + size) ──

#[derive(Serialize)]
pub struct TableInfo {
    pub rows: Option<i64>,
    pub bytes: Option<i64>,
    pub approximate: bool,
}

#[tauri::command]
pub fn db_table_info(state: State<DbManager>, id: String, table: String) -> Result<TableInfo, String> {
    let conns = state.conns.lock().unwrap();
    let conn = conns.get(&id).ok_or("Connection not found")?;
    match conn {
        Conn::Mysql(pool) => {
            let mut c = pool.get_conn().map_err(|e| e.to_string())?;
            use mysql::prelude::Queryable;
            let t = table.replace('\'', "''");
            let row: Option<(Option<i64>, Option<i64>)> = c
                .query_first(format!(
                    "SELECT table_rows, data_length + index_length FROM information_schema.tables \
                     WHERE table_schema = DATABASE() AND table_name = '{t}'"
                ))
                .map_err(|e| e.to_string())?;
            let (rows, bytes) = row.unwrap_or((None, None));
            Ok(TableInfo { rows, bytes, approximate: true })
        }
        Conn::Sqlite(path) => {
            let c = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
            let q = format!("SELECT COUNT(*) FROM \"{}\"", table.replace('"', "\"\""));
            let cnt: i64 = c.query_row(&q, [], |r| r.get(0)).unwrap_or(-1);
            Ok(TableInfo { rows: Some(cnt).filter(|n| *n >= 0), bytes: None, approximate: false })
        }
        Conn::Postgres(m) => {
            let mut client = m.lock().unwrap();
            let t = table.replace('\'', "''");
            let (_c, rows_data, _a, _tr) = pg_query(
                &mut client,
                &format!(
                    "SELECT c.reltuples::bigint, pg_total_relation_size(c.oid) FROM pg_class c \
                     JOIN pg_namespace n ON n.oid = c.relnamespace \
                     WHERE c.relname = '{t}' AND n.nspname NOT IN ('pg_catalog','information_schema') \
                     ORDER BY (n.nspname = 'public') DESC LIMIT 1"
                ),
            )?;
            let first = rows_data.into_iter().next().unwrap_or_default();
            let rows = first.first().cloned().flatten().and_then(|s| s.parse::<i64>().ok()).filter(|n| *n >= 0);
            let bytes = first.get(1).cloned().flatten().and_then(|s| s.parse::<i64>().ok());
            Ok(TableInfo { rows, bytes, approximate: true })
        }
        Conn::Clickhouse(client) => {
            let t = table.replace('\'', "''");
            let rows_ch: Vec<ChRow> = tauri::async_runtime::block_on(client.query_collect::<ChRow>(
                format!("SELECT total_rows, total_bytes FROM system.tables WHERE database = currentDatabase() AND name = '{t}'"),
            ))
            .map_err(|e| e.to_string())?;
            let (rows, bytes) = match rows_ch.into_iter().next() {
                Some(r) => {
                    let v: Vec<Option<String>> = r.cells.iter().map(|(_, x)| ch_value_to_string(x)).collect();
                    (
                        v.first().cloned().flatten().and_then(|s| s.parse().ok()),
                        v.get(1).cloned().flatten().and_then(|s| s.parse().ok()),
                    )
                }
                None => (None, None),
            };
            Ok(TableInfo { rows, bytes, approximate: true })
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
        Conn::Postgres(m) => {
            let mut client = m.lock().unwrap();
            let (columns, rows, affected, truncated) = pg_query(&mut client, &sql)?;
            let elapsed_ms = start.elapsed().as_millis() as u64;
            if select {
                Ok(QueryResult { columns, rows, rows_affected: None, elapsed_ms, is_select: true, truncated })
            } else {
                Ok(QueryResult { columns: vec![], rows: vec![], rows_affected: affected, elapsed_ms, is_select: false, truncated: false })
            }
        }
        Conn::Clickhouse(client) => {
            if select {
                let chrows: Vec<ChRow> =
                    tauri::async_runtime::block_on(client.query_collect::<ChRow>(sql.clone()))
                        .map_err(|e| e.to_string())?;
                let columns: Vec<String> = chrows
                    .first()
                    .map(|r| r.cells.iter().map(|(n, _)| n.clone()).collect())
                    .unwrap_or_default();
                let mut rows = Vec::new();
                let mut truncated = false;
                for r in chrows {
                    if rows.len() >= MAX_ROWS {
                        truncated = true;
                        break;
                    }
                    rows.push(r.cells.iter().map(|(_, v)| ch_value_to_string(v)).collect());
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
                tauri::async_runtime::block_on(client.execute(sql.clone())).map_err(|e| e.to_string())?;
                Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected: None,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    is_select: false,
                    truncated: false,
                })
            }
        }
    }
}
