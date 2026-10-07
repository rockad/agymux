use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for TmuxConfig {
    fn default() -> Self {
        Self {
            isolated_socket: false,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            theme: ThemeConfig::default(),
            keybindings: KeybindingsConfig::default(),
            tmux: TmuxConfig::default(),
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
