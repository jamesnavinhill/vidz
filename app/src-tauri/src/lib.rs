mod commands;
mod db;
mod jobs;
mod models;
mod scanner;
mod watcher;

use std::sync::Arc;
use parking_lot::Mutex;
use tauri::Manager;
use commands::{get_ffmpeg_path, get_ffprobe_path, AppState};
use db::Database;
use jobs::JobQueue;
use watcher::FileWatcher;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let db = Database::new().expect("Failed to initialize database");
            
            let ffprobe_path = get_ffprobe_path(&app.handle());
            let ffmpeg_path = get_ffmpeg_path(&app.handle());
            let job_queue = JobQueue::new(db.clone(), ffprobe_path, ffmpeg_path);

            if let Ok(resource_dir) = app.path().resource_dir() {
                let ffprobe_bundled = resource_dir.join("bin").join("ffprobe.exe");
                if !ffprobe_bundled.exists() {
                    eprintln!("Bundled ffprobe not found at {}", ffprobe_bundled.display());
                }

                let ffmpeg_bundled = resource_dir.join("bin").join("ffmpeg.exe");
                if !ffmpeg_bundled.exists() {
                    eprintln!("Bundled ffmpeg not found at {}", ffmpeg_bundled.display());
                }
            }
            
            let state = Arc::new(AppState {
                db,
                watcher: Mutex::new(FileWatcher::new()),
                job_queue,
                scan_cancel: Arc::new(Mutex::new(false)),
            });
            
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_library,
            commands::set_favorite,
            commands::scan_directories,
            commands::cancel_scan,
            commands::add_watched_folder,
            commands::get_watched_folders,
            commands::remove_watched_folder,
            commands::process_pending_jobs,
            commands::start_file_watcher,
            commands::get_app_settings,
            commands::save_app_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
