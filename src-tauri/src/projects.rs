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
    // `symbolic-ref` returns the branch name even for an unborn branch (one with
    // no commits yet) and only fails on a genuine detached HEAD — unlike
    // `rev-parse --abbrev-ref HEAD`, which reports "HEAD" for unborn branches and
    // made the panel wrongly show "(detached)".
    let current = git(&path, &["symbolic-ref", "--quiet", "--short", "HEAD"]).filter(|b| !b.is_empty());
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

/// How a supported editor is launched: a CLI on PATH (`zed`, `code`, …), or a
/// macOS `.app` bundle with no CLI shim (launched via `open -a`).
enum EditorLauncher {
    Cli(&'static str),
    App {
        /// `.app` bundle name (without the extension), e.g. "e" for `e.app`.
        app_name: &'static str,
        /// `CFBundleIdentifier`, used as a fallback lookup via `mdfind`/Spotlight
        /// when the app isn't in one of the common install locations.
        bundle_id: &'static str,
    },
}

/// Known launchers for supported editors, keyed by the id `detect_editors` /
/// `open_in_editor` use on the frontend.
const EDITORS: &[(&str, EditorLauncher)] = &[
    ("zed", EditorLauncher::Cli("zed")),
    ("vscode", EditorLauncher::Cli("code")),
    ("cursor", EditorLauncher::Cli("cursor")),
    ("e", EditorLauncher::App { app_name: "e", bundle_id: "dev.e.editor" }),
];

/// Resolve a `.app` bundle to a path: common install locations first, then
/// Spotlight (`mdfind kMDItemCFBundleIdentifier == '<id>'`) as a fallback for
/// apps installed elsewhere.
fn find_app(app_name: &str, bundle_id: &str) -> Option<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    let candidates = [
        format!("/Applications/{app_name}.app"),
        format!("{home}/Applications/{app_name}.app"),
    ];
    for c in candidates {
        let p = PathBuf::from(&c);
        if p.exists() {
            return Some(p);
        }
    }
    let out = std::process::Command::new("mdfind")
        .arg(format!("kMDItemCFBundleIdentifier == '{bundle_id}'"))
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| PathBuf::from(l.trim()))
        .find(|p| p.exists())
}

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
        .filter(|(_, launcher)| match launcher {
            EditorLauncher::Cli(bin) => find_bin(bin).is_some(),
            EditorLauncher::App { app_name, bundle_id } => find_app(app_name, bundle_id).is_some(),
        })
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
    let launcher = EDITORS
        .iter()
        .find(|(name, _)| *name == editor)
        .map(|(_, launcher)| launcher)
        .ok_or_else(|| format!("unknown editor: {editor}"))?;

    match launcher {
        EditorLauncher::Cli(bin) => {
            let resolved = find_bin(bin).ok_or_else(|| format!("{bin} not found on PATH"))?;
            std::process::Command::new(resolved)
                .arg(&path)
                .spawn()
                .map_err(|e| format!("failed to launch {editor}: {e}"))?;
        }
        EditorLauncher::App { app_name, bundle_id } => {
            let app = find_app(app_name, bundle_id).ok_or_else(|| format!("{app_name}.app not found in /Applications"))?;
            std::process::Command::new("open")
                .arg("-a")
                .arg(app)
                .arg(&path)
                .spawn()
                .map_err(|e| format!("failed to launch {editor}: {e}"))?;
        }
    }

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

// ── Runbook verification ─────────────────────────────────────────────────────

#[derive(serde::Serialize)]
pub struct StepResult {
    code: i32,
    output: String,
    timed_out: bool,
}

