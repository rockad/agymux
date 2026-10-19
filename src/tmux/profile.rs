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
        let cyrillic = config.keybindings.bilingual_cyrillic;

        let mut lines = vec![
            "# agymux dynamic tmux configuration".to_string(),
            "set -g default-terminal \"tmux-256color\"".to_string(),
            "set -ga terminal-overrides \",*256col*:Tc\"".to_string(),
            "set -ga terminal-features \",xterm-ghostty:RGB:sync:cstyle:ccolour\"".to_string(),
            "set -s escape-time 0".to_string(),
            "set -g focus-events on".to_string(),
            "set-hook -g pane-focus-in \"set-option -w @external_sync 0\"".to_string(),
            "set -g mouse on".to_string(),
            "set -g base-index 1".to_string(),
            "setw -g pane-base-index 1".to_string(),
            "set -g renumber-windows on".to_string(),
        ];

        // Status bar layout (inherits terminal background and foreground by default)
        let effective_position = if config.theme.tabs.position != "top" {
            &config.theme.tabs.position
        } else if config.theme.status_position != "top" {
            &config.theme.status_position
        } else {
            &config.theme.tabs.position
        };

        lines.push(format!("set -g status-position {}", effective_position));
        if config.theme.tabs.spacer_lines == 0 {
            lines.push("set -g status on".to_string());
        } else {
            lines.push("set -g status 2".to_string());
            if effective_position == "bottom" {
                lines.push("set -g \"status-format[1]\" \"#[align=left range=left #{E:status-left-style}]#[push-default]#{T;=/#{status-left-length}:status-left}#[pop-default]#[norange default]#[list=on align=#{status-justify}]#[list=left-marker]<#[list=right-marker]>#[list=on]#{W:#[range=window|#{window_index} #{E:window-status-style}#{?#{&&:#{window_last_flag},#{!=:#{E:window-status-last-style},default}}, #{E:window-status-last-style},}#{?#{&&:#{window_bell_flag},#{!=:#{E:window-status-bell-style},default}}, #{E:window-status-bell-style},#{?#{&&:#{||:#{window_activity_flag},#{window_silence_flag}},#{!=:#{E:window-status-activity-style},default}}, #{E:window-status-activity-style},}}]#[push-default]#{T:window-status-format}#[pop-default]#[norange default]#{?loop_last_flag,,#{window-status-separator}},#[range=window|#{window_index} list=focus #{?#{!=:#{E:window-status-current-style},default},#{E:window-status-current-style},#{E:window-status-style}}#{?#{&&:#{window_last_flag},#{!=:#{E:window-status-last-style},default}}, #{E:window-status-last-style},}#{?#{&&:#{window_bell_flag},#{!=:#{E:window-status-bell-style},default}}, #{E:window-status-bell-style},#{?#{&&:#{||:#{window_activity_flag},#{window_silence_flag}},#{!=:#{E:window-status-activity-style},default}}, #{E:window-status-activity-style},}}]#[push-default]#{T:window-status-current-format}#[pop-default]#[norange list=on default]#{?loop_last_flag,,#{window-status-separator}}}#[nolist align=right range=right #{E:status-right-style}]#[push-default]#{T;=/#{status-right-length}:status-right}#[pop-default]#[norange default]\"".to_string());
                lines.push("set -g \"status-format[0]\" \"#[bg=default,fill= ] \"".to_string());
            } else {
                lines.push("set -g \"status-format[1]\" \"#[bg=default,fill= ] \"".to_string());
            }
        }
        lines.push(format!(
            "set -g status-justify {}",
            config.theme.tabs.alignment
        ));
        lines.push("set -g status-interval 1".to_string());

        let status_style = config
            .theme
            .status_style
            .as_deref()
            .unwrap_or("bg=default,fg=default");
        lines.push(format!("set -g status-style \"{}\"", status_style));
        lines.push("set -g status-left-length 40".to_string());
        lines.push("set -g status-right-length 70".to_string());

        // Status Left: inherit terminal colors unless explicitly overridden (dynamic prefix indicator)
        let status_left = config.theme.status_left.as_deref().unwrap_or(
            "#{?#{client_prefix},#[fg=black]#[bg=brightred]#[bold] 󰘳 PREFIX #[default] ,}",
        );
        lines.push(format!("set -g status-left \"{}\"", status_left));

        lines.push("setw -g window-status-separator \" \"".to_string());

        // Window status format: inactive tab with soft text on dark slate background, and alert on action required
        let close_suffix = if config.theme.tabs.show_close_button {
            "#[range=user|close] 󰅖#[norange] "
        } else {
            " "
        };
        let close_suffix_dim = if config.theme.tabs.show_close_button {
            "#[range=user|close] #[fg=colour244]󰅖#[fg=colour250]#[norange] "
        } else {
            " "
        };

        let tab_width = config.theme.tabs.standard_width;
        let title_token = format!("#{{={}:window_name}}", tab_width);

        let default_window_format = format!(
            "#{{?#{{||:#{{window_activity_flag}},#{{window_bell_flag}}}},#[fg=colour16]#[bg=colour220]#[bold] 󰂞 #I {title_token}#{{?#{{@remote_control}}, 󰢹,}}{close_suffix}#[default],#{{?#{{@external_sync}},#[fg=colour39]#[bg=colour237]#[bold] 󰛌 #I {title_token}#{{?#{{@remote_control}}, 󰢹,}}{close_suffix}#[default],#[fg=colour250]#[bg=colour237] #I {title_token}#{{?#{{@remote_control}}, #[fg=colour75]󰢹#[fg=colour250],}}{close_suffix_dim}#[default]}}}}"
        );
        let window_format = config
            .theme
            .window_status_format
            .as_deref()
            .unwrap_or(&default_window_format);
        lines.push(format!(
            "setw -g window-status-format \"{}\"",
            window_format
        ));

        // Active window format: focused tab using muted teal accent (colour73) with dark ink text
        let default_active_window_format = format!(
            "#[fg=colour234]#[bg=colour73]#[bold] #I {title_token}#{{?#{{@remote_control}}, 󰢹,}}{close_suffix}#[default]"
        );
        let active_window_format = config
            .theme
            .window_status_current_format
            .as_deref()
            .unwrap_or(&default_active_window_format);
        lines.push(format!(
            "setw -g window-status-current-format \"{}\"",
            active_window_format
        ));

        // Status Right: Model info & project · time · date (distinctive text colors)
        let status_right = config
            .theme
            .status_right
            .as_deref()
            .unwrap_or("#{?#{@agy_running},#[fg=yellow,bold]● RUNNING  ,}#[fg=magenta]#S #[fg=white]· #[fg=white]%H:%M #[fg=white]· #[fg=white]%Y-%m-%d #[default]");
        lines.push(format!("set -g status-right \"{}\"", status_right));

        // Prefix key
        lines.push("unbind C-b".to_string());
        lines.push(format!("set -g prefix {}", prefix));
        lines.push(format!("bind {} send-prefix", prefix));

        // Keybindings & popups
        // Switcher (2-pane switcher with live preview)
        lines.push("bind b display-popup -E -w 85% -h 75% \"agymux switch\"".to_string());
        lines.push("bind B display-popup -E -w 85% -h 75% \"agymux switch\"".to_string());

        // Project switcher
        lines.push("bind p display-popup -E -w 85% -h 70% \"agymux project\"".to_string());
        lines.push("bind P display-popup -E -w 85% -h 70% \"agymux project\"".to_string());

        // Actions / Command palette
        lines.push("bind a display-popup -E -w 75% -h 60% \"agymux palette\"".to_string());
        lines.push("bind A display-popup -E -w 75% -h 60% \"agymux palette\"".to_string());

        // Tab management
        lines.push("bind c new-window -n \"+ new\" \"agymux run\"".to_string());
        lines.push("bind C new-window -n \"+ new\" \"agymux run\"".to_string());

        lines.push("bind x confirm-before -p \"Close tab #W? (y/n)\" kill-window".to_string());
        lines.push("bind X confirm-before -p \"Close tab #W? (y/n)\" kill-window".to_string());

        lines.push("bind r command-prompt -I \"#W\" \"rename-window '%%'\"".to_string());
        lines.push("bind R command-prompt -I \"#W\" \"rename-window '%%'\"".to_string());

        lines.push("bind d detach-client".to_string());
        lines.push("bind D detach-client".to_string());

        lines.push("bind k confirm-before -p \"Kill session #S? (y/n)\" kill-session".to_string());
        lines.push("bind K confirm-before -p \"Kill session #S? (y/n)\" kill-session".to_string());

        // Shortcut helper / cheatsheet
        lines.push("bind h display-popup -E -w 75% -h 70% \"agymux helper\"".to_string());
        lines.push("bind H display-popup -E -w 75% -h 70% \"agymux helper\"".to_string());
        lines.push("bind ? display-popup -E -w 75% -h 70% \"agymux helper\"".to_string());

        lines.push("bind t new-window -n \"term\" \"$SHELL\"".to_string());
        lines.push("bind T new-window -n \"term\" \"$SHELL\"".to_string());

        lines.push(
            "bind m run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\"".to_string(),
        );
        lines.push(
            "bind M run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\"".to_string(),
        );

        // Bilingual Cyrillic bindings
        if cyrillic {
            lines.push("# Bilingual RussianWin pairings".to_string());
            lines.push("bind и display-popup -E -w 85% -h 75% \"agymux switch\"".to_string());
            lines.push("bind И display-popup -E -w 85% -h 75% \"agymux switch\"".to_string());

            lines.push("bind з display-popup -E -w 85% -h 70% \"agymux project\"".to_string());
            lines.push("bind З display-popup -E -w 85% -h 70% \"agymux project\"".to_string());

            lines.push("bind ф display-popup -E -w 75% -h 60% \"agymux palette\"".to_string());
            lines.push("bind Ф display-popup -E -w 75% -h 60% \"agymux palette\"".to_string());

            lines.push("bind с new-window -n \"+ new\" \"agymux run\"".to_string());
            lines.push("bind С new-window -n \"+ new\" \"agymux run\"".to_string());

            lines.push("bind ч confirm-before -p \"Close tab #W? (y/n)\" kill-window".to_string());
            lines.push("bind Ч confirm-before -p \"Close tab #W? (y/n)\" kill-window".to_string());

            lines.push("bind к command-prompt -I \"#W\" \"rename-window '%%'\"".to_string());
            lines.push("bind К command-prompt -I \"#W\" \"rename-window '%%'\"".to_string());

            lines.push("bind в detach-client".to_string());
            lines.push("bind В detach-client".to_string());

            lines.push(
                "bind л confirm-before -p \"Kill session #S? (y/n)\" kill-session".to_string(),
            );
            lines.push(
                "bind Л confirm-before -p \"Kill session #S? (y/n)\" kill-session".to_string(),
            );

            lines.push("bind р display-popup -E -w 75% -h 70% \"agymux helper\"".to_string());
            lines.push("bind Р display-popup -E -w 75% -h 70% \"agymux helper\"".to_string());

            lines.push("bind е new-window -n \"term\" \"$SHELL\"".to_string());
            lines.push("bind Е new-window -n \"term\" \"$SHELL\"".to_string());

            lines.push(
                "bind ь run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\"".to_string(),
            );
            lines.push(
                "bind Ь run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\"".to_string(),
            );
        }

        // Direct tab switching (prefix-less Alt + 1..9)
        for i in 1..=9 {
            lines.push(format!("bind-key -n M-{} select-window -t :{}", i, i));
        }

        // Cycling (prefix-less Alt + Left / Right)
        lines.push("bind-key -n M-Left previous-window".to_string());
        lines.push("bind-key -n M-Right next-window".to_string());

        // Direct tab switching (Prefix + 1..9)
        for i in 1..=9 {
            lines.push(format!("bind {} select-window -t :{}", i, i));
        }

        // Cycling (Prefix + Left / Right)
        lines.push("bind Left previous-window".to_string());
        lines.push("bind Right next-window".to_string());

        // Mouse interactions for tabs
        lines.push("bind-key -T root MouseDown1Status if-shell -F '#{==:#{mouse_status_range},close}' { if-shell -F '#{>:#{session_windows},1}' { kill-window -t = } { confirm-before -p \"Close final tab #W? (y/n)\" \"kill-window -t =\" } } { switch-client -t = }".to_string());
        lines.push("bind-key -T root MouseDown2Status if-shell -F '#{>:#{session_windows},1}' { kill-window -t = } { confirm-before -p \"Close final tab #W? (y/n)\" \"kill-window -t =\" }".to_string());
        lines.push("bind-key -T root MouseDown3Status display-menu -t = -T \"#W Actions\" \"Close Tab\" x \"if-shell -F '#{>:#{session_windows},1}' { kill-window -t = } { confirm-before -p \\\"Close final tab #W? (y/n)\\\" \\\"kill-window -t =\\\" }\" \"Rename Tab\" r \"command-prompt -I '#W' 'rename-window -t = %%'\" \"Fork Tab\" f \"display-popup -E -w 65% -h 35% 'agymux palette fork'\" \"Reload Session\" R \"run-shell 'agymux palette -a reload'\"".to_string());

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

        for line_key in &[
            "status-left",
            "window-status-format",
            "window-status-current-format",
        ] {
            let line = content
                .lines()
                .find(|l| l.contains(line_key))
                .unwrap_or_else(|| panic!("{} found", line_key));
            let mut idx = 0;
            while let Some(open) = line[idx..].find("#[") {
                let start = idx + open + 2;
                if let Some(close) = line[start..].find(']') {
                    let style_token = &line[start..start + close];
                    assert!(
                        !style_token.contains(','),
                        "Found illegal comma inside {} style bracket #[{}]: would break tmux conditional formatting",
                        line_key,
                        style_token
                    );
                    idx = start + close + 1;
                } else {
                    break;
                }
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
        assert!(content.contains("set-hook -g pane-focus-in \"set-option -w @external_sync 0\""));
        assert!(content.contains("setw -g window-status-format \"#{?#{||:#{window_activity_flag},#{window_bell_flag}},#[fg=colour16]#[bg=colour220]#[bold] 󰂞 #I #{=22:window_name}#{?#{@remote_control}, 󰢹,}#[range=user|close] 󰅖#[norange] #[default],#{?#{@external_sync},#[fg=colour39]#[bg=colour237]#[bold] 󰛌 #I #{=22:window_name}#{?#{@remote_control}, 󰢹,}#[range=user|close] 󰅖#[norange] #[default],#[fg=colour250]#[bg=colour237] #I #{=22:window_name}#{?#{@remote_control}, #[fg=colour75]󰢹#[fg=colour250],}#[range=user|close] #[fg=colour244]󰅖#[fg=colour250]#[norange] #[default]}}\""));
        assert!(content
            .contains("setw -g window-status-current-format \"#[fg=colour234]#[bg=colour73]#[bold] #I #{=22:window_name}#{?#{@remote_control}, 󰢹,}#[range=user|close] 󰅖#[norange] #[default]\""));
        assert!(content.contains("set -g status-left \"#{?#{client_prefix},#[fg=black]#[bg=brightred]#[bold] 󰘳 PREFIX #[default] ,}\""));
        assert!(content.contains("set -g status-position top"));
        assert!(content.contains("set -g status 2"));
        assert!(content.contains("set -g \"status-format[1]\" \"#[bg=default,fill= ] \""));
        assert!(content.contains("set -g prefix C-Space"));
        assert!(content.contains("bind-key -T root MouseDown1Status"));
        assert!(content.contains("bind-key -T root MouseDown2Status"));
        assert!(content.contains("bind-key -T root MouseDown3Status"));
        assert!(content.contains("bind c new-window -n \"+ new\" \"agymux run\""));
        assert!(content.contains("bind C new-window -n \"+ new\" \"agymux run\""));
        assert!(content.contains("bind d detach-client"));
        assert!(content.contains("bind D detach-client"));
        assert!(
            content.contains("bind k confirm-before -p \"Kill session #S? (y/n)\" kill-session")
        );
        assert!(
            content.contains("bind K confirm-before -p \"Kill session #S? (y/n)\" kill-session")
        );
        assert!(content.contains("bind b display-popup -E -w 85% -h 75% \"agymux switch\""));
        assert!(content.contains("bind B display-popup -E -w 85% -h 75% \"agymux switch\""));
        assert!(content.contains("bind p display-popup -E -w 85% -h 70% \"agymux project\""));
        assert!(content.contains("bind P display-popup -E -w 85% -h 70% \"agymux project\""));
        assert!(content.contains("bind a display-popup -E -w 75% -h 60% \"agymux palette\""));
        assert!(content.contains("bind A display-popup -E -w 75% -h 60% \"agymux palette\""));
        assert!(!content.contains("C-S-p"));
        assert!(!content.contains("C-S-P"));
        assert!(content.contains("bind-key -n M-1 select-window -t :1"));
        assert!(content.contains("bind-key -n M-9 select-window -t :9"));
        assert!(content.contains("bind-key -n M-Left previous-window"));
        assert!(content.contains("bind-key -n M-Right next-window"));
        assert!(content.contains("bind 1 select-window -t :1"));
        assert!(content.contains("bind 9 select-window -t :9"));
        assert!(content.contains("bind Left previous-window"));
        assert!(content.contains("bind Right next-window"));
        assert!(content.contains("bind x confirm-before -p \"Close tab #W? (y/n)\" kill-window"));
        assert!(content.contains("bind X confirm-before -p \"Close tab #W? (y/n)\" kill-window"));
        assert!(content.contains("bind r command-prompt -I \"#W\" \"rename-window '%%'\""));
        assert!(content.contains("bind R command-prompt -I \"#W\" \"rename-window '%%'\""));
        assert!(content.contains("bind h display-popup -E -w 75% -h 70% \"agymux helper\""));
        assert!(content.contains("bind H display-popup -E -w 75% -h 70% \"agymux helper\""));
        assert!(content.contains("bind ? display-popup -E -w 75% -h 70% \"agymux helper\""));
        assert!(content.contains("bind t new-window -n \"term\" \"$SHELL\""));
        assert!(content.contains("bind T new-window -n \"term\" \"$SHELL\""));
        assert!(content.contains("bind m run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\""));
        assert!(content.contains("bind M run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\""));
        // Check cyrillic bindings by default
        assert!(content.contains("bind с new-window -n \"+ new\" \"agymux run\""));
        assert!(content.contains("bind С new-window -n \"+ new\" \"agymux run\""));
        assert!(content.contains("bind и display-popup -E -w 85% -h 75% \"agymux switch\""));
        assert!(content.contains("bind И display-popup -E -w 85% -h 75% \"agymux switch\""));
        assert!(content.contains("bind з display-popup -E -w 85% -h 70% \"agymux project\""));
        assert!(content.contains("bind З display-popup -E -w 85% -h 70% \"agymux project\""));
        assert!(content.contains("bind ф display-popup -E -w 75% -h 60% \"agymux palette\""));
        assert!(content.contains("bind Ф display-popup -E -w 75% -h 60% \"agymux palette\""));
        assert!(content.contains("bind в detach-client"));
        assert!(content.contains("bind В detach-client"));
        assert!(
            content.contains("bind л confirm-before -p \"Kill session #S? (y/n)\" kill-session")
        );
        assert!(
            content.contains("bind Л confirm-before -p \"Kill session #S? (y/n)\" kill-session")
        );
        assert!(content.contains("bind ч confirm-before -p \"Close tab #W? (y/n)\" kill-window"));
        assert!(content.contains("bind Ч confirm-before -p \"Close tab #W? (y/n)\" kill-window"));
        assert!(content.contains("bind к command-prompt -I \"#W\" \"rename-window '%%'\""));
        assert!(content.contains("bind К command-prompt -I \"#W\" \"rename-window '%%'\""));
        assert!(content.contains("bind р display-popup -E -w 75% -h 70% \"agymux helper\""));
        assert!(content.contains("bind Р display-popup -E -w 75% -h 70% \"agymux helper\""));
        assert!(content.contains("bind е new-window -n \"term\" \"$SHELL\""));
        assert!(content.contains("bind Е new-window -n \"term\" \"$SHELL\""));
        assert!(content.contains("bind ь run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\""));
        assert!(content.contains("bind Ь run-shell \"tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main\""));
    }

    #[test]
    fn test_profile_config_style_overrides() {
        let mut config = Config::default();
        config.theme.status_style = Some("bg=black,fg=white".to_string());
        config.theme.window_status_current_format =
            Some("#[bold,fg=green] #I #W #[default]".to_string());
        let content = TmuxProfile::render_content(&config);

        assert!(content.contains("set -g status-style \"bg=black,fg=white\""));
        assert!(content.contains(
            "setw -g window-status-current-format \"#[bold,fg=green] #I #W #[default]\""
        ));
    }

    #[test]
    fn test_profile_default_inherits_terminal() {
        let config = Config::default();
        let content = TmuxProfile::render_content(&config);

        assert!(content.contains("set -g status-style \"bg=default,fg=default\""));
        assert!(content.contains("setw -g window-status-format \"#{?#{||:#{window_activity_flag},#{window_bell_flag}},#[fg=colour16]#[bg=colour220]#[bold] 󰂞 #I #{=22:window_name}#{?#{@remote_control}, 󰢹,}#[range=user|close] 󰅖#[norange] #[default],#{?#{@external_sync},#[fg=colour39]#[bg=colour237]#[bold] 󰛌 #I #{=22:window_name}#{?#{@remote_control}, 󰢹,}#[range=user|close] 󰅖#[norange] #[default],#[fg=colour250]#[bg=colour237] #I #{=22:window_name}#{?#{@remote_control}, #[fg=colour75]󰢹#[fg=colour250],}#[range=user|close] #[fg=colour244]󰅖#[fg=colour250]#[norange] #[default]}}\""));
        assert!(content.contains("setw -g window-status-current-format \"#[fg=colour234]#[bg=colour73]#[bold] #I #{=22:window_name}#{?#{@remote_control}, 󰢹,}#[range=user|close] 󰅖#[norange] #[default]\""));
        assert!(content.contains("set -g status-left \"#{?#{client_prefix},#[fg=black]#[bg=brightred]#[bold] 󰘳 PREFIX #[default] ,}\""));
    }

    #[test]
    fn test_profile_tab_width_truncation() {
        let mut config = Config::default();
        config.theme.tabs.standard_width = 16;
        let content = TmuxProfile::render_content(&config);
        assert!(content.contains("#{=16:window_name}"));
    }

    #[test]
    fn test_profile_custom_prefix_and_cyrillic_disabled() {
        let mut config = Config::default();
        config.keybindings.prefix = "C-a".to_string();
        config.theme.tabs.position = "bottom".to_string();
        config.keybindings.bilingual_cyrillic = false;

        let content = TmuxProfile::render_content(&config);
        assert!(content.contains("set -g status-position bottom"));
        assert!(content.contains("set -g prefix C-a"));
        assert!(content.contains("bind C-a send-prefix"));
        assert!(!content.contains("bind и display-popup"));
        assert!(!content.contains("bind с new-window"));
    }

    #[test]
    fn test_profile_bottom_position_and_spacers() {
        let mut config = Config::default();
        config.theme.tabs.position = "bottom".to_string();
        config.theme.tabs.spacer_lines = 1;

        let content = TmuxProfile::render_content(&config);
        assert!(content.contains("set -g status-position bottom"));
        assert!(content.contains("set -g status 2"));
        assert!(content.contains("set -g \"status-format[0]\" \"#[bg=default,fill= ] \""));
        assert!(content.contains(
            "set -g \"status-format[1]\" \"#[align=left range=left #{E:status-left-style}]"
        ));
    }

    #[test]
    fn test_profile_compact_zero_spacers() {
        let mut config = Config::default();
        config.theme.tabs.spacer_lines = 0;

        let content = TmuxProfile::render_content(&config);
        assert!(content.contains("set -g status on"));
        assert!(!content.contains("set -g status 2"));
        assert!(!content.contains("status-format[1]"));
    }

    #[test]
    fn test_profile_show_close_button_toggle() {
        let mut config = Config::default();
        config.theme.tabs.show_close_button = false;

        let content = TmuxProfile::render_content(&config);
        assert!(!content.contains("range=user|close"));
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
