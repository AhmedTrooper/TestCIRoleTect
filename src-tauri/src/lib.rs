pub mod ai;
pub mod commands;
pub mod db;
pub mod server;
pub mod s3;

use rusqlite::Connection;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<Option<Connection>>,
    pub data_dirty: Mutex<bool>,
    pub startup_snapshot_hash: Mutex<Option<String>>,
}

impl AppState {
    pub fn mark_dirty(&self) {
        if let Ok(mut dirty) = self.data_dirty.lock() {
            *dirty = true;
        }
    }
}

impl AppState {
    /// Helper to access the database, waiting for initialization if needed.
    pub async fn with_db<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut Connection) -> Result<R, String>,
    {
        for _ in 0..50 { // 5 seconds timeout
            {
                let mut db_guard = self.db.lock().map_err(|e| e.to_string())?;
                if let Some(conn) = db_guard.as_mut() {
                    return f(conn);
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        Err("Database initialization timed out".to_string())
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_localhost::Builder::new(1420).build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 1. Initialize Stronghold with secure Argon2 hashing
            let local_data_dir = app
                .path()
                .app_local_data_dir()
                .expect("could not resolve app local data path");

            std::fs::create_dir_all(&local_data_dir).expect("Failed to create local data dir");

            let salt_path = local_data_dir.join("salt.txt");

            app.handle()
                .plugin(tauri_plugin_stronghold::Builder::with_argon2(&salt_path).build())
                .expect("Failed to initialize Stronghold");

            // 2. Initialize SQLite Database Asynchronously
            let app_handle = app.handle().clone();
            let app_state = AppState {
                db: Mutex::new(None),
                data_dirty: Mutex::new(false),
                startup_snapshot_hash: Mutex::new(None),
            };
            app.manage(app_state);

            tauri::async_runtime::spawn(async move {
                match db::init_db(&app_handle) {
                    Ok(conn) => {
                        if let Ok(mut db_guard) = app_handle.state::<AppState>().db.lock() {
                            *db_guard = Some(conn);
                        }
                    }
                    Err(e) => {
                        eprintln!("Database initialization failed: {}", e);
                    }
                }
            });

            // 3. Start Extension Ingest Server
            let app_handle_for_server = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                server::start_server(app_handle_for_server).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings::save_model_pref,
            commands::settings::get_model_pref,
            commands::jobs::parse_job,
            commands::jobs::save_job,
            commands::jobs::get_job_by_id,
            commands::jobs::get_all_jobs,
            commands::jobs::delete_job,
            commands::jobs::delete_jobs_batch,
            commands::jobs::delete_all_jobs,
            commands::jobs::update_tailored_resume,
            commands::jobs::update_job_status,
            commands::jobs::update_job_metadata,
            commands::jobs::get_tailored_resume,
            commands::jobs::get_latest_tailored_resume,
            commands::jobs::tailor_resume,
            commands::resumes::get_all_resumes,
            commands::resumes::get_resume_by_id,
            commands::resumes::create_new_resume,
            commands::resumes::update_resume,
            commands::resumes::delete_resume,
            commands::resumes::check_resume_usage,
            commands::cover_letters::get_all_cover_letters,
            commands::cover_letters::get_cover_letter_by_id,
            commands::cover_letters::create_new_cover_letter,
            commands::cover_letters::update_cover_letter,
            commands::cover_letters::delete_cover_letter,
            commands::cover_letters::check_cl_usage,
            commands::cover_letters::tailor_cover_letter,
            commands::cover_letters::get_tailored_cover_letter,
            commands::cover_letters::get_latest_tailored_cover_letter,
            commands::cover_letters::update_tailored_cover_letter,
            commands::compiler::save_compiler_state,
            commands::compiler::get_compiler_state,
            commands::pdf::compile_resume_to_pdf,
            commands::pdf::compile_workspace_to_pdf,
            commands::pdf::fix_latex_with_ai,
            commands::pdf::refine_latex_with_ai,
            commands::pdf::refine_diagram_with_ai,
            commands::pdf::fix_diagram_with_ai,
            commands::data::export_all_data,
            commands::data::import_data,
            commands::data::auto_local_backup,
            commands::downloads::record_download,
            commands::downloads::get_downloads,
            commands::settings::get_all_themes,
            commands::settings::save_custom_theme,
            commands::settings::delete_theme,
            commands::settings::save_active_theme,
            commands::settings::get_active_theme,
            commands::settings::save_workspace_path,
            commands::settings::get_workspace_path,
            commands::settings::save_last_opened_file,
            commands::settings::get_last_opened_file,
            commands::settings::save_diagram_workspace_path,
            commands::settings::get_diagram_workspace_path,
            commands::settings::save_last_opened_diagram,
            commands::settings::get_last_opened_diagram,
            commands::settings::save_setting,
            commands::settings::get_setting,
            commands::settings::clear_tectonic_cache,
            commands::settings::test_ai_connection,
            commands::inbox::get_all_inbox_jobs,
            commands::inbox::get_inbox_job_by_id,
            commands::inbox::delete_inbox_job,
            commands::inbox::delete_all_inbox_jobs,
            commands::inbox::mark_inbox_job_processed,
            commands::inbox::get_extension_config,
            commands::inbox::reset_extension_secret,
            commands::cloud::test_s3_connection,
            commands::cloud::check_data_dirty,
            commands::cloud::upload_backup_to_s3,
            commands::cloud::list_s3_backups,
            commands::cloud::restore_from_s3,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
