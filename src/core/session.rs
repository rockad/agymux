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
    } else if diff <= 2592000 {
        format!("{}d ago", diff / 86400)
    } else if diff < 31536000 {
        format!("{}mo ago", diff / 2592000)
    } else {
        format!("{}y ago", diff / 31536000)
    }
}

/// URL decode a percent-encoded string
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let h1 = bytes.next();
            let h2 = bytes.next();
            if let (Some(h1), Some(h2)) = (h1, h2) {
                let hex_str = [h1, h2];
                if let Ok(hex_str_utf8) = std::str::from_utf8(&hex_str) {
                    if let Ok(val) = u8::from_str_radix(hex_str_utf8, 16) {
                        result.push(val as char);
                        continue;
                    }
                }
                result.push('%');
                result.push(h1 as char);
                result.push(h2 as char);
            } else {
                result.push('%');
                if let Some(h1) = h1 {
                    result.push(h1 as char);
                }
            }
        } else {
            result.push(b as char);
        }
    }
    result
}

/// Clean up titles by trimming, collapsing spaces, and replacing line breaks
fn clean_title(title: &str) -> String {
    let collapsed: Vec<&str> = title.split_whitespace().collect();
    if collapsed.is_empty() {
        "Untitled".to_string()
    } else {
        collapsed.join(" ")
    }
}

