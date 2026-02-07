use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

use crate::db::Database;
use crate::models::VideoItem;

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "webm", "avi", "mov", "wmv", "flv", "m4v", "mpg", "mpeg", "3gp",
];
const SCAN_DB_BATCH_SIZE: usize = 128;
const SCAN_EVENT_BATCH_SIZE: usize = 64;

#[cfg(windows)]
fn apply_no_window(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn apply_no_window(_command: &mut std::process::Command) {}

pub fn compute_video_id(path: &Path) -> String {
    let canonical = path.to_string_lossy();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..16])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_video_id_is_stable() {
        let path = Path::new("C:/videos/example.mp4");
        let first = compute_video_id(path);
        let second = compute_video_id(path);
        assert_eq!(first, second);
    }
}

pub fn is_video_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
        .unwrap_or(false)
}

pub fn scan_directory(
    db: &Database,
    root: &Path,
    should_cancel: impl Fn() -> bool,
    on_progress: impl Fn(usize, usize, &str),
    on_discovered_batch: impl Fn(&[VideoItem]),
) -> Result<(Vec<VideoItem>, bool), String> {
    let mut videos = Vec::new();
    let mut entries: Vec<PathBuf> = Vec::new();
    let mut pending_db = Vec::with_capacity(SCAN_DB_BATCH_SIZE);
    let mut pending_events = Vec::with_capacity(SCAN_EVENT_BATCH_SIZE);

    for entry in WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() && is_video_file(path) {
            entries.push(path.to_path_buf());
        }
    }

    let total = entries.len();

    let mut cancelled = false;
    for (idx, path) in entries.iter().enumerate() {
        if should_cancel() {
            cancelled = true;
            break;
        }
        on_progress(idx + 1, total, &path.to_string_lossy());

        if let Some(video) = create_video_item(path) {
            pending_db.push(video.clone());
            pending_events.push(video.clone());
            videos.push(video);

            if pending_db.len() >= SCAN_DB_BATCH_SIZE {
                flush_db_batch(db, &mut pending_db);
            }
            if pending_events.len() >= SCAN_EVENT_BATCH_SIZE {
                on_discovered_batch(&pending_events);
                pending_events.clear();
            }
        }
    }

    flush_db_batch(db, &mut pending_db);
    if !pending_events.is_empty() {
        on_discovered_batch(&pending_events);
    }

    Ok((videos, cancelled))
}

fn flush_db_batch(db: &Database, pending_db: &mut Vec<VideoItem>) {
    if pending_db.is_empty() {
        return;
    }

    if let Err(e) = db.upsert_videos_batch(pending_db) {
        eprintln!("Failed to batch upsert videos: {}", e);
        for video in pending_db.iter() {
            if let Err(single_err) = db.upsert_video(video) {
                eprintln!(
                    "Failed to recover upsert for video {}: {}",
                    video.path, single_err
                );
            }
        }
    }

    pending_db.clear();
}

fn create_video_item(path: &Path) -> Option<VideoItem> {
    let metadata = std::fs::metadata(path).ok()?;
    let mtime = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;

    let folder = path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    Some(VideoItem {
        id: compute_video_id(path),
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
    })
}

pub fn extract_metadata(video_path: &str, ffprobe_path: &Path) -> Option<(i64, i32, i32)> {
    use std::process::Command;

    let mut command = Command::new(ffprobe_path);
    apply_no_window(&mut command);
    let output = command
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            video_path,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;

    let duration_ms = json["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .map(|d| (d * 1000.0) as i64)?;

    let streams = json["streams"].as_array()?;
    let video_stream = streams.iter().find(|s| s["codec_type"] == "video")?;

    let width = video_stream["width"].as_i64()? as i32;
    let height = video_stream["height"].as_i64()? as i32;

    Some((duration_ms, width, height))
}

pub fn generate_thumbnail(
    video_path: &str,
    thumb_path: &Path,
    ffmpeg_path: &Path,
) -> Result<(), String> {
    use std::process::Command;

    if let Some(parent) = thumb_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut command = Command::new(ffmpeg_path);
    apply_no_window(&mut command);
    let output = command
        .args([
            "-y",
            "-i",
            video_path,
            "-vframes",
            "1",
            "-q:v",
            "2",
            "-vf",
            "scale=320:-1",
            &thumb_path.to_string_lossy(),
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
