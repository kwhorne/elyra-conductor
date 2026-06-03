mod agent;
mod db;
mod fs;
mod projects;
mod pty;

use agent::AgentManager;
use db::DbManager;
use pty::PtyManager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_notification::init())
        .manage(PtyManager::default())
        .manage(AgentManager::default())
        .manage(DbManager::default())
        .invoke_handler(tauri::generate_handler![
            pty::pty_spawn,
            pty::pty_write,
            pty::pty_resize,
            pty::pty_kill,
            pty::pty_title,
            agent::agent_spawn,
            agent::agent_send,
            agent::agent_kill,
            projects::list_projects,
            projects::git_status,
            projects::git_changes,
            projects::git_commit,
            projects::detect_editors,
            projects::detect_elyra,
            projects::open_in_editor,
            projects::detect_terminal,
            projects::run_in_external_terminal,
            projects::home_dir,
            fs::list_dir,
            fs::read_file,
            fs::write_file,
            fs::write_bytes,
            fs::list_queries,
            fs::save_queries,
            fs::list_runbooks,
            fs::list_tasks,
            db::db_from_env,
            db::list_connections,
            db::save_connections,
            db::db_connect,
            db::db_disconnect,
            db::db_tables,
            db::db_columns,
            db::db_table_info,
            db::db_query,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
