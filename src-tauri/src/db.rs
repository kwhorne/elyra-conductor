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
    /// For SQL Anywhere / libsql: remote server URL (libsql:// or https://).
    #[serde(default)]
    pub url: String,
    /// For SQL Anywhere / libsql: auth token (sent as a bearer token).
    #[serde(default)]
    pub token: String,
    /// Friendly label (e.g. project name).
    #[serde(default)]
    pub label: String,
    /// Connect over TLS (Postgres / ClickHouse).
    #[serde(default)]
    pub tls: bool,
    /// Skip certificate verification (self-signed / internal hosts).
    #[serde(default)]
    pub tls_insecure: bool,
    /// Optional group/folder name for organising connections in the panel.
    #[serde(default)]
    pub group: String,
    /// Tunnel the DB connection through SSH (remote databases).
    #[serde(default)]
    pub use_ssh: bool,
    #[serde(default)]
    pub ssh_host: String,
    #[serde(default)]
    pub ssh_port: u16,
    #[serde(default)]
    pub ssh_user: String,
    /// "key" | "password"
    #[serde(default)]
    pub ssh_auth: String,
    #[serde(default)]
    pub ssh_password: String,
    #[serde(default)]
    pub ssh_key_path: String,
    #[serde(default)]
    pub ssh_passphrase: String,
}

/// Columns, rows (each cell text-or-null), rows-affected, and a truncated flag.
type QueryRows = (Vec<String>, Vec<Vec<Option<String>>>, Option<u64>, bool);

// A handful of connections live in the registry; the size difference between
// variants is irrelevant here.
#[allow(clippy::large_enum_variant)]
enum Conn {
    Mysql(mysql::Pool),
    Sqlite(String), // file path; opened per query (cheap, avoids !Sync issues)
    Postgres(Mutex<postgres::Client>),
    Clickhouse(klickhouse::Client), // native TCP protocol (port 9000)
    SqlAnywhere(libsql::Database), // remote SQL Anywhere / libsql (sqld) over HTTP
}

