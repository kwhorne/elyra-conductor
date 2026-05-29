mod fs;
mod projects;
mod pty;

use pty::PtyManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(PtyManager::default())
        .invoke_handler(tauri::generate_handler![
            pty::pty_spawn,
            pty::pty_write,
            pty::pty_resize,
            pty::pty_kill,
            pty::pty_title,
            projects::list_projects,
            projects::git_status,
            projects::git_changes,
            projects::git_commit,
            projects::detect_editors,
            projects::open_in_editor,
            projects::detect_terminal,
            projects::run_in_external_terminal,
            projects::home_dir,
            fs::list_dir,
            fs::read_file,
            fs::write_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
