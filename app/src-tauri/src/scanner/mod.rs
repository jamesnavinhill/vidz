use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use sha2::{Sha256, Digest};
use walkdir::WalkDir;

use crate::db::Database;
use crate::models::VideoItem;

const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "webm", "avi", "mov", "wmv", "flv", "m4v", "mpg", "mpeg", "3gp"
];

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
) -> Result<(Vec<VideoItem>, bool), String> {
    let mut videos = Vec::new();
    let mut entries: Vec<PathBuf> = Vec::new();

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
        on_progress(idx, total, &path.to_string_lossy());
        
        if let Some(video) = create_video_item(path) {
            if let Err(e) = db.upsert_video(&video) {
                eprintln!("Failed to upsert video {}: {}", path.display(), e);
            } else {
                videos.push(video);
            }
        }
    }

    Ok((videos, cancelled))
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
    
    let output = Command::new(ffprobe_path)
        .args([
            "-v", "quiet",
            "-print_format", "json",
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

    let output = Command::new(ffmpeg_path)
        .args([
            "-y",
            "-i", video_path,
            "-vframes", "1",
            "-q:v", "2",
            "-vf", "scale=320:-1",
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
