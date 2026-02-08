use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoItem {
    pub id: String,
    pub path: String,
    pub folder: String,
    pub size_bytes: i64,
    pub mtime: i64,
    pub duration_ms: Option<i64>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub aspect_ratio: Option<f64>,
    pub codec_name: Option<String>,
    pub favorite: bool,
    pub thumb_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub total: usize,
    pub processed: usize,
    pub current_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub autoplay: bool,
    pub density: f64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            autoplay: true,
            density: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanBatchTelemetry {
    pub folder: String,
    pub batch_index: usize,
    pub batch_size: usize,
    pub db_write_ms: u128,
    pub emit_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiActivityHints {
    pub priority_ids: Vec<String>,
    pub is_scrolling: bool,
    pub scroll_velocity: f64,
    pub estimated_tile_width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobFailureCounters {
    pub metadata_probe_failed: u64,
    pub metadata_update_failed: u64,
    pub thumb_generate_failed: u64,
    pub thumb_update_failed: u64,
    pub metadata_retry_exhausted: u64,
    pub thumb_retry_exhausted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JobTelemetry {
    pub metadata_candidates: usize,
    pub metadata_processed: usize,
    pub thumbnail_candidates: usize,
    pub thumbnail_processed: usize,
    pub thumbnail_batch_limit: usize,
    pub prioritized_ids: usize,
    pub ui_scrolling: bool,
    pub scroll_velocity: f64,
    pub estimated_tile_width: i32,
    pub failures: JobFailureCounters,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanCursors {
    pub folders: HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WatcherTelemetry {
    pub queue_capacity: usize,
    pub received_events: u64,
    pub processed_events: u64,
    pub debounced_events: u64,
    pub queue_saturation_events: u64,
    pub queue_dropped_events: u64,
    pub current_debounce_ms: u64,
    pub burst_events_per_sec: usize,
    pub reconciliation_runs: u64,
    pub reconciliation_added: u64,
    pub reconciliation_removed: u64,
    pub recovery_runs: u64,
}
