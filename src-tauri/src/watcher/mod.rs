use notify::event::ModifyKind;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;
use tokio::time::{Instant, MissedTickBehavior};

use crate::db::Database;
use crate::jobs::JobQueue;
use crate::models::{VideoItem, WatcherTelemetry};
use crate::scanner::{self, is_video_file};

const WATCHER_QUEUE_CAPACITY: usize = 1024;
const FLUSH_INTERVAL_MS: u64 = 120;
const TELEMETRY_INTERVAL_SECS: u64 = 5;
const RECONCILE_INTERVAL_SECS: u64 = 300;
const RECONCILE_COOLDOWN_SECS: u64 = 20;
const BURST_WINDOW_SECS: u64 = 2;
const MIN_DEBOUNCE_MS: u64 = 220;
const BASE_DEBOUNCE_MS: u64 = 380;
const MAX_DEBOUNCE_MS: u64 = 1800;

#[derive(Clone, Copy)]
enum PendingAction {
    Upsert,
    Remove,
}

struct PendingPathEvent {
    action: PendingAction,
    due_at: Instant,
}

#[derive(Default)]
struct WatcherCounters {
    received_events: u64,
    processed_events: u64,
    debounced_events: u64,
    queue_saturation_events: u64,
    queue_dropped_events: u64,
    current_debounce_ms: u64,
    burst_events_per_sec: usize,
    reconciliation_runs: u64,
    reconciliation_added: u64,
    reconciliation_removed: u64,
    recovery_runs: u64,
}

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
        let (tx, mut rx) = mpsc::channel::<Event>(WATCHER_QUEUE_CAPACITY);
        let counters = Arc::new(Mutex::new(WatcherCounters {
            current_debounce_ms: BASE_DEBOUNCE_MS,
            ..WatcherCounters::default()
        }));
        let recovery_requested = Arc::new(AtomicBool::new(false));
        let reconcile_running = Arc::new(AtomicBool::new(false));

        let db_clone = db.clone();
        let app_clone = app.clone();
        let job_queue_clone = job_queue.clone();
        let watched_paths = Arc::clone(&self.watched_paths);
        let counters_clone = Arc::clone(&counters);
        let recovery_requested_clone = Arc::clone(&recovery_requested);
        let reconcile_running_clone = Arc::clone(&reconcile_running);

        tokio::spawn(async move {
            let mut pending: HashMap<PathBuf, PendingPathEvent> = HashMap::new();
            let mut recent_events: VecDeque<Instant> = VecDeque::new();
            let mut flush_tick = tokio::time::interval(Duration::from_millis(FLUSH_INTERVAL_MS));
            flush_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let mut telemetry_tick =
                tokio::time::interval(Duration::from_secs(TELEMETRY_INTERVAL_SECS));
            telemetry_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let mut reconcile_tick =
                tokio::time::interval(Duration::from_secs(RECONCILE_INTERVAL_SECS));
            reconcile_tick.set_missed_tick_behavior(MissedTickBehavior::Skip);
            let mut last_reconcile = Instant::now() - Duration::from_secs(RECONCILE_COOLDOWN_SECS);

            loop {
                tokio::select! {
                    maybe_event = rx.recv() => {
                        let Some(event) = maybe_event else {
                            break;
                        };

                        let now = Instant::now();
                        track_burst(&mut recent_events, now);
                        let debounce = adaptive_debounce(&recent_events, pending.len());

                        {
                            let mut state = counters_clone.lock();
                            state.current_debounce_ms = debounce.as_millis() as u64;
                            state.burst_events_per_sec = burst_events_per_sec(&recent_events);
                        }

                        queue_event(
                            event,
                            now,
                            debounce,
                            &mut pending,
                            &counters_clone,
                        );
                    }
                    _ = flush_tick.tick() => {
                        let should_recover = recovery_requested_clone.load(Ordering::Relaxed);
                        if should_recover
                            && !reconcile_running_clone.load(Ordering::Relaxed)
                            && last_reconcile.elapsed() >= Duration::from_secs(RECONCILE_COOLDOWN_SECS)
                        {
                            recovery_requested_clone.store(false, Ordering::Relaxed);
                            last_reconcile = Instant::now();
                            schedule_reconciliation(
                                db_clone.clone(),
                                app_clone.clone(),
                                job_queue_clone.clone(),
                                Arc::clone(&watched_paths),
                                Arc::clone(&counters_clone),
                                Arc::clone(&reconcile_running_clone),
                                true,
                            );
                        }

                        process_due_events(
                            &mut pending,
                            &db_clone,
                            &app_clone,
                            &job_queue_clone,
                            &counters_clone,
                        ).await;
                    }
                    _ = telemetry_tick.tick() => {
                        emit_watcher_telemetry(&app_clone, &counters_clone);
                    }
                    _ = reconcile_tick.tick() => {
                        if !reconcile_running_clone.load(Ordering::Relaxed)
                            && last_reconcile.elapsed() >= Duration::from_secs(RECONCILE_COOLDOWN_SECS)
                        {
                            last_reconcile = Instant::now();
                            schedule_reconciliation(
                                db_clone.clone(),
                                app_clone.clone(),
                                job_queue_clone.clone(),
                                Arc::clone(&watched_paths),
                                Arc::clone(&counters_clone),
                                Arc::clone(&reconcile_running_clone),
                                false,
                            );
                        }
                    }
                }
            }
        });

        let tx_clone = tx.clone();
        let counters_for_cb = Arc::clone(&counters);
        let recovery_for_cb = Arc::clone(&recovery_requested);
        let watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                if let Ok(event) = result {
                    let mut state = counters_for_cb.lock();
                    state.received_events += 1;
                    drop(state);

                    if tx_clone.try_send(event).is_err() {
                        let mut state = counters_for_cb.lock();
                        state.queue_saturation_events += 1;
                        state.queue_dropped_events += 1;
                        recovery_for_cb.store(true, Ordering::Relaxed);
                    }
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

    pub fn unwatch(&mut self, path: &Path) -> Result<(), String> {
        if let Some(ref mut watcher) = self.watcher {
            watcher.unwatch(path).map_err(|e| e.to_string())?;

            let mut paths = self.watched_paths.lock();
            paths.retain(|p| p != path);
        }
        Ok(())
    }
}