// SQL Anywhere is SQLite-compatible, so its values map like SQLite's.
fn libsql_value_to_string(v: &libsql::Value) -> Option<String> {
    match v {
        libsql::Value::Null => None,
        libsql::Value::Integer(n) => Some(n.to_string()),
        libsql::Value::Real(f) => Some(f.to_string()),
        libsql::Value::Text(t) => Some(t.clone()),
        libsql::Value::Blob(b) => Some(format!("<{} bytes>", b.len())),
    }
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
            cells: map
                .into_iter()
                .map(|(n, _t, v)| (n.to_string(), v))
                .collect(),
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
fn pg_query(client: &mut postgres::Client, sql: &str, max_rows: usize) -> Result<QueryRows, String> {
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
                if rows.len() >= max_rows {
                    truncated = true;
                    continue;
                }
                let vals = (0..row.len())
                    .map(|i| row.get(i).map(|s| s.to_string()))
                    .collect();
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
    tunnels: Mutex<HashMap<String, SshTunnel>>,
    seq: Mutex<u64>,
}

// ── SSH tunnel (remote databases) ──────────────────────────────
// We shell out to the system `ssh` with a local port-forward. It handles both
// key and password auth and multiple concurrent forwards cleanly. Conductor is
// a GUI app with no controlling tty, so ssh uses SSH_ASKPASS for secrets.
struct SshTunnel {
    child: std::process::Child,
    local_port: u16,
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn engine_default_port(engine: &str) -> u16 {
    match engine {
        "mysql" => 3306,
        "postgres" | "postgresql" | "pgsql" => 5432,
        "clickhouse" | "ch" => 9000,
        _ => 0,
    }
}

fn free_local_port() -> Result<u16, String> {
    let l = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let p = l.local_addr().map_err(|e| e.to_string())?.port();
    Ok(p)
}

/// Write a temporary SSH_ASKPASS helper that echoes the secret. Mode 0700.
fn write_askpass(secret: &str) -> Result<std::path::PathBuf, String> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    let mut path = std::env::temp_dir();
    path.push(format!("ec-askpass-{}-{}.sh", std::process::id(), rand_suffix()));
    let escaped = secret.replace('\'', "'\\''");
    let script = format!("#!/bin/sh\nprintf '%s\\n' '{escaped}'\n");
    let mut f = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    f.write_all(script.as_bytes()).map_err(|e| e.to_string())?;
    let perms = std::fs::Permissions::from_mode(0o700);
    std::fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
    Ok(path)
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

fn rand_suffix() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

fn wait_port(
    port: u16,
    child: &mut std::process::Child,
    timeout: std::time::Duration,
) -> Result<(), String> {
    use std::io::Read;
    let start = std::time::Instant::now();
    loop {
        if let Some(_status) = child.try_wait().map_err(|e| e.to_string())? {
            let mut err = String::new();
            if let Some(mut s) = child.stderr.take() {
                let _ = s.read_to_string(&mut err);
            }
            let msg = err.trim();
            return Err(if msg.is_empty() {
                "SSH tunnel failed (ssh exited)".to_string()
            } else {
                format!("SSH tunnel failed: {msg}")
            });
        }
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        if start.elapsed() > timeout {
            return Err("SSH tunnel timed out (port did not open)".to_string());
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
}

fn start_tunnel(config: &DbConfig) -> Result<SshTunnel, String> {
    if config.ssh_host.trim().is_empty() {
        return Err("SSH host is required".to_string());
    }
    if config.ssh_user.trim().is_empty() {
        return Err("SSH user is required".to_string());
    }
    let local_port = free_local_port()?;
    let db_host = if config.host.is_empty() {
        "127.0.0.1".to_string()
    } else {
        config.host.clone()
    };
    let db_port = if config.port == 0 {
        engine_default_port(&config.engine)
    } else {
        config.port
    };
    let ssh_port = if config.ssh_port == 0 { 22 } else { config.ssh_port };

    let mut cmd = std::process::Command::new("ssh");
    cmd.arg("-N")
        .args(["-o", "ExitOnForwardFailure=yes"])
        .args(["-o", "StrictHostKeyChecking=accept-new"])
        .args(["-o", "ConnectTimeout=10"])
        .args(["-o", "ServerAliveInterval=30"])
        .args(["-o", "ServerAliveCountMax=3"])
        .args(["-o", "NumberOfPasswordPrompts=1"])
        .args(["-p", &ssh_port.to_string()])
        .args(["-L", &format!("127.0.0.1:{local_port}:{db_host}:{db_port}")]);

    let password_auth = config.ssh_auth == "password";
    let secret = if password_auth {
        config.ssh_password.clone()
    } else {
        config.ssh_passphrase.clone()
    };

    if password_auth {
        cmd.args(["-o", "PubkeyAuthentication=no"]);
        cmd.args(["-o", "PreferredAuthentications=password,keyboard-interactive"]);
    } else if !config.ssh_key_path.trim().is_empty() {
        cmd.args(["-i", &expand_tilde(config.ssh_key_path.trim())]);
        cmd.args(["-o", "IdentitiesOnly=yes"]);
    }

    let mut askpass: Option<std::path::PathBuf> = None;
    if secret.is_empty() {
        cmd.args(["-o", "BatchMode=yes"]);
    } else {
        let p = write_askpass(&secret)?;
        cmd.env("SSH_ASKPASS", &p);
        cmd.env("SSH_ASKPASS_REQUIRE", "force");
        cmd.env("DISPLAY", "localhost:0");
        askpass = Some(p);
    }

    cmd.arg(format!("{}@{}", config.ssh_user, config.ssh_host));
    cmd.stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("failed to start ssh: {e}"))?;
    let res = wait_port(local_port, &mut child, std::time::Duration::from_secs(15));
    // Remove the askpass helper as soon as auth is done (success or fail).
    if let Some(p) = &askpass {
        let _ = std::fs::remove_file(p);
    }
    res?;
    Ok(SshTunnel { child, local_port })
}

/// Resolve a config to what `open_conn` should actually connect to. When SSH is
/// enabled, start a tunnel and point the connection at the local forwarded port.
fn prepare(config: &DbConfig) -> Result<(DbConfig, Option<SshTunnel>), String> {
    if config.use_ssh && config.engine != "sqlite" {
        let tunnel = start_tunnel(config)?;
        let mut eff = config.clone();
        eff.host = "127.0.0.1".to_string();
        eff.port = tunnel.local_port;
        // The SSH tunnel is the transport encryption; TLS to 127.0.0.1 would
        // fail hostname verification, so disable it for tunneled connections.
        eff.tls = false;
        Ok((eff, Some(tunnel)))
    } else {
        Ok((config.clone(), None))
    }
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
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
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
            host: env
                .get("DB_HOST")
                .cloned()
                .unwrap_or_else(|| "127.0.0.1".into()),
            port: env
                .get("DB_PORT")
                .and_then(|p| p.parse().ok())
                .unwrap_or(3306),
            tls: false,
            tls_insecure: false,
            group: String::new(),
            database: env.get("DB_DATABASE").cloned().unwrap_or_default(),
            username: env.get("DB_USERNAME").cloned().unwrap_or_default(),
            password: env.get("DB_PASSWORD").cloned().unwrap_or_default(),
            path: String::new(),
            label,
            ..Default::default()
        }),
        "pgsql" | "postgres" | "postgresql" => Some(DbConfig {
            engine: "postgres".into(),
            host: env
                .get("DB_HOST")
                .cloned()
                .unwrap_or_else(|| "127.0.0.1".into()),
            port: env
                .get("DB_PORT")
                .and_then(|p| p.parse().ok())
                .unwrap_or(5432),
            tls: false,
            tls_insecure: false,
            group: String::new(),
            database: env.get("DB_DATABASE").cloned().unwrap_or_default(),
            username: env.get("DB_USERNAME").cloned().unwrap_or_default(),
            password: env.get("DB_PASSWORD").cloned().unwrap_or_default(),
            path: String::new(),
            label,
            ..Default::default()
        }),
        "clickhouse" => Some(DbConfig {
            engine: "clickhouse".into(),
            host: env
                .get("DB_HOST")
                .cloned()
                .unwrap_or_else(|| "127.0.0.1".into()),
            port: env
                .get("DB_PORT")
                .and_then(|p| p.parse().ok())
                .unwrap_or(9000),
            tls: false,
            tls_insecure: false,
            group: String::new(),
            database: env.get("DB_DATABASE").cloned().unwrap_or_default(),
            username: env
                .get("DB_USERNAME")
                .cloned()
                .unwrap_or_else(|| "default".into()),
            password: env.get("DB_PASSWORD").cloned().unwrap_or_default(),
            path: String::new(),
            label,
            ..Default::default()
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

fn open_conn(config: &DbConfig) -> Result<Conn, String> {
    Ok(match config.engine.as_str() {
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
            pg.host(if config.host.is_empty() {
                "127.0.0.1"
            } else {
                &config.host
            })
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
            let host = if config.host.is_empty() {
                "127.0.0.1".to_string()
            } else {
                config.host.clone()
            };
            let port = if config.port == 0 { 9000 } else { config.port };
            let opts = klickhouse::ClientOptions {
                username: if config.username.is_empty() {
                    "default".into()
                } else {
                    config.username.clone()
                },
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
                    let tcp = tokio::net::TcpStream::connect(&addr)
                        .await
                        .map_err(|e| e.to_string())?;
                    let _ = tcp.set_nodelay(true);
                    let tls = connector
                        .connect(&host_cloned, tcp)
                        .await
                        .map_err(|e| e.to_string())?;
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
            tauri::async_runtime::block_on(client.execute("SELECT 1"))
                .map_err(|e| e.to_string())?;
            Conn::Clickhouse(client)
        }
        "sqlanywhere" | "libsql" | "turso" => {
            let url = config.url.trim().to_string();
            if url.is_empty() {
                return Err("SQL Anywhere URL is required (libsql:// or https://)".into());
            }
            let token = config.token.clone();
            let db = tauri::async_runtime::block_on(async move {
                let db = libsql::Builder::new_remote(url, token)
                    .build()
                    .await
                    .map_err(|e| e.to_string())?;
                // Validate eagerly so errors surface at connect time.
                let conn = db.connect().map_err(|e| e.to_string())?;
                conn.query("SELECT 1", ()).await.map_err(|e| e.to_string())?;
                Ok::<_, String>(db)
            })?;
            Conn::SqlAnywhere(db)
        }
        other => return Err(format!("Unsupported engine: {other}")),
    })
}

#[tauri::command]
pub fn db_connect(state: State<DbManager>, config: DbConfig) -> Result<String, String> {
    let (eff, tunnel) = prepare(&config)?;
    let conn = open_conn(&eff)?;
    let mut seq = state.seq.lock().unwrap();
    *seq += 1;
    let id = format!("db-{}", *seq);
    drop(seq);
    state.conns.lock().unwrap().insert(id.clone(), conn);
    if let Some(t) = tunnel {
        state.tunnels.lock().unwrap().insert(id.clone(), t);
    }
    Ok(id)
}

/// Try a connection without storing it (the "Test connection" button). The
/// tunnel (if any) is torn down when this returns.
#[tauri::command]
pub fn db_test(config: DbConfig) -> Result<(), String> {
    let (eff, _tunnel) = prepare(&config)?;
    open_conn(&eff)?;
    Ok(())
}

#[tauri::command]
pub fn db_disconnect(state: State<DbManager>, id: String) {
    state.conns.lock().unwrap().remove(&id);
    state.tunnels.lock().unwrap().remove(&id);
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
                MAX_ROWS,
            )?;
            Ok(rows
                .into_iter()
                .filter_map(|r| r.into_iter().next().flatten())
                .collect())
        }
        Conn::Clickhouse(client) => {
            let rows: Vec<ChRow> =
                tauri::async_runtime::block_on(client.query_collect::<ChRow>("SHOW TABLES"))
                    .map_err(|e| e.to_string())?;
            Ok(rows
                .into_iter()
                .filter_map(|r| {
                    r.cells
                        .into_iter()
                        .next()
                        .and_then(|(_, v)| ch_value_to_string(&v))
                })
                .collect())
        }
        Conn::SqlAnywhere(db) => tauri::async_runtime::block_on(async {
            let conn = db.connect().map_err(|e| e.to_string())?;
            let mut rows = conn
                .query(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
                    (),
                )
                .await
                .map_err(|e| e.to_string())?;
            let mut out = Vec::new();
            while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
                if let Some(s) = libsql_value_to_string(&row.get_value(0).map_err(|e| e.to_string())?) {
                    out.push(s);
                }
            }
            Ok(out)
        }),
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
                MAX_ROWS,
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
                MAX_ROWS,
            )?;
            Ok(rows
                .into_iter()
                .map(|r| {
                    let name = r.first().cloned().flatten().unwrap_or_default();
                    let data_type = r.get(1).cloned().flatten().unwrap_or_default();
                    let nullable = r
                        .get(2)
                        .cloned()
                        .flatten()
                        .unwrap_or_default()
                        .eq_ignore_ascii_case("YES");
                    let key = if pks.contains(&name) {
                        "PRI".to_string()
                    } else {
                        String::new()
                    };
                    ColumnInfo {
                        name,
                        data_type,
                        nullable,
                        key,
                    }
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
                    let v: Vec<Option<String>> =
                        r.cells.iter().map(|(_, x)| ch_value_to_string(x)).collect();
                    let name = v.first().cloned().flatten().unwrap_or_default();
                    let data_type = v.get(1).cloned().flatten().unwrap_or_default();
                    let nullable = data_type.starts_with("Nullable(");
                    ColumnInfo {
                        name,
                        data_type,
                        nullable,
                        key: String::new(),
                    }
                })
                .collect())
        }
        Conn::SqlAnywhere(db) => {
            let q = format!("PRAGMA table_info(\"{}\")", table.replace('"', "\"\""));
            tauri::async_runtime::block_on(async {
                let conn = db.connect().map_err(|e| e.to_string())?;
                let mut rows = conn.query(&q, ()).await.map_err(|e| e.to_string())?;
                let mut out = Vec::new();
                while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
                    // PRAGMA table_info: cid, name, type, notnull, dflt_value, pk
                    let name = libsql_value_to_string(&row.get_value(1).map_err(|e| e.to_string())?).unwrap_or_default();
                    let data_type = libsql_value_to_string(&row.get_value(2).map_err(|e| e.to_string())?).unwrap_or_default();
                    let notnull = matches!(row.get_value(3), Ok(libsql::Value::Integer(n)) if n != 0);
                    let pk = matches!(row.get_value(5), Ok(libsql::Value::Integer(n)) if n > 0);
                    out.push(ColumnInfo {
                        name,
                        data_type,
                        nullable: !notnull,
                        key: if pk { "PRI".into() } else { String::new() },
                    });
                }
                Ok(out)
            })
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
pub fn db_table_info(
    state: State<DbManager>,
    id: String,
    table: String,
) -> Result<TableInfo, String> {
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
            Ok(TableInfo {
                rows,
                bytes,
                approximate: true,
            })
        }
        Conn::Sqlite(path) => {
            let c = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
            let q = format!("SELECT COUNT(*) FROM \"{}\"", table.replace('"', "\"\""));
            let cnt: i64 = c.query_row(&q, [], |r| r.get(0)).unwrap_or(-1);
            Ok(TableInfo {
                rows: Some(cnt).filter(|n| *n >= 0),
                bytes: None,
                approximate: false,
            })
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
                MAX_ROWS,
            )?;
            let first = rows_data.into_iter().next().unwrap_or_default();
            let rows = first
                .first()
                .cloned()
                .flatten()
                .and_then(|s| s.parse::<i64>().ok())
                .filter(|n| *n >= 0);
            let bytes = first
                .get(1)
                .cloned()
                .flatten()
                .and_then(|s| s.parse::<i64>().ok());
            Ok(TableInfo {
                rows,
                bytes,
                approximate: true,
            })
        }
        Conn::Clickhouse(client) => {
            let t = table.replace('\'', "''");
            let rows_ch: Vec<ChRow> = tauri::async_runtime::block_on(client.query_collect::<ChRow>(
                format!("SELECT total_rows, total_bytes FROM system.tables WHERE database = currentDatabase() AND name = '{t}'"),
            ))
            .map_err(|e| e.to_string())?;
            let (rows, bytes) = match rows_ch.into_iter().next() {
                Some(r) => {
                    let v: Vec<Option<String>> =
                        r.cells.iter().map(|(_, x)| ch_value_to_string(x)).collect();
                    (
                        v.first().cloned().flatten().and_then(|s| s.parse().ok()),
                        v.get(1).cloned().flatten().and_then(|s| s.parse().ok()),
                    )
                }
                None => (None, None),
            };
            Ok(TableInfo {
                rows,
                bytes,
                approximate: true,
            })
        }
        Conn::SqlAnywhere(db) => {
            let q = format!("SELECT COUNT(*) FROM \"{}\"", table.replace('"', "\"\""));
            let rows = tauri::async_runtime::block_on(async {
                let conn = db.connect().map_err(|e| e.to_string())?;
                let mut rs = conn.query(&q, ()).await.map_err(|e| e.to_string())?;
                let n = match rs.next().await.map_err(|e| e.to_string())? {
                    Some(row) => match row.get_value(0).map_err(|e| e.to_string())? {
                        libsql::Value::Integer(n) => Some(n),
                        _ => None,
                    },
                    None => None,
                };
                Ok::<_, String>(n)
            })?;
            Ok(TableInfo {
                rows,
                bytes: None,
                approximate: false,
            })
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
    s.starts_with("select")
        || s.starts_with("show")
        || s.starts_with("pragma")
        || s.starts_with("explain")
        || s.starts_with("describe")
        || s.starts_with("desc ")
        || s.starts_with("with")
}

