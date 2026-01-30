use rusqlite::{Connection, Result, params};
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::Mutex;
use directories::ProjectDirs;

use crate::models::VideoItem;

pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        
        let conn = Connection::open(&db_path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.apply_pragmas()?;
        db.init_schema()?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.apply_pragmas()?;
        db.init_schema()?;
        Ok(db)
    }

    fn apply_pragmas(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;",
        )?;
        Ok(())
    }

    fn get_db_path() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "vidz", "Vidz") {
            proj_dirs.data_dir().join("library.db")
        } else {
            PathBuf::from("library.db")
        }
    }

    pub fn get_thumbs_dir() -> PathBuf {
        if let Some(proj_dirs) = ProjectDirs::from("com", "vidz", "Vidz") {
            let path = proj_dirs.cache_dir().join("thumbs");
            std::fs::create_dir_all(&path).ok();
            path
        } else {
            let path = PathBuf::from("thumbs");
            std::fs::create_dir_all(&path).ok();
            path
        }
    }

    pub fn cleanup_orphaned_thumbnails(&self) -> Result<usize> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT thumb_path FROM videos WHERE thumb_path IS NOT NULL")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut valid = std::collections::HashSet::new();
        for row in rows {
            if let Ok(path) = row {
                valid.insert(path);
            }
        }
        drop(stmt);

        let thumbs_dir = Self::get_thumbs_dir();
        let mut removed = 0;
        if let Ok(entries) = std::fs::read_dir(&thumbs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let path_str = path.to_string_lossy().to_string();
                if !valid.contains(&path_str) {
                    if std::fs::remove_file(&path).is_ok() {
                        removed += 1;
                    }
                }
            }
        }
        Ok(removed)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS videos (
                id TEXT PRIMARY KEY,
                path TEXT UNIQUE NOT NULL,
                folder TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                mtime INTEGER NOT NULL,
                duration_ms INTEGER,
                width INTEGER,
                height INTEGER,
                aspect_ratio REAL,
                favorite INTEGER NOT NULL DEFAULT 0,
                thumb_path TEXT,
                last_scanned INTEGER NOT NULL DEFAULT 0
            );
            
            CREATE INDEX IF NOT EXISTS idx_videos_path ON videos(path);
            CREATE INDEX IF NOT EXISTS idx_videos_folder ON videos(folder);
            CREATE INDEX IF NOT EXISTS idx_videos_favorite ON videos(favorite);
            CREATE INDEX IF NOT EXISTS idx_videos_size ON videos(size_bytes);
            CREATE INDEX IF NOT EXISTS idx_videos_duration ON videos(duration_ms);
            
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );"
        )?;
        Ok(())
    }

    pub fn upsert_video(&self, video: &VideoItem) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO videos (id, path, folder, size_bytes, mtime, duration_ms, width, height, aspect_ratio, favorite, thumb_path, last_scanned)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, strftime('%s', 'now'))
             ON CONFLICT(id) DO UPDATE SET
                path = excluded.path,
                folder = excluded.folder,
                size_bytes = excluded.size_bytes,
                mtime = excluded.mtime,
                duration_ms = CASE WHEN excluded.mtime != mtime THEN NULL ELSE COALESCE(excluded.duration_ms, duration_ms) END,
                width = CASE WHEN excluded.mtime != mtime THEN NULL ELSE COALESCE(excluded.width, width) END,
                height = CASE WHEN excluded.mtime != mtime THEN NULL ELSE COALESCE(excluded.height, height) END,
                aspect_ratio = CASE WHEN excluded.mtime != mtime THEN NULL ELSE COALESCE(excluded.aspect_ratio, aspect_ratio) END,
                thumb_path = CASE WHEN excluded.mtime != mtime THEN NULL ELSE COALESCE(excluded.thumb_path, thumb_path) END,
                last_scanned = strftime('%s', 'now')",
            params![
                video.id,
                video.path,
                video.folder,
                video.size_bytes,
                video.mtime,
                video.duration_ms,
                video.width,
                video.height,
                video.aspect_ratio,
                video.favorite as i32,
                video.thumb_path,
            ],
        )?;
        Ok(())
    }

    pub fn get_all_videos(&self) -> Result<Vec<VideoItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, path, folder, size_bytes, mtime, duration_ms, width, height, aspect_ratio, favorite, thumb_path
             FROM videos ORDER BY path"
        )?;
        
        let videos = stmt.query_map([], |row| {
            Ok(VideoItem {
                id: row.get(0)?,
                path: row.get(1)?,
                folder: row.get(2)?,
                size_bytes: row.get(3)?,
                mtime: row.get(4)?,
                duration_ms: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                aspect_ratio: row.get(8)?,
                favorite: row.get::<_, i32>(9)? != 0,
                thumb_path: row.get(10)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        
        Ok(videos)
    }

    pub fn set_favorite(&self, id: &str, favorite: bool) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE videos SET favorite = ?1 WHERE id = ?2",
            params![favorite as i32, id],
        )?;
        Ok(())
    }

    pub fn update_metadata(&self, id: &str, duration_ms: i64, width: i32, height: i32) -> Result<()> {
        let conn = self.conn.lock();
        let aspect_ratio = if height > 0 { width as f64 / height as f64 } else { 1.0 };
        conn.execute(
            "UPDATE videos SET duration_ms = ?1, width = ?2, height = ?3, aspect_ratio = ?4 WHERE id = ?5",
            params![duration_ms, width, height, aspect_ratio, id],
        )?;
        Ok(())
    }

    pub fn update_thumb(&self, id: &str, thumb_path: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE videos SET thumb_path = ?1 WHERE id = ?2",
            params![thumb_path, id],
        )?;
        Ok(())
    }

    pub fn get_video_by_id(&self, id: &str) -> Result<Option<VideoItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, path, folder, size_bytes, mtime, duration_ms, width, height, aspect_ratio, favorite, thumb_path
             FROM videos WHERE id = ?1"
        )?;
        
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(VideoItem {
                id: row.get(0)?,
                path: row.get(1)?,
                folder: row.get(2)?,
                size_bytes: row.get(3)?,
                mtime: row.get(4)?,
                duration_ms: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                aspect_ratio: row.get(8)?,
                favorite: row.get::<_, i32>(9)? != 0,
                thumb_path: row.get(10)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_videos_needing_metadata(&self) -> Result<Vec<VideoItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, path, folder, size_bytes, mtime, duration_ms, width, height, aspect_ratio, favorite, thumb_path
             FROM videos WHERE duration_ms IS NULL OR width IS NULL"
        )?;
        
        let videos = stmt.query_map([], |row| {
            Ok(VideoItem {
                id: row.get(0)?,
                path: row.get(1)?,
                folder: row.get(2)?,
                size_bytes: row.get(3)?,
                mtime: row.get(4)?,
                duration_ms: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                aspect_ratio: row.get(8)?,
                favorite: row.get::<_, i32>(9)? != 0,
                thumb_path: row.get(10)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        
        Ok(videos)
    }

    pub fn get_videos_needing_thumbs(&self) -> Result<Vec<VideoItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, path, folder, size_bytes, mtime, duration_ms, width, height, aspect_ratio, favorite, thumb_path
             FROM videos WHERE thumb_path IS NULL"
        )?;
        
        let videos = stmt.query_map([], |row| {
            Ok(VideoItem {
                id: row.get(0)?,
                path: row.get(1)?,
                folder: row.get(2)?,
                size_bytes: row.get(3)?,
                mtime: row.get(4)?,
                duration_ms: row.get(5)?,
                width: row.get(6)?,
                height: row.get(7)?,
                aspect_ratio: row.get(8)?,
                favorite: row.get::<_, i32>(9)? != 0,
                thumb_path: row.get(10)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        
        Ok(videos)
    }

    pub fn save_watched_folders(&self, folders: &[String]) -> Result<()> {
        let conn = self.conn.lock();
        let json = serde_json::to_string(folders).unwrap_or_default();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('watched_folders', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![json],
        )?;
        Ok(())
    }

    pub fn get_watched_folders(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = 'watched_folders'")?;
        let mut rows = stmt.query([])?;
        
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            Ok(serde_json::from_str(&json).unwrap_or_default())
        } else {
            Ok(Vec::new())
        }
    }

    pub fn delete_video(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM videos WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_videos_by_folder_prefix(&self, folder: &str) -> Result<Vec<VideoItem>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT id, path, folder, size_bytes, mtime, duration_ms, width, height, aspect_ratio, favorite, thumb_path
             FROM videos WHERE folder = ?1 OR folder LIKE ?2"
        )?;

        let like_pattern = format!("{}%", folder.trim_end_matches(['/', '\\']));
        let videos = stmt
            .query_map(params![folder, like_pattern], |row| {
                Ok(VideoItem {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    folder: row.get(2)?,
                    size_bytes: row.get(3)?,
                    mtime: row.get(4)?,
                    duration_ms: row.get(5)?,
                    width: row.get(6)?,
                    height: row.get(7)?,
                    aspect_ratio: row.get(8)?,
                    favorite: row.get::<_, i32>(9)? != 0,
                    thumb_path: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(videos)
    }

    pub fn delete_videos_by_folder_prefix(&self, folder: &str) -> Result<()> {
        let conn = self.conn.lock();
        let like_pattern = format!("{}%", folder.trim_end_matches(['/', '\\']));
        conn.execute(
            "DELETE FROM videos WHERE folder = ?1 OR folder LIKE ?2",
            params![folder, like_pattern],
        )?;
        Ok(())
    }

    pub fn get_app_settings(&self) -> Result<crate::models::AppSettings> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = 'app_settings'")?;
        let mut rows = stmt.query([])?;
        
        if let Some(row) = rows.next()? {
            let json: String = row.get(0)?;
            Ok(serde_json::from_str(&json).unwrap_or_default())
        } else {
            Ok(crate::models::AppSettings::default())
        }
    }

    pub fn save_app_settings(&self, settings: &crate::models::AppSettings) -> Result<()> {
        let conn = self.conn.lock();
        let json = serde_json::to_string(settings).unwrap_or_default();
        conn.execute(
            "INSERT INTO settings (key, value) VALUES ('app_settings', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![json],
        )?;
        Ok(())
    }
}

impl Clone for Database {
    fn clone(&self) -> Self {
        Self {
            conn: Arc::clone(&self.conn),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_video(id: &str, path: &str, mtime: i64) -> VideoItem {
        VideoItem {
            id: id.to_string(),
            path: path.to_string(),
            folder: "C:/videos".to_string(),
            size_bytes: 123,
            mtime,
            duration_ms: Some(1000),
            width: Some(320),
            height: Some(200),
            aspect_ratio: Some(1.6),
            favorite: false,
            thumb_path: Some("thumbs/one.jpg".to_string()),
        }
    }

    #[test]
    fn upsert_preserves_metadata_when_mtime_unchanged() {
        let db = Database::new_in_memory().expect("db");
        let mut video = sample_video("id-1", "C:/videos/one.mp4", 10);
        db.upsert_video(&video).expect("upsert");

        video.duration_ms = None;
        video.width = None;
        video.height = None;
        video.aspect_ratio = None;
        video.thumb_path = None;
        db.upsert_video(&video).expect("upsert 2");

        let stored = db.get_video_by_id("id-1").expect("get").unwrap();
        assert_eq!(stored.duration_ms, Some(1000));
        assert_eq!(stored.width, Some(320));
        assert_eq!(stored.height, Some(200));
        assert_eq!(stored.thumb_path.as_deref(), Some("thumbs/one.jpg"));
    }

    #[test]
    fn upsert_resets_metadata_when_mtime_changes() {
        let db = Database::new_in_memory().expect("db");
        let video = sample_video("id-2", "C:/videos/two.mp4", 10);
        db.upsert_video(&video).expect("upsert");

        let mut updated = sample_video("id-2", "C:/videos/two.mp4", 11);
        updated.duration_ms = None;
        updated.width = None;
        updated.height = None;
        updated.aspect_ratio = None;
        updated.thumb_path = None;
        db.upsert_video(&updated).expect("upsert 2");

        let stored = db.get_video_by_id("id-2").expect("get").unwrap();
        assert_eq!(stored.duration_ms, None);
        assert_eq!(stored.width, None);
        assert_eq!(stored.height, None);
        assert_eq!(stored.thumb_path, None);
    }
}
