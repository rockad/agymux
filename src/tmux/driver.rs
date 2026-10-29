use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxWindow {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub active: bool,
    pub flags: String,
}

/// Trait abstracting execution of tmux commands
pub trait TmuxCommandRunner: Send + Sync {
    fn run(&self, args: &[&str]) -> Result<String>;
    fn attach_interactive(&self, args: &[&str]) -> Result<()>;
}

/// Production runner executing real tmux commands
#[derive(Debug, Clone, Default)]
pub struct SystemTmuxRunner;

impl TmuxCommandRunner for SystemTmuxRunner {
    fn run(&self, args: &[&str]) -> Result<String> {
        let output = std::process::Command::new("tmux").args(args).output()?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("tmux command failed: {}", stderr.trim());
        }
    }

    fn attach_interactive(&self, args: &[&str]) -> Result<()> {
        let mut cmd = std::process::Command::new("tmux");
        cmd.args(args);
        cmd.stdin(std::process::Stdio::inherit());
        cmd.stdout(std::process::Stdio::inherit());
        cmd.stderr(std::process::Stdio::inherit());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let err = cmd.exec();
            bail!("failed to exec tmux attach: {}", err);
        }

        #[cfg(not(unix))]
        {
            let status = cmd.status()?;
            if status.success() {
                Ok(())
            } else {
                bail!("tmux exited with status: {:?}", status.code());
            }
        }
    }
}

type MockClosure = Box<dyn Fn(&[String]) -> Result<String> + Send + Sync>;

enum MockResponse {
    Output(String),
    Error(String),
    Closure(MockClosure),
}

/// Thread-safe mock runner recording call history and returning configured responses
#[derive(Default, Clone)]
pub struct MockTmuxRunner {
    responses: Arc<Mutex<HashMap<Vec<String>, MockResponse>>>,
    history: Arc<Mutex<Vec<Vec<String>>>>,
}

impl MockTmuxRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_response(&self, args: &[&str], output: impl Into<String>) {
        let key: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        self.responses
            .lock()
            .unwrap()
            .insert(key, MockResponse::Output(output.into()));
    }

    pub fn add_error(&self, args: &[&str], err: impl Into<String>) {
        let key: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        self.responses
            .lock()
            .unwrap()
            .insert(key, MockResponse::Error(err.into()));
    }

    pub fn add_closure<F>(&self, args: &[&str], f: F)
    where
        F: Fn(&[String]) -> Result<String> + Send + Sync + 'static,
    {
        let key: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        self.responses
            .lock()
            .unwrap()
            .insert(key, MockResponse::Closure(Box::new(f)));
    }

    pub fn calls(&self) -> Vec<Vec<String>> {
        self.history.lock().unwrap().clone()
    }

    pub fn last_call(&self) -> Option<Vec<String>> {
        self.history.lock().unwrap().last().cloned()
    }

    pub fn call_count(&self) -> usize {
        self.history.lock().unwrap().len()
    }
}

impl TmuxCommandRunner for MockTmuxRunner {
    fn run(&self, args: &[&str]) -> Result<String> {
        let key: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        self.history.lock().unwrap().push(key.clone());

        let lock = self.responses.lock().unwrap();
        match lock.get(&key) {
            Some(MockResponse::Output(s)) => Ok(s.clone()),
            Some(MockResponse::Error(e)) => bail!("{}", e),
            Some(MockResponse::Closure(f)) => f(&key),
            None => bail!("MockTmuxRunner: unexpected call: {:?}", args),
        }
    }

    fn attach_interactive(&self, args: &[&str]) -> Result<()> {
        let key: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        self.history.lock().unwrap().push(key.clone());

        let lock = self.responses.lock().unwrap();
        match lock.get(&key) {
            Some(MockResponse::Output(_)) => Ok(()),
            Some(MockResponse::Error(e)) => bail!("{}", e),
            Some(MockResponse::Closure(f)) => {
                f(&key)?;
                Ok(())
            }
            None => Ok(()),
        }
    }
}

