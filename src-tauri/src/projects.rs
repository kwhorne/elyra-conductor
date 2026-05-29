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

/// Known CLI launchers for supported editors.
const EDITORS: &[(&str, &str)] = &[("zed", "zed"), ("vscode", "code"), ("cursor", "cursor")];

fn find_bin(bin: &str) -> Option<String> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(paths) = std::env::var("PATH") {
        for dir in std::env::split_paths(&paths) {
            candidates.push(dir.join(bin));
        }
    }
    // Common locations that may not be on a GUI-launched PATH.
    candidates.push(PathBuf::from(format!("/opt/homebrew/bin/{bin}")));
    candidates.push(PathBuf::from(format!("/usr/local/bin/{bin}")));

    candidates
        .into_iter()
        .find(|p| p.is_file())
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn detect_editors() -> Vec<String> {
    EDITORS
        .iter()
        .filter(|(_, bin)| find_bin(bin).is_some())
        .map(|(name, _)| name.to_string())
        .collect()
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