const MAX_ROWS: usize = 1000;

#[tauri::command]
pub fn db_query(
    state: State<DbManager>,
    id: String,
    sql: String,
    max: Option<usize>,
) -> Result<QueryResult, String> {
    let max_rows = max.unwrap_or(MAX_ROWS);
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
                let columns: Vec<String> = result
                    .columns()
                    .as_ref()
                    .iter()
                    .map(|c| c.name_str().to_string())
                    .collect();
                let mut rows = Vec::new();
                let mut truncated = false;
                for row in result {
                    let row = row.map_err(|e| e.to_string())?;
                    if rows.len() >= max_rows {
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
                    if rows.len() >= max_rows {
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
            let (columns, rows, affected, truncated) = pg_query(&mut client, &sql, max_rows)?;
            let elapsed_ms = start.elapsed().as_millis() as u64;
            if select {
                Ok(QueryResult {
                    columns,
                    rows,
                    rows_affected: None,
                    elapsed_ms,
                    is_select: true,
                    truncated,
                })
            } else {
                Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected: affected,
                    elapsed_ms,
                    is_select: false,
                    truncated: false,
                })
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
                    if rows.len() >= max_rows {
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
                tauri::async_runtime::block_on(client.execute(sql.clone()))
                    .map_err(|e| e.to_string())?;
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
        Conn::SqlAnywhere(db) => {
            if select {
                let (columns, rows, truncated) = tauri::async_runtime::block_on(async {
                    let conn = db.connect().map_err(|e| e.to_string())?;
                    let mut rs = conn.query(&sql, ()).await.map_err(|e| e.to_string())?;
                    let ncols = rs.column_count();
                    let columns: Vec<String> = (0..ncols)
                        .map(|i| rs.column_name(i).unwrap_or("").to_string())
                        .collect();
                    let mut rows: Vec<Vec<Option<String>>> = Vec::new();
                    let mut truncated = false;
                    while let Some(row) = rs.next().await.map_err(|e| e.to_string())? {
                        if rows.len() >= max_rows {
                            truncated = true;
                            break;
                        }
                        let vals = (0..ncols)
                            .map(|i| row.get_value(i).ok().and_then(|v| libsql_value_to_string(&v)))
                            .collect();
                        rows.push(vals);
                    }
                    Ok::<_, String>((columns, rows, truncated))
                })?;
                Ok(QueryResult {
                    columns,
                    rows,
                    rows_affected: None,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    is_select: true,
                    truncated,
                })
            } else {
                let affected = tauri::async_runtime::block_on(async {
                    let conn = db.connect().map_err(|e| e.to_string())?;
                    conn.execute(&sql, ()).await.map_err(|e| e.to_string())
                })?;
                Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    rows_affected: Some(affected),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                    is_select: false,
                    truncated: false,
                })
            }
        }
    }
}