/// Run one runbook step headless in the user's login shell and capture its
/// outcome. Used by "Verify runbook": each ```bash step runs sequentially so
/// stale documentation is flagged instead of silently rotting. The child is
/// killed after `timeout_secs` (long-running steps like dev servers should be
/// marked `no-verify` in the fence instead).
#[tauri::command]
pub fn run_step(cwd: String, command: String, timeout_secs: Option<u64>) -> StepResult {
    use std::io::Read;
    use std::time::{Duration, Instant};

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let timeout = Duration::from_secs(timeout_secs.unwrap_or(60).clamp(1, 600));

    let mut child = match std::process::Command::new(&shell)
        .arg("-lc")
        .arg(&command)
        .current_dir(&cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return StepResult { code: -1, output: format!("failed to spawn: {e}"), timed_out: false }
        }
    };

    // Drain stdout/stderr on threads so a chatty child never blocks on a full
    // pipe while we poll for exit.
    let mut readers = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        readers.push(std::thread::spawn(move || {
            let mut s = String::new();
            let _ = out.read_to_string(&mut s);
            s
        }));
    }
    if let Some(mut err) = child.stderr.take() {
        readers.push(std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            s
        }));
    }

    let started = Instant::now();
    let (code, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (status.code().unwrap_or(-1), false),
            Ok(None) => {
                if started.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    break (-1, true);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => break (-1, false),
        }
    };

    let mut output: String = readers
        .into_iter()
        .filter_map(|t| t.join().ok())
        .collect::<Vec<_>>()
        .join("");
    // Keep the tail — that's where the error usually is.
    if output.len() > 8000 {
        let cut = output.len() - 8000;
        let cut = output.char_indices().map(|(i, _)| i).find(|&i| i >= cut).unwrap_or(cut);
        output = output[cut..].to_string();
    }

    StepResult { code, output, timed_out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_step_success() {
        let r = run_step("/tmp".into(), "echo hello".into(), Some(10));
        assert_eq!(r.code, 0);
        assert!(!r.timed_out);
        assert!(r.output.contains("hello"));
    }

    #[test]
    fn run_step_failure_captures_exit_and_stderr() {
        let r = run_step("/tmp".into(), "echo oops >&2; exit 3".into(), Some(10));
        assert_eq!(r.code, 3);
        assert!(!r.timed_out);
        assert!(r.output.contains("oops"));
    }

    #[test]
    fn run_step_times_out_and_kills() {
        let t = std::time::Instant::now();
        let r = run_step("/tmp".into(), "sleep 30".into(), Some(1));
        assert!(r.timed_out);
        assert!(t.elapsed() < std::time::Duration::from_secs(10));
    }
    #[test]
    fn worktree_add_list_remove_roundtrip() {
        let base = std::env::temp_dir().join(format!("conductor-wt-test-{}", std::process::id()));
        let repo = base.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let r = repo.to_string_lossy().to_string();
        let run = |args: &[&str]| {
            std::process::Command::new("git").arg("-C").arg(&r).args(args).output().unwrap();
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.email", "t@t.t"]);
        run(&["config", "user.name", "t"]);
        std::fs::write(repo.join("a.txt"), "hi").unwrap();
        run(&["add", "-A"]);
        run(&["commit", "-qm", "init"]);

        let list = git_worktree_list(r.clone());
        assert_eq!(list.len(), 1);
        assert!(list[0].is_main);

        let path = git_worktree_add(r.clone(), "feature/x".into(), None).expect("add");
        assert!(std::path::Path::new(&path).is_dir());
        let list = git_worktree_list(r.clone());
        assert_eq!(list.len(), 2);
        assert!(list.iter().any(|w| w.branch.as_deref() == Some("feature/x") && !w.is_main));

        git_worktree_remove(r.clone(), path, false).expect("remove");
        assert_eq!(git_worktree_list(r.clone()).len(), 1);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn finds_e_app_in_applications() {
        // Only meaningful on a machine with e.app installed; skip gracefully otherwise.
        if !std::path::Path::new("/Applications/e.app").exists() {
            return;
        }
        assert!(find_app("e", "dev.e.editor").is_some());
    }
}

// ── Git worktrees ───────────────────────────────────────────────────
// Isolated checkouts that share one .git, so several agents can work different
// branches in parallel without colliding. New worktrees live in a sibling
// "<repo>.worktrees/<branch>" folder, keeping them out of the main working tree
// but easy to find next to the repo.

#[derive(serde::Serialize)]
pub struct Worktree {
    path: String,
    branch: Option<String>,
    head: String,
    is_main: bool,
    locked: bool,
}

fn git_result(repo: &str, args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn worktree_location(repo: &str, branch: &str) -> std::path::PathBuf {
    let repo_path = std::path::Path::new(repo);
    let base = repo_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("repo");
    let parent = repo_path.parent().unwrap_or_else(|| std::path::Path::new("/tmp"));
    let safe: String = branch
        .chars()
        .map(|c| if c == '/' || c == ' ' { '-' } else { c })
        .collect();
    parent.join(format!("{base}.worktrees")).join(safe)
}

#[tauri::command]
pub fn git_worktree_list(path: String) -> Vec<Worktree> {
    let Some(out) = git(&path, &["worktree", "list", "--porcelain"]) else {
        return vec![];
    };
    let mut res = Vec::new();
    let (mut wt_path, mut head) = (String::new(), String::new());
    let mut branch: Option<String> = None;
    let (mut locked, mut have, mut first) = (false, false, true);
    let flush = |res: &mut Vec<Worktree>,
                 wt_path: &mut String,
                 head: &mut String,
                 branch: &mut Option<String>,
                 locked: &mut bool,
                 have: &mut bool,
                 first: &mut bool| {
        if *have {
            res.push(Worktree {
                path: std::mem::take(wt_path),
                branch: branch.take(),
                head: std::mem::take(head),
                is_main: *first,
                locked: *locked,
            });
            *first = false;
            *locked = false;
            *have = false;
        }
    };
    for line in out.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            wt_path = p.to_string();
            have = true;
        } else if let Some(h) = line.strip_prefix("HEAD ") {
            head = h.chars().take(8).collect();
        } else if let Some(b) = line.strip_prefix("branch ") {
            branch = Some(b.strip_prefix("refs/heads/").unwrap_or(b).to_string());
        } else if line.starts_with("locked") {
            locked = true;
        } else if line.trim().is_empty() {
            flush(&mut res, &mut wt_path, &mut head, &mut branch, &mut locked, &mut have, &mut first);
        }
    }
    flush(&mut res, &mut wt_path, &mut head, &mut branch, &mut locked, &mut have, &mut first);
    res
}

#[tauri::command]
pub fn git_worktree_add(repo: String, branch: String, base: Option<String>) -> Result<String, String> {
    let branch = branch.trim().to_string();
    if branch.is_empty() {
        return Err("Branch name is required".into());
    }
    let dir = worktree_location(&repo, &branch);
    let dir_str = dir.to_string_lossy().to_string();
    if dir.exists() {
        return Err(format!("{dir_str} already exists"));
    }
    if let Some(parent) = dir.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let local = git(&repo, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_some();
    let remote_ref = format!("refs/remotes/origin/{branch}");
    let remote = git(&repo, &["rev-parse", "--verify", "--quiet", &remote_ref]).is_some();
    if local {
        // Existing local branch: just check it out into the new worktree.
        git_result(&repo, &["worktree", "add", &dir_str, &branch])?;
    } else if remote {
        // Branch exists on origin only (e.g. a PR branch): create a local branch
        // that tracks it, so the worktree has the real PR contents.
        git_result(&repo, &["worktree", "add", "--track", "-b", &branch, &dir_str, &format!("origin/{branch}")])?;
    } else {
        let base_ref = base.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or("HEAD");
        git_result(&repo, &["worktree", "add", "-b", &branch, &dir_str, base_ref])?;
    }
    Ok(dir_str)
}

#[tauri::command]
pub fn git_worktree_remove(repo: String, worktree_path: String, force: bool) -> Result<(), String> {
    let mut args: Vec<&str> = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&worktree_path);
    git_result(&repo, &args)?;
    Ok(())
}

// ── GitHub PR status (via the `gh` CLI) ──────────────────────────────────────
// Surfaces, per branch/worktree, the open PR and its check rollup so the
// parallel-branches view shows not just "an agent is on this branch" but
// "here's its PR and whether CI is green". Pure read-only; needs an
// authenticated `gh`. Best-effort: returns empty/err, never blocks the UI.

#[derive(serde::Serialize)]
pub struct PrInfo {
    branch: String,
    number: u64,
    title: String,
    state: String,
    is_draft: bool,
    url: String,
    review_decision: String,
    checks_passed: u32,
    checks_failed: u32,
    checks_pending: u32,
}

#[tauri::command]
pub fn detect_gh() -> bool {
    let Some(bin) = find_bin("gh") else {
        return false;
    };
    // Authenticated? `gh auth status` exits non-zero when not logged in.
    std::process::Command::new(&bin)
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tauri::command]
pub fn gh_pr_list(repo: String) -> Result<Vec<PrInfo>, String> {
    let bin = find_bin("gh").ok_or("gh (GitHub CLI) not found on PATH")?;
    let out = std::process::Command::new(&bin)
        .args([
            "pr",
            "list",
            "--state",
            "open",
            "--limit",
            "100",
            "--json",
            "number,headRefName,state,isDraft,title,url,reviewDecision,statusCheckRollup",
        ])
        .current_dir(&repo)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let json: serde_json::Value =
        serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let mut res = Vec::new();
    for pr in json.as_array().into_iter().flatten() {
        let (mut passed, mut failed, mut pending) = (0u32, 0u32, 0u32);
        if let Some(checks) = pr.get("statusCheckRollup").and_then(|v| v.as_array()) {
            for c in checks {
                // CheckRun carries status/conclusion; StatusContext carries state.
                let conclusion = c.get("conclusion").and_then(|v| v.as_str()).unwrap_or("");
                let status = c.get("status").and_then(|v| v.as_str()).unwrap_or("");
                let state = c.get("state").and_then(|v| v.as_str()).unwrap_or("");
                let ok = matches!(conclusion, "SUCCESS" | "NEUTRAL" | "SKIPPED")
                    || matches!(state, "SUCCESS");
                let bad = matches!(
                    conclusion,
                    "FAILURE" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED" | "STARTUP_FAILURE"
                ) || matches!(state, "FAILURE" | "ERROR");
                if bad {
                    failed += 1;
                } else if ok {
                    passed += 1;
                } else if status == "COMPLETED" || state == "EXPECTED" || !state.is_empty() || !status.is_empty() {
                    pending += 1;
                }
            }
        }
        res.push(PrInfo {
            branch: pr.get("headRefName").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            number: pr.get("number").and_then(|v| v.as_u64()).unwrap_or(0),
            title: pr.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            state: pr.get("state").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            is_draft: pr.get("isDraft").and_then(|v| v.as_bool()).unwrap_or(false),
            url: pr.get("url").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            review_decision: pr.get("reviewDecision").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            checks_passed: passed,
            checks_failed: failed,
            checks_pending: pending,
        });
    }
    Ok(res)
}

/// Merge an open PR (auto-merge queue: "CI is green, ship it"). `method` is
/// "squash" | "rebase" | "merge" (defaults to squash). Deletes the remote
/// branch on merge — the local worktree (if any) is left for the caller to
/// remove separately via `git_worktree_remove`.
#[tauri::command]
pub fn gh_pr_merge(repo: String, number: u64, method: Option<String>) -> Result<(), String> {
    let bin = find_bin("gh").ok_or("gh (GitHub CLI) not found on PATH")?;
    let flag = match method.as_deref() {
        Some("rebase") => "--rebase",
        Some("merge") => "--merge",
        _ => "--squash",
    };
    let out = std::process::Command::new(&bin)
        .args(["pr", "merge", &number.to_string(), flag, "--delete-branch"])
        .current_dir(&repo)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if msg.is_empty() { "gh pr merge failed".to_string() } else { msg });
    }
    Ok(())
}
