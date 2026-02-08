export interface VideoItem {
  id: string;
  path: string;
  folder: string;
  size_bytes: number;
  mtime: number;
  duration_ms: number | null;
  width: number | null;
  height: number | null;
  aspect_ratio: number | null;
  codec_name: string | null;
  favorite: boolean;
  thumb_path: string | null;
}

export interface ScanProgress {
  total: number;
  processed: number;
  current_file: string | null;
}

export type SortMode = 'size' | 'resolution' | 'aspect' | 'duration' | 'folder' | 'favorites';

export type SortDirection = 'asc' | 'desc';

export interface AppSettings {
  autoplay: boolean;
  density: number;
}

export interface ScanBatchTelemetry {
  folder: string;
  batch_index: number;
  batch_size: number;
  db_write_ms: number;
  emit_ms: number;
}

export interface JobFailureCounters {
  metadata_probe_failed: number;
  metadata_update_failed: number;
  thumb_generate_failed: number;
  thumb_update_failed: number;
  metadata_retry_exhausted: number;
  thumb_retry_exhausted: number;
}

export interface JobTelemetry {
  metadata_candidates: number;
  metadata_processed: number;
  thumbnail_candidates: number;
  thumbnail_processed: number;
  thumbnail_batch_limit: number;
  prioritized_ids: number;
  ui_scrolling: boolean;
  scroll_velocity: number;
  estimated_tile_width: number;
  failures: JobFailureCounters;
}

export interface WatcherTelemetry {
  queue_capacity: number;
  received_events: number;
  processed_events: number;
  debounced_events: number;
  queue_saturation_events: number;
  queue_dropped_events: number;
  current_debounce_ms: number;
  burst_events_per_sec: number;
  reconciliation_runs: number;
  reconciliation_added: number;
  reconciliation_removed: number;
  recovery_runs: number;
}
