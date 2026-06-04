use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct Project {
    name: String,
    path: String,
    is_git: bool,
    branch: Option<String>,
}

#[tauri::command]
pub fn home_dir() -> Result<String, String> {
    std::env::var("HOME").map_err(|_| "HOME not set".to_string())
}

#[tauri::command]
pub fn list_projects(root: String) -> Result<Vec<Project>, String> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&root).map_err(|e| format!("{root}: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }

        let head = path.join(".git").join("HEAD");
        let (is_git, branch) = if head.exists() {
            let contents = std::fs::read_to_string(&head).unwrap_or_default();
            let branch = contents
                .strip_prefix("ref: refs/heads/")
                .map(|s| s.trim().to_string());
            (true, branch)
        } else {
            (false, None)
        };

        out.push(Project {
            name,
            path: path.to_string_lossy().to_string(),
            is_git,
            branch,
        });
    }

    out.sort_by_key(|a| a.name.to_lowercase());
    Ok(out)
}

#[derive(Serialize)]
pub struct GitStatus {
    branch: Option<String>,
    dirty: bool,
    ahead: u32,
    behind: u32,
}

fn git(path: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[tauri::command]
pub fn git_status(path: String) -> GitStatus {
    // A single `git status` call yields branch, ahead/behind, and dirty state in
    // one process. Doing three separate git invocations per project caused a
    // process storm (3 × N repos) on every window focus. Porcelain v2 with
    // --branch prints `# branch.head <name>`, `# branch.ab +A -B`, and one line
    // per changed path.
    let out = git(&path, &["status", "--porcelain=v2", "--branch"]);
    let Some(out) = out else {
        return GitStatus {
            branch: None,
            dirty: false,
            ahead: 0,
            behind: 0,
        };
    };

    let mut branch: Option<String> = None;
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut dirty = false;
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            branch = Some(rest.trim().to_string()).filter(|b| !b.is_empty() && b != "(detached)");
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            // Format: "+<ahead> -<behind>"
            for tok in rest.split_whitespace() {
                if let Some(a) = tok.strip_prefix('+') {
                    ahead = a.parse().unwrap_or(0);
                } else if let Some(b) = tok.strip_prefix('-') {
                    behind = b.parse().unwrap_or(0);
                }
            }
        } else if !line.starts_with('#') && !line.is_empty() {
            dirty = true;
        }
    }

    GitStatus {
        branch,
        dirty,
        ahead,
        behind,
    }
}

#[derive(Serialize)]
pub struct GitChange {
    status: String,
    file: String,
}

