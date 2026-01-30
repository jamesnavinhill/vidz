use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;

use crate::db::Database;
use crate::scanner;

pub struct JobQueue {
    db: Database,
    ffprobe_path: PathBuf,
    ffmpeg_path: PathBuf,
    metadata_semaphore: Arc<Semaphore>,
    thumbnail_semaphore: Arc<Semaphore>,
    running: Arc<Mutex<bool>>,
}

impl JobQueue {
    pub fn new(db: Database, ffprobe_path: PathBuf, ffmpeg_path: PathBuf) -> Self {
        Self {
            db,
            ffprobe_path,
            ffmpeg_path,
            metadata_semaphore: Arc::new(Semaphore::new(4)),
            thumbnail_semaphore: Arc::new(Semaphore::new(2)),
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub async fn process_all(&self, app: AppHandle) {
        {
            let mut running = self.running.lock();
            if *running {
                return;
            }
            *running = true;
        }

        self.process_metadata(&app).await;
        self.process_thumbnails(&app).await;

        {
            let mut running = self.running.lock();
            *running = false;
        }
    }

    async fn process_metadata(&self, app: &AppHandle) {
        let videos = match self.db.get_videos_needing_metadata() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to get videos needing metadata: {}", e);
                return;
            }
        };

        let mut handles = Vec::new();

        for video in videos {
            let permit = self.metadata_semaphore.clone().acquire_owned().await;
            let db = self.db.clone();
            let ffprobe = self.ffprobe_path.clone();
            let app_clone = app.clone();
            let video_id = video.id.clone();
            let video_path = video.path.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;

                if let Some((duration_ms, width, height)) =
                    scanner::extract_metadata(&video_path, &ffprobe)
                {
                    if let Err(e) = db.update_metadata(&video_id, duration_ms, width, height) {
                        eprintln!("Failed to update metadata for {}: {}", video_path, e);
                    } else if let Ok(Some(updated)) = db.get_video_by_id(&video_id) {
                        let _ = app_clone.emit("library:updated", vec![updated]);
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }
    }

    async fn process_thumbnails(&self, app: &AppHandle) {
        let videos = match self.db.get_videos_needing_thumbs() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Failed to get videos needing thumbs: {}", e);
                return;
            }
        };

        let thumbs_dir = Database::get_thumbs_dir();
        let mut handles = Vec::new();

        for video in videos {
            let permit = self.thumbnail_semaphore.clone().acquire_owned().await;
            let db = self.db.clone();
            let ffmpeg = self.ffmpeg_path.clone();
            let app_clone = app.clone();
            let video_id = video.id.clone();
            let video_path = video.path.clone();
            let thumb_path = thumbs_dir.join(format!("{}.jpg", video.id));

            let handle = tokio::spawn(async move {
                let _permit = permit;

                if let Err(e) = scanner::generate_thumbnail(&video_path, &thumb_path, &ffmpeg) {
                    eprintln!("Failed to generate thumbnail for {}: {}", video_path, e);
                } else {
                    let thumb_str = thumb_path.to_string_lossy().to_string();
                    if let Err(e) = db.update_thumb(&video_id, &thumb_str) {
                        eprintln!("Failed to update thumb path for {}: {}", video_path, e);
                    } else if let Ok(Some(updated)) = db.get_video_by_id(&video_id) {
                        let _ = app_clone.emit("library:updated", vec![updated]);
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }
    }
}

impl Clone for JobQueue {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            ffprobe_path: self.ffprobe_path.clone(),
            ffmpeg_path: self.ffmpeg_path.clone(),
            metadata_semaphore: Arc::clone(&self.metadata_semaphore),
            thumbnail_semaphore: Arc::clone(&self.thumbnail_semaphore),
            running: Arc::clone(&self.running),
        }
    }
}