/// Extract clean file system path from project_uri or workspace_uris JSON
fn extract_project_path(raw_uri: Option<&str>) -> Option<String> {
    let s = raw_uri?.trim();
    if s.is_empty() {
        return None;
    }

    // Check if JSON array e.g. ["file:///home/user/projects/repo"]
    if s.starts_with('[') {
        if let Ok(serde_json::Value::Array(arr)) = serde_json::from_str::<serde_json::Value>(s) {
            for v in arr {
                if let Some(uri_str) = v.as_str() {
                    let cleaned = uri_str.strip_prefix("file://").unwrap_or(uri_str);
                    let decoded = url_decode(cleaned);
                    if !decoded.is_empty() {
                        return Some(decoded);
                    }
                }
            }
        }
    }

    let cleaned = s.strip_prefix("file://").unwrap_or(s);
    let decoded = url_decode(cleaned);
    if !decoded.is_empty() {
        Some(decoded)
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
    pub fn table_exists(&self, table_name: &str) -> bool {
        table_exists(&self.conn, table_name)
    }

    /// Check if a specific column exists in a given table
    pub fn column_exists(&self, table_name: &str, column_name: &str) -> bool {
        table_has_column(&self.conn, table_name, column_name)
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
            let hygiene = session_hygiene_conditions(&self.conn, "conversations");
            let hygiene_sql = if hygiene.is_empty() {
                String::new()
            } else {
                format!(" AND {}", hygiene.join(" AND "))
            };

            let sql = format!("
                SELECT conversation_id,
                       COALESCE(NULLIF(title, ''), NULLIF(summary, ''), 'Untitled') AS title_clean,
                       project_uri,
                       project_id,
                       CASE
                         WHEN typeof(updated_at) = 'integer' THEN updated_at
                         ELSE CAST(strftime('%s', updated_at) AS INTEGER)
                       END AS epoch_sec
                FROM conversations
                WHERE (project_uri LIKE ?1 OR project_uri LIKE ?2 OR project_id = ?3 OR project_id = ?4){}
                ORDER BY epoch_sec DESC;
            ", hygiene_sql);
            let mut stmt = self.conn.prepare(&sql)?;
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
            let hygiene = session_hygiene_conditions(&self.conn, "conversation_summaries");
            let hygiene_sql = if hygiene.is_empty() {
                String::new()
            } else {
                format!(" AND {}", hygiene.join(" AND "))
            };

            let sql = format!("
                SELECT conversation_id,
                       COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled') AS title_clean,
                       workspace_uris,
                       project_id,
                       CASE
                         WHEN typeof(last_modified_time) = 'integer' THEN last_modified_time
                         ELSE CAST(strftime('%s', last_modified_time) AS INTEGER)
                       END AS epoch_sec
                FROM conversation_summaries
                WHERE (workspace_uris LIKE ?1 OR workspace_uris LIKE ?2 OR project_id = ?3 OR project_id = ?4){}
                ORDER BY epoch_sec DESC;
            ", hygiene_sql);
            let mut stmt = self.conn.prepare(&sql)?;
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
            let hygiene = session_hygiene_conditions(&self.conn, "conversations");
            let where_clause = if hygiene.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", hygiene.join(" AND "))
            };

            let sql = format!("
                SELECT conversation_id,
                       COALESCE(NULLIF(title, ''), NULLIF(summary, ''), 'Untitled') AS title_clean,
                       project_uri,
                       project_id,
                       CASE
                         WHEN typeof(updated_at) = 'integer' THEN updated_at
                         ELSE CAST(strftime('%s', updated_at) AS INTEGER)
                       END AS epoch_sec
                FROM conversations
                {}
                ORDER BY epoch_sec DESC
                LIMIT 200;
            ", where_clause);
            let mut stmt = self.conn.prepare(&sql)?;
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
            let hygiene = session_hygiene_conditions(&self.conn, "conversation_summaries");
            let where_clause = if hygiene.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", hygiene.join(" AND "))
            };

            let sql = format!("
                SELECT conversation_id,
                       COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled') AS title_clean,
                       workspace_uris,
                       project_id,
                       CASE
                         WHEN typeof(last_modified_time) = 'integer' THEN last_modified_time
                         ELSE CAST(strftime('%s', last_modified_time) AS INTEGER)
                       END AS epoch_sec
                FROM conversation_summaries
                {}
                ORDER BY epoch_sec DESC
                LIMIT 200;
            ", where_clause);
            let mut stmt = self.conn.prepare(&sql)?;
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

/// Check if a specific table exists in the database
pub fn table_exists(conn: &Connection, table_name: &str) -> bool {
    let mut stmt = match conn.prepare("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1") {
        Ok(s) => s,
        Err(_) => return false,
    };
    stmt.exists([table_name]).unwrap_or(false)
}

/// Check if a specific column exists in a given table
pub fn table_has_column(conn: &Connection, table_name: &str, column_name: &str) -> bool {
    let pragma_sql = format!("PRAGMA table_info({})", table_name);
    let mut stmt = match conn.prepare(&pragma_sql) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let mut rows = match stmt.query([]) {
        Ok(r) => r,
        Err(_) => return false,
    };
    while let Ok(Some(row)) = rows.next() {
        if let Ok(col_name) = row.get::<_, String>(1) {
            if col_name.eq_ignore_ascii_case(column_name) {
                return true;
            }
        }
    }
    false
}

/// Generate SQL WHERE conditions enforcing subagent exclusion and filtering abandoned 0-step sessions
pub fn session_hygiene_conditions(conn: &Connection, table_name: &str) -> Vec<String> {
    let mut filters = Vec::new();

    let has_parent = table_has_column(conn, table_name, "parent_conversation_id");
    let has_nesting = table_has_column(conn, table_name, "nesting_depth");

    match (has_parent, has_nesting) {
        (true, true) => {
            filters.push(
                "((parent_conversation_id IS NULL OR parent_conversation_id = '') AND (nesting_depth IS NULL OR nesting_depth = 0))".to_string(),
            );
        }
        (true, false) => {
            filters.push("(parent_conversation_id IS NULL OR parent_conversation_id = '')".to_string());
        }
        (false, true) => {
            filters.push("(nesting_depth IS NULL OR nesting_depth = 0)".to_string());
        }
        (false, false) => {}
    }

    let has_step_count = table_has_column(conn, table_name, "step_count");
    let has_title = table_has_column(conn, table_name, "title");
    let has_preview = table_has_column(conn, table_name, "preview");
    let has_summary = table_has_column(conn, table_name, "summary");

    let mut non_empty_content = Vec::new();
    if has_title {
        non_empty_content.push("(title IS NOT NULL AND title != '')");
    }
    if has_preview {
        non_empty_content.push("(preview IS NOT NULL AND preview != '')");
    } else if has_summary {
        non_empty_content.push("(summary IS NOT NULL AND summary != '')");
    }

    if has_step_count {
        if !non_empty_content.is_empty() {
            filters.push(format!("(step_count > 0 OR {})", non_empty_content.join(" OR ")));
        } else {
            filters.push("(step_count > 0)".to_string());
        }
    } else if !non_empty_content.is_empty() {
        filters.push(format!("({})", non_empty_content.join(" OR ")));
    }

    filters
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
             VALUES ('conv-1', 'agymux', 'file:///home/user/projects/agymux', 'Refactor engine', 'summary', ?1)",
            [now - 120],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO conversations (conversation_id, project_id, project_uri, title, summary, updated_at)
             VALUES ('conv-2', 'other', 'file:///home/user/projects/other', 'Other work', '', ?1)",
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
            Some("/home/user/projects/agymux")
        );
        assert_eq!(global[0].relative_time, "2m ago");

        let local = db
            .get_local_conversations(Path::new("/home/user/projects/agymux"))
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
             VALUES ('conv-cs-1', 'testproj', '[\"file:///home/user/projects/testproj\"]', 'Vault cleanup', 'preview', '2026-10-07 10:00:00+00:00')",
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
            Some("/home/user/projects/testproj")
        );
    }

    #[test]
    fn test_real_db_if_exists() {
        if let Ok(db) = SessionDb::open() {
            let global = db.get_global_conversations();
            assert!(global.is_ok());
        }
    }

    #[test]
    fn test_format_relative_time_boundaries() {
        let now = chrono::Utc::now().timestamp();
        assert_eq!(format_relative_time(now), "just now");
        assert_eq!(format_relative_time(now - 45), "45s ago");
        assert_eq!(format_relative_time(now - 59), "59s ago");
        assert_eq!(format_relative_time(now - 60), "1m ago");
        assert_eq!(format_relative_time(now - 3599), "59m ago");
        assert_eq!(format_relative_time(now - 3600), "1h ago");
        assert_eq!(format_relative_time(now - 82800), "23h ago");
        assert_eq!(format_relative_time(now - 86400 * 5), "5d ago");
        assert_eq!(format_relative_time(now - 86400 * 45), "1mo ago");
        // Future timestamp edge case
        assert_eq!(format_relative_time(now + 100), "just now");
    }

    #[test]
    fn test_extract_project_path_variants() {
        assert_eq!(extract_project_path(None), None);
        assert_eq!(extract_project_path(Some("")), None);
        assert_eq!(
            extract_project_path(Some("[\"file:///home/user/code\"]")),
            Some("/home/user/code".to_string())
        );
        assert_eq!(
            extract_project_path(Some("file:///home/user/repo%20space")),
            Some("/home/user/repo space".to_string())
        );
        assert_eq!(
            extract_project_path(Some("/direct/path/to/project")),
            Some("/direct/path/to/project".to_string())
        );
    }

    #[test]
    fn test_clean_title_variations() {
        assert_eq!(clean_title("  normal title  "), "normal title");
        assert_eq!(clean_title("multi   spaced   words"), "multi spaced words");
        assert_eq!(clean_title(""), "Untitled");
        assert_eq!(clean_title("   "), "Untitled");
    }

    #[test]
    fn test_empty_db_no_tables() {
        let conn = Connection::open_in_memory().unwrap();
        let db = SessionDb::from_connection(conn);
        let global = db.get_global_conversations().unwrap();
        assert!(global.is_empty());
        let local = db
            .get_local_conversations(Path::new("/home/user/projects/agymux"))
            .unwrap();
        assert!(local.is_empty());
    }

    #[test]
    fn test_subagent_and_abandoned_exclusion_full_schema() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE conversation_summaries (
                conversation_id TEXT PRIMARY KEY,
                project_id TEXT,
                workspace_uris TEXT,
                title TEXT,
                preview TEXT,
                last_modified_time TEXT,
                parent_conversation_id TEXT,
                nesting_depth INTEGER,
                step_count INTEGER
            )",
            [],
        )
        .unwrap();

        // 1. Valid root conversation
        conn.execute(
            "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
             VALUES ('root-1', 'proj', '[\"file:///home/user/projects/proj\"]', 'Root task', 'Doing work', '2026-10-13 10:00:00', '', 0, 10)",
            [],
        )
        .unwrap();

        // 2. Subagent with parent_conversation_id set (should be excluded)
        conn.execute(
            "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
             VALUES ('sub-1', 'proj', '[\"file:///home/user/projects/proj\"]', 'Subagent task', 'Subagent preview', '2026-10-13 10:05:00', 'root-1', 1, 5)",
            [],
        )
        .unwrap();

        // 3. Subagent with empty parent but nesting_depth > 0 (should be excluded)
        conn.execute(
            "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
             VALUES ('sub-2', 'proj', '[\"file:///home/user/projects/proj\"]', 'Deep agent', 'Subagent preview 2', '2026-10-13 10:06:00', '', 2, 8)",
            [],
        )
        .unwrap();

        // 4. Abandoned session: 0 steps, empty title, empty preview (should be excluded)
        conn.execute(
            "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
             VALUES ('abandoned-1', 'proj', '[\"file:///home/user/projects/proj\"]', '', '', '2026-10-13 10:07:00', '', 0, 0)",
            [],
        )
        .unwrap();

        // 5. Fresh active session: 0 steps but non-empty title (should be included)
        conn.execute(
            "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
             VALUES ('fresh-1', 'proj', '[\"file:///home/user/projects/proj\"]', 'Initial prompt title', '', '2026-10-13 10:08:00', '', 0, 0)",
            [],
        )
        .unwrap();

        // 6. Fresh active session: 0 steps but non-empty preview (should be included)
        conn.execute(
            "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
             VALUES ('fresh-2', 'proj', '[\"file:///home/user/projects/proj\"]', '', 'Initial prompt in preview', '2026-10-13 10:09:00', '', 0, 0)",
            [],
        )
        .unwrap();

        // 7. Root session with NULL parent_conversation_id and NULL nesting_depth (should be included)
        conn.execute(
            "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
             VALUES ('null-parent', 'proj', '[\"file:///home/user/projects/proj\"]', 'Null parent task', 'preview', '2026-10-13 10:10:00', NULL, NULL, 3)",
            [],
        )
        .unwrap();

        let db = SessionDb::from_connection(conn);
        let global = db.get_global_conversations().unwrap();

        let returned_ids: Vec<String> = global.into_iter().map(|item| item.id).collect();
        assert!(returned_ids.contains(&"root-1".to_string()));
        assert!(returned_ids.contains(&"fresh-1".to_string()));
        assert!(returned_ids.contains(&"fresh-2".to_string()));
        assert!(returned_ids.contains(&"null-parent".to_string()));

        // Crucial assertions: subagents and abandoned sessions are excluded
        assert!(!returned_ids.contains(&"sub-1".to_string()), "sub-1 with parent_conversation_id must be excluded");
        assert!(!returned_ids.contains(&"sub-2".to_string()), "sub-2 with nesting_depth > 0 must be excluded");
        assert!(!returned_ids.contains(&"abandoned-1".to_string()), "abandoned-1 with 0 steps and empty title/preview must be excluded");

        let local = db
            .get_local_conversations(Path::new("/home/user/projects/proj"))
            .unwrap();
        let local_ids: Vec<String> = local.into_iter().map(|item| item.id).collect();
        assert!(local_ids.contains(&"root-1".to_string()));
        assert!(local_ids.contains(&"fresh-1".to_string()));
        assert!(local_ids.contains(&"fresh-2".to_string()));
        assert!(local_ids.contains(&"null-parent".to_string()));
        assert!(!local_ids.contains(&"sub-1".to_string()));
        assert!(!local_ids.contains(&"sub-2".to_string()));
        assert!(!local_ids.contains(&"abandoned-1".to_string()));
    }

    #[test]
    fn test_subagent_exclusion_in_conversations_table_with_parent_col() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE conversations (
                conversation_id TEXT PRIMARY KEY,
                project_id TEXT,
                project_uri TEXT,
                title TEXT,
                summary TEXT,
                updated_at INTEGER,
                parent_conversation_id TEXT
            )",
            [],
        )
        .unwrap();

        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO conversations (conversation_id, project_id, project_uri, title, summary, updated_at, parent_conversation_id)
             VALUES ('conv-root', 'proj', 'file:///home/user/projects/proj', 'Root task', 'summary', ?1, '')",
            [now - 10],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO conversations (conversation_id, project_id, project_uri, title, summary, updated_at, parent_conversation_id)
             VALUES ('conv-sub', 'proj', 'file:///home/user/projects/proj', 'Subagent task', 'summary', ?1, 'conv-root')",
            [now - 5],
        )
        .unwrap();

        let db = SessionDb::from_connection(conn);
        let global = db.get_global_conversations().unwrap();
        let global_ids: Vec<String> = global.into_iter().map(|item| item.id).collect();
        assert_eq!(global_ids, vec!["conv-root"]);

        let local = db
            .get_local_conversations(Path::new("/home/user/projects/proj"))
            .unwrap();
        let local_ids: Vec<String> = local.into_iter().map(|item| item.id).collect();
        assert_eq!(local_ids, vec!["conv-root"]);
    }
}