#[tauri::command]
pub fn git_changes(path: String) -> Vec<GitChange> {
    git(&path, &["status", "--porcelain"])
        .map(|s| {
            s.lines()
                .filter(|l| l.len() >= 3)
                .map(|l| GitChange {
                    status: l[0..2].trim().to_string(),
                    file: l[3..].to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn run_git_capture(path: &str, args: &[&str], log: &mut String) -> Result<(), String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    log.push_str(&format!("$ git {}\n{stdout}{stderr}\n", args.join(" ")));
    if !out.status.success() {
        return Err(format!("git {} failed:\n{stderr}", args.join(" ")));
    }
    Ok(())
}

/// Run git and return stdout on success, or a readable error from stderr.
fn git_try(path: &str, args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

#[derive(Serialize)]
pub struct GitFile {
    file: String,
    x: String, // index (staged) status
    y: String, // worktree (unstaged) status
    staged: bool,
    untracked: bool,
}

/// Richer change list: separates staged (index) from unstaged (worktree) state
/// using the two porcelain status columns.
#[tauri::command]
pub fn git_files(path: String) -> Vec<GitFile> {
    let Some(out) = git(&path, &["status", "--porcelain"]) else {
        return vec![];
    };
    out.lines()
        .filter(|l| l.len() >= 3)
        .map(|l| {
            let x = &l[0..1];
            let y = &l[1..2];
            let mut file = l[3..].to_string();
            // Renames look like "old -> new"; keep the new path.
            if let Some(idx) = file.find(" -> ") {
                file = file[idx + 4..].to_string();
            }
            GitFile {
                file,
                x: x.to_string(),
                y: y.to_string(),
                staged: x != " " && x != "?",
                untracked: x == "?" && y == "?",
            }
        })
        .collect()
}

/// Unified diff for a single file. `staged` shows the index diff; untracked
/// files are shown as an all-added diff against /dev/null.
#[tauri::command]
pub fn git_diff(path: String, file: String, staged: bool, untracked: bool) -> String {
    let args: Vec<&str> = if untracked {
        vec!["diff", "--no-color", "--no-index", "--", "/dev/null", &file]
    } else if staged {
        vec!["diff", "--no-color", "--cached", "--", &file]
    } else {
        vec!["diff", "--no-color", "--", &file]
    };
    // --no-index returns exit code 1 when files differ, so don't treat that as
    // an error; just take whatever stdout we got.
    std::process::Command::new("git")
        .arg("-C")
        .arg(&path)
        .args(&args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

#[tauri::command]
pub fn git_stage(path: String, file: String) -> Result<(), String> {
    git_try(&path, &["add", "--", &file]).map(|_| ())
}

#[tauri::command]
pub fn git_unstage(path: String, file: String) -> Result<(), String> {
    git_try(&path, &["reset", "-q", "HEAD", "--", &file]).map(|_| ())
}

#[tauri::command]
pub fn git_stage_all(path: String) -> Result<(), String> {
    git_try(&path, &["add", "-A"]).map(|_| ())
}

#[tauri::command]
pub fn git_unstage_all(path: String) -> Result<(), String> {
    git_try(&path, &["reset", "-q", "HEAD"]).map(|_| ())
}

/// Discard worktree changes for a tracked file (restore from index/HEAD).
#[tauri::command]
pub fn git_discard(path: String, file: String) -> Result<(), String> {
    git_try(&path, &["restore", "--", &file]).map(|_| ())
}

#[derive(Serialize)]
pub struct Branches {
    current: Option<String>,
    all: Vec<String>,
}

#[tauri::command]
pub fn git_branches(path: String) -> Branches {
    let current = git(&path, &["rev-parse", "--abbrev-ref", "HEAD"]).filter(|b| b != "HEAD");
    let all = git(&path, &["branch", "--format=%(refname:short)"])
        .map(|s| {
            s.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default();
    Branches { current, all }
}

#[tauri::command]
pub fn git_checkout(path: String, branch: String) -> Result<(), String> {
    git_try(&path, &["checkout", &branch]).map(|_| ())
}

#[tauri::command]
pub fn git_create_branch(path: String, name: String) -> Result<(), String> {
    git_try(&path, &["checkout", "-b", &name]).map(|_| ())
}

#[derive(Serialize)]
pub struct Stash {
    index: u32,
    text: String,
}

#[tauri::command]
pub fn git_stash_list(path: String) -> Vec<Stash> {
    let Some(out) = git(&path, &["stash", "list"]) else {
        return vec![];
    };
    out.lines()
        .enumerate()
        .map(|(i, l)| Stash {
            index: i as u32,
            text: l.to_string(),
        })
        .collect()
}

#[tauri::command]
pub fn git_stash_push(path: String, message: String) -> Result<(), String> {
    let mut args = vec!["stash", "push"];
    if !message.trim().is_empty() {
        args.push("-m");
        args.push(&message);
    }
    git_try(&path, &args).map(|_| ())
}

#[tauri::command]
pub fn git_stash_pop(path: String, index: u32) -> Result<(), String> {
    let r = format!("stash@{{{index}}}");
    git_try(&path, &["stash", "pop", &r]).map(|_| ())
}

#[tauri::command]
pub fn git_stash_drop(path: String, index: u32) -> Result<(), String> {
    let r = format!("stash@{{{index}}}");
    git_try(&path, &["stash", "drop", &r]).map(|_| ())
}

#[tauri::command]
pub fn git_commit(path: String, message: String, push: bool) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("Commit message is empty".to_string());
    }
    let mut log = String::new();
    run_git_capture(&path, &["add", "-A"], &mut log)?;
    run_git_capture(&path, &["commit", "-m", &message], &mut log)?;
    if push {
        run_git_capture(&path, &["push"], &mut log)?;
    }
    Ok(log)
}

/// Commit only what's already staged (the index) — used by the Git panel where
/// staging is explicit. Fails if nothing is staged.
#[tauri::command]
pub fn git_commit_index(path: String, message: String, push: bool) -> Result<String, String> {
    if message.trim().is_empty() {
        return Err("Commit message is empty".to_string());
    }
    let mut log = String::new();
    run_git_capture(&path, &["commit", "-m", &message], &mut log)?;
    if push {
        run_git_capture(&path, &["push"], &mut log)?;
    }
    Ok(log)
}

/// Known CLI launchers for supported editors.
const EDITORS: &[(&str, &str)] = &[("zed", "zed"), ("vscode", "code"), ("cursor", "cursor")];

pub fn find_bin(bin: &str) -> Option<String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            candidates.push(dir.join(bin));
        }
    }
    // Common locations that may not be on a GUI-launched PATH.
    candidates.push(PathBuf::from(format!("/opt/homebrew/bin/{bin}")));
    candidates.push(PathBuf::from(format!("/usr/local/bin/{bin}")));

    if let Some(p) = candidates.into_iter().find(|p| p.is_file()) {
        return Some(p.to_string_lossy().to_string());
    }

    // Fallback for GUI launches: a Finder/`open`-launched app gets a minimal PATH
    // that misses version managers (nvm, asdf, …). Ask the user's login shell to
    // resolve the binary, which sources their profile.
    resolve_via_login_shell(bin)
}

/// The user's full PATH as seen by their login shell (includes nvm/asdf dirs).
/// GUI-launched apps otherwise get a minimal PATH that breaks `#!/usr/bin/env node`.
pub fn login_shell_path() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let out = std::process::Command::new(&shell)
        .arg("-lic")
        .arg("printf 'CONDUCTOR_PATH:%s\\n' \"$PATH\"")
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .lines()
        .find_map(|l| l.strip_prefix("CONDUCTOR_PATH:"))
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
}

fn resolve_via_login_shell(bin: &str) -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let out = std::process::Command::new(&shell)
        .arg("-lic")
        .arg(format!("command -v {bin} 2>/dev/null"))
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .lines()
        .map(|l| l.trim())
        .find(|l| l.starts_with('/') && std::path::Path::new(l).is_file())
        .map(|l| l.to_string())
}

#[tauri::command]
pub fn detect_editors() -> Vec<String> {
    EDITORS
        .iter()
        .filter(|(_, bin)| find_bin(bin).is_some())
        .map(|(name, _)| name.to_string())
        .collect()
}

/// Detect the Elyra coding agent CLI (returns its version string if present).
#[tauri::command]
pub fn detect_elyra() -> Option<String> {
    let bin = find_bin("elyra")?;
    let out = std::process::Command::new(&bin)
        .arg("--version")
        .output()
        .ok()?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some(if v.is_empty() {
        "installed".to_string()
    } else {
        v
    })
}

/// Name of the external terminal we will launch (for labelling the UI).
#[tauri::command]
pub fn detect_terminal() -> String {
    if std::path::Path::new("/Applications/iTerm.app").exists() {
        "iTerm2".to_string()
    } else {
        "Terminal".to_string()
    }
}

/// Run an executable file in an external macOS terminal (iTerm2 if present,
/// otherwise Terminal.app), executing `./<name>` from the file's directory.
#[tauri::command]
pub fn run_in_external_terminal(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let dir = p
        .parent()
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());
    let base = p
        .file_name()
        .map(|b| b.to_string_lossy().to_string())
        .ok_or_else(|| "invalid path".to_string())?;

    // Single-quote the shell parts so spaces are safe.
    let shell_cmd = format!("cd '{dir}' && './{base}'");

    let script = if std::path::Path::new("/Applications/iTerm.app").exists() {
        format!(
            "tell application \"iTerm\"\n\
             activate\n\
             create window with default profile\n\
             tell current session of current window\n\
             write text \"{shell_cmd}\"\n\
             end tell\n\
             end tell"
        )
    } else {
        format!(
            "tell application \"Terminal\"\n\
             activate\n\
             do script \"{shell_cmd}\"\n\
             end tell"
        )
    };

    std::process::Command::new("osascript")
        .arg("-e")
        .arg(script)
        .spawn()
        .map_err(|e| format!("osascript failed: {e}"))?;

    Ok(())
}

#[tauri::command]
pub fn open_in_editor(editor: String, path: String) -> Result<(), String> {
    let bin = EDITORS
        .iter()
        .find(|(name, _)| *name == editor)
        .map(|(_, bin)| *bin)
        .ok_or_else(|| format!("unknown editor: {editor}"))?;

    let resolved = find_bin(bin).ok_or_else(|| format!("{bin} not found on PATH"))?;

    std::process::Command::new(resolved)
        .arg(&path)
        .spawn()
        .map_err(|e| format!("failed to launch {editor}: {e}"))?;

    Ok(())
}

// ── Port dashboard ──────────────────────────────────────────────
#[derive(Serialize)]
pub struct PortInfo {
    port: u16,
    pid: u32,
    process: String,
    addr: String,
    cwd: String,
}

/// List local TCP ports in LISTEN state (via `lsof`), one entry per port,
/// including each owning process's working directory (so ports can be attributed
/// to a project).
#[tauri::command]
pub fn list_ports() -> Vec<PortInfo> {
    use std::collections::HashMap;
    let mut map: HashMap<u16, PortInfo> = HashMap::new();
    let out = std::process::Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN"])
        .output();
    if let Ok(o) = out {
        let text = String::from_utf8_lossy(&o.stdout);
        for line in text.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 9 {
                continue;
            }
            let process = cols[0].to_string();
            let pid: u32 = cols[1].parse().unwrap_or(0);
            let name = cols[8]; // e.g. 127.0.0.1:5173, *:8080, [::1]:3000
            if let Some(idx) = name.rfind(':') {
                if let Ok(port) = name[idx + 1..].parse::<u16>() {
                    let addr = name[..idx].to_string();
                    map.entry(port).or_insert(PortInfo {
                        port,
                        pid,
                        process,
                        addr,
                        cwd: String::new(),
                    });
                }
            }
        }
    }
    // Resolve each owning process's cwd in one batched lsof call.
    let pids: Vec<String> = {
        let mut s: Vec<u32> = map.values().map(|p| p.pid).filter(|p| *p > 0).collect();
        s.sort_unstable();
        s.dedup();
        s.iter().map(|p| p.to_string()).collect()
    };
    if !pids.is_empty() {
        if let Ok(o) = std::process::Command::new("lsof")
            .args(["-a", "-d", "cwd", "-Fn", "-p", &pids.join(",")])
            .output()
        {
            let text = String::from_utf8_lossy(&o.stdout);
            let mut cwd_by_pid: HashMap<u32, String> = HashMap::new();
            let mut cur: u32 = 0;
            for line in text.lines() {
                if let Some(rest) = line.strip_prefix('p') {
                    cur = rest.parse().unwrap_or(0);
                } else if let Some(rest) = line.strip_prefix('n') {
                    cwd_by_pid.entry(cur).or_insert_with(|| rest.to_string());
                }
            }
            for p in map.values_mut() {
                if let Some(c) = cwd_by_pid.get(&p.pid) {
                    p.cwd = c.clone();
                }
            }
        }
    }
    let mut v: Vec<PortInfo> = map.into_values().collect();
    v.sort_by_key(|p| p.port);
    v
}

