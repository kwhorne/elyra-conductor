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
    std::fs::write(&path, content).map_err(|e| format!("{path}: {e}"))
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
