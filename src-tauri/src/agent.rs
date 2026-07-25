// JSONL bridge to the Elyra coding agent running in `--mode rpc`.
//
// IMPORTANT (see ARCHITECTURE.md): this is pure transport. Conductor spawns the
// external `elyra` process, streams its stdout JSON lines to the UI, and writes
// command JSON lines to its stdin. No AI logic, keys, or model calls live here.

use serde_json::Value;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use crate::util::lock_recover;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

struct AgentSession {
    child: Child,
    stdin: ChildStdin,
}

#[derive(Default)]
pub struct AgentManager {
    sessions: Mutex<HashMap<String, AgentSession>>,
}

#[tauri::command(async)]
pub fn agent_spawn(
    app: AppHandle,
    state: State<AgentManager>,
    id: String,
    cwd: String,
) -> Result<(), String> {
    let bin = crate::projects::find_bin("elyra").ok_or("elyra not found on PATH")?;

    // elyra is a Node script (`#!/usr/bin/env node`). A GUI-launched app has a
    // minimal PATH without nvm/node, so give the child the login-shell PATH
    // (plus the dir holding the resolved elyra binary, where node also lives).
    let mut path = crate::projects::login_shell_path()
        .or_else(|| std::env::var("PATH").ok())
        .unwrap_or_default();
    if let Some(dir) = std::path::Path::new(&bin).parent() {
        path = format!("{}:{}", dir.display(), path);
    }

    let mut cmd = Command::new(&bin);
    cmd.arg("--mode").arg("rpc");
    cmd.env("PATH", path);
    if std::path::Path::new(&cwd).is_dir() {
        cmd.current_dir(&cwd);
    }
    cmd.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start elyra: {e}"))?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;
    let stdin = child.stdin.take().ok_or("no stdin")?;

    // stdout: one JSON value per line -> emit to the UI.
    let app_out = app.clone();
    let id_out = id.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    let t = l.trim();
                    if t.is_empty() {
                        continue;
                    }
                    if let Ok(v) = serde_json::from_str::<Value>(t) {
                        let _ = app_out.emit(&format!("agent://event/{id_out}"), v);
                    }
                }
                Err(_) => break,
            }
        }
        let _ = app_out.emit(&format!("agent://exit/{id_out}"), ());
    });

    // stderr: surface as plain log lines (diagnostics only).
    let app_err = app.clone();
    let id_err = id.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            let _ = app_err.emit(&format!("agent://stderr/{id_err}"), line);
        }
    });

    lock_recover(&state.sessions).insert(id, AgentSession { child, stdin });
    Ok(())
}

#[tauri::command]
pub fn agent_send(state: State<AgentManager>, id: String, command: Value) -> Result<(), String> {
    let mut sessions = lock_recover(&state.sessions);
    // Report a missing session instead of silently succeeding: the UI appends
    // the message to the transcript optimistically, so swallowing this made a
    // prompt look delivered when the agent had actually exited.
    let s = sessions
        .get_mut(&id)
        .ok_or_else(|| format!("agent session {id} is no longer running"))?;
    let line = serde_json::to_string(&command).map_err(|e| e.to_string())?;
    s.stdin
        .write_all(line.as_bytes())
        .map_err(|e| e.to_string())?;
    s.stdin.write_all(b"\n").map_err(|e| e.to_string())?;
    s.stdin.flush().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command(async)]
pub fn agent_kill(state: State<AgentManager>, id: String) -> Result<(), String> {
    let session = lock_recover(&state.sessions).remove(&id);
    if let Some(mut s) = session {
        let _ = s.child.kill();
        // Reap it. Without the wait() the killed `elyra` lingers as a zombie
        // for the lifetime of the app — open and close a few agent tabs and
        // they accumulate. Dropping the stdin handle first lets the child see
        // EOF, so it usually exits before we even get here.
        drop(s.stdin);
        let _ = s.child.wait();
    }
    Ok(())
}
