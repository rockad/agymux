use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationItem {
    pub id: String,
    pub title: String,
    pub project_path: Option<String>,
    pub updated_at: i64,
    pub relative_time: String,
}

pub struct SessionDb {
    conn: Connection,
}

/// Convert unix epoch seconds into human-readable relative time (e.g. '5m ago', '2h ago', '3d ago')
pub fn format_relative_time(epoch_secs: i64) -> String {
    if epoch_secs <= 0 {
        return "never".to_string();
    }
    let now = chrono::Utc::now().timestamp();
    let diff = now - epoch_secs;

    if diff <= 0 {
        "just now".to_string()
    } else if diff < 60 {
        format!("{diff}s ago")
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else if diff < 2592000 {
        format!("{}d ago", diff / 86400)
    } else if diff < 31536000 {
        format!("{}mo ago", diff / 2592000)
    } else {
        format!("{}y ago", diff / 31536000)
    }
}

/// Clean up titles by trimming and replacing line breaks
fn clean_title(title: &str) -> String {
    let clean = title.replace('\n', " ").replace('\r', "");
    let trimmed = clean.trim();
    if trimmed.is_empty() {
        "Untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

/// Extract clean file system path from project_uri or workspace_uris JSON
fn extract_project_path(raw_uri: Option<&str>) -> Option<String> {
    let s = raw_uri?.trim();
    if s.is_empty() {
        return None;
    }

    // Check if JSON array e.g. ["file:///home/rockad/projects/doppelganger"]
    if s.starts_with('[') {
        if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(s) {
            for v in arr {
                if let Some(uri_str) = v.as_str() {
                    let cleaned = uri_str.strip_prefix("file://").unwrap_or(uri_str);
                    if !cleaned.is_empty() {
                        return Some(cleaned.to_string());
                    }
                }
            }
        }
    }

    let cleaned = s.strip_prefix("file://").unwrap_or(s);
    if !cleaned.is_empty() {
        Some(cleaned.to_string())
    } else {
        None
    }
}

impl SessionDb {
    /// Open the conversation summaries SQLite database in read-only mode
    pub fn open() -> Result<Self> {
        let path = if let Ok(custom) = std::env::var("AGYMUX_DB_PATH") {
            PathBuf::from(custom)
        } else {
            let home =
                dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Home directory not found"))?;
            home.join(".gemini/antigravity-cli/conversation_summaries.db")
        };

        Self::open_path(&path)
    }

    /// Open SQLite database at a specific path
    pub fn open_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        let conn = Connection::open_with_flags(path.as_ref(), flags)
            .with_context(|| format!("Failed to open database at {:?}", path.as_ref()))?;
        Ok(Self { conn })
    }

    /// Construct SessionDb from an existing connection (for testing)
    pub fn from_connection(conn: Connection) -> Self {
        Self { conn }
    }

    /// Check if a specific table exists in the database
    fn table_exists(&self, table_name: &str) -> bool {
        let mut stmt = match self
            .conn
            .prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1")
        {
            Ok(s) => s,
            Err(_) => return false,
        };
        stmt.exists([table_name]).unwrap_or(false)
    }

    /// Query conversations local to a specific project directory
    pub fn get_local_conversations(&self, project_dir: &Path) -> Result<Vec<ConversationItem>> {
        let dir_str = project_dir.to_string_lossy();
        let dir_name = project_dir
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let pattern_full = format!("%{}%", dir_str);
        let pattern_name = format!("%{}%", dir_name);

        if self.table_exists("conversations") {
            let sql = "
                SELECT conversation_id,
                       COALESCE(NULLIF(title, ''), NULLIF(summary, ''), 'Untitled') AS title_clean,
                       project_uri,
                       project_id,
                       CASE
                         WHEN typeof(updated_at) = 'integer' THEN updated_at
                         ELSE CAST(strftime('%s', updated_at) AS INTEGER)
                       END AS epoch_sec
                FROM conversations
                WHERE (project_uri LIKE ?1 OR project_uri LIKE ?2 OR project_id = ?3 OR project_id = ?4)
                ORDER BY epoch_sec DESC;
            ";
            let mut stmt = self.conn.prepare(sql)?;
            let rows = stmt.query_map(
                rusqlite::params![pattern_full, pattern_name, dir_name, dir_str.as_ref()],
                |row| {
                    let id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let uri: Option<String> = row.get(2)?;
                    let epoch: Option<i64> = row.get(4)?;
                    Ok((id, title, uri, epoch.unwrap_or(0)))
                },
            )?;

            let mut items = Vec::new();
            for r in rows {
                let (id, title, uri, updated_at) = r?;
                let project_path = extract_project_path(uri.as_deref());
                let relative_time = format_relative_time(updated_at);
                items.push(ConversationItem {
                    id,
                    title: clean_title(&title),
                    project_path,
                    updated_at,
                    relative_time,
                });
            }
            Ok(items)
        } else if self.table_exists("conversation_summaries") {
            let sql = "
                SELECT conversation_id,
                       COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled') AS title_clean,
                       workspace_uris,
                       project_id,
                       CASE
                         WHEN typeof(last_modified_time) = 'integer' THEN last_modified_time
                         ELSE CAST(strftime('%s', last_modified_time) AS INTEGER)
                       END AS epoch_sec
                FROM conversation_summaries
                WHERE (workspace_uris LIKE ?1 OR workspace_uris LIKE ?2 OR project_id = ?3 OR project_id = ?4)
                ORDER BY epoch_sec DESC;
            ";
            let mut stmt = self.conn.prepare(sql)?;
            let rows = stmt.query_map(
                rusqlite::params![pattern_full, pattern_name, dir_name, dir_str.as_ref()],
                |row| {
                    let id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let uris: Option<String> = row.get(2)?;
                    let epoch: Option<i64> = row.get(4)?;
                    Ok((id, title, uris, epoch.unwrap_or(0)))
                },
            )?;

            let mut items = Vec::new();
            for r in rows {
                let (id, title, uris, updated_at) = r?;
                let project_path = extract_project_path(uris.as_deref());
                let relative_time = format_relative_time(updated_at);
                items.push(ConversationItem {
                    id,
                    title: clean_title(&title),
                    project_path,
                    updated_at,
                    relative_time,
                });
            }
            Ok(items)
        } else {
            Ok(Vec::new())
        }
    }

    /// Query global conversations across all projects
    pub fn get_global_conversations(&self) -> Result<Vec<ConversationItem>> {
        if self.table_exists("conversations") {
            let sql = "
                SELECT conversation_id,
                       COALESCE(NULLIF(title, ''), NULLIF(summary, ''), 'Untitled') AS title_clean,
                       project_uri,
                       project_id,
                       CASE
                         WHEN typeof(updated_at) = 'integer' THEN updated_at
                         ELSE CAST(strftime('%s', updated_at) AS INTEGER)
                       END AS epoch_sec
                FROM conversations
                ORDER BY epoch_sec DESC
                LIMIT 200;
            ";
            let mut stmt = self.conn.prepare(sql)?;
            let rows = stmt.query_map([], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let uri: Option<String> = row.get(2)?;
                let epoch: Option<i64> = row.get(4)?;
                Ok((id, title, uri, epoch.unwrap_or(0)))
            })?;

            let mut items = Vec::new();
            for r in rows {
                let (id, title, uri, updated_at) = r?;
                let project_path = extract_project_path(uri.as_deref());
                let relative_time = format_relative_time(updated_at);
                items.push(ConversationItem {
                    id,
                    title: clean_title(&title),
                    project_path,
                    updated_at,
                    relative_time,
                });
            }
            Ok(items)
        } else if self.table_exists("conversation_summaries") {
            let sql = "
                SELECT conversation_id,
                       COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled') AS title_clean,
                       workspace_uris,
                       project_id,
                       CASE
                         WHEN typeof(last_modified_time) = 'integer' THEN last_modified_time
                         ELSE CAST(strftime('%s', last_modified_time) AS INTEGER)
                       END AS epoch_sec
                FROM conversation_summaries
                ORDER BY epoch_sec DESC
                LIMIT 200;
            ";
            let mut stmt = self.conn.prepare(sql)?;
            let rows = stmt.query_map([], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let uris: Option<String> = row.get(2)?;
                let epoch: Option<i64> = row.get(4)?;
                Ok((id, title, uris, epoch.unwrap_or(0)))
            })?;

            let mut items = Vec::new();
            for r in rows {
                let (id, title, uris, updated_at) = r?;
                let project_path = extract_project_path(uris.as_deref());
                let relative_time = format_relative_time(updated_at);
                items.push(ConversationItem {
                    id,
                    title: clean_title(&title),
                    project_path,
                    updated_at,
                    relative_time,
                });
            }
            Ok(items)
        } else {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_relative_time() {
        let now = chrono::Utc::now().timestamp();
        assert_eq!(format_relative_time(now + 10), "just now");
        assert_eq!(format_relative_time(now - 15), "15s ago");
        assert_eq!(format_relative_time(now - 300), "5m ago");
        assert_eq!(format_relative_time(now - 7200), "2h ago");
        assert_eq!(format_relative_time(now - 172800), "2d ago");
    }

    #[test]
    fn test_conversations_table_mock() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE conversations (
                conversation_id TEXT PRIMARY KEY,
                project_id TEXT,
                project_uri TEXT,
                title TEXT,
                summary TEXT,
                updated_at INTEGER
            )",
            [],
        )
        .unwrap();

        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO conversations (conversation_id, project_id, project_uri, title, summary, updated_at)
             VALUES ('conv-1', 'agymux', 'file:///home/rockad/projects/agymux', 'Refactor engine', 'summary', ?1)",
            [now - 120],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO conversations (conversation_id, project_id, project_uri, title, summary, updated_at)
             VALUES ('conv-2', 'other', 'file:///home/rockad/projects/other', 'Other work', '', ?1)",
            [now - 500],
        )
        .unwrap();

        let db = SessionDb::from_connection(conn);
        let global = db.get_global_conversations().unwrap();
        assert_eq!(global.len(), 2);
        assert_eq!(global[0].id, "conv-1");
        assert_eq!(global[0].title, "Refactor engine");
        assert_eq!(
            global[0].project_path.as_deref(),
            Some("/home/rockad/projects/agymux")
        );
        assert_eq!(global[0].relative_time, "2m ago");

        let local = db
            .get_local_conversations(Path::new("/home/rockad/projects/agymux"))
            .unwrap();
        assert_eq!(local.len(), 1);
        assert_eq!(local[0].id, "conv-1");
    }

    #[test]
    fn test_conversation_summaries_table_mock() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE conversation_summaries (
                conversation_id TEXT PRIMARY KEY,
                project_id TEXT,
                workspace_uris TEXT,
                title TEXT,
                preview TEXT,
                last_modified_time TEXT
            )",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time)
             VALUES ('conv-cs-1', 'doppelganger', '[\"file:///home/rockad/projects/doppelganger\"]', 'Vault cleanup', 'preview', '2026-10-07 10:00:00+00:00')",
            [],
        )
        .unwrap();

        let db = SessionDb::from_connection(conn);
        let global = db.get_global_conversations().unwrap();
        assert_eq!(global.len(), 1);
        assert_eq!(global[0].id, "conv-cs-1");
        assert_eq!(global[0].title, "Vault cleanup");
        assert_eq!(
            global[0].project_path.as_deref(),
            Some("/home/rockad/projects/doppelganger")
        );
    }

    #[test]
    fn test_real_db_if_exists() {
        if let Ok(db) = SessionDb::open() {
            let global = db.get_global_conversations();
            assert!(global.is_ok());
        }
    }
}

