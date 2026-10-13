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

        let mut lines = Vec::new();
        lines.push("# agymux dynamic tmux configuration".to_string());
        lines.push("set -g default-terminal \"tmux-256color\"".to_string());
        lines.push("set -ga terminal-overrides \",*256col*:Tc\"".to_string());
        lines.push(
            "set -ga terminal-features \",xterm-ghostty:RGB:sync:cstyle:ccolour\"".to_string(),
        );
        lines.push("set -s escape-time 0".to_string());
        lines.push("set -g focus-events on".to_string());
        lines.push("set -g mouse on".to_string());
        lines.push("set -g base-index 1".to_string());
        lines.push("setw -g pane-base-index 1".to_string());
        lines.push("set -g renumber-windows on".to_string());

        // Status bar layout (inherits terminal background and foreground by default)
        lines.push(format!("set -g status-position {}", position));
        lines.push("set -g status-interval 1".to_string());

        let status_style = config
            .theme
            .status_style
            .as_deref()
            .unwrap_or("bg=default,fg=default");
        lines.push(format!("set -g status-style \"{}\"", status_style));
        lines.push("set -g status-left-length 40".to_string());
        lines.push("set -g status-right-length 60".to_string());

        // Status Left: inherit terminal colors unless explicitly overridden
        let status_left = config
            .theme
            .status_left
            .as_deref()
            .unwrap_or("#{?client_prefix,#[fg=black]#[bg=brightred]#[bold] 󰘳 PREFIX #[default] ,#[fg=magenta] #S #[default] }#[fg=brightblack]|#[default] ");
        lines.push(format!("set -g status-left \"{}\"", status_left));

        lines.push("setw -g window-status-separator \" \"".to_string());

        // Window status format: inactive tab with distinct dark background, and highlighted alert on action/response required
        let window_format = config
            .theme
            .window_status_format
            .as_deref()
            .unwrap_or("#{?#{||:#{window_activity_flag},#{window_bell_flag}},#[fg=black]#[bg=yellow]#[bold] 󰂞 #I #W #[default],#[fg=white]#[bg=brightblack] #I #W #[default]}");
        lines.push(format!(
            "setw -g window-status-format \"{}\"",
            window_format
        ));

        // Active window format: highlighted active tab using terminal green
        let active_window_format = config
            .theme
            .window_status_current_format
            .as_deref()
            .unwrap_or("#[fg=black]#[bg=green]#[bold] #I #W #[default]");
        lines.push(format!(
            "setw -g window-status-current-format \"{}\"",
            active_window_format
        ));

        // Status Right: Model info & time (inherits terminal colors)
        let status_right = config
            .theme
            .status_right
            .as_deref()
            .unwrap_or("#[fg=brightblack]%H:%M #[fg=cyan,bold]agymux #[default]");
        lines.push(format!("set -g status-right \"{}\"", status_right));

        // Prefix key
        lines.push("unbind C-b".to_string());
        lines.push(format!("set -g prefix {}", prefix));
        lines.push(format!("bind {} send-prefix", prefix));

        // Keybindings & popups
        // Switcher (Local tab/conv picker)
        lines.push(
            "bind s display-popup -E -w 85% -h 80% \"agymux switch --scope local\"".to_string(),
        );
        lines.push(
            "bind S display-popup -E -w 85% -h 80% \"agymux switch --scope global\"".to_string(),
        );

        // Project picker
        lines.push("bind P display-popup -E -w 70% -h 70% \"agymux project\"".to_string());

        // Command palette
        lines.push("bind p display-popup -E -w 60% -h 65% \"agymux palette\"".to_string());

        // Shortcut helper / cheatsheet
        lines.push("bind h display-popup -E -w 65% -h 70% \"agymux helper\"".to_string());

        // Tab management
        lines.push(
            "bind c new-window -c \"#{pane_current_path}\" -n \"+ new\" \"agymux run\"".to_string(),
        );
        lines.push("bind x kill-window".to_string());
        lines.push("bind d detach-client".to_string());
        lines.push("bind r refresh-client".to_string());
        lines.push("bind q detach-client".to_string());
        lines.push("bind b display-popup -E -w 75% -h 60% \"agymux bind\"".to_string());
        lines.push("bind t new-window -n \"term\" \"$SHELL\"".to_string());
        lines.push(
            "bind m run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\"".to_string(),
        );

        // Bilingual Cyrillic bindings
        if cyrillic {
            lines.push("# Bilingual RussianWin pairings".to_string());
            lines.push(
                "bind ы display-popup -E -w 85% -h 80% \"agymux switch --scope local\"".to_string(),
            );
            lines.push(
                "bind Ы display-popup -E -w 85% -h 80% \"agymux switch --scope global\""
                    .to_string(),
            );
            lines.push("bind З display-popup -E -w 70% -h 70% \"agymux project\"".to_string());
            lines.push("bind з display-popup -E -w 60% -h 65% \"agymux palette\"".to_string());
            lines.push("bind р display-popup -E -w 65% -h 70% \"agymux helper\"".to_string());
            lines.push(
                "bind с new-window -c \"#{pane_current_path}\" -n \"+ new\" \"agymux run\""
                    .to_string(),
            );
            lines.push("bind ч kill-window".to_string());
            lines.push("bind в detach-client".to_string());
            lines.push("bind к refresh-client".to_string());
            lines.push("bind й detach-client".to_string());
            lines.push("bind и display-popup -E -w 75% -h 60% \"agymux bind\"".to_string());
            lines.push("bind е new-window -n \"term\" \"$SHELL\"".to_string());
            lines.push(
                "bind ь run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\"".to_string(),
            );
        }

        // Direct tab switching (Prefix + 1..9)
        for i in 1..=9 {
            lines.push(format!("bind {} select-window -t :{}", i, i));
        }

        // Cycling (Prefix + Left / Right)
        lines.push("bind Left previous-window".to_string());
        lines.push("bind Right next-window".to_string());

        lines.join("\n") + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_no_commas_in_style_brackets() {
        let config = Config::default();
        let content = TmuxProfile::render_content(&config);

        // Scan status-left conditional #[...] brackets and verify none contain commas
        let status_left = content
            .lines()
            .find(|l| l.contains("status-left"))
            .expect("status-left found");
        let mut idx = 0;
        while let Some(open) = status_left[idx..].find("#[") {
            let start = idx + open + 2;
            if let Some(close) = status_left[start..].find(']') {
                let style_token = &status_left[start..start + close];
                assert!(
                    !style_token.contains(','),
                    "Found illegal comma inside status-left style bracket #[{}]: would break tmux status conditional formatting",
                    style_token
                );
                idx = start + close + 1;
            } else {
                break;
            }
        }
    }

    #[test]
    fn test_profile_flat_tabs_and_defaults() {
        let config = Config::default();
        let content = TmuxProfile::render_content(&config);

        // Flat tabs and terminal inheritance by default (zero hardcoded theme, zero rounded glyphs)
        assert!(!content.contains(''));
        assert!(!content.contains(''));
        assert!(content.contains("set -g status-style \"bg=default,fg=default\""));
        assert!(content.contains("setw -g window-status-format \"#{?#{||:#{window_activity_flag},#{window_bell_flag}},#[fg=black]#[bg=yellow]#[bold] 󰂞 #I #W #[default],#[fg=white]#[bg=brightblack] #I #W #[default]}\""));
        assert!(content
            .contains("setw -g window-status-current-format \"#[fg=black]#[bg=green]#[bold] #I #W #[default]\""));
        assert!(content.contains("set -g status-left \"#{?client_prefix,#[fg=black]#[bg=brightred]#[bold] 󰘳 PREFIX #[default] ,#[fg=magenta] #S #[default] }#[fg=brightblack]|#[default] \""));
        assert!(content.contains("set -g status-position top"));
        assert!(content.contains("set -g prefix C-Space"));
        assert!(content
            .contains("bind c new-window -c \"#{pane_current_path}\" -n \"+ new\" \"agymux run\""));
        assert!(content.contains("bind q detach-client"));
        assert!(content.contains("bind b display-popup -E -w 75% -h 60% \"agymux bind\""));
        assert!(content.contains("bind t new-window -n \"term\" \"$SHELL\""));
        assert!(content.contains("bind m run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\""));
        // Check cyrillic bindings by default
        assert!(content.contains("bind с new-window"));
        assert!(content.contains("bind ы display-popup"));
        assert!(content.contains("bind й detach-client"));
        assert!(content.contains("bind и display-popup -E -w 75% -h 60% \"agymux bind\""));
        assert!(content.contains("bind е new-window -n \"term\" \"$SHELL\""));
        assert!(content.contains("bind ь run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\""));
    }

    #[test]
    fn test_profile_config_style_overrides() {
        let mut config = Config::default();
        config.theme.status_style = Some("bg=black,fg=white".to_string());
        config.theme.window_status_current_format =
            Some("#[bold,fg=green] #I #W #[default]".to_string());
        let content = TmuxProfile::render_content(&config);

        assert!(content.contains("set -g status-style \"bg=black,fg=white\""));
        assert!(content.contains("setw -g window-status-current-format \"#[bold,fg=green] #I #W #[default]\""));
    }

    #[test]
    fn test_profile_default_inherits_terminal() {
        let config = Config::default();
        let content = TmuxProfile::render_content(&config);

        assert!(content.contains("set -g status-style \"bg=default,fg=default\""));
        assert!(content.contains("setw -g window-status-format \"#{?#{||:#{window_activity_flag},#{window_bell_flag}},#[fg=black]#[bg=yellow]#[bold] 󰂞 #I #W #[default],#[fg=white]#[bg=brightblack] #I #W #[default]}\""));
        assert!(content.contains("setw -g window-status-current-format \"#[fg=black]#[bg=green]#[bold] #I #W #[default]\""));
        assert!(content.contains("set -g status-left \"#{?client_prefix,#[fg=black]#[bg=brightred]#[bold] 󰘳 PREFIX #[default] ,#[fg=magenta] #S #[default] }#[fg=brightblack]|#[default] \""));
    }

    #[test]
    fn test_profile_custom_prefix_and_cyrillic_disabled() {
        let mut config = Config::default();
        config.keybindings.prefix = "C-a".to_string();
        config.theme.status_position = "bottom".to_string();
        config.keybindings.bilingual_cyrillic = false;

        let content = TmuxProfile::render_content(&config);
        assert!(content.contains("set -g status-position bottom"));
        assert!(content.contains("set -g prefix C-a"));
        assert!(content.contains("bind C-a send-prefix"));
        assert!(!content.contains("bind ы display-popup"));
        assert!(!content.contains("bind с new-window"));
    }

    #[test]
    fn test_profile_file_generation() {
        let config = Config::default();
        let path = TmuxProfile::generate(&config, "test-session-profile").unwrap();
        assert!(path.exists());
        let read = std::fs::read_to_string(&path).unwrap();
        assert!(read.contains("agymux dynamic tmux configuration"));
        let _ = std::fs::remove_file(path);
    }
}
