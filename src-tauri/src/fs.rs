use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct DirEntry {
    name: String,
    path: String,
    is_dir: bool,
}

#[tauri::command]
pub fn list_dir(path: String) -> Result<Vec<DirEntry>, String> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&path).map_err(|e| format!("{path}: {e}"))?;
    for entry in entries.flatten() {
        let p = entry.path();
        out.push(DirEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path: p.to_string_lossy().to_string(),
            is_dir: p.is_dir(),
        });
    }
    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });
    Ok(out)
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    // Create parent directories so saving e.g. a new .conductor/notes/foo.md
    // (or any file in a not-yet-existing folder) just works.
    if let Some(parent) = Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
    }
    std::fs::write(&path, content).map_err(|e| format!("{path}: {e}"))
}

#[derive(Serialize, serde::Deserialize, Clone)]
pub struct SavedQuery {
    pub name: String,
    pub sql: String,
}

/// Per-project saved queries live in `<project>/.conductor/queries/`, which is
/// kept **private** (never committed) via a `.gitignore` that ignores the whole
/// folder. Runbooks (`.conductor/notes`) stay versionable; queries do not.
fn ensure_queries_dir(project: &str) -> Result<std::path::PathBuf, String> {
    let dir = Path::new(project).join(".conductor").join("queries");
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let gi = dir.join(".gitignore");
    if !gi.exists() {
        let _ = std::fs::write(&gi, "# Private to this machine - never committed.\n*\n");
    }
    Ok(dir)
}

#[tauri::command]
pub fn list_queries(project: String) -> Result<Vec<SavedQuery>, String> {
    let dir = ensure_queries_dir(&project)?;
    let file = dir.join("queries.json");
    if !file.exists() {
        return Ok(Vec::new());
    }
    let txt = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(serde_json::from_str(&txt).unwrap_or_default())
}

#[tauri::command]
pub fn save_queries(project: String, queries: Vec<SavedQuery>) -> Result<(), String> {
    let dir = ensure_queries_dir(&project)?;
    let file = dir.join("queries.json");
    let json = serde_json::to_string_pretty(&queries).map_err(|e| e.to_string())?;
    std::fs::write(&file, json).map_err(|e| format!("{}: {e}", file.display()))
}

/// Write raw bytes to a path (used for binary exports like .xlsx). Creates
/// parent directories as needed.
#[tauri::command]
pub fn write_bytes(path: String, bytes: Vec<u8>) -> Result<(), String> {
    if let Some(parent) = Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
    }
    std::fs::write(&path, bytes).map_err(|e| format!("{path}: {e}"))
}

/// List markdown runbooks for a project, stored under `<project>/.conductor/notes`.
/// Creates the directory if it does not exist so the first save always works.
#[tauri::command]
pub fn list_runbooks(project: String) -> Result<Vec<DirEntry>, String> {
    let dir = Path::new(&project).join(".conductor").join("notes");
    if !dir.exists() {
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten() {
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if p.is_file() && name.to_lowercase().ends_with(".md") {
            out.push(DirEntry {
                name,
                path: p.to_string_lossy().to_string(),
                is_dir: false,
            });
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

/// A runnable task discovered in a project (npm script, make target, etc).
/// `command` is a plain shell command — Conductor only launches it; it never
/// interprets or reasons about it.
#[derive(Serialize)]
pub struct Task {
    label: String,
    command: String,
    source: String,
}

/// Pick the JS package manager by lockfile, defaulting to npm.
fn detect_js_pm(dir: &Path) -> &'static str {
    if dir.join("pnpm-lock.yaml").exists() {
        "pnpm"
    } else if dir.join("bun.lockb").exists() || dir.join("bun.lock").exists() {
        "bun"
    } else if dir.join("yarn.lock").exists() {
        "yarn"
    } else {
        "npm"
    }
}

fn json_script_keys(dir: &Path, file: &str) -> Vec<String> {
    let Ok(txt) = std::fs::read_to_string(dir.join(file)) else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&txt) else {
        return Vec::new();
    };
    json.get("scripts")
        .and_then(|s| s.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

/// Scan a project folder for runnable tasks across common task runners.
#[tauri::command]
pub fn list_tasks(path: String) -> Vec<Task> {
    let dir = Path::new(&path);
    let mut tasks = Vec::new();

    // package.json scripts -> "<pm> run <name>"
    let pm = detect_js_pm(dir);
    for name in json_script_keys(dir, "package.json") {
        tasks.push(Task {
            command: format!("{pm} run {name}"),
            label: name,
            source: "package.json".into(),
        });
    }

    // composer.json scripts -> "composer run-script <name>"
    for name in json_script_keys(dir, "composer.json") {
        tasks.push(Task {
            command: format!("composer run-script {name}"),
            label: name,
            source: "composer.json".into(),
        });
    }

    // Makefile targets -> "make <name>"
    for mk in ["Makefile", "makefile", "GNUmakefile"] {
        let Ok(txt) = std::fs::read_to_string(dir.join(mk)) else {
            continue;
        };
        for line in txt.lines() {
            let first = match line.chars().next() {
                Some(c) => c,
                None => continue,
            };
            // Skip recipes (indented), comments, and special/.PHONY targets.
            if first == ' ' || first == '\t' || first == '#' || first == '.' {
                continue;
            }
            let Some(colon) = line.find(':') else { continue };
            let name = line[..colon].trim();
            let after = &line[colon..];
            // Skip variable assignments (FOO := bar) and malformed names.
            if after.starts_with(":=") || name.is_empty() || name.contains('=') || name.contains(' ') {
                continue;
            }
            tasks.push(Task {
                command: format!("make {name}"),
                label: name.to_string(),
                source: mk.into(),
            });
        }
        break;
    }

    // justfile recipes -> "just <name>"
    for jf in ["justfile", "Justfile", ".justfile"] {
        let Ok(txt) = std::fs::read_to_string(dir.join(jf)) else {
            continue;
        };
        for line in txt.lines() {
            let first = match line.chars().next() {
                Some(c) => c,
                None => continue,
            };
            if first == ' ' || first == '\t' || first == '#' || first == '@' {
                continue;
            }
            let Some(colon) = line.find(':') else { continue };
            let after = &line[colon..];
            if after.starts_with(":=") {
                continue; // variable assignment
            }
            // Recipe name is the first token before any parameters.
            let name = line[..colon].split_whitespace().next().unwrap_or("");
            if name.is_empty() || name.contains('=') {
                continue;
            }
            tasks.push(Task {
                command: format!("just {name}"),
                label: name.to_string(),
                source: jf.into(),
            });
        }
        break;
    }

    tasks
}