fn track_burst(recent_events: &mut VecDeque<Instant>, now: Instant) {
    recent_events.push_back(now);
    let keep_for = Duration::from_secs(BURST_WINDOW_SECS);
    while let Some(front) = recent_events.front() {
        if now.duration_since(*front) > keep_for {
            recent_events.pop_front();
        } else {
            break;
        }
    }
}

fn burst_events_per_sec(recent_events: &VecDeque<Instant>) -> usize {
    let window = BURST_WINDOW_SECS as usize;
    if window == 0 {
        return recent_events.len();
    }
    if recent_events.is_empty() {
        0
    } else {
        (recent_events.len() + window - 1) / window.max(1)
    }
}

fn adaptive_debounce(recent_events: &VecDeque<Instant>, pending_paths: usize) -> Duration {
    let burst = recent_events.len();
    let mut debounce_ms = if burst > 700 {
        MAX_DEBOUNCE_MS
    } else if burst > 320 {
        1300
    } else if burst > 120 {
        900
    } else if burst > 40 {
        620
    } else {
        BASE_DEBOUNCE_MS
    };

    if pending_paths > 2500 {
        debounce_ms = debounce_ms.max(1500);
    } else if pending_paths > 1200 {
        debounce_ms = debounce_ms.max(1000);
    } else if pending_paths > 500 {
        debounce_ms = debounce_ms.max(700);
    }

    Duration::from_millis(debounce_ms.clamp(MIN_DEBOUNCE_MS, MAX_DEBOUNCE_MS))
}

