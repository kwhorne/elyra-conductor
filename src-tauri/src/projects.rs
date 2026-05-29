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

    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
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
    let branch = git(&path, &["rev-parse", "--abbrev-ref", "HEAD"]).filter(|b| !b.is_empty());
    let dirty = git(&path, &["status", "--porcelain"])
        .map(|s| !s.is_empty())
        .unwrap_or(false);

    // "<behind>\t<ahead>" relative to the upstream branch (empty if no upstream).
    let (ahead, behind) = git(&path, &["rev-list", "--left-right", "--count", "@{u}...HEAD"])
        .and_then(|s| {
            let mut parts = s.split_whitespace();
            let behind = parts.next()?.parse().ok()?;
            let ahead = parts.next()?.parse().ok()?;
            Some((ahead, behind))
        })
        .unwrap_or((0, 0));

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
    let out = std::process::Command::new(&bin).arg("--version").output().ok()?;
    let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some(if v.is_empty() { "installed".to_string() } else { v })
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
