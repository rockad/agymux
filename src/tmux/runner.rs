use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::tmux::driver::TmuxWindow;

/// Trait abstracting execution of tmux commands
pub trait TmuxCommandRunner: Send + Sync {
    fn run(&self, args: &[&str]) -> Result<String>;
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
        crate::tmux::driver::TmuxDriver::parse_windows(&output)
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

    pub fn rename_window(&self, title: &str) -> Result<()> {
        self.runner.run(&["rename-window", title])?;
        Ok(())
    }

    pub fn kill_window(&self, target: Option<&str>) -> Result<()> {
        if let Some(t) = target {
            self.runner.run(&["kill-window", "-t", t])?;
        } else {
            self.runner.run(&["kill-window"])?;
        }
        Ok(())
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
        self.runner.run(&["attach-session", "-t", name])?;
        Ok(())
    }

    pub fn ensure_session(
        &self,
        name: &str,
        dir: &Path,
        config: &crate::config::Config,
    ) -> Result<()> {
        if self.session_exists(name) {
            return Ok(());
        }

        let conf_path = crate::tmux::TmuxProfile::generate(config, name)?;
        let dir_str = dir.to_string_lossy();

        self.runner.run(&[
            "new-session",
            "-d",
            "-s",
            name,
            "-c",
            &dir_str,
            "-n",
            "main",
        ])?;
        let _ = self
            .runner
            .run(&["source-file", &conf_path.to_string_lossy()]);
        let _ = self
            .runner
            .run(&["select-window", "-t", &format!("{}:1", name)]);

        Ok(())
    }
}
