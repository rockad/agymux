use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TabRegistry {
    conn: Connection,
}

impl TabRegistry {
    pub fn db_path() -> PathBuf {
        if let Ok(custom) = std::env::var("AGYMUX_TAB_REGISTRY_DB") {
            PathBuf::from(custom)
        } else if let Ok(custom) = std::env::var("AGYMUX_TAB_REGISTRY_PATH") {
            PathBuf::from(custom)
        } else {
            dirs::config_dir()
                .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
                .join("agymux")
                .join("agymux.db")
        }
    }

    pub fn open() -> Result<Self> {
        let path = Self::db_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {:?}", parent))?;
        }

        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", path))?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS tab_bindings (
                 window_id TEXT PRIMARY KEY,
                 conversation_id TEXT NOT NULL,
                 project TEXT,
                 title TEXT,
                 updated_at INTEGER NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_tab_bindings_conv ON tab_bindings(conversation_id);",
        )?;

        Ok(Self { conn })
    }

    pub fn bind(&mut self, window_id: &str, conv_id: &str) -> Result<()> {
        self.bind_full(window_id, conv_id, None, None)
    }

    pub fn bind_full(
        &mut self,
        window_id: &str,
        conv_id: &str,
        project: Option<&str>,
        title: Option<&str>,
    ) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let tx = self.conn.transaction()?;

        // If this conversation was previously bound to another window, remove the old mapping
        tx.execute(
            "DELETE FROM tab_bindings WHERE conversation_id = ?1 AND window_id != ?2",
            params![conv_id, window_id],
        )?;

        tx.execute(
            "INSERT INTO tab_bindings (window_id, conversation_id, project, title, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(window_id) DO UPDATE SET
                 conversation_id = excluded.conversation_id,
                 project = coalesce(excluded.project, tab_bindings.project),
                 title = coalesce(excluded.title, tab_bindings.title),
                 updated_at = excluded.updated_at;",
            params![window_id, conv_id, project, title, now],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn unbind_window(&mut self, window_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM tab_bindings WHERE window_id = ?1",
            params![window_id],
        )?;
        Ok(())
    }

    pub fn unbind_conv(&mut self, conv_id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM tab_bindings WHERE conversation_id = ?1",
            params![conv_id],
        )?;
        Ok(())
    }

    pub fn get_window_for_conv(&self, conv_id: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT window_id FROM tab_bindings WHERE conversation_id = ?1 LIMIT 1",
                params![conv_id],
                |r| r.get(0),
            )
            .ok()
    }

    pub fn get_conv_for_window(&self, window_id: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT conversation_id FROM tab_bindings WHERE window_id = ?1 LIMIT 1",
                params![window_id],
                |r| r.get(0),
            )
            .ok()
    }

    pub fn prune(&mut self, live_ids: &[String]) -> Result<()> {
        if live_ids.is_empty() {
            self.conn.execute("DELETE FROM tab_bindings", [])?;
            return Ok(());
        }

        let placeholders = live_ids
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect::<Vec<_>>()
            .join(",");

        let sql = format!(
            "DELETE FROM tab_bindings WHERE window_id NOT IN ({})",
            placeholders
        );
        let params: Vec<&dyn rusqlite::ToSql> =
            live_ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect();

        self.conn.execute(&sql, params.as_slice())?;
        Ok(())
    }

    // Static convenience helpers for callers that don't maintain a long-lived instance
    pub fn bind_static(window_id: &str, conv_id: &str) -> Result<()> {
        let mut reg = Self::open()?;
        reg.bind(window_id, conv_id)
    }

    pub fn unbind_window_static(window_id: &str) -> Result<()> {
        let mut reg = Self::open()?;
        reg.unbind_window(window_id)
    }

    pub fn unbind_conv_static(conv_id: &str) -> Result<()> {
        let mut reg = Self::open()?;
        reg.unbind_conv(conv_id)
    }

    pub fn get_window_for_conv_static(conv_id: &str) -> Option<String> {
        Self::open().ok()?.get_window_for_conv(conv_id)
    }

    pub fn get_conv_for_window_static(window_id: &str) -> Option<String> {
        Self::open().ok()?.get_conv_for_window(window_id)
    }

    pub fn prune_static(live_ids: &[String]) -> Result<()> {
        let mut reg = Self::open()?;
        reg.prune(live_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static DB_TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct TempDbGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        prev: Option<String>,
        path: PathBuf,
    }

    impl TempDbGuard {
        fn new(name: &str) -> Self {
            let lock = DB_TEST_MUTEX.lock().unwrap();
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let temp_dir = std::env::temp_dir().join(format!("agymux-test-db-{}", nanos));
            let path = temp_dir.join(name);
            let prev = std::env::var("AGYMUX_TAB_REGISTRY_DB").ok();
            std::env::set_var("AGYMUX_TAB_REGISTRY_DB", path.to_str().unwrap());
            Self {
                _lock: lock,
                prev,
                path,
            }
        }
    }

    impl Drop for TempDbGuard {
        fn drop(&mut self) {
            if let Some(ref p) = self.prev {
                std::env::set_var("AGYMUX_TAB_REGISTRY_DB", p);
            } else {
                std::env::remove_var("AGYMUX_TAB_REGISTRY_DB");
            }
            if self.path.exists() {
                let _ = std::fs::remove_file(&self.path);
            }
            let wal = self.path.with_extension("db-wal");
            if wal.exists() {
                let _ = std::fs::remove_file(wal);
            }
            let shm = self.path.with_extension("db-shm");
            if shm.exists() {
                let _ = std::fs::remove_file(shm);
            }
            if let Some(parent) = self.path.parent() {
                let _ = std::fs::remove_dir(parent);
            }
        }
    }

    #[test]
    fn test_tab_registry_sqlite_bind_and_lookup() {
        let _guard = TempDbGuard::new("agymux.db");
        let mut reg = TabRegistry::open().expect("open SQLite DB");

        assert!(reg.get_window_for_conv("conv-1").is_none());
        assert!(reg.get_conv_for_window("@0").is_none());

        reg.bind("@0", "conv-1").unwrap();
        assert_eq!(reg.get_window_for_conv("conv-1"), Some("@0".to_string()));
        assert_eq!(reg.get_conv_for_window("@0"), Some("conv-1".to_string()));

        // Rebinding conversation to another window removes the old binding
        reg.bind("@1", "conv-1").unwrap();
        assert_eq!(reg.get_window_for_conv("conv-1"), Some("@1".to_string()));
        assert_eq!(reg.get_conv_for_window("@0"), None);
        assert_eq!(reg.get_conv_for_window("@1"), Some("conv-1".to_string()));

        // Unbind window
        reg.unbind_window("@1").unwrap();
        assert_eq!(reg.get_window_for_conv("conv-1"), None);
        assert_eq!(reg.get_conv_for_window("@1"), None);
    }

    #[test]
    fn test_tab_registry_prune() {
        let _guard = TempDbGuard::new("agymux-prune.db");
        let mut reg = TabRegistry::open().expect("open SQLite DB");

        reg.bind("@0", "conv-0").unwrap();
        reg.bind("@1", "conv-1").unwrap();
        reg.bind("@2", "conv-2").unwrap();

        assert_eq!(reg.get_window_for_conv("conv-0"), Some("@0".to_string()));
        assert_eq!(reg.get_window_for_conv("conv-1"), Some("@1".to_string()));
        assert_eq!(reg.get_window_for_conv("conv-2"), Some("@2".to_string()));

        // Prune keeping only @1 and @2
        let live = vec!["@1".to_string(), "@2".to_string()];
        reg.prune(&live).unwrap();

        assert_eq!(reg.get_window_for_conv("conv-0"), None);
        assert_eq!(reg.get_window_for_conv("conv-1"), Some("@1".to_string()));
        assert_eq!(reg.get_window_for_conv("conv-2"), Some("@2".to_string()));

        // Prune with empty live list clears all
        reg.prune(&[]).unwrap();
        assert_eq!(reg.get_window_for_conv("conv-1"), None);
        assert_eq!(reg.get_window_for_conv("conv-2"), None);
    }

    #[test]
    fn test_tab_registry_static_wrappers() {
        let _guard = TempDbGuard::new("agymux-static.db");

        TabRegistry::bind_static("@10", "conv-static").unwrap();
        assert_eq!(
            TabRegistry::get_window_for_conv_static("conv-static"),
            Some("@10".to_string())
        );
        assert_eq!(
            TabRegistry::get_conv_for_window_static("@10"),
            Some("conv-static".to_string())
        );

        TabRegistry::unbind_conv_static("conv-static").unwrap();
        assert_eq!(TabRegistry::get_window_for_conv_static("conv-static"), None);
    }
}
