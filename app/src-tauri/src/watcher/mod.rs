use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::db::Database;
use crate::jobs::JobQueue;
use crate::models::VideoItem;
use crate::scanner::{self, is_video_file};

pub struct FileWatcher {
    watcher: Option<RecommendedWatcher>,
    watched_paths: Arc<Mutex<Vec<PathBuf>>>,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            watcher: None,
            watched_paths: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn start(
        &mut self,
        db: Database,
        app: AppHandle,
        job_queue: JobQueue,
    ) -> Result<(), String> {
        let (tx, mut rx) = mpsc::channel::<Event>(100);

        let db_clone = db.clone();
        let app_clone = app.clone();
        let job_queue_clone = job_queue.clone();

        tokio::spawn(async move {
            let mut debounce_map: std::collections::HashMap<PathBuf, tokio::time::Instant> =
                std::collections::HashMap::new();
            let debounce_duration = Duration::from_millis(500);

            while let Some(event) = rx.recv().await {
                for path in event.paths {
                    if !is_video_file(&path) {
                        continue;
                    }

                    let now = tokio::time::Instant::now();
                    if let Some(last) = debounce_map.get(&path) {
                        if now.duration_since(*last) < debounce_duration {
                            continue;
                        }
                    }
                    debounce_map.insert(path.clone(), now);

                    match event.kind {
                        EventKind::Create(_) | EventKind::Modify(_) => {
                            handle_file_change(&db_clone, &app_clone, &job_queue_clone, &path)
                                .await;
                        }
                        EventKind::Remove(_) => {
                            handle_file_remove(&db_clone, &app_clone, &path).await;
                        }
                        _ => {}
                    }
                }
            }
        });

        let tx_clone = tx.clone();
        let watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let _ = tx_clone.blocking_send(event);
                }
            },
            Config::default().with_poll_interval(Duration::from_secs(2)),
        )
        .map_err(|e| e.to_string())?;

        self.watcher = Some(watcher);
        Ok(())
    }

    pub fn watch(&mut self, path: PathBuf) -> Result<(), String> {
        if let Some(ref mut watcher) = self.watcher {
            watcher
                .watch(&path, RecursiveMode::Recursive)
                .map_err(|e| e.to_string())?;

            let mut paths = self.watched_paths.lock();
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
        Ok(())
    }

    pub fn unwatch(&mut self, path: &PathBuf) -> Result<(), String> {
        if let Some(ref mut watcher) = self.watcher {
            watcher.unwatch(path).map_err(|e| e.to_string())?;

            let mut paths = self.watched_paths.lock();
            paths.retain(|p| p != path);
        }
        Ok(())
    }
}

async fn handle_file_change(
    db: &Database,
    app: &AppHandle,
    job_queue: &JobQueue,
    path: &PathBuf,
) {
    if !path.exists() {
        return;
    }

    for _ in 0..3 {
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > 0 {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let id = scanner::compute_video_id(path);
    let folder = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let metadata = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return,
    };

    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let video = VideoItem {
        id: id.clone(),
        path: path.to_string_lossy().to_string(),
        folder,
        size_bytes: metadata.len() as i64,
        mtime,
        duration_ms: None,
        width: None,
        height: None,
        aspect_ratio: None,
        favorite: false,
        thumb_path: None,
    };

    if let Err(e) = db.upsert_video(&video) {
        eprintln!("Failed to upsert video from watcher: {}", e);
        return;
    }

    let _ = app.emit("library:discovered", vec![video]);

    let app_clone = app.clone();
    let job_queue_clone = job_queue.clone();
    tokio::spawn(async move {
        job_queue_clone.process_all(app_clone).await;
    });
}

async fn handle_file_remove(db: &Database, app: &AppHandle, path: &PathBuf) {
    let id = scanner::compute_video_id(path);

    if let Ok(Some(video)) = db.get_video_by_id(&id) {
        if let Some(thumb) = &video.thumb_path {
            let _ = std::fs::remove_file(thumb);
        }
    }

    if let Err(e) = db.delete_video(&id) {
        eprintln!("Failed to delete video from watcher: {}", e);
        return;
    }

    let _ = app.emit("library:removed", id);
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}
