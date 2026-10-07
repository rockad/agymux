use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

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
    /// Check whether a tmux session with the given name exists
    pub fn session_exists(name: &str) -> bool {
        let output = Command::new("tmux")
            .args(["has-session", "-t", name])
            .output();
        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    /// List all windows in the specified tmux session
    pub fn list_windows(session: &str) -> Result<Vec<TmuxWindow>> {
        let output = Command::new("tmux")
            .args([
                "list-windows",
                "-t",
                session,
                "-F",
                "#{window_id}\t#{window_index}\t#{window_name}\t#{window_active}\t#{window_flags}",
            ])
            .output()?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            bail!(
                "tmux list-windows failed for session '{}': {}",
                session,
                err.trim()
            );
        }

        let text = String::from_utf8_lossy(&output.stdout);
        Self::parse_windows(&text)
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

    /// Ensure a tmux session exists; if not, create detached session,
    /// source profile generated from `TmuxProfile`, and set initial window.
    pub fn ensure_session(name: &str, dir: &Path, config: &crate::config::Config) -> Result<()> {
        if Self::session_exists(name) {
            return Ok(());
        }

        let conf_path = crate::tmux::TmuxProfile::generate(config, name)?;
        let dir_str = dir.to_string_lossy();

        // 1. Create detached session
        let output = Command::new("tmux")
            .args(["new-session", "-d", "-s", name, "-c", &dir_str, "-n", "main"])
            .output()?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to create tmux session '{}': {}", name, err.trim());
        }

        // 2. Source generated profile
        let _ = Command::new("tmux")
            .args(["source-file", &conf_path.to_string_lossy()])
            .output();

        // 3. Set initial window
        let _ = Command::new("tmux")
            .args(["select-window", "-t", &format!("{}:1", name)])
            .output();

        Ok(())
    }

    /// Attach to an existing tmux session
    pub fn attach_session(name: &str) -> Result<()> {
        let status = Command::new("tmux")
            .args(["attach-session", "-t", name])
            .status()?;
        if !status.success() {
            bail!("Failed to attach to tmux session '{}'", name);
        }
        Ok(())
    }

    /// Open a new window in the current tmux session
    pub fn new_window(title: Option<&str>, command: Option<&str>) -> Result<()> {
        let mut cmd = Command::new("tmux");
        cmd.arg("new-window");
        if let Some(t) = title {
            cmd.args(["-n", t]);
        }
        if let Some(c) = command {
            cmd.arg(c);
        }
        let status = cmd.status()?;
        if !status.success() {
            bail!("Failed to create new tmux window");
        }
        Ok(())
    }

    /// Select a target window in the current session
    pub fn select_window(target: &str) -> Result<()> {
        let status = Command::new("tmux")
            .args(["select-window", "-t", target])
            .status()?;
        if !status.success() {
            bail!("Failed to select window '{}'", target);
        }
        Ok(())
    }

    /// Rename the current window
    pub fn rename_window(title: &str) -> Result<()> {
        let status = Command::new("tmux")
            .args(["rename-window", title])
            .status()?;
        if !status.success() {
            bail!("Failed to rename window to '{}'", title);
        }
        Ok(())
    }

    /// Kill target window (or current window if None)
    pub fn kill_window(target: Option<&str>) -> Result<()> {
        let mut cmd = Command::new("tmux");
        cmd.arg("kill-window");
        if let Some(t) = target {
            cmd.args(["-t", t]);
        }
        let status = cmd.status()?;
        if !status.success() {
            bail!("Failed to kill window");
        }
        Ok(())
    }

    /// Get current session name if inside tmux
    pub fn current_session() -> Option<String> {
        let output = Command::new("tmux")
            .args(["display-message", "-p", "#{session_name}"])
            .output()
            .ok()?;
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
    }

    /// Get current window name if inside tmux
    pub fn current_window_name() -> Option<String> {
        let output = Command::new("tmux")
            .args(["display-message", "-p", "#{window_name}"])
            .output()
            .ok()?;
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
        None
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
    fn test_session_exists_nonexistent() {
        assert!(!TmuxDriver::session_exists(
            "__agymux_nonexistent_test_session_xyz__"
        ));
    }
}
