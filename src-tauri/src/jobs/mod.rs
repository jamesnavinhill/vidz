use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::sync::Semaphore;
use tokio::time::{sleep, Duration};

use crate::db::Database;
use crate::models::{JobFailureCounters, JobTelemetry, UiActivityHints, VideoItem};
use crate::scanner;

const METADATA_RETRY_BUDGET: usize = 2;
const THUMB_RETRY_BUDGET: usize = 2;
const DEFAULT_METADATA_BATCH_LIMIT: usize = 512;
const SCROLLING_METADATA_BATCH_LIMIT: usize = 128;
const SCROLLING_THUMB_BATCH_LIMIT: usize = 16;
const FAST_SCROLLING_THUMB_BATCH_LIMIT: usize = 8;

pub struct JobQueue {
    db: Database,
    ffprobe_path: PathBuf,
    ffmpeg_path: PathBuf,
    metadata_semaphore: Arc<Semaphore>,
    thumbnail_semaphore: Arc<Semaphore>,
    ui_hints: Arc<Mutex<UiActivityHints>>,
    failure_counters: Arc<Mutex<JobFailureCounters>>,
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
            ui_hints: Arc::new(Mutex::new(UiActivityHints::default())),
            failure_counters: Arc::new(Mutex::new(JobFailureCounters::default())),
            running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn update_ui_hints(&self, hints: UiActivityHints) {
        let mut guard = self.ui_hints.lock();
        *guard = hints;
    }

    pub async fn process_all(&self, app: AppHandle) {
        {
            let mut running = self.running.lock();
            if *running {
                return;
            }
            *running = true;
        }

        let hints = { self.ui_hints.lock().clone() };
        let (metadata_candidates, metadata_processed) = self.process_metadata(&app, &hints).await;
        let (thumbnail_candidates, thumbnail_processed, thumbnail_batch_limit) =
            self.process_thumbnails(&app, &hints).await;

        let failures = self.failure_counters.lock().clone();
        let telemetry = JobTelemetry {
            metadata_candidates,
            metadata_processed,
            thumbnail_candidates,
            thumbnail_processed,
            thumbnail_batch_limit,
            prioritized_ids: hints.priority_ids.len(),
            ui_scrolling: hints.is_scrolling,
            scroll_velocity: hints.scroll_velocity,
            estimated_tile_width: hints.estimated_tile_width,
            failures,
        };
        let _ = app.emit("library:job_telemetry", telemetry);

        {
            let mut running = self.running.lock();
            *running = false;
        }
    }

    pub fn clear_running(&self) {
        let mut running = self.running.lock();
        *running = false;
    }

