use crate::config::Config;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub struct TmuxProfile;

impl TmuxProfile {
    pub fn generate(config: &Config, session_name: &str) -> Result<PathBuf> {
        let conf_path = std::env::temp_dir().join(format!("agymux-{}.conf", session_name));
        let content = Self::render_content(config);
        fs::write(&conf_path, content)?;
        Ok(conf_path)
    }

    pub fn render_content(config: &Config) -> String {
        let prefix = &config.keybindings.prefix;
        let position = &config.theme.status_position;
        let cyrillic = config.keybindings.bilingual_cyrillic;

        // Monokai Pro Octagon tokens
        let bg = "#282a3a";
        let fg = "#eaf2f1";
        let inactive_bg = "#221f22";
        let inactive_fg = "#727072";
        let active_accent = "#a9dc76"; // green pill
        let active_text = "#282a3a";
        let prefix_alert = "#ff6188";  // pink
        let _running_dot = "#fc9867";  // orange

        let mut lines = Vec::new();
        lines.push("# agymux dynamic tmux configuration".to_string());
        lines.push("set -g default-terminal \"tmux-256color\"".to_string());
        lines.push("set -ga terminal-overrides \",*256col*:Tc\"".to_string());
        lines.push("set -s escape-time 0".to_string());
        lines.push("set -g focus-events on".to_string());
        lines.push("set -g mouse on".to_string());
        lines.push("set -g base-index 1".to_string());
        lines.push("setw -g pane-base-index 1".to_string());
        lines.push("set -g renumber-windows on".to_string());

        // Status bar layout
        lines.push(format!("set -g status-position {}", position));
        lines.push("set -g status-interval 1".to_string());
        lines.push(format!("set -g status-style \"bg={},fg={}\"", bg, fg));
        lines.push("set -g status-left-length 40".to_string());
        lines.push("set -g status-right-length 60".to_string());

        // Status Left: Prefix indicator & session name
        let status_left = format!(
            "#[bg={},fg={},bold] #S #[bg={},fg={}]#{p_alert} ",
            bg, fg, bg, fg,
            p_alert = format!("?client_prefix,#[fg={},bold] PREFIX ,#[fg={}][NORMAL]", prefix_alert, inactive_fg)
        );
        lines.push(format!("set -g status-left \"{}\"", status_left));

        // Window status format
        let window_format = format!(
            "#[fg={},bg={}] #I #[fg={},bg={}] #W ",
            inactive_fg, inactive_bg, fg, inactive_bg
        );
        let active_window_format = format!(
            "#[fg={},bg={}]#[fg={},bg={},bold]#I:#W#[fg={},bg={}]",
            active_accent, bg, active_text, active_accent, active_accent, bg
        );

        lines.push(format!("setw -g window-status-format \"{}\"", window_format));
        lines.push(format!("setw -g window-status-current-format \"{}\"", active_window_format));

        // Status Right: Model info & time
        let status_right = format!(
            "#[fg={}]agy #[fg={},bg={}] %H:%M #[default]",
            inactive_fg, fg, bg
        );
        lines.push(format!("set -g status-right \"{}\"", status_right));

        // Prefix key
        lines.push(format!("unbind C-b"));
        lines.push(format!("set -g prefix {}", prefix));
        lines.push(format!("bind {} send-prefix", prefix));

        // Keybindings & popups
        // Switcher (Local tab/conv picker)
        lines.push("bind s display-popup -E -w 85% -h 80% \"agymux switch --scope local\"".to_string());
        lines.push("bind S display-popup -E -w 85% -h 80% \"agymux switch --scope global\"".to_string());

        // Project picker
        lines.push("bind P display-popup -E -w 70% -h 70% \"agymux project\"".to_string());

        // Command palette
        lines.push("bind p display-popup -E -w 60% -h 65% \"agymux palette\"".to_string());

        // Shortcut helper / cheatsheet
        lines.push("bind h display-popup -E -w 65% -h 70% \"agymux helper\"".to_string());

        // Tab management
        lines.push("bind c new-window -c \"#{pane_current_path}\" \"agymux run\"".to_string());
        lines.push("bind x kill-window".to_string());
        lines.push("bind d detach-client".to_string());
        lines.push("bind r refresh-client".to_string());

        // Bilingual Cyrillic bindings
        if cyrillic {
            lines.push("# Bilingual RussianWin pairings".to_string());
            lines.push("bind ы display-popup -E -w 85% -h 80% \"agymux switch --scope local\"".to_string());
            lines.push("bind Ы display-popup -E -w 85% -h 80% \"agymux switch --scope global\"".to_string());
            lines.push("bind З display-popup -E -w 70% -h 70% \"agymux project\"".to_string());
            lines.push("bind з display-popup -E -w 60% -h 65% \"agymux palette\"".to_string());
            lines.push("bind р display-popup -E -w 65% -h 70% \"agymux helper\"".to_string());
            lines.push("bind с new-window -c \"#{pane_current_path}\" \"agymux run\"".to_string());
            lines.push("bind ч kill-window".to_string());
            lines.push("bind в detach-client".to_string());
            lines.push("bind к refresh-client".to_string());
        }

        // Direct tab switching Alt+1..9
        for i in 1..=9 {
            lines.push(format!("bind -n M-{} select-window -t :{}", i, i));
        }

        // Cycling Alt+Left / Alt+Right
        lines.push("bind -n M-Left previous-window".to_string());
        lines.push("bind -n M-Right next-window".to_string());

        lines.join("\n") + "\n"
    }
}