fn queue_event(
    event: Event,
    now: Instant,
    debounce: Duration,
    pending: &mut HashMap<PathBuf, PendingPathEvent>,
    counters: &Arc<Mutex<WatcherCounters>>,
) {
    match event.kind {
        EventKind::Modify(ModifyKind::Name(_)) => {
            if event.paths.len() >= 2 {
                queue_pending_action(&event.paths[0], PendingAction::Remove, now, debounce, pending, counters);
                queue_pending_action(&event.paths[event.paths.len() - 1], PendingAction::Upsert, now, debounce, pending, counters);
            } else {
                for path in event.paths {
                    queue_pending_action(&path, PendingAction::Upsert, now, debounce, pending, counters);
                }
            }
        }
        EventKind::Create(_) | EventKind::Modify(_) => {
            for path in event.paths {
                queue_pending_action(&path, PendingAction::Upsert, now, debounce, pending, counters);
            }
        }
        EventKind::Remove(_) => {
            for path in event.paths {
                queue_pending_action(&path, PendingAction::Remove, now, debounce, pending, counters);
            }
        }
        _ => {}
    }
}

fn queue_pending_action(
    path: &Path,
    action: PendingAction,
    now: Instant,
    debounce: Duration,
    pending: &mut HashMap<PathBuf, PendingPathEvent>,
    counters: &Arc<Mutex<WatcherCounters>>,
) {
    if !is_video_file(path) {
        return;
    }

    let due_at = now + debounce;
    if let Some(existing) = pending.get_mut(path) {
        existing.action = action;
        existing.due_at = due_at;
        counters.lock().debounced_events += 1;
        return;
    }

    pending.insert(
        path.to_path_buf(),
        PendingPathEvent { action, due_at },
    );
}

async fn process_due_events(
    pending: &mut HashMap<PathBuf, PendingPathEvent>,
    db: &Database,
    app: &AppHandle,
    job_queue: &JobQueue,
    counters: &Arc<Mutex<WatcherCounters>>,
) {
    if pending.is_empty() {
        return;
    }

    let now = Instant::now();
    let due_paths: Vec<PathBuf> = pending
        .iter()
        .filter_map(|(path, queued)| {
            if queued.due_at <= now {
                Some(path.clone())
            } else {
                None
            }
        })
        .collect();

    if due_paths.is_empty() {
        return;
    }

    let mut should_process_jobs = false;

    for path in due_paths {
        let Some(queued) = pending.remove(&path) else {
            continue;
        };

        match queued.action {
            PendingAction::Upsert => {
                if handle_file_change(db, app, &path).await {
                    should_process_jobs = true;
                }
            }
            PendingAction::Remove => {
                let _ = handle_file_remove(db, app, &path).await;
            }
        }

        counters.lock().processed_events += 1;
    }

    if should_process_jobs {
        let app_clone = app.clone();
        let job_queue_clone = job_queue.clone();
        tokio::spawn(async move {
            job_queue_clone.process_all(app_clone).await;
        });
    }
}

fn emit_watcher_telemetry(app: &AppHandle, counters: &Arc<Mutex<WatcherCounters>>) {
    let state = counters.lock();
    let payload = WatcherTelemetry {
        queue_capacity: WATCHER_QUEUE_CAPACITY,
        received_events: state.received_events,
        processed_events: state.processed_events,
        debounced_events: state.debounced_events,
        queue_saturation_events: state.queue_saturation_events,
        queue_dropped_events: state.queue_dropped_events,
        current_debounce_ms: state.current_debounce_ms,
        burst_events_per_sec: state.burst_events_per_sec,
        reconciliation_runs: state.reconciliation_runs,
        reconciliation_added: state.reconciliation_added,
        reconciliation_removed: state.reconciliation_removed,
        recovery_runs: state.recovery_runs,
    };
    let _ = app.emit("library:watcher_telemetry", payload);
}

fn schedule_reconciliation(
    db: Database,
    app: AppHandle,
    job_queue: JobQueue,
    watched_paths: Arc<Mutex<Vec<PathBuf>>>,
    counters: Arc<Mutex<WatcherCounters>>,
    reconcile_running: Arc<AtomicBool>,
    recovery_run: bool,
) {
    if reconcile_running.swap(true, Ordering::AcqRel) {
        return;
    }

    tokio::spawn(async move {
        let (added, removed) = reconcile_watcher_state(&db, &app, &job_queue, &watched_paths).await;

        {
            let mut state = counters.lock();
            state.reconciliation_runs += 1;
            state.reconciliation_added += added as u64;
            state.reconciliation_removed += removed as u64;
            if recovery_run {
                state.recovery_runs += 1;
            }
        }

        emit_watcher_telemetry(&app, &counters);
        reconcile_running.store(false, Ordering::Release);
    });
}