    async fn process_metadata(&self, app: &AppHandle, hints: &UiActivityHints) -> (usize, usize) {
        let videos = match self.db.get_videos_needing_metadata() {
            Ok(v) => v,
            Err(e) => {
                let _ = app.emit(
                    "library:warning",
                    format!("Failed to get videos needing metadata: {}", e),
                );
                return (0, 0);
            }
        };

        let mut videos = prioritize_jobs(videos, &hints.priority_ids);
        let batch_limit = if hints.is_scrolling {
            SCROLLING_METADATA_BATCH_LIMIT
        } else {
            DEFAULT_METADATA_BATCH_LIMIT
        };
        if videos.len() > batch_limit {
            videos.truncate(batch_limit);
        }

        let candidates = videos.len();
        let processed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut handles = Vec::new();

        for video in videos {
            let permit = self.metadata_semaphore.clone().acquire_owned().await;
            let db = self.db.clone();
            let ffprobe = self.ffprobe_path.clone();
            let app_clone = app.clone();
            let video_id = video.id.clone();
            let video_path = video.path.clone();
            let processed_clone = Arc::clone(&processed);
            let failure_counters = Arc::clone(&self.failure_counters);

            let handle = tokio::spawn(async move {
                let _permit = permit;
                let mut last_error: Option<String> = None;
                let mut completed = false;

                for attempt in 0..=METADATA_RETRY_BUDGET {
                    if let Some((duration_ms, width, height, codec_name)) =
                        scanner::extract_metadata(&video_path, &ffprobe)
                    {
                        match db.update_metadata(
                            &video_id,
                            duration_ms,
                            width,
                            height,
                            codec_name.as_deref(),
                        ) {
                            Ok(()) => {
                                if let Ok(Some(updated)) = db.get_video_by_id(&video_id) {
                                    let _ = app_clone.emit("library:updated", vec![updated]);
                                }
                                processed_clone
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                completed = true;
                                break;
                            }
                            Err(e) => {
                                last_error = Some(e.to_string());
                                if attempt == METADATA_RETRY_BUDGET {
                                    let mut counters = failure_counters.lock();
                                    counters.metadata_update_failed += 1;
                                    counters.metadata_retry_exhausted += 1;
                                    break;
                                } else {
                                    sleep(Duration::from_millis(120 * (attempt as u64 + 1))).await;
                                }
                            }
                        }
                        continue;
                    }
                    if attempt == METADATA_RETRY_BUDGET {
                        let mut counters = failure_counters.lock();
                        counters.metadata_probe_failed += 1;
                        counters.metadata_retry_exhausted += 1;
                        break;
                    }
                    sleep(Duration::from_millis(120 * (attempt as u64 + 1))).await;
                }

                if !completed {
                    let message = if let Some(err) = last_error {
                        format!("Metadata update failed for {}: {}", video_path, err)
                    } else {
                        format!("Metadata probe failed for {}", video_path)
                    };
                    let _ = app_clone.emit("library:warning", message);
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        (candidates, processed.load(std::sync::atomic::Ordering::Relaxed))
    }

    async fn process_thumbnails(
        &self,
        app: &AppHandle,
        hints: &UiActivityHints,
    ) -> (usize, usize, usize) {
        let videos = match self.db.get_videos_needing_thumbs() {
            Ok(v) => v,
            Err(e) => {
                let _ = app.emit(
                    "library:warning",
                    format!("Failed to get videos needing thumbs: {}", e),
                );
                return (0, 0, 0);
            }
        };

        let mut videos = prioritize_jobs(videos, &hints.priority_ids);
        let priority_set: HashSet<String> = hints.priority_ids.iter().cloned().collect();
        if hints.is_scrolling && !priority_set.is_empty() {
            let prioritized: Vec<VideoItem> = videos
                .iter()
                .filter(|video| priority_set.contains(&video.id))
                .cloned()
                .collect();
            if !prioritized.is_empty() {
                videos = prioritized;
            }
        }

        let thumb_batch_limit = if hints.is_scrolling {
            if hints.scroll_velocity > 2.0 {
                FAST_SCROLLING_THUMB_BATCH_LIMIT
            } else {
                SCROLLING_THUMB_BATCH_LIMIT
            }
        } else {
            videos.len()
        };
        if videos.len() > thumb_batch_limit {
            videos.truncate(thumb_batch_limit);
        }

        let dynamic_run_limit = if hints.is_scrolling && hints.scroll_velocity > 1.2 {
            1usize
        } else {
            2usize
        };
        let run_semaphore = Arc::new(Semaphore::new(dynamic_run_limit));
        let (target_width, jpeg_quality) = thumbnail_params(hints.estimated_tile_width);
        let candidates = videos.len();
        let processed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let thumbs_dir = Database::get_thumbs_dir();
        let mut handles = Vec::new();

        for video in videos {
            let global_permit = self.thumbnail_semaphore.clone().acquire_owned().await;
            let run_permit = run_semaphore.clone().acquire_owned().await;
            let db = self.db.clone();
            let ffmpeg = self.ffmpeg_path.clone();
            let app_clone = app.clone();
            let video_id = video.id.clone();
            let video_path = video.path.clone();
            let thumb_path = thumbs_dir.join(format!("{}.jpg", video.id));
            let processed_clone = Arc::clone(&processed);
            let failure_counters = Arc::clone(&self.failure_counters);

            let handle = tokio::spawn(async move {
                let _global_permit = global_permit;
                let _run_permit = run_permit;
                let mut last_error: Option<String> = None;
                let mut completed = false;

                for attempt in 0..=THUMB_RETRY_BUDGET {
                    match scanner::generate_thumbnail(
                        &video_path,
                        &thumb_path,
                        &ffmpeg,
                        target_width,
                        jpeg_quality,
                    ) {
                        Ok(()) => {
                            let thumb_str = thumb_path.to_string_lossy().to_string();
                            if let Err(e) = db.update_thumb(&video_id, &thumb_str) {
                                last_error = Some(format!(
                                    "Failed to update thumbnail for {}: {}",
                                    video_path, e
                                ));
                                if attempt == THUMB_RETRY_BUDGET {
                                    let mut counters = failure_counters.lock();
                                    counters.thumb_update_failed += 1;
                                    counters.thumb_retry_exhausted += 1;
                                    break;
                                } else {
                                    sleep(Duration::from_millis(160 * (attempt as u64 + 1))).await;
                                }
                            } else if let Ok(Some(updated)) = db.get_video_by_id(&video_id) {
                                let _ = app_clone.emit("library:updated", vec![updated]);
                                processed_clone
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                completed = true;
                                break;
                            } else {
                                completed = true;
                                break;
                            }
                        }
                        Err(e) => {
                            last_error = Some(format!(
                                "Thumbnail generation failed for {}: {}",
                                video_path, e
                            ));
                            if attempt == THUMB_RETRY_BUDGET {
                                let mut counters = failure_counters.lock();
                                counters.thumb_generate_failed += 1;
                                counters.thumb_retry_exhausted += 1;
                                break;
                            } else {
                                sleep(Duration::from_millis(160 * (attempt as u64 + 1))).await;
                            }
                        }
                    }
                }

                if !completed {
                    if let Some(err) = last_error {
                        let _ = app_clone.emit("library:warning", err);
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }

        (
            candidates,
            processed.load(std::sync::atomic::Ordering::Relaxed),
            thumb_batch_limit,
        )
    }
}

fn prioritize_jobs(mut videos: Vec<VideoItem>, priority_ids: &[String]) -> Vec<VideoItem> {
    if priority_ids.is_empty() {
        return videos;
    }

    let mut priority_index = HashMap::new();
    for (idx, id) in priority_ids.iter().enumerate() {
        priority_index.insert(id.clone(), idx);
    }

    videos.sort_by_key(|video| {
        (
            priority_index.get(&video.id).copied().unwrap_or(usize::MAX),
            video.path.clone(),
        )
    });

    videos
}

fn thumbnail_params(estimated_tile_width: i32) -> (i32, i32) {
    let width = if estimated_tile_width > 0 {
        estimated_tile_width
    } else {
        240
    };
    let target_width = ((width as f64) * 1.35).round() as i32;
    let target_width = target_width.clamp(180, 640);
    let jpeg_quality = if target_width <= 260 {
        4
    } else if target_width <= 420 {
        5
    } else {
        6
    };

    (target_width, jpeg_quality)
}

impl Clone for JobQueue {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            ffprobe_path: self.ffprobe_path.clone(),
            ffmpeg_path: self.ffmpeg_path.clone(),
            metadata_semaphore: Arc::clone(&self.metadata_semaphore),
            thumbnail_semaphore: Arc::clone(&self.thumbnail_semaphore),
            ui_hints: Arc::clone(&self.ui_hints),
            failure_counters: Arc::clone(&self.failure_counters),
            running: Arc::clone(&self.running),
        }
    }
}
