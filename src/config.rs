use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub keybindings: KeybindingsConfig,
    #[serde(default)]
    pub tmux: TmuxConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    #[serde(default = "default_projects_dir")]
    pub projects_dir: String,
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default = "default_effort")]
    pub default_effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    #[serde(default = "default_preset")]
    pub preset: String,
    #[serde(default = "default_true")]
    pub rounded_pills: bool,
    #[serde(default = "default_status_position")]
    pub status_position: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingsConfig {
    #[serde(default = "default_prefix")]
    pub prefix: String,
    #[serde(default = "default_true")]
    pub bilingual_cyrillic: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TmuxConfig {
    #[serde(default)]
    pub isolated_socket: bool,
}

fn default_projects_dir() -> String {
    "~/projects".to_string()
}
fn default_model() -> String {
    "Gemini 3.6 Flash (Medium)".to_string()
}
fn default_effort() -> String {
    "medium".to_string()
}
fn default_preset() -> String {
    "monokai-pro-octagon".to_string()
}
fn default_true() -> bool {
    true
}
fn default_status_position() -> String {
    "top".to_string()
}
fn default_prefix() -> String {
    "C-Space".to_string()
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            projects_dir: default_projects_dir(),
            default_model: default_model(),
            default_effort: default_effort(),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            preset: default_preset(),
            rounded_pills: default_true(),
            status_position: default_status_position(),
        }
    }
}

impl Default for KeybindingsConfig {
    fn default() -> Self {
        Self {
            prefix: default_prefix(),
            bilingual_cyrillic: default_true(),
        }
    }
}

impl Config {
    /// Return the canonical config directory: $XDG_CONFIG_HOME/agymux or ~/.config/agymux
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("agymux")
    }

    /// Return the canonical config file path
    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    /// Load configuration from file, falling back to defaults if not found
    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file at {:?}", path))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config TOML at {:?}", path))?;
        Ok(config)
    }

    /// Save default configuration if none exists
    pub fn save_default_if_missing() -> Result<()> {
        let path = Self::config_path();
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let default_cfg = Self::default();
            let toml_str = toml::to_string_pretty(&default_cfg)?;
            fs::write(&path, toml_str)?;
        }
        Ok(())
    }

    /// Expand tilde in path string
    pub fn expand_path(p: &str) -> PathBuf {
        if let Some(stripped) = p.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(stripped);
            }
        }
        PathBuf::from(p)
    }

    /// Get resolved projects directory PathBuf
    pub fn resolved_projects_dir(&self) -> PathBuf {
        Self::expand_path(&self.general.projects_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_default_config_values() {
        let config = Config::default();
        assert_eq!(config.general.projects_dir, "~/projects");
        assert_eq!(config.general.default_model, "Gemini 3.6 Flash (Medium)");
        assert_eq!(config.general.default_effort, "medium");
        assert_eq!(config.theme.preset, "monokai-pro-octagon");
        assert!(config.theme.rounded_pills);
        assert_eq!(config.theme.status_position, "top");
        assert_eq!(config.keybindings.prefix, "C-Space");
        assert!(config.keybindings.bilingual_cyrillic);
        assert!(!config.tmux.isolated_socket);
    }

    #[test]
    fn test_config_deserialize_custom_toml() {
        let toml_str = r#"
[general]
projects_dir = "/custom/projects"
default_model = "Claude 3.7 Sonnet"
default_effort = "high"

[theme]
preset = "catppuccin-mocha"
rounded_pills = false
status_position = "bottom"

[keybindings]
prefix = "C-a"
bilingual_cyrillic = false

[tmux]
isolated_socket = true
"#;
        let config: Config = toml::from_str(toml_str).expect("custom toml parses");
        assert_eq!(config.general.projects_dir, "/custom/projects");
        assert_eq!(config.general.default_model, "Claude 3.7 Sonnet");
        assert_eq!(config.general.default_effort, "high");
        assert_eq!(config.theme.preset, "catppuccin-mocha");
        assert!(!config.theme.rounded_pills);
        assert_eq!(config.theme.status_position, "bottom");
        assert_eq!(config.keybindings.prefix, "C-a");
        assert!(!config.keybindings.bilingual_cyrillic);
        assert!(config.tmux.isolated_socket);
    }

    #[test]
    fn test_config_partial_toml_fallbacks() {
        let toml_str = r#"
[general]
default_effort = "low"
"#;
        let config: Config = toml::from_str(toml_str).expect("partial toml parses");
        assert_eq!(config.general.default_effort, "low");
        assert_eq!(config.general.projects_dir, "~/projects");
        assert_eq!(config.theme.preset, "monokai-pro-octagon");
        assert_eq!(config.keybindings.prefix, "C-Space");
    }

    #[test]
    fn test_config_expand_path() {
        let expanded = Config::expand_path("~/work");
        if let Some(home) = dirs::home_dir() {
            assert_eq!(expanded, home.join("work"));
        }

        let non_tilde = Config::expand_path("/var/log");
        assert_eq!(non_tilde, PathBuf::from("/var/log"));
    }

    #[test]
    fn test_config_invalid_toml_fails() {
        let bad_toml = "general = [unclosed array";
        assert!(toml::from_str::<Config>(bad_toml).is_err());
    }

    #[test]
    fn test_config_save_default_if_missing() {
        let dir = tempdir().unwrap();
        let target_file = dir.path().join("sub").join("config.toml");
        assert!(!target_file.exists());

        // Test creating parent directory and serializing default
        if let Some(parent) = target_file.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let default_cfg = Config::default();
        let toml_str = toml::to_string_pretty(&default_cfg).unwrap();
        fs::write(&target_file, toml_str).unwrap();

        assert!(target_file.exists());
        let reloaded: Config = toml::from_str(&fs::read_to_string(&target_file).unwrap()).unwrap();
        assert_eq!(reloaded.general.projects_dir, "~/projects");
    }
}
