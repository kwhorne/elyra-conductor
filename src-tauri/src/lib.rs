mod agent;
mod db;
mod fs;
mod history;
mod projects;
mod pty;

use agent::AgentManager;
use db::DbManager;
use pty::PtyManager;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::Emitter;
use tauri::Manager;

// Build the macOS menu bar, replacing the standard "About" with a custom item so
// it opens our own About dialog (consistent with the in-app one) instead of the
// native panel. We keep Edit (copy/paste) and Window so those still work.
fn build_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let about = MenuItem::with_id(app, "about", "About Elyra Conductor", true, None::<&str>)?;
    let app_menu = Submenu::with_items(
        app,
        "Elyra Conductor",
        true,
        &[
            &about,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, None)?,
            &PredefinedMenuItem::hide_others(app, None)?,
            &PredefinedMenuItem::show_all(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, None)?,
        ],
    )?;
    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::maximize(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;
    let data_transfer = MenuItem::with_id(app, "data-transfer", "Data Transfer…", true, None::<&str>)?;
    let compare_schemas = MenuItem::with_id(app, "compare-schemas", "Compare Schemas…", true, None::<&str>)?;
    let tools_menu = Submenu::with_items(app, "Tools", true, &[&data_transfer, &compare_schemas])?;
    Menu::with_items(app, &[&app_menu, &edit_menu, &window_menu, &tools_menu])
}

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
        .setup(|app| {
            let handle = app.handle();
            let menu = build_menu(handle)?;
            app.set_menu(menu)?;
            match history::HistoryStore::new(handle) {
                Ok(store) => {
                    app.manage(store);
                }
                Err(e) => eprintln!("history store init failed: {e}"),
            }
            app.on_menu_event(|app, event| {
                if event.id() == "about" {
                    let _ = app.emit("menu://about", ());
                } else if event.id() == "data-transfer" {
                    let _ = app.emit("menu://data-transfer", ());
                } else if event.id() == "compare-schemas" {
                    let _ = app.emit("menu://compare-schemas", ());
                }
            });
            Ok(())
        })
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
            projects::git_commit_index,
            projects::git_files,
            projects::git_diff,
            projects::git_stage,
            projects::git_unstage,
            projects::git_stage_all,
            projects::git_unstage_all,
            projects::git_discard,
            projects::git_branches,
            projects::git_checkout,
            projects::git_create_branch,
            projects::git_stash_list,
            projects::git_stash_push,
            projects::git_stash_pop,
            projects::git_stash_drop,
            projects::detect_editors,
            projects::detect_elyra,
            projects::open_in_editor,
            projects::list_ports,
            projects::kill_process,
            projects::open_url,
            projects::list_containers,
            projects::detect_terminal,
            projects::run_in_external_terminal,
            projects::home_dir,
            projects::run_step,
            projects::git_worktree_list,
            projects::git_worktree_conflicts,
            projects::git_worktree_add,
            projects::git_worktree_remove,
            projects::detect_gh,
            projects::gh_pr_list,
            projects::gh_pr_merge,
            history::history_add,
            history::history_query,
            history::history_stats,
            history::history_clear,
            fs::list_dir,
            fs::read_file,
            fs::write_file,
            fs::write_bytes,
            fs::create_file,
            fs::create_folder,
            fs::rename_path,
            fs::copy_path,
            fs::trash_path,
            fs::reveal_path,
            fs::list_files,
            fs::search_content,
            fs::list_queries,
            fs::save_queries,
            fs::list_runbooks,
            fs::list_tasks,
            db::db_from_env,
            db::list_connections,
            db::save_connections,
            db::db_connect,
            db::db_test,
            db::db_disconnect,
            db::db_tables,
            db::db_columns,
            db::db_table_info,
            db::db_query,
            db::db_transfer_tables,
            db::db_schema_diff,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
