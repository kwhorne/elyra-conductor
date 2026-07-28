// One-shot bridge to `elyra --print` for the inline terminal ask bar.
//
// IMPORTANT (see ARCHITECTURE.md): this is pure transport, exactly like
// agent.rs. Conductor spawns the external `elyra` binary, pipes a prompt to its
// stdin, and streams stdout back to the UI. It holds no API keys, chooses no
// provider or model, and defines no system prompt — `elyra` owns all of that
// through its own configuration, which the user has already set up once.
//
// The prompt goes over **stdin, never argv**: a context block on the command
// line would be visible to every other process on the machine via `ps`, and
// would eventually hit ARG_MAX once a few hundred scrollback lines are included.
//
// Tools are disabled (`--no-tools`). The inline bar answers questions; it does
// not act. When you want an agent that can actually edit and run things, the UI
// escalates to a full Elyra tab, where there is a transcript to review.

use crate::util::lock_recover;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct AskManager {
    running: Mutex<HashMap<String, Child>>,
}

/// Split off the longest valid UTF-8 prefix of `pending`, leaving any
/// incomplete trailing sequence in the buffer for the next read.
///
/// Chunked reads cut multi-byte characters in half, so decoding each raw chunk
/// on its own would corrupt any non-ASCII output (box-drawing, emoji, æøå).
fn take_utf8(pending: &mut Vec<u8>) -> Option<String> {
    let valid_len = match std::str::from_utf8(pending) {
        Ok(_) => pending.len(),
        Err(e) => {
            // A genuinely invalid sequence (rather than a truncated one) would
            // otherwise wedge this loop forever, since valid_up_to() never
            // advances. Drop the offending byte and continue.
            if e.valid_up_to() == 0 && e.error_len().is_some() {
                pending.remove(0);
                return Some("\u{fffd}".into());
            }
            e.valid_up_to()
        }
    };
    if valid_len == 0 {
        return None;
    }
    let s = String::from_utf8_lossy(&pending[..valid_len]).into_owned();
    pending.drain(..valid_len);
    Some(s)
}

#[tauri::command(async)]
pub fn elyra_ask(
    app: AppHandle,
    state: State<AskManager>,
    id: String,
    cwd: String,
    prompt: String,
) -> Result<(), String> {
    let bin = crate::projects::find_bin("elyra").ok_or("elyra not found on PATH")?;

    // Same PATH problem as agent.rs: elyra is a Node script, and a GUI-launched
    // app has a minimal PATH with no node on it.
    let mut path = crate::projects::login_shell_path()
        .or_else(|| std::env::var("PATH").ok())
        .unwrap_or_default();
    if let Some(dir) = std::path::Path::new(&bin).parent() {
        path = format!("{}:{}", dir.display(), path);
    }

    let mut cmd = Command::new(&bin);
    // --print: answer and exit. --no-session: a throwaway question should not
    // land in the session history you resume with `elyra -c`.
    cmd.arg("--print").arg("--no-session").arg("--no-tools");
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

    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let mut stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    // Write the prompt on a thread and drop the handle to signal EOF. Doing this
    // inline would deadlock on a prompt larger than the pipe buffer (64 KiB is
    // typical, and a scrollback tail can exceed it): the child blocks writing
    // its answer while we block writing the prompt.
    std::thread::spawn(move || {
        let _ = stdin.write_all(prompt.as_bytes());
        let _ = stdin.flush();
        drop(stdin);
    });

    // stdout: stream incrementally so the answer appears as it is produced.
    let app_out = app.clone();
    let id_out = id.clone();
    std::thread::spawn(move || {
        let mut pending: Vec<u8> = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            match stdout.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    pending.extend_from_slice(&buf[..n]);
                    while let Some(text) = take_utf8(&mut pending) {
                        let _ = app_out.emit(&format!("ask://chunk/{id_out}"), text);
                        if pending.is_empty() {
                            break;
                        }
                    }
                }
            }
        }
        let _ = app_out.emit(&format!("ask://done/{id_out}"), ());
    });

    // stderr: only surfaced if the answer is empty, so a misconfigured provider
    // reports something actionable instead of failing silently.
    let app_err = app.clone();
    let id_err = id.clone();
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            let _ = app_err.emit(&format!("ask://stderr/{id_err}"), line);
        }
    });

    lock_recover(&state.running).insert(id, child);
    Ok(())
}

#[tauri::command(async)]
pub fn elyra_ask_cancel(state: State<AskManager>, id: String) -> Result<(), String> {
    let child = lock_recover(&state.running).remove(&id);
    if let Some(mut c) = child {
        let _ = c.kill();
        // Reap it, or every cancelled question leaves a zombie for the lifetime
        // of the app (the same leak agent_kill had to fix).
        let _ = c.wait();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::take_utf8;

    #[test]
    fn take_utf8_holds_back_split_characters() {
        // "æ" is 0xC3 0xA6. Arriving split across two reads, it must not be
        // decoded as a replacement char.
        let mut pending = vec![b'o', b'k', 0xC3];
        assert_eq!(take_utf8(&mut pending).unwrap(), "ok");
        assert_eq!(pending, vec![0xC3], "incomplete tail must be kept");

        pending.push(0xA6);
        assert_eq!(take_utf8(&mut pending).unwrap(), "æ");
        assert!(pending.is_empty());
    }

    #[test]
    fn take_utf8_returns_none_when_nothing_is_complete() {
        let mut pending = vec![0xE2, 0x9C]; // first two bytes of "✓"
        assert!(take_utf8(&mut pending).is_none());
        assert_eq!(pending.len(), 2, "buffer must be left intact");
    }

    #[test]
    fn take_utf8_makes_progress_on_invalid_bytes() {
        // A lone 0xFF is not a truncated sequence, it is invalid. Without the
        // error_len() branch this would spin forever without draining.
        let mut pending = vec![0xFF, b'h', b'i'];
        assert_eq!(take_utf8(&mut pending).unwrap(), "\u{fffd}");
        assert_eq!(take_utf8(&mut pending).unwrap(), "hi");
        assert!(pending.is_empty());
    }
}
