use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectItem {
    pub name: String,
    pub path: PathBuf,
    pub active_windows: usize,
}

pub struct ProjectScanner;

impl ProjectScanner {
    /// Scan directories containing `.git` under base_dir and annotate with active tmux window counts
    pub fn scan_projects(base_dir: &Path) -> Result<Vec<ProjectItem>> {
        if !base_dir.exists() {
            return Ok(Vec::new());
        }

        let active_map = Self::query_active_tmux_sessions();
        let mut discovered = Vec::new();
        let mut seen_paths = HashSet::new();

        // 1. If base_dir itself is a git repo
        if base_dir.join(".git").exists() {
            let canon = base_dir
                .canonicalize()
                .unwrap_or_else(|_| base_dir.to_path_buf());
            let name = base_dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "project".to_string());
            let active_windows = Self::resolve_active_windows(&name, &active_map);
            seen_paths.insert(canon.clone());
            discovered.push(ProjectItem {
                name,
                path: canon,
                active_windows,
            });
        }

        // 2. Discover subdirectories containing .git up to depth 3
        for entry in WalkDir::new(base_dir)
            .max_depth(3)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() > 0 && e.file_name() == ".git" {
                    return true;
                }
                if e.depth() > 0 && e.file_name().to_string_lossy().starts_with('.') {
                    return false;
                }
                true
            })
            .filter_map(|e| e.ok())
        {
            if entry.file_name() == ".git" {
                if let Some(parent) = entry.path().parent() {
                    let canon = parent
                        .canonicalize()
                        .unwrap_or_else(|_| parent.to_path_buf());
                    if seen_paths.insert(canon.clone()) {
                        let name = parent
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| "project".to_string());
                        let active_windows = Self::resolve_active_windows(&name, &active_map);
                        discovered.push(ProjectItem {
                            name,
                            path: canon,
                            active_windows,
                        });
                    }
                }
            }
        }

        // Sort descending by active windows, then alphabetically by project name
        discovered.sort_by(|a, b| {
            b.active_windows
                .cmp(&a.active_windows)
                .then_with(|| a.name.cmp(&b.name))
        });

        Ok(discovered)
    }

    /// Resolve active window count for a project name against tmux session map
    pub fn resolve_active_windows(name: &str, active_map: &HashMap<String, usize>) -> usize {
        let prefixed = format!("agy-{}", name);
        if let Some(&count) = active_map.get(&prefixed) {
            return count;
        }
        if let Some(&count) = active_map.get(name) {
            return count;
        }
        0
    }

    /// Query tmux for active sessions and window counts
    pub fn query_active_tmux_sessions() -> HashMap<String, usize> {
        let mut map = HashMap::new();
        let output = match Command::new("tmux")
            .args(["list-sessions", "-F", "#{session_name}\t#{session_windows}"])
            .output()
        {
            Ok(o) => o,
            Err(_) => return map,
        };

        if !output.status.success() {
            return map;
        }

        let text = String::from_utf8_lossy(&output.stdout);
        for line in text.lines() {
            let parts: Vec<&str> = line.trim().split('\t').collect();
            if parts.len() >= 2 {
                let sname = parts[0];
                let count = parts[1].parse::<usize>().unwrap_or(0);
                map.insert(sname.to_string(), count);
                if let Some(stripped) = sname.strip_prefix("agy-") {
                    map.insert(stripped.to_string(), count);
                }
            }
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_scan_projects() {
        let temp_dir = std::env::temp_dir().join(format!(
            "agymux_test_scan_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_a = temp_dir.join("repo-alpha");
        let repo_b = temp_dir.join("repo-beta");
        let non_repo = temp_dir.join("regular-folder");

        fs::create_dir_all(repo_a.join(".git")).unwrap();
        fs::create_dir_all(repo_b.join(".git")).unwrap();
        fs::create_dir_all(&non_repo).unwrap();

        let scanned = ProjectScanner::scan_projects(&temp_dir).expect("scan should succeed");
        assert_eq!(scanned.len(), 2);

        let names: Vec<String> = scanned.into_iter().map(|p| p.name).collect();
        assert!(names.contains(&"repo-alpha".to_string()));
        assert!(names.contains(&"repo-beta".to_string()));
        assert!(!names.contains(&"regular-folder".to_string()));

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_resolve_active_windows() {
        let mut map = HashMap::new();
        map.insert("agy-myproject".to_string(), 3);
        map.insert("other".to_string(), 1);

        assert_eq!(
            ProjectScanner::resolve_active_windows("myproject", &map),
            3
        );
        assert_eq!(ProjectScanner::resolve_active_windows("other", &map), 1);
        assert_eq!(ProjectScanner::resolve_active_windows("unknown", &map), 0);
    }
}
