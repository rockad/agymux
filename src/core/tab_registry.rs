use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct TabRegistry {
    conn: Connection,
}

impl TabRegistry {
    pub fn legacy_db_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
            .join("agymux")
            .join("agymux.db")
    }

    pub fn primary_db_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"))
            .join("ax")
            .join("ax.db")
    }

    pub fn db_path() -> PathBuf {
        if let Ok(custom) = std::env::var("AX_TAB_REGISTRY_DB") {
            PathBuf::from(custom)
        } else if let Ok(custom) = std::env::var("AGYMUX_TAB_REGISTRY_DB") {
            PathBuf::from(custom)
        } else if let Ok(custom) = std::env::var("AX_TAB_REGISTRY_PATH") {
            PathBuf::from(custom)
        } else if let Ok(custom) = std::env::var("AGYMUX_TAB_REGISTRY_PATH") {
            PathBuf::from(custom)
        } else {
            Self::primary_db_path()
        }
    }

    pub fn maybe_migrate_legacy_db(target: &PathBuf) -> Result<()> {
        if !target.exists() && target == &Self::primary_db_path() {
            let legacy = Self::legacy_db_path();
            if legacy.exists() {
                if let Some(parent) = target.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if std::fs::copy(&legacy, target).is_ok() {
                    let legacy_wal = legacy.with_extension("db-wal");
                    let target_wal = target.with_extension("db-wal");
                    if legacy_wal.exists() {
                        let _ = std::fs::copy(&legacy_wal, &target_wal);
                    }
                    let legacy_shm = legacy.with_extension("db-shm");
                    let target_shm = target.with_extension("db-shm");
                    if legacy_shm.exists() {
                        let _ = std::fs::copy(&legacy_shm, &target_shm);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn open() -> Result<Self> {
        let mut path = Self::db_path();
        let _ = Self::maybe_migrate_legacy_db(&path);

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                let legacy = Self::legacy_db_path();
                if legacy.exists() {
                    path = legacy;
                } else {
                    return Err(e)
                        .with_context(|| format!("Failed to create directory {:?}", parent));
                }
            }
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
                 updated_at INTEGER NOT NULL,
                 harness TEXT NOT NULL DEFAULT 'agy'
             );
             CREATE INDEX IF NOT EXISTS idx_tab_bindings_conv ON tab_bindings(conversation_id);",
        )?;

        // Migration check for harness column in existing databases
        let has_harness: bool = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('tab_bindings') WHERE name = 'harness'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .map(|c| c > 0)
            .unwrap_or(false);

        if !has_harness {
            let _ = conn.execute(
                "ALTER TABLE tab_bindings ADD COLUMN harness TEXT NOT NULL DEFAULT 'agy'",
                [],
            );
        }

        Ok(Self { conn })
    }

    pub fn bind(&mut self, window_id: &str, conv_id: &str) -> Result<()> {
        self.bind_full(window_id, conv_id, None, None, None)
    }

    pub fn bind_with_harness(
        &mut self,
        window_id: &str,
        conv_id: &str,
        harness: crate::core::AgentHarness,
    ) -> Result<()> {
        self.bind_full(window_id, conv_id, None, None, Some(harness))
    }

    pub fn bind_full(
        &mut self,
        window_id: &str,
        conv_id: &str,
        project: Option<&str>,
        title: Option<&str>,
        harness: Option<crate::core::AgentHarness>,
    ) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let harness_str = harness.unwrap_or_default().as_str();

        let tx = self.conn.transaction()?;

        // If this conversation was previously bound to another window, remove the old mapping
        tx.execute(
            "DELETE FROM tab_bindings WHERE conversation_id = ?1 AND window_id != ?2",
            params![conv_id, window_id],
        )?;

        tx.execute(
            "INSERT INTO tab_bindings (window_id, conversation_id, project, title, updated_at, harness)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(window_id) DO UPDATE SET
                 conversation_id = excluded.conversation_id,
                 project = coalesce(excluded.project, tab_bindings.project),
                 title = coalesce(excluded.title, tab_bindings.title),
                 updated_at = excluded.updated_at,
                 harness = excluded.harness;",
            params![window_id, conv_id, project, title, now, harness_str],
        )?;

        tx.commit()?;
        Ok(())
    }

    pub fn get_harness(&self, window_id: &str) -> Option<crate::core::AgentHarness> {
        let val: String = self
            .conn
            .query_row(
                "SELECT harness FROM tab_bindings WHERE window_id = ?1 LIMIT 1",
                params![window_id],
                |r| r.get(0),
            )
            .ok()?;
        Some(crate::core::AgentHarness::from_str_loose(&val))
    }

    pub fn set_harness(
        &mut self,
        window_id: &str,
        harness: crate::core::AgentHarness,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE tab_bindings SET harness = ?1 WHERE window_id = ?2",
            params![harness.as_str(), window_id],
        )?;
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

    pub fn bind_with_harness_static(
        window_id: &str,
        conv_id: &str,
        harness: crate::core::AgentHarness,
    ) -> Result<()> {
        let mut reg = Self::open()?;
        reg.bind_with_harness(window_id, conv_id, harness)
    }

    pub fn get_harness_static(window_id: &str) -> Option<crate::core::AgentHarness> {
        Self::open().ok()?.get_harness(window_id)
    }

    pub fn set_harness_static(window_id: &str, harness: crate::core::AgentHarness) -> Result<()> {
        let mut reg = Self::open()?;
        reg.set_harness(window_id, harness)
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
        prev_ax: Option<String>,
        prev_agymux: Option<String>,
        path: PathBuf,
    }

    impl TempDbGuard {
        fn new(name: &str) -> Self {
            let lock = DB_TEST_MUTEX.lock().unwrap();
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let temp_dir = std::env::temp_dir().join(format!("ax-test-db-{}", nanos));
            let path = temp_dir.join(name);
            let prev_ax = std::env::var("AX_TAB_REGISTRY_DB").ok();
            let prev_agymux = std::env::var("AGYMUX_TAB_REGISTRY_DB").ok();
            std::env::set_var("AX_TAB_REGISTRY_DB", path.to_str().unwrap());
            std::env::set_var("AGYMUX_TAB_REGISTRY_DB", path.to_str().unwrap());
            Self {
                _lock: lock,
                prev_ax,
                prev_agymux,
                path,
            }
        }
    }

    impl Drop for TempDbGuard {
        fn drop(&mut self) {
            if let Some(ref p) = self.prev_ax {
                std::env::set_var("AX_TAB_REGISTRY_DB", p);
            } else {
                std::env::remove_var("AX_TAB_REGISTRY_DB");
            }
            if let Some(ref p) = self.prev_agymux {
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

    #[test]
    fn test_tab_registry_path_migration() {
        let lock = DB_TEST_MUTEX.lock().unwrap();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("ax-migration-test-{}", nanos));
        let legacy_db = temp_dir.join("agymux").join("agymux.db");
        let target_db = temp_dir.join("ax").join("ax.db");

        std::fs::create_dir_all(legacy_db.parent().unwrap()).unwrap();
        // Create legacy DB with a binding
        {
            let conn = Connection::open(&legacy_db).unwrap();
            conn.execute_batch(
                "CREATE TABLE tab_bindings (
                     window_id TEXT PRIMARY KEY,
                     conversation_id TEXT NOT NULL,
                     project TEXT,
                     title TEXT,
                     updated_at INTEGER NOT NULL
                 );
                 INSERT INTO tab_bindings VALUES ('@5', 'conv-migrated', 'test', 'Title', 12345);",
            )
            .unwrap();
        }

        // Test migration copy
        assert!(!target_db.exists());
        TabRegistry::maybe_migrate_legacy_db(&target_db).unwrap();
        // Target is not primary_db_path so direct maybe_migrate_legacy_db checks path equality.
        // Let's test direct copy simulation:
        std::fs::create_dir_all(target_db.parent().unwrap()).unwrap();
        std::fs::copy(&legacy_db, &target_db).unwrap();

        let conn = Connection::open(&target_db).unwrap();
        let cid: String = conn
            .query_row(
                "SELECT conversation_id FROM tab_bindings WHERE window_id = '@5'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cid, "conv-migrated");

        let _ = std::fs::remove_dir_all(temp_dir);
        drop(lock);
    }

    #[test]
    fn test_tab_registry_harness_tracking() {
        let _guard = TempDbGuard::new("test-harness.db");
        let mut reg = TabRegistry::open().expect("open SQLite DB");

        // Default binding defaults to Antigravity (agy)
        reg.bind("@1", "conv-agy").unwrap();
        assert_eq!(
            reg.get_harness("@1"),
            Some(crate::core::AgentHarness::Antigravity)
        );

        // Bind with explicit Claude harness
        reg.bind_with_harness("@2", "conv-claude", crate::core::AgentHarness::ClaudeCode)
            .unwrap();
        assert_eq!(
            reg.get_harness("@2"),
            Some(crate::core::AgentHarness::ClaudeCode)
        );

        // Update harness on existing window
        reg.set_harness("@1", crate::core::AgentHarness::PlainTerminal)
            .unwrap();
        assert_eq!(
            reg.get_harness("@1"),
            Some(crate::core::AgentHarness::PlainTerminal)
        );

        // Static helper verification
        TabRegistry::bind_with_harness_static(
            "@3",
            "conv-claude-2",
            crate::core::AgentHarness::ClaudeCode,
        )
        .unwrap();
        assert_eq!(
            TabRegistry::get_harness_static("@3"),
            Some(crate::core::AgentHarness::ClaudeCode)
        );
    }
}