/// Send SIGTERM to a process (used by the port dashboard to stop a dev server).
#[tauri::command]
pub fn kill_process(pid: u32) -> Result<(), String> {
    std::process::Command::new("kill")
        .arg(pid.to_string())
        .status()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Open a URL in the default browser.
#[tauri::command]
pub fn open_url(url: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(&url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)]
pub struct Container {
    name: String,
    state: String,
    working_dir: String,
}

/// List Docker containers and the Compose working-dir they belong to, so the UI
/// can show a per-project container badge. Returns empty if Docker isn't
/// available (no daemon, not installed) — best-effort, never an error.
#[tauri::command]
pub fn list_containers() -> Vec<Container> {
    let out = std::process::Command::new("docker")
        .args([
            "ps",
            "--all",
            "--format",
            "{{.Names}}\t{{.State}}\t{{.Label \"com.docker.compose.project.working_dir\"}}",
        ])
        .output();
    let Ok(out) = out else {
        return vec![];
    };
    if !out.status.success() {
        return vec![];
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut parts = l.splitn(3, '\t');
            let name = parts.next()?.to_string();
            let state = parts.next().unwrap_or("").to_string();
            let working_dir = parts.next().unwrap_or("").to_string();
            if name.is_empty() {
                return None;
            }
            Some(Container {
                name,
                state,
                working_dir,
            })
        })
        .collect()
}