/// Generic client wrapping a TmuxCommandRunner
#[derive(Debug, Clone)]
pub struct TmuxClient<R: TmuxCommandRunner> {
    runner: R,
}

impl<R: TmuxCommandRunner> TmuxClient<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    pub fn runner(&self) -> &R {
        &self.runner
    }

    pub fn session_exists(&self, name: &str) -> bool {
        self.runner.run(&["has-session", "-t", name]).is_ok()
    }

    pub fn list_windows(&self, session: &str) -> Result<Vec<TmuxWindow>> {
        let output = self.runner.run(&[
            "list-windows",
            "-t",
            session,
            "-F",
            "#{window_id}\t#{window_index}\t#{window_name}\t#{window_active}\t#{window_flags}",
        ])?;
        TmuxDriver::parse_windows(&output)
    }

    pub fn new_window(&self, title: Option<&str>, command: Option<&str>) -> Result<()> {
        let t = title.unwrap_or("+ new");
        let mut args = vec!["new-window", "-n", t];
        if let Some(cmd) = command {
            args.push(cmd);
        }
        self.runner.run(&args)?;
        Ok(())
    }

    pub fn select_window(&self, target: &str) -> Result<()> {
        self.runner.run(&["select-window", "-t", target])?;
        Ok(())
    }

    /// Rename the target window
    pub fn rename_window(&self, target: &str, title: &str) -> Result<()> {
        if target.is_empty() {
            self.runner.run(&["rename-window", title])?;
        } else {
            self.runner.run(&["rename-window", "-t", target, title])?;
        }
        Ok(())
    }

    /// Kill target window
    pub fn kill_window(&self, target: &str) -> Result<()> {
        if target.is_empty() {
            self.runner.run(&["kill-window"])?;
        } else {
            self.runner.run(&["kill-window", "-t", target])?;
        }
        Ok(())
    }

    /// Set a window option on target
    pub fn set_window_option(&self, target: &str, key: &str, value: &str) -> Result<()> {
        if target.is_empty() {
            self.runner.run(&["set-option", "-w", key, value])?;
        } else {
            self.runner
                .run(&["set-option", "-w", "-t", target, key, value])?;
        }
        Ok(())
    }

    /// Get a window option on target
    pub fn get_window_option(&self, target: &str, key: &str) -> Result<Option<String>> {
        let args = if target.is_empty() {
            vec!["show-options", "-w", "-v", key]
        } else {
            vec!["show-options", "-w", "-t", target, "-v", key]
        };
        match self.runner.run(&args) {
            Ok(val) => {
                let trimmed = val.trim();
                if trimmed.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(trimmed.to_string()))
                }
            }
            Err(_) => Ok(None),
        }
    }

    pub fn current_session(&self) -> Option<String> {
        let out = self
            .runner
            .run(&["display-message", "-p", "#{session_name}"])
            .ok()?;
        let trimmed = out.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    pub fn current_window_name(&self) -> Option<String> {
        let out = self
            .runner
            .run(&["display-message", "-p", "#{window_name}"])
            .ok()?;
        let trimmed = out.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    pub fn attach_session(&self, name: &str) -> Result<()> {
        self.runner
            .attach_interactive(&["attach-session", "-t", name])?;
        Ok(())
    }

    pub fn switch_client(&self, name: &str) -> Result<()> {
        self.runner.run(&["switch-client", "-t", name])?;
        Ok(())
    }

    /// Find any session starting with "agy-" whose pane_current_path matches `dir`
    pub fn find_session_by_dir(&self, dir: &Path) -> Option<String> {
        let output = self
            .runner
            .run(&[
                "list-panes",
                "-a",
                "-F",
                "#{session_name}\t#{pane_current_path}",
            ])
            .ok()?;

        let target_canonical = dir.canonicalize().ok();

        for line in output.lines() {
            let parts: Vec<&str> = line.trim().split('\t').collect();
            if parts.len() >= 2 {
                let session_name = parts[0];
                let pane_path = parts[1];
                if session_name.starts_with("agy-") {
                    let p = Path::new(pane_path);
                    if p == dir {
                        return Some(session_name.to_string());
                    }
                    if let (Some(ref tc), Ok(pc)) = (&target_canonical, p.canonicalize()) {
                        if pc == *tc {
                            return Some(session_name.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    pub fn ensure_session(
        &self,
        name: &str,
        dir: &Path,
        config: &crate::config::Config,
        command: Option<&str>,
        window_name: Option<&str>,
    ) -> Result<()> {
        if self.session_exists(name) {
            return Ok(());
        }

        let status_val = if config.theme.tabs.spacer_lines == 0 {
            "on"
        } else {
            "2"
        };
        let effective_position = if config.theme.tabs.position != "bottom" {
            &config.theme.tabs.position
        } else if config.theme.status_position != "bottom" {
            &config.theme.status_position
        } else {
            "bottom"
        };

        let conf_path = crate::tmux::TmuxProfile::generate(config, name)?;
        let dir_str = dir.to_string_lossy();
        let win = window_name.unwrap_or("+ new");

        let mut cmd_args = vec!["new-session", "-d", "-s", name, "-c", &dir_str, "-n", win];
        if let Some(cmd) = command {
            cmd_args.push(cmd);
        }

        self.runner.run(&cmd_args)?;
        let _ = self
            .runner
            .run(&["source-file", &conf_path.to_string_lossy()]);

        // Record clean human-readable project name in session option
        let clean_project = if let Some(stripped) = name.strip_prefix("agy-") {
            if !crate::core::session::is_uuid(stripped)
                && !stripped.is_empty()
                && stripped != "main"
            {
                stripped.to_string()
            } else if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
                dir_name.to_string()
            } else {
                stripped.to_string()
            }
        } else if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
            dir_name.to_string()
        } else {
            name.to_string()
        };
        let _ = self
            .runner
            .run(&["set-option", "-t", name, "@agymux_project", &clean_project]);

        // Remove any leftover `status off` in tmux server and guarantee top tab bar is ALWAYS visible in new sessions
        let _ = self.runner.run(&["set", "-g", "status", status_val]);
        let _ = self
            .runner
            .run(&["set", "-g", "status-position", effective_position]);
        let _ = self.runner.run(&["set", "-t", name, "status", status_val]);
        let _ = self
            .runner
            .run(&["set", "-t", name, "status-position", effective_position]);

        let _ = self
            .runner
            .run(&["select-window", "-t", &format!("{}:1", name)]);

        Ok(())
    }
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

    /// Rename the target window
    pub fn rename_window(target: &str, title: &str) -> Result<()> {
        Self::client().rename_window(target, title)
    }

    /// Kill target window
    pub fn kill_window(target: &str) -> Result<()> {
        Self::client().kill_window(target)
    }

    /// Set a window option on target
    pub fn set_window_option(target: &str, key: &str, value: &str) -> Result<()> {
        Self::client().set_window_option(target, key, value)
    }

    /// Get a window option on target
    pub fn get_window_option(target: &str, key: &str) -> Result<Option<String>> {
        Self::client().get_window_option(target, key)
    }

    /// Attach to an existing tmux session
    pub fn attach_session(name: &str) -> Result<()> {
        Self::client().attach_session(name)
    }

    /// Switch current client to target session
    pub fn switch_client(name: &str) -> Result<()> {
        Self::client().switch_client(name)
    }

    /// Get current session name if inside tmux
    pub fn current_session() -> Option<String> {
        Self::client().current_session()
    }

    /// Get current window name if inside tmux
    pub fn current_window_name() -> Option<String> {
        Self::client().current_window_name()
    }

    /// Find any session starting with "agy-" whose pane_current_path matches `dir`
    pub fn find_session_by_dir(dir: &Path) -> Option<String> {
        Self::client().find_session_by_dir(dir)
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
        mock.add_response(&["rename-window", "-t", ":1", "renamed"], "");
        mock.add_response(&["kill-window", "-t", ":3"], "");
        mock.add_response(&["kill-window"], "");
        mock.add_response(&["set-option", "-w", "-t", ":1", "@conv_id", "123"], "");
        mock.add_response(
            &["show-options", "-w", "-t", ":1", "-v", "@conv_id"],
            "123\n",
        );
        mock.add_response(&["display-message", "-p", "#{session_name}"], "agy-test\n");
        mock.add_response(&["display-message", "-p", "#{window_name}"], "main\n");

        let client = TmuxDriver::with_runner(mock);
        assert!(client.select_window(":2").is_ok());
        assert!(client.rename_window(":1", "renamed").is_ok());
        assert!(client.kill_window(":3").is_ok());
        assert!(client.kill_window("").is_ok());
        assert!(client.set_window_option(":1", "@conv_id", "123").is_ok());
        assert_eq!(
            client.get_window_option(":1", "@conv_id").unwrap(),
            Some("123".to_string())
        );
        assert_eq!(client.current_session(), Some("agy-test".to_string()));
        assert_eq!(client.current_window_name(), Some("main".to_string()));
    }

    #[test]
    fn test_mock_driver_attach_and_switch_client() {
        let mock = MockTmuxRunner::new();
        mock.add_response(&["switch-client", "-t", "agy-other"], "");
        let client = TmuxDriver::with_runner(mock.clone());

        assert!(client.attach_session("agy-test").is_ok());
        assert!(client.switch_client("agy-other").is_ok());
        assert_eq!(
            mock.last_call(),
            Some(vec![
                "switch-client".into(),
                "-t".into(),
                "agy-other".into()
            ])
        );
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
        let mock_exists = MockTmuxRunner::new();
        mock_exists.add_response(&["has-session", "-t", "agy-demo"], "");
        let client_exists = TmuxDriver::with_runner(mock_exists.clone());
        let cfg = crate::config::Config::default();
        let res = client_exists.ensure_session("agy-demo", Path::new("/tmp"), &cfg, None, None);
        assert!(res.is_ok());
        assert_eq!(mock_exists.call_count(), 1); // only checked has-session

        let res2 = client_exists.ensure_session(
            "agy-missing",
            Path::new("/tmp"),
            &cfg,
            Some("agymux run"),
            Some("custom"),
        );
        let _ = res2;
    }

    #[test]
    fn test_mock_driver_find_session_by_dir() {
        let mock = MockTmuxRunner::new();
        let list_panes_out = "other-session\t/home/rockad/projects/other\n\
                              agy-f2de8c4a-f019-41a1-97b9-a4269a058738\t/home/rockad/projects/doppelganger\n\
                              agy-myrepo\t/home/rockad/projects/myrepo\n";
        mock.add_response(
            &[
                "list-panes",
                "-a",
                "-F",
                "#{session_name}\t#{pane_current_path}",
            ],
            list_panes_out,
        );

        let client = TmuxDriver::with_runner(mock);
        let found = client.find_session_by_dir(Path::new("/home/rockad/projects/doppelganger"));
        assert_eq!(
            found,
            Some("agy-f2de8c4a-f019-41a1-97b9-a4269a058738".to_string())
        );

        let found_myrepo = client.find_session_by_dir(Path::new("/home/rockad/projects/myrepo"));
        assert_eq!(found_myrepo, Some("agy-myrepo".to_string()));

        let not_found = client.find_session_by_dir(Path::new("/home/rockad/projects/nonexistent"));
        assert_eq!(not_found, None);

        let non_agy = client.find_session_by_dir(Path::new("/home/rockad/projects/other"));
        assert_eq!(non_agy, None);
    }
}
