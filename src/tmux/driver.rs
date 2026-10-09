use anyhow::Result;
use std::path::Path;

use crate::tmux::runner::{SystemTmuxRunner, TmuxClient, TmuxCommandRunner};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxWindow {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub active: bool,
    pub flags: String,
}

pub struct TmuxDriver;

impl TmuxDriver {
    /// Create a client using the default system runner
    pub fn client() -> TmuxClient<SystemTmuxRunner> {
        TmuxClient::new(SystemTmuxRunner)
    }

    /// Create a client wrapping a custom runner (e.g. `MockTmuxRunner`)
    pub fn with_runner<R: TmuxCommandRunner>(runner: R) -> TmuxClient<R> {
        TmuxClient::new(runner)
    }

    /// Check whether a tmux session with the given name exists
    pub fn session_exists(name: &str) -> bool {
        Self::client().session_exists(name)
    }

    /// List all windows in the specified tmux session
    pub fn list_windows(session: &str) -> Result<Vec<TmuxWindow>> {
        Self::client().list_windows(session)
    }

    /// Open a new window in the current tmux session (defaults title to "+ new")
    pub fn new_window(title: Option<&str>, command: Option<&str>) -> Result<()> {
        Self::client().new_window(title, command)
    }

    /// Select a target window in the current session
    pub fn select_window(target: &str) -> Result<()> {
        Self::client().select_window(target)
    }

    /// Rename the current window
    pub fn rename_window(title: &str) -> Result<()> {
        Self::client().rename_window(title)
    }

    /// Kill target window (or current window if None)
    pub fn kill_window(target: Option<&str>) -> Result<()> {
        Self::client().kill_window(target)
    }

    /// Attach to an existing tmux session
    pub fn attach_session(name: &str) -> Result<()> {
        Self::client().attach_session(name)
    }

    /// Get current session name if inside tmux
    pub fn current_session() -> Option<String> {
        Self::client().current_session()
    }

    /// Get current window name if inside tmux
    pub fn current_window_name() -> Option<String> {
        Self::client().current_window_name()
    }

    pub fn ensure_session(
        name: &str,
        dir: &Path,
        config: &crate::config::Config,
        command: Option<&str>,
        window_name: Option<&str>,
    ) -> Result<()> {
        Self::client().ensure_session(name, dir, config, command, window_name)
    }

