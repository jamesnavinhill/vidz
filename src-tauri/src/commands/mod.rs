use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::db::Database;
use crate::jobs::JobQueue;
use crate::models::{AppSettings, ScanProgress, UiActivityHints, VideoItem};
use crate::scanner;
use crate::watcher::FileWatcher;

pub struct AppState {
    pub db: Database,
    pub watcher: Mutex<FileWatcher>,
    pub job_queue: JobQueue,
    pub scan_cancel: Arc<Mutex<bool>>,
}

#[tauri::command]
pub async fn get_library(state: State<'_, Arc<AppState>>) -> Result<Vec<VideoItem>, String> {
    state.db.get_all_videos().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_favorite(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    id: String,
    favorite: bool,
) -> Result<(), String> {
    state
        .db
        .set_favorite(&id, favorite)
        .map_err(|e| e.to_string())?;

    if let Ok(Some(video)) = state.db.get_video_by_id(&id) {
        let _ = app.emit("library:updated", vec![video]);
    }

    Ok(())
}

#[tauri::command]
pub async fn scan_directories(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    paths: Vec<String>,
) -> Result<Vec<VideoItem>, String> {
    run_scan_inner(state.inner().clone(), app, paths, false).await
}

async fn run_scan_inner(
    state: Arc<AppState>,
    app: AppHandle,
    paths: Vec<String>,
    incremental: bool,
) -> Result<Vec<VideoItem>, String> {
    let db = state.db.clone();
    let app_handle = app.clone();
    let paths_clone = paths;
    let cancel_state = Arc::clone(&state.scan_cancel);
    {
        let mut cancel = cancel_state.lock();
        *cancel = false;
    }

    let result = tokio::task::spawn_blocking(move || {
        let mut all_videos = Vec::new();
        let mut cursors = db.get_scan_cursors().unwrap_or_default();
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        for path in &paths_clone {
            let root = PathBuf::from(path);
            if !root.exists() {
                continue;
            }

            let app_progress = app_handle.clone();
            let app_discovered = app_handle.clone();
            let app_batch = app_handle.clone();
            let cancel_guard = Arc::clone(&cancel_state);
            let cursor = if incremental {
                cursors.get(path).copied()
            } else {
                None
            };
            let (videos, cancelled) = scanner::scan_directory(
                &db,
                &root,
                cursor,
                || *cancel_guard.lock(),
                |processed, total, current| {
                    let _ = app_progress.emit(
                        "library:scan_progress",
                        ScanProgress {
                            total,
                            processed,
                            current_file: Some(current.to_string()),
                        },
                    );
                },
                |batch| {
                    let _ = app_discovered.emit("library:discovered", batch.to_vec());
                },
                |telemetry| {
                    let _ = app_batch.emit("library:scan_batch", telemetry);
                },
            )?;

            if cancelled {
                let mut cancel = cancel_guard.lock();
                *cancel = true;
            } else {
                cursors.insert(path.clone(), now_epoch);
            }

            all_videos.extend(videos);
        }

        let _ = db.save_scan_cursors(&cursors);

        Ok::<_, String>(all_videos)
    })
    .await
    .map_err(|e| e.to_string())??;

    let cancelled = *state.scan_cancel.lock();
    if cancelled {
        let _ = app.emit("library:scan_cancelled", ());
        state.job_queue.clear_running();
    }
    let _ = app.emit("library:scan_finished", ());

    Ok(result)
}

#[tauri::command]
pub async fn cancel_scan(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut cancel = state.scan_cancel.lock();
    *cancel = true;
    Ok(())
}

#[tauri::command]
pub async fn add_watched_folder(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    path: String,
) -> Result<Vec<VideoItem>, String> {
    let mut folders = state.db.get_watched_folders().map_err(|e| e.to_string())?;

    if !folders.contains(&path) {
        folders.push(path.clone());
        state
            .db
            .save_watched_folders(&folders)
            .map_err(|e| e.to_string())?;
        let _ = app.emit("library:watched_folders_updated", folders.clone());
    }

    {
        let mut watcher = state.watcher.lock();
        watcher.watch(PathBuf::from(&path)).ok();
    }

    let videos = run_scan_inner(state.inner().clone(), app.clone(), vec![path], false).await?;

    let job_queue = state.job_queue.clone();
    let app_clone = app.clone();
    tokio::spawn(async move {
        job_queue.process_all(app_clone).await;
    });

    Ok(videos)
}

#[tauri::command]
pub async fn scan_watched_folders_incremental(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
) -> Result<Vec<VideoItem>, String> {
    let folders = state.db.get_watched_folders().map_err(|e| e.to_string())?;
    if folders.is_empty() {
        return Ok(Vec::new());
    }

    run_scan_inner(state.inner().clone(), app, folders, true).await
}

#[tauri::command]
pub async fn get_watched_folders(state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    state.db.get_watched_folders().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_watched_folder(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
    path: String,
) -> Result<(), String> {
    let mut folders = state.db.get_watched_folders().map_err(|e| e.to_string())?;
    folders.retain(|f| f != &path);
    state
        .db
        .save_watched_folders(&folders)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("library:watched_folders_updated", folders.clone());

    let videos = state
        .db
        .get_videos_by_folder_prefix(&path)
        .map_err(|e| e.to_string())?;

    {
        let mut watcher = state.watcher.lock();
        watcher.unwatch(&PathBuf::from(&path)).ok();
    }

    for video in &videos {
        if let Some(thumb) = &video.thumb_path {
            let _ = std::fs::remove_file(thumb);
        }
    }

    state
        .db
        .delete_videos_by_folder_prefix(&path)
        .map_err(|e| e.to_string())?;

    let removed_ids: Vec<String> = videos.into_iter().map(|video| video.id).collect();
    if !removed_ids.is_empty() {
        let _ = app.emit("library:removed_bulk", removed_ids);
    }

    Ok(())
}

#[tauri::command]
pub async fn process_pending_jobs(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let job_queue = state.job_queue.clone();
    tokio::spawn(async move {
        job_queue.process_all(app).await;
    });
    Ok(())
}

#[tauri::command]
pub async fn update_ui_activity(
    state: State<'_, Arc<AppState>>,
    hints: UiActivityHints,
) -> Result<(), String> {
    state.job_queue.update_ui_hints(hints);
    Ok(())
}

#[tauri::command]
pub async fn start_file_watcher(
    state: State<'_, Arc<AppState>>,
    app: AppHandle,
) -> Result<(), String> {
    let folders = state.db.get_watched_folders().map_err(|e| e.to_string())?;

    {
        let mut watcher = state.watcher.lock();
        watcher.start(state.db.clone(), app.clone(), state.job_queue.clone())?;

        for folder in folders {
            watcher.watch(PathBuf::from(folder)).ok();
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn get_app_settings(state: State<'_, Arc<AppState>>) -> Result<AppSettings, String> {
    state.db.get_app_settings().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_app_settings(
    state: State<'_, Arc<AppState>>,
    settings: AppSettings,
) -> Result<(), String> {
    state
        .db
        .save_app_settings(&settings)
        .map_err(|e| e.to_string())
}

pub fn get_ffprobe_path(app: &AppHandle) -> PathBuf {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("bin").join("ffprobe.exe");
        if bundled.exists() {
            return bundled;
        }
    }
    PathBuf::from("ffprobe")
}

pub fn get_ffmpeg_path(app: &AppHandle) -> PathBuf {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir.join("bin").join("ffmpeg.exe");
        if bundled.exists() {
            return bundled;
        }
    }
    PathBuf::from("ffmpeg")
}
