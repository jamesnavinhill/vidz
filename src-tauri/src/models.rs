use serde::{Deserialize, Serialize};

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