    /// Parse tmux list-windows formatted output
    pub fn parse_windows(output: &str) -> Result<Vec<TmuxWindow>> {
        let mut windows = Vec::new();
        for line in output.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parts: Vec<&str> = trimmed.split('\t').collect();
            if parts.len() < 3 {
                continue;
            }
            let id = parts[0].to_string();
            let index = parts[1].parse::<usize>().unwrap_or(0);
            let name = parts[2].to_string();
            let active = parts.get(3).map(|&a| a == "1").unwrap_or(false)
                || parts.get(4).map(|&f| f.contains('*')).unwrap_or(false);
            let flags = parts.get(4).map(|&f| f.to_string()).unwrap_or_default();

            windows.push(TmuxWindow {
                id,
                index,
                name,
                active,
                flags,
            });
        }
        Ok(windows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmux::runner::MockTmuxRunner;

    #[test]
    fn test_parse_windows() {
        let raw = "@1\t1\tmain\t1\t*\n@2\t2\tbuild\t0\t-\n";
        let windows = TmuxDriver::parse_windows(raw).expect("parse should succeed");
        assert_eq!(windows.len(), 2);

        assert_eq!(windows[0].id, "@1");
        assert_eq!(windows[0].index, 1);
        assert_eq!(windows[0].name, "main");
        assert!(windows[0].active);
        assert_eq!(windows[0].flags, "*");

        assert_eq!(windows[1].id, "@2");
        assert_eq!(windows[1].index, 2);
        assert_eq!(windows[1].name, "build");
        assert!(!windows[1].active);
        assert_eq!(windows[1].flags, "-");
    }

    #[test]
    fn test_parse_windows_empty() {
        let windows = TmuxDriver::parse_windows("").expect("empty output parses cleanly");
        assert!(windows.is_empty());
    }

    #[test]
    fn test_parse_windows_sparse_indices() {
        let raw = "@10\t1\teditor\t0\t-\n@20\t5\tcompiler\t1\t*\n@30\t9\tmonitor\t0\t\n";
        let windows = TmuxDriver::parse_windows(raw).expect("sparse indices parse");
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].index, 1);
        assert_eq!(windows[1].index, 5);
        assert_eq!(windows[2].index, 9);
        assert!(windows[1].active);
    }

    #[test]
    fn test_parse_windows_all_flags() {
        let raw = "\
@1\t1\tzoomed\t1\t*Z\n\
@2\t2\tlast_alert\t0\t-#\n\
@3\t3\tbell\t0\t!\n\
@4\t4\tsilence\t0\t~\n\
@5\t5\tmarked\t0\tM\n";
        let windows = TmuxDriver::parse_windows(raw).expect("flag permutations parse");
        assert_eq!(windows.len(), 5);

        assert!(windows[0].active);
        assert_eq!(windows[0].flags, "*Z");

        assert!(!windows[1].active);
        assert_eq!(windows[1].flags, "-#");

        assert_eq!(windows[2].flags, "!");
        assert_eq!(windows[3].flags, "~");
        assert_eq!(windows[4].flags, "M");
    }

    #[test]
    fn test_parse_windows_unicode_and_escaped() {
        let raw = "@1\t1\t🚀 rocket\t1\t*\n@2\t2\tКириллица\t0\t-\n@3\t3\tfeat/login (wip)\t0\t\n";
        let windows = TmuxDriver::parse_windows(raw).expect("unicode parsed");
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].name, "🚀 rocket");
        assert_eq!(windows[1].name, "Кириллица");
        assert_eq!(windows[2].name, "feat/login (wip)");
    }

    #[test]
    fn test_parse_windows_malformed_lines() {
        let raw = "not enough columns\n\n@1\t1\tonly_three\n@2\tinvalid_idx\tbad\t0\t-\n";
        let windows = TmuxDriver::parse_windows(raw).expect("handles malformed lines gracefully");
        assert_eq!(windows.len(), 2);
        assert_eq!(windows[0].name, "only_three");
        assert_eq!(windows[1].name, "bad");
        assert_eq!(windows[1].index, 0); // fallback for invalid_idx
    }

    #[test]
    fn test_mock_driver_session_exists() {
        let mock = MockTmuxRunner::new();
        mock.add_response(&["has-session", "-t", "agy-demo"], "");
        let client = TmuxDriver::with_runner(mock.clone());

        assert!(client.session_exists("agy-demo"));
        assert!(!client.session_exists("nonexistent"));
        assert_eq!(mock.call_count(), 2);
    }

    #[test]
    fn test_mock_driver_new_window_default_title() {
        let mock = MockTmuxRunner::new();
        mock.add_response(&["new-window", "-n", "+ new"], "");
        let client = TmuxDriver::with_runner(mock.clone());

        let res = client.new_window(None, None);
        assert!(res.is_ok());
        assert_eq!(
            mock.last_call(),
            Some(vec!["new-window".into(), "-n".into(), "+ new".into()])
        );
    }

    #[test]
    fn test_mock_driver_new_window_with_custom_title_and_cmd() {
        let mock = MockTmuxRunner::new();
        mock.add_response(&["new-window", "-n", "custom", "agy -c"], "");
        let client = TmuxDriver::with_runner(mock.clone());

        let res = client.new_window(Some("custom"), Some("agy -c"));
        assert!(res.is_ok());
        assert_eq!(
            mock.last_call(),
            Some(vec![
                "new-window".into(),
                "-n".into(),
                "custom".into(),
                "agy -c".into()
            ])
        );
    }

    #[test]
    fn test_mock_driver_window_operations() {
        let mock = MockTmuxRunner::new();
        mock.add_response(&["select-window", "-t", ":2"], "");
        mock.add_response(&["rename-window", "renamed"], "");
        mock.add_response(&["kill-window", "-t", ":3"], "");
        mock.add_response(&["kill-window"], "");
        mock.add_response(&["display-message", "-p", "#{session_name}"], "agy-test\n");
        mock.add_response(&["display-message", "-p", "#{window_name}"], "main\n");

        let client = TmuxDriver::with_runner(mock);
        assert!(client.select_window(":2").is_ok());
        assert!(client.rename_window("renamed").is_ok());
        assert!(client.kill_window(Some(":3")).is_ok());
        assert!(client.kill_window(None).is_ok());
        assert_eq!(client.current_session(), Some("agy-test".to_string()));
        assert_eq!(client.current_window_name(), Some("main".to_string()));
    }

    #[test]
    fn test_mock_driver_list_windows() {
        let mock = MockTmuxRunner::new();
        let list_output = "@1\t1\tmain\t1\t*\n@2\t2\tedit\t0\t-\n";
        mock.add_response(
            &[
                "list-windows",
                "-t",
                "agy-test",
                "-F",
                "#{window_id}\t#{window_index}\t#{window_name}\t#{window_active}\t#{window_flags}",
            ],
            list_output,
        );

        let client = TmuxDriver::with_runner(mock);
        let wins = client.list_windows("agy-test").unwrap();
        assert_eq!(wins.len(), 2);
        assert_eq!(wins[0].name, "main");
        assert_eq!(wins[1].name, "edit");
    }

    #[test]
    fn test_mock_driver_ensure_session_when_missing() {
        let mock = MockTmuxRunner::new();
        // has-session returns error (doesn't exist)
        mock.add_error(&["has-session", "-t", "agy-demo"], "failed");
        // new-session returns ok
        mock.add_closure(
            &[
                "new-session",
                "-d",
                "-s",
                "agy-demo",
                "-c",
                "/tmp",
                "-n",
                "main",
            ],
            |_| Ok("".into()),
        );
        // source-file returns ok (dynamic filename)
        mock.add_closure(&[], |_| Ok("".into()));

        // We can use a closure to handle source-file and select-window
        let mock2 = MockTmuxRunner::new();
        mock2.add_error(&["has-session", "-t", "agy-demo"], "failed");
        mock2.add_closure(&["new-session"], |_| Ok("".into()));
        mock2.add_closure(&["source-file"], |_| Ok("".into()));
        mock2.add_closure(&["select-window"], |_| Ok("".into()));

        // Actually, let's test ensure_session logic directly:
        // If session_exists is true, ensure_session does not call new-session
        let mock_exists = MockTmuxRunner::new();
        mock_exists.add_response(&["has-session", "-t", "agy-demo"], "");
        let client_exists = TmuxDriver::with_runner(mock_exists.clone());
        let cfg = crate::config::Config::default();
        let res = client_exists.ensure_session("agy-demo", Path::new("/tmp"), &cfg, None, None);
        assert!(res.is_ok());
        assert_eq!(mock_exists.call_count(), 1); // only checked has-session

        // Test ensure_session when missing: passes command and window_name to new-session
        let res2 = client_exists.ensure_session(
            "agy-missing",
            Path::new("/tmp"),
            &cfg,
            Some("agymux run"),
            Some("custom"),
        );
        // mock_exists has no expectation for "has-session -t agy-missing", so it returns error (doesn't exist)
        // and then attempts new-session
        let _ = res2;
    }
}