async fn reconcile_watcher_state(
    db: &Database,
    app: &AppHandle,
    job_queue: &JobQueue,
    watched_paths: &Arc<Mutex<Vec<PathBuf>>>,
) -> (usize, usize) {
    let roots = watched_paths.lock().clone();
    if roots.is_empty() {
        return (0, 0);
    }

    let db_for_scan = db.clone();
    let roots_for_scan = roots.clone();
    let snapshot = tokio::task::spawn_blocking(move || {
        let mut discovered = Vec::new();
        let mut removed = Vec::new();

        for root in roots_for_scan {
            if !root.exists() {
                continue;
            }

            let root_str = root.to_string_lossy().to_string();
            let existing = db_for_scan
                .get_videos_by_folder_prefix(&root_str)
                .unwrap_or_default();

            let mut existing_by_path: HashMap<String, VideoItem> = HashMap::new();
            for video in existing {
                existing_by_path.insert(video.path.clone(), video);
            }

            let mut fs_paths: HashSet<String> = HashSet::new();
            for entry in walkdir::WalkDir::new(&root)
                .follow_links(true)
                .into_iter()
                .filter_map(|entry| entry.ok())
            {
                let path = entry.path();
                if path.is_file() && is_video_file(path) {
                    let path_str = path.to_string_lossy().to_string();
                    fs_paths.insert(path_str.clone());
                    if !existing_by_path.contains_key(&path_str) {
                        discovered.push(PathBuf::from(path_str));
                    }
                }
            }

            for (existing_path, video) in existing_by_path {
                if !fs_paths.contains(&existing_path) {
                    removed.push((video.id, video.thumb_path));
                }
            }
        }

        (discovered, removed)
    })
    .await;

    let Ok((discovered, removed)) = snapshot else {
        return (0, 0);
    };

    let mut added_count = 0usize;
    let mut removed_count = 0usize;
    let mut should_process_jobs = false;

    for path in discovered {
        if handle_file_change(db, app, &path).await {
            added_count += 1;
            should_process_jobs = true;
        }
    }

    for (id, thumb_path) in removed {
        if remove_video_by_id(db, app, &id, thumb_path.as_deref()) {
            removed_count += 1;
        }
    }

    if should_process_jobs {
        let app_clone = app.clone();
        let job_queue_clone = job_queue.clone();
        tokio::spawn(async move {
            job_queue_clone.process_all(app_clone).await;
        });
    }

    (added_count, removed_count)
}

async fn handle_file_change(db: &Database, app: &AppHandle, path: &Path) -> bool {
    if !path.exists() {
        return false;
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
        Err(_) => return false,
    };

    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let video = VideoItem {
        id,
        path: path.to_string_lossy().to_string(),
        folder,
        size_bytes: metadata.len() as i64,
        mtime,
        duration_ms: None,
        width: None,
        height: None,
        aspect_ratio: None,
        codec_name: None,
        favorite: false,
        thumb_path: None,
    };

    if let Err(e) = db.upsert_video(&video) {
        eprintln!("Failed to upsert video from watcher: {}", e);
        return false;
    }

    let _ = app.emit("library:discovered", vec![video]);
    true
}

async fn handle_file_remove(db: &Database, app: &AppHandle, path: &Path) -> bool {
    let id = scanner::compute_video_id(path);

    if let Ok(Some(video)) = db.get_video_by_id(&id) {
        return remove_video_by_id(db, app, &id, video.thumb_path.as_deref());
    }

    false
}

fn remove_video_by_id(db: &Database, app: &AppHandle, id: &str, thumb_path: Option<&str>) -> bool {
    if let Some(thumb) = thumb_path {
        let _ = std::fs::remove_file(thumb);
    }

    if let Err(e) = db.delete_video(id) {
        eprintln!("Failed to delete video from watcher: {}", e);
        return false;
    }

    let _ = app.emit("library:removed", id.to_string());
    true
}

impl Default for FileWatcher {
    fn default() -> Self {
        Self::new()
    }
}
