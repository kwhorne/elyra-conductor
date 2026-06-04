use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
}

#[derive(Default)]
pub struct PtyManager {
    sessions: Mutex<HashMap<String, PtySession>>,
}

#[tauri::command]
pub fn pty_spawn(
    app: AppHandle,
    state: State<PtyManager>,
    id: String,
    cwd: String,
    cols: u16,
    rows: u16,
    run_command: Option<String>,
) -> Result<(), String> {
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    match run_command
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        // Run a command deterministically at startup, then drop into a normal
        // interactive login shell. Using `-i -c` (instead of typing into the
        // PTY) avoids the race where keystrokes are sent before an interactive
        // shell with a slow rc / instant-prompt is ready to read them.
        Some(run) => {
            cmd.arg("-l");
            cmd.arg("-i");
            cmd.arg("-c");
            cmd.arg(format!("{run}; exec {shell} -l -i"));
        }
        None => {
            cmd.arg("-l");
        }
    }
    if std::path::Path::new(&cwd).is_dir() {
        cmd.cwd(cwd);
    }
    cmd.env("TERM", "xterm-256color");

    let mut child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
    let killer = child.clone_killer();
    drop(pair.slave);

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let id_for_thread = id.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let _ = app.emit(&format!("pty://data/{id_for_thread}"), &buf[..n]);
                }
                Err(_) => break,
            }
        }
        // The pty reached EOF because the child exited; collect its code.
        let code: i64 = child.wait().map(|s| s.exit_code() as i64).unwrap_or(-1);
        let _ = app.emit(&format!("pty://exit/{id_for_thread}"), code);
    });

    state.sessions.lock().unwrap().insert(
        id,
        PtySession {
            master: pair.master,
            writer,
            killer,
        },
    );

    Ok(())
}

#[tauri::command]
pub fn pty_write(state: State<PtyManager>, id: String, data: String) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get_mut(&id) {
        session
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
        session.writer.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn pty_resize(
    state: State<PtyManager>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get(&id) {
        session
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Name of the foreground process running in the pty (e.g. "bun", "vim").
/// Returns None when it's just an idle shell, so the UI can keep its own title.
#[tauri::command]
pub fn pty_title(state: State<PtyManager>, id: String) -> Option<String> {
    let pid = {
        let sessions = state.sessions.lock().unwrap();
        sessions.get(&id)?.master.process_group_leader()?
    };

    let out = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()?;
    let raw = String::from_utf8_lossy(&out.stdout);
    let line = raw.trim();
    if line.is_empty() {
        return None;
    }
    // comm may be a full path and login shells appear as "-zsh".
    let base = line
        .rsplit('/')
        .next()
        .unwrap_or(line)
        .trim_start_matches('-');

    const SHELLS: &[&str] = &["zsh", "bash", "sh", "fish", "dash", "tcsh", "ksh", "login"];
    if SHELLS.contains(&base) {
        return None;
    }
    Some(base.to_string())
}

#[tauri::command]
pub fn pty_kill(state: State<PtyManager>, id: String) -> Result<(), String> {
    if let Some(mut session) = state.sessions.lock().unwrap().remove(&id) {
        let _ = session.killer.kill();
    }
    Ok(())
}
