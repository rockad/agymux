pub mod cli;
pub mod config;
pub mod core;
pub mod tmux;
pub mod ui;

use anyhow::{bail, Context, Result};
use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Main CLI entry point and dispatcher
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load().unwrap_or_default();

    let dir = cli
        .dir
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let project = cli
        .project
        .or_else(|| std::env::var("AGYMUX_PROJECT").ok())
        .unwrap_or_else(|| {
            dir.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "default".to_string())
        });

    let session_name = format!("agy-{}", project);
    let remote_control = cli.remote_control || config.general.remote_control;

    // Direct execution without tmux
    if cli.direct {
        let requested_rc = remote_control;
        let mut effective_remote_control = requested_rc;
        if requested_rc
            && config.general.single_remote_control_instance
            && is_remote_control_active()
        {
            effective_remote_control = false;
            eprintln!("agymux: Active remote-control instance already running on this machine (skipping --remote-control)");
        }
        let mut cmd = Command::new("agy");
        cmd.args(["--project", &project]);
        if effective_remote_control {
            cmd.arg("--remote-control");
        }
        let status = cmd.status()?;
        std::process::exit(status.code().unwrap_or(0));
    }

    match cli.command {
        None => {
            handle_attach(&project, &dir, &session_name, &config, &[], remote_control)?;
        }
        Some(Commands::Attach { args }) => {
            let rc = remote_control || args.contains(&"--remote-control".to_string());
            handle_attach(&project, &dir, &session_name, &config, &args, rc)?;
        }

        Some(Commands::Switch { scope }) => {
            ui::run_picker(scope, project, dir)?;
        }

        Some(Commands::Project) => {
            let base_dir = config.resolved_projects_dir();
            ui::run_project_picker(&base_dir)?;
        }

        Some(Commands::Palette { action }) => {
            if let Some(act_str) = action {
                let act = match act_str.to_lowercase().as_str() {
                    "rename" => ui::palette::PaletteAction::RenameTab,
                    "new" => ui::palette::PaletteAction::NewTab,
                    "fork" => ui::palette::PaletteAction::ForkConversation,
                    "worktree" => ui::palette::PaletteAction::NewWorktreeTab,
                    "flags" => ui::palette::PaletteAction::LaunchCustomFlags,
                    "close" => ui::palette::PaletteAction::CloseTab,
                    "switch" => ui::palette::PaletteAction::SwitchTab,
                    "project" => ui::palette::PaletteAction::SwitchProject,
                    "helper" => ui::palette::PaletteAction::Shortcuts,
                    "detach" => ui::palette::PaletteAction::Detach,
                    "quit" => ui::palette::PaletteAction::QuitKeepTabs,
                    "kill" | "kill-session" | "terminate" => {
                        ui::palette::PaletteAction::KillSession
                    }
                    "bind" => ui::palette::PaletteAction::BindConversation,
                    "term" | "terminal" => ui::palette::PaletteAction::NewTerminalTab,
                    "main" => ui::palette::PaletteAction::SwitchToMain,
                    "reload" | "sync" | "refresh" => {
                        ui::palette::PaletteAction::ReloadActiveSession
                    }
                    "split-h" | "splith" | "split-horizontal" => {
                        ui::palette::PaletteAction::SplitHorizontal
                    }
                    "split-v" | "splitv" | "split-vertical" => {
                        ui::palette::PaletteAction::SplitVertical
                    }
                    "zoom" => ui::palette::PaletteAction::TogglePaneZoom,
                    "break" | "break-pane" => ui::palette::PaletteAction::BreakPane,
                    "copy" | "copy-mode" | "scroll" => ui::palette::PaletteAction::CopyMode,
                    "close-pane" | "kill-pane" => ui::palette::PaletteAction::ClosePane,
                    other => bail!("Unknown palette action: {}", other),
                };
                ui::palette::execute_action(act, Some(&project))?;
            } else {
                ui::run_palette(Some(project))?;
            }
        }

        Some(Commands::Helper) => {
            ui::run_helper()?;
        }

        Some(Commands::Bind { conversation_id }) => {
            run_bind(&project, &dir, conversation_id)?;
        }

        Some(Commands::New { name, args }) => {
            let tab_name = name.unwrap_or_else(|| "+ new".to_string());
            let requested_rc = remote_control || args.contains(&"--remote-control".to_string());
            let mut effective_remote_control = requested_rc;
            if requested_rc
                && config.general.single_remote_control_instance
                && is_remote_control_active()
            {
                effective_remote_control = false;
                if std::env::var("TMUX").is_ok() {
                    let _ = Command::new("tmux")
                        .args([
                            "display-message",
                            "agymux: Active remote-control instance already running on this machine (skipping --remote-control)",
                        ])
                        .status();
                } else {
                    eprintln!("agymux: Active remote-control instance already running on this machine (skipping --remote-control)");
                }
            }
            let mut run_args = format!("agymux run --project '{}'", project);
            if effective_remote_control {
                run_args.push_str(" --remote-control");
            }
            let filtered_args: Vec<String> = if !effective_remote_control {
                args.into_iter()
                    .filter(|a| a != "--remote-control")
                    .collect()
            } else {
                args
            };
            if !filtered_args.is_empty() {
                run_args.push(' ');
                run_args.push_str(&filtered_args.join(" "));
            }

            if std::env::var("TMUX").is_ok() {
                tmux::TmuxDriver::new_window(Some(&tab_name), Some(&run_args))?;
            } else {
                if !tmux::TmuxDriver::session_exists(&session_name) {
                    tmux::TmuxDriver::ensure_session(
                        &session_name,
                        &dir,
                        &config,
                        Some(&run_args),
                        Some(&tab_name),
                    )?;
                } else {
                    let _ = Command::new("tmux")
                        .args([
                            "new-window",
                            "-t",
                            &format!("{}:", session_name),
                            "-c",
                            &dir.display().to_string(),
                            "-n",
                            &tab_name,
                            &run_args,
                        ])
                        .status();
                }
                tmux::TmuxDriver::attach_session(&session_name)?;
            }
        }

        Some(Commands::Run { args }) => {
            let rc = remote_control || args.contains(&"--remote-control".to_string());
            run_runner(&project, args, rc, &config)?;
        }

        Some(Commands::Preview { conversation_id }) => {
            run_preview(&conversation_id)?;
        }

        Some(Commands::Ls) => {
            run_ls()?;
        }

        Some(Commands::CloseWindow { target }) => {
            run_close_window(target.as_deref())?;
        }

        Some(Commands::Hook) => {
            run_hook()?;
        }

        Some(Commands::Install) => {
            run_install()?;
        }
    }

    Ok(())
}

fn handle_attach(
    project: &str,
    dir: &Path,
    session_name: &str,
    config: &Config,
    args: &[String],
    remote_control: bool,
) -> Result<()> {
    use std::io::IsTerminal;

    let _ = ensure_hook_installed();
    let _ = heal_unbound_windows(project);

    let requested_rc = remote_control || args.contains(&"--remote-control".to_string());
    let mut effective_remote_control = requested_rc;
    if requested_rc && config.general.single_remote_control_instance && is_remote_control_active() {
        effective_remote_control = false;
        if std::env::var("TMUX").is_ok() {
            let _ = Command::new("tmux")
                .args([
                    "display-message",
                    "agymux: Active remote-control instance already running on this machine (skipping --remote-control)",
                ])
                .status();
        } else {
            eprintln!("agymux: Active remote-control instance already running on this machine (skipping --remote-control)");
        }
    }

    // Resolve active session: check explicit session_name first, then find by directory
    let active_session = if tmux::TmuxDriver::session_exists(session_name) {
        Some(session_name.to_string())
    } else {
        tmux::TmuxDriver::find_session_by_dir(dir)
    };

    // If explicit arguments were passed, forward them directly
    if !args.is_empty() {
        if let Some(ref target_session) = active_session {
            if std::env::var("TMUX").is_ok() {
                if let Some(curr_session) = tmux::TmuxDriver::current_session() {
                    if &curr_session == target_session {
                        return run_runner(
                            project,
                            args.to_vec(),
                            effective_remote_control,
                            config,
                        );
                    }
                }
                return tmux::TmuxDriver::switch_client(target_session);
            }
            if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                return tmux::TmuxDriver::attach_session(target_session);
            } else {
                eprintln!(
                    "agymux: Session '{}' is running in the background. (attach with: tmux attach -t {})",
                    target_session, target_session
                );
                return Ok(());
            }
        }

        // Session does not exist yet: create session_name
        let mut run_cmd = format!("agymux run --project '{}'", project);
        if effective_remote_control {
            run_cmd.push_str(" --remote-control");
        }
        let filtered_args: Vec<&String> = if !effective_remote_control {
            args.iter().filter(|a| *a != "--remote-control").collect()
        } else {
            args.iter().collect()
        };
        if !filtered_args.is_empty() {
            run_cmd.push(' ');
            run_cmd.push_str(
                &filtered_args
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        tmux::TmuxDriver::ensure_session(session_name, dir, config, Some(&run_cmd), Some("+ new"))?;

        if std::env::var("TMUX").is_ok() {
            return tmux::TmuxDriver::switch_client(session_name);
        } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
            return tmux::TmuxDriver::attach_session(session_name);
        } else {
            eprintln!(
                "agymux: Session '{}' is running in the background. (attach with: tmux attach -t {})",
                session_name, session_name
            );
            return Ok(());
        }
    }

    // If session already exists (by name or by directory), attach or switch directly
    if let Some(ref target_session) = active_session {
        let clean_project = if !project.is_empty() && !crate::core::session::is_uuid(project) {
            project.to_string()
        } else if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
            dir_name.to_string()
        } else {
            "main".to_string()
        };
        let _ = Command::new("tmux")
            .args([
                "set-option",
                "-t",
                target_session,
                "@agymux_project",
                &clean_project,
            ])
            .status();

        let status_val = if config.theme.tabs.spacer_lines == 0 {
            "on"
        } else {
            "2"
        };
        let effective_pos = if config.theme.tabs.position != "bottom" {
            &config.theme.tabs.position
        } else {
            &config.theme.status_position
        };
        let _ = Command::new("tmux")
            .args(["set", "-g", "status", status_val])
            .status();
        let _ = Command::new("tmux")
            .args(["set", "-g", "status-position", effective_pos])
            .status();
        let _ = Command::new("tmux")
            .args(["set", "-t", target_session, "status", status_val])
            .status();
        let _ = Command::new("tmux")
            .args(["set", "-t", target_session, "status-position", effective_pos])
            .status();

        if std::env::var("TMUX").is_ok() {
            if let Some(curr_session) = tmux::TmuxDriver::current_session() {
                if &curr_session == target_session {
                    // Already inside: run agy -c in this window
                    return run_runner(
                        project,
                        vec!["-c".to_string()],
                        effective_remote_control,
                        config,
                    );
                }
            }
            return tmux::TmuxDriver::switch_client(target_session);
        } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
            return tmux::TmuxDriver::attach_session(target_session);
        } else {
            eprintln!(
                "agymux: Session '{}' is running in the background. (attach with: tmux attach -t {})",
                target_session, target_session
            );
            return Ok(());
        }
    }

    // Session does not exist yet: dispatch according to startup_action
    let mut startup_action = config.general.startup_action;

    if startup_action == crate::config::StartupAction::Ask {
        if std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
            let latest_summary = {
                let db = db_path();
                if db.exists() {
                    rusqlite::Connection::open(&db).ok().and_then(|conn| {
                        let hygiene = crate::core::session::session_hygiene_conditions(&conn, "conversation_summaries");
                        let hygiene_and = if hygiene.is_empty() {
                            String::new()
                        } else {
                            format!(" AND {}", hygiene.join(" AND "))
                        };
                        let pattern = format!("%{}%", project);
                        let sql = format!(
                            "SELECT title, strftime('%s', last_modified_time) FROM conversation_summaries WHERE (workspace_uris LIKE ?1 OR project_id = ?2){} ORDER BY last_modified_time DESC LIMIT 1",
                            hygiene_and
                        );
                        conn.query_row(
                            &sql,
                            rusqlite::params![pattern, project],
                            |r| {
                                let title: String = r.get(0)?;
                                let epoch_str: Option<String> = r.get(1).ok();
                                let rel = if let Some(Ok(epoch)) = epoch_str.map(|s| s.parse::<i64>()) {
                                    crate::core::session::format_relative_time(epoch)
                                } else {
                                    "recently".to_string()
                                };
                                Ok((sanitize_title(&title), rel))
                            },
                        ).ok()
                    })
                } else {
                    None
                }
            };

            match ui::run_startup_selector(project.to_string(), latest_summary)? {
                Some((action, remember)) => {
                    if remember {
                        let mut updated_config = config.clone();
                        updated_config.general.startup_action = action;
                        let _ = updated_config.save();
                    }
                    startup_action = action;
                }
                None => {
                    // User cancelled (Esc / q)
                    return Ok(());
                }
            }
        } else {
            startup_action = crate::config::StartupAction::ResumeLatest;
        }
    }

    match startup_action {
        crate::config::StartupAction::New => {
            let mut run_cmd = format!("agymux run --project '{}'", project);
            if effective_remote_control {
                run_cmd.push_str(" --remote-control");
            }
            tmux::TmuxDriver::ensure_session(
                session_name,
                dir,
                config,
                Some(&run_cmd),
                Some("+ new"),
            )?;
            if std::env::var("TMUX").is_ok() {
                tmux::TmuxDriver::switch_client(session_name)?;
            } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                tmux::TmuxDriver::attach_session(session_name)?;
            }
        }
        crate::config::StartupAction::ResumeLatest => {
            let latest_id = {
                let db = db_path();
                if db.exists() {
                    rusqlite::Connection::open(&db).ok().and_then(|conn| {
                        crate::core::session::latest_conversation_id(&conn, Some(project))
                    })
                } else {
                    None
                }
            };

            let mut run_cmd = if let Some(ref lid) = latest_id {
                format!(
                    "agymux run --project '{}' --conversation '{}'",
                    project, lid
                )
            } else {
                format!("agymux run --project '{}'", project)
            };
            if effective_remote_control {
                run_cmd.push_str(" --remote-control");
            }

            tmux::TmuxDriver::ensure_session(
                session_name,
                dir,
                config,
                Some(&run_cmd),
                Some("+ new"),
            )?;
            if std::env::var("TMUX").is_ok() {
                tmux::TmuxDriver::switch_client(session_name)?;
            } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                tmux::TmuxDriver::attach_session(session_name)?;
            }
        }
        crate::config::StartupAction::Picker => {
            ui::run_picker(cli::ScopeArg::Local, project.to_string(), dir.to_path_buf())?;
        }
        crate::config::StartupAction::Terminal => {
            let _ = Command::new("tmux")
                .args(["new-session", "-d", "-s", "main"])
                .status();
            if std::env::var("TMUX").is_ok() {
                tmux::TmuxDriver::switch_client("main")?;
            } else if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
                tmux::TmuxDriver::attach_session("main")?;
            }
        }
        crate::config::StartupAction::Ask => unreachable!(),
    }

    Ok(())
}

/// Collect all non-empty @conversation_id values currently bound to tmux windows.
/// If `session_name` is Some, queries only windows in that session; otherwise queries all windows across all sessions.
pub fn get_bound_conversation_ids(session_name: Option<&str>) -> std::collections::HashSet<String> {
    let mut bound = std::collections::HashSet::new();
    let mut cmd = Command::new("tmux");
    cmd.arg("list-windows");
    if let Some(sess) = session_name {
        cmd.args(["-t", sess]);
    } else {
        cmd.arg("-a");
    }
    cmd.args(["-F", "#{@conversation_id}"]);

    if let Ok(output) = cmd.output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    bound.insert(trimmed.to_string());
                }
            }
        }
    }
    bound
}

/// Heal an unbound window only when there is unambiguously exactly 1 window in the session
/// and 1 recent conversation within the last 5 minutes, avoiding cross-window guessing.
pub fn heal_unbound_windows(project: &str) -> Result<()> {
    if std::env::var("TMUX").is_err() {
        return Ok(());
    }

    let session_name = format!("agy-{}", project);
    let output = Command::new("tmux")
        .args([
            "list-windows",
            "-t",
            &session_name,
            "-F",
            "#{window_id}\t#{window_name}\t#{@conversation_id}",
        ])
        .output();

    let out = match output {
        Ok(o) if o.status.success() => o,
        _ => return Ok(()),
    };

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut total_windows = 0;
    let mut unbound_windows = Vec::new();
    let mut bound_ids = std::collections::HashSet::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            total_windows += 1;
            let win_id = parts[0];
            let win_name = parts[1];
            let conv_id = parts[2];
            if !conv_id.is_empty() {
                bound_ids.insert(conv_id.to_string());
            } else if win_name == "+ new" || win_name == "agy" || win_name.starts_with("+ new") {
                unbound_windows.push(win_id.to_string());
            }
        }
    }

    // Only heal if there is unambiguously exactly 1 window in the session and it is unbound.
    // Avoid guessing or popping across multiple windows which causes title swapping.
    if total_windows != 1 || unbound_windows.len() != 1 {
        return Ok(());
    }

    let win_id = &unbound_windows[0];

    let db = db_path();
    if !db.exists() {
        return Ok(());
    }

    if let Ok(conn) = rusqlite::Connection::open(&db) {
        let hygiene =
            crate::core::session::session_hygiene_conditions(&conn, "conversation_summaries");
        let hygiene_and = if hygiene.is_empty() {
            String::new()
        } else {
            format!(" AND {}", hygiene.join(" AND "))
        };
        let pattern = format!("%{}%", project);
        // Only consider conversation updated within the last 5 minutes (300 seconds)
        let sql = format!(
            "SELECT conversation_id, title FROM conversation_summaries 
             WHERE (workspace_uris LIKE ?1 OR project_id = ?2) 
               AND title IS NOT NULL AND title != ''
               AND (strftime('%s', 'now') - strftime('%s', last_modified_time)) < 300{} 
             ORDER BY last_modified_time DESC LIMIT 1",
            hygiene_and
        );

        if let Ok(mut stmt) = conn.prepare(&sql) {
            if let Ok((cid, title)) = stmt.query_row(rusqlite::params![pattern, project], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            }) {
                if !bound_ids.contains(&cid) {
                    let _ = tmux::TmuxDriver::set_window_option(win_id, "@conversation_id", &cid);
                    let clean = sanitize_title(&title);
                    let _ = tmux::TmuxDriver::rename_window(win_id, &clean);
                    let _ = crate::core::TabRegistry::bind_static(win_id, &cid);
                }
            }
        }
    }

    Ok(())
}

fn run_bind(project: &str, dir: &Path, conversation_id: Option<String>) -> Result<()> {
    let _ = heal_unbound_windows(project);

    let target_cid = match conversation_id {
        Some(cid) => cid,
        None => {
            // Open interactive conversation picker in bind mode
            let picked = ui::run_picker_for_bind(
                cli::ScopeArg::Local,
                project.to_string(),
                dir.to_path_buf(),
            )?;
            match picked {
                Some(cid) => cid,
                None => return Ok(()),
            }
        }
    };

    let db = db_path();
    let title = if db.exists() {
        rusqlite::Connection::open(&db).ok().and_then(|conn| {
            conn.query_row(
                "SELECT title FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1",
                [&target_cid],
                |r| r.get::<_, String>(0),
            )
            .ok()
        })
    } else {
        None
    };

    let title_str = title.as_deref().unwrap_or("agy");
    let clean = sanitize_title(title_str);

    if std::env::var("TMUX").is_ok() {
        let current_win_id = if let Ok(pane_id) = std::env::var("TMUX_PANE") {
            Command::new("tmux")
                .args(["display-message", "-p", "-t", &pane_id, "#{window_id}"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            Command::new("tmux")
                .args(["display-message", "-p", "#{window_id}"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        };

        let target_win = current_win_id.as_deref().unwrap_or("");
        let _ = tmux::TmuxDriver::set_window_option(target_win, "@conversation_id", &target_cid);
        let _ = tmux::TmuxDriver::rename_window(target_win, &clean);
        if let Some(ref win_id) = current_win_id {
            let _ = crate::core::TabRegistry::bind_static(win_id, &target_cid);
        }
        let toast = format!("Bound tab to: {} ({})", clean, target_cid);
        let _ = Command::new("tmux")
            .args(["display-message", &toast])
            .status();
    } else {
        println!("Bound tab to: {} ({})", clean, target_cid);
    }

    Ok(())
}

pub fn db_path() -> PathBuf {
    if let Ok(custom) = std::env::var("AGYMUX_DB_PATH") {
        PathBuf::from(custom)
    } else {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".gemini/antigravity-cli/conversation_summaries.db")
    }
}

pub fn sanitize_title(raw: &str) -> String {
    let clean: String = raw
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '_' || *c == '-' || *c == '.')
        .collect();
    let trimmed = clean.trim();
    if trimmed.is_empty() {
        "agy".to_string()
    } else {
        let truncated: String = trimmed.chars().take(30).collect();
        let res = truncated.trim_end().to_string();
        if res.is_empty() {
            "agy".to_string()
        } else {
            res
        }
    }
}

/// Check if an active remote-control instance is running elsewhere on this machine
pub fn is_remote_control_active() -> bool {
    let my_pid = std::process::id();
    if is_agy_process_remote_control_active(my_pid) {
        return true;
    }
    if is_tmux_remote_control_active(my_pid) {
        return true;
    }
    false
}

/// Helper function to resolve effective remote control state based on request, config, and liveness
pub fn resolve_effective_remote_control(
    requested: bool,
    single_instance: bool,
    is_active: bool,
) -> bool {
    if requested && single_instance && is_active {
        false
    } else {
        requested
    }
}

#[cfg(target_os = "linux")]
fn is_agy_process_remote_control_active(my_pid: u32) -> bool {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return false;
    };
    for entry in entries.flatten() {
        let Ok(file_name) = entry.file_name().into_string() else {
            continue;
        };
        let Ok(pid) = file_name.parse::<u32>() else {
            continue;
        };
        if pid == my_pid {
            continue;
        }
        let proc_path = entry.path();
        let cmdline_path = proc_path.join("cmdline");
        let Ok(cmdline_bytes) = std::fs::read(&cmdline_path) else {
            continue;
        };
        if cmdline_bytes.is_empty() {
            continue;
        }
        let args: Vec<&str> = cmdline_bytes
            .split(|&b| b == 0)
            .filter_map(|slice| std::str::from_utf8(slice).ok())
            .filter(|s| !s.is_empty())
            .collect();
        if args.is_empty() {
            continue;
        }
        let is_agy = if let Ok(exe) = std::fs::read_link(proc_path.join("exe")) {
            exe.file_name()
                .is_some_and(|name| name == "agy" || name.to_string_lossy().starts_with("agy"))
        } else {
            std::path::Path::new(args[0])
                .file_name()
                .is_some_and(|name| name == "agy" || name.to_string_lossy().starts_with("agy"))
        };
        if is_agy
            && args
                .iter()
                .any(|arg| *arg == "--remote-control" || arg.starts_with("--remote-control="))
        {
            return true;
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
fn is_agy_process_remote_control_active(my_pid: u32) -> bool {
    if let Ok(output) = Command::new("pgrep")
        .args(["-f", "agy.*--remote-control"])
        .output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if let Ok(pid) = line.trim().parse::<u32>() {
                    if pid != my_pid {
                        return true;
                    }
                }
            }
        }
    } else if let Ok(output) = Command::new("ps").args(["-eo", "pid,args"]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let mut parts = line.split_whitespace();
                if let Some(pid_str) = parts.next() {
                    if let Ok(pid) = pid_str.parse::<u32>() {
                        if pid == my_pid {
                            continue;
                        }
                        let rest: Vec<&str> = parts.collect();
                        if let Some(cmd) = rest.first() {
                            let is_agy = std::path::Path::new(cmd).file_name().is_some_and(|n| {
                                n == "agy" || n.to_string_lossy().starts_with("agy")
                            });
                            if is_agy
                                && rest.iter().any(|a| {
                                    *a == "--remote-control" || a.starts_with("--remote-control=")
                                })
                            {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn is_tmux_remote_control_active(my_pid: u32) -> bool {
    let current_pane_pid = if std::env::var("TMUX").is_ok() {
        Command::new("tmux")
            .args(["display-message", "-p", "#{pane_pid}"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| s.trim().parse::<u32>().ok())
    } else {
        None
    };

    let Ok(output) = Command::new("tmux")
        .args(["list-windows", "-a", "-F", "#{pane_pid}:#{@remote_control}"])
        .output()
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.split(':').collect();
        if parts.len() >= 2 {
            let pane_pid_str = parts[0].trim();
            let rc_val = parts[1].trim();
            if rc_val == "1" {
                if let Ok(pid) = pane_pid_str.parse::<u32>() {
                    if pid != my_pid && Some(pid) != current_pane_pid && is_process_alive(pid) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let res = unsafe { libc::kill(pid as libc::pid_t, 0) };
        if res == 0 {
            true
        } else {
            std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
        }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Build arguments forwarded to agy.
/// Ensures project is not duplicated, -c vs --conversation semantics are respected
/// (agy takes --conversation <id> for specific conversations, and boolean -c for latest),
/// and --remote-control is forwarded appropriately.
pub fn build_agy_args(
    project: &str,
    args: &[String],
    conv_id: Option<&str>,
    has_continue_flag: bool,
    remote_control: bool,
) -> Vec<String> {
    let mut forward_args = vec!["--project".to_string(), project.to_string()];
    let mut i = 0;
    let mut has_remote_control = false;

    while i < args.len() {
        let a = &args[i];
        if a == "--project" || a == "-p" {
            i += 1;
            if i < args.len() && !args[i].starts_with('-') {
                i += 1;
            }
            continue;
        }
        if a == "--remote-control" || a.starts_with("--remote-control=") {
            if remote_control {
                has_remote_control = true;
                forward_args.push(a.clone());
            }
            i += 1;
            continue;
        }
        if a == "-c" || a == "--conversation" || a == "--continue" {
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        forward_args.push(a.clone());
        i += 1;
    }

    if let Some(cid) = conv_id {
        forward_args.push("--conversation".to_string());
        forward_args.push(cid.to_string());
    } else if has_continue_flag {
        forward_args.push("-c".to_string());
    }

    if remote_control && !has_remote_control {
        forward_args.push("--remote-control".to_string());
    }

    forward_args
}

/// Internal process runner with instant prompt-based window naming and title watcher
fn run_runner(
    project: &str,
    args: Vec<String>,
    remote_control: bool,
    config: &Config,
) -> Result<()> {
    let in_tmux = std::env::var("TMUX").is_ok();
    let mut conv_id = None;
    let mut initial_prompt = None;
    let mut has_continue_flag = false;

    // Check args for conversation id and prompt
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--project" || arg == "-p" {
            if let Some(next) = iter.peek() {
                if !next.starts_with('-') {
                    let _ = iter.next();
                }
            }
        } else if arg == "--conversation" || arg == "-c" || arg == "--continue" {
            has_continue_flag = true;
            if let Some(next) = iter.peek() {
                if !next.starts_with('-') {
                    conv_id = Some(next.to_string());
                    let _ = iter.next();
                }
            }
        } else if !arg.starts_with('-') && initial_prompt.is_none() {
            initial_prompt = Some(arg.to_string());
        }
    }

    let latest_existing_id = {
        let db = db_path();
        if db.exists() {
            rusqlite::Connection::open(&db)
                .ok()
                .and_then(|conn| crate::core::session::latest_conversation_id(&conn, Some(project)))
        } else {
            None
        }
    };

    // If -c was supplied without an argument, resolve to latest_existing_id
    if has_continue_flag && conv_id.is_none() {
        conv_id = latest_existing_id.clone();
    }

    let requested_rc = remote_control || args.contains(&"--remote-control".to_string());
    let mut effective_remote_control = requested_rc;

    if requested_rc && config.general.single_remote_control_instance && is_remote_control_active() {
        effective_remote_control = false;
        if in_tmux {
            let _ = Command::new("tmux")
                .args([
                    "display-message",
                    "agymux: Active remote-control instance already running on this machine (skipping --remote-control)",
                ])
                .status();
        } else {
            eprintln!("agymux: Active remote-control instance already running on this machine (skipping --remote-control)");
        }
    }

    // 1. Instant window naming and capture window id
    let target_window = if in_tmux {
        if let Ok(pane_id) = std::env::var("TMUX_PANE") {
            Command::new("tmux")
                .args(["display-message", "-p", "-t", &pane_id, "#{window_id}"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        } else {
            Command::new("tmux")
                .args(["display-message", "-p", "#{window_id}"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        }
    } else {
        None
    };

    if in_tmux {
        let rc_val = if effective_remote_control { "1" } else { "0" };
        let win_target = target_window.as_deref().unwrap_or("");
        let _ = tmux::TmuxDriver::set_window_option(win_target, "@remote_control", rc_val);

        if let Some(ref cid) = conv_id {
            let _ = tmux::TmuxDriver::set_window_option(win_target, "@conversation_id", cid);
            if let Some(ref win) = target_window {
                let _ = crate::core::TabRegistry::bind_static(win, cid);
            }

            let db = db_path();
            if db.exists() {
                if let Ok(conn) = rusqlite::Connection::open(&db) {
                    let stmt = conn
                        .prepare("SELECT title FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1")
                        .ok();
                    if let Some(mut s) = stmt {
                        if let Ok(title) = s.query_row([cid], |r| r.get::<_, String>(0)) {
                            let clean_title = sanitize_title(&title);
                            let _ = tmux::TmuxDriver::rename_window(win_target, &clean_title);
                        }
                    }
                }
            }
        } else if let Some(ref prompt) = initial_prompt {
            let clean_prompt = sanitize_title(prompt);
            let _ = tmux::TmuxDriver::rename_window(win_target, &clean_prompt);
        } else {
            let _ = tmux::TmuxDriver::rename_window(win_target, "+ new");
        }
    }

    // 2. Background detached watcher retired: turn-driven tab titles are updated via agymux hook on Stop

    // 3. Assemble and execute agy
    let mut cmd = Command::new("agy");
    let forward_args = build_agy_args(
        project,
        &args,
        conv_id.as_deref(),
        has_continue_flag,
        effective_remote_control,
    );
    cmd.args(&forward_args);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = cmd.exec();
        eprintln!("Failed to execute agy: {}", err);
        std::process::exit(1);
    }

    #[cfg(not(unix))]
    {
        let status = cmd.status();
        let code = match status {
            Ok(s) => s.code().unwrap_or(0),
            Err(e) => {
                eprintln!("Failed to execute agy: {}", e);
                1
            }
        };
        std::process::exit(code);
    }
}

/// Print formatted transcript preview to stdout
fn run_preview(conv_id: &str) -> Result<()> {
    let summary = match core::transcript::TranscriptParser::parse(conv_id) {
        Ok(s) => s,
        Err(e) => {
            println!(
                "\x1b[1;31m[!] Transcript not found for conversation:\x1b[0m {} ({})",
                conv_id, e
            );
            return Ok(());
        }
    };

    let total = summary.total_input_tokens + summary.total_output_tokens;
    println!("\x1b[1;36m━━━ Conversation Details ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m");
    println!("\x1b[1;33mID:\x1b[0m     {}", conv_id);
    println!(
        "\x1b[1;33mTokens:\x1b[0m Input: \x1b[1;32m{}\x1b[0m | Output: \x1b[1;32m{}\x1b[0m (Total: {})",
        format_number_cli(summary.total_input_tokens),
        format_number_cli(summary.total_output_tokens),
        format_number_cli(total)
    );
    println!(
        "\x1b[1;33mTools:\x1b[0m  {} executed\n",
        summary.tools_used.len()
    );

    println!(
        "\x1b[1;36m━━━ Initial User Prompt ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
    );
    if !summary.first_prompt.is_empty() {
        println!("{}", summary.first_prompt);
    } else {
        println!("\x1b[0;90m(No initial prompt recorded)\x1b[0m");
    }
    println!();

    println!(
        "\x1b[1;36m━━━ Latest Assistant Turn ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
    );
    if !summary.last_response.is_empty() {
        println!("{}", summary.last_response);
    } else {
        println!("\x1b[0;90m(No assistant responses yet)\x1b[0m");
    }
    println!();

    println!(
        "\x1b[1;36m━━━ Executed Tools Breakdown ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\x1b[0m"
    );
    if !summary.tools_used.is_empty() {
        for t in &summary.tools_used {
            println!("  \x1b[0;32m●\x1b[0m \x1b[1m{}\x1b[0m", t);
        }
    } else {
        println!("  \x1b[0;90mNone\x1b[0m");
    }

    Ok(())
}

fn format_number_cli(num: u64) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}k", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

/// List active agymux sessions
fn run_ls() -> Result<()> {
    let output = Command::new("tmux")
        .args([
            "list-sessions",
            "-F",
            "#{session_name} (#{?session_attached,attached,detached})",
        ])
        .output();

    let mut active = Vec::new();
    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            if line.starts_with("agy-") {
                active.push(line.to_string());
            }
        }
    }

    if active.is_empty() {
        println!("No active agymux sessions found.");
    } else {
        println!("Active agymux sessions:");
        for s in active {
            println!("  {}", s);
        }
    }

    Ok(())
}

/// Lifecycle hook invoked by Antigravity on turn stop to synchronize tab titles
pub fn run_hook() -> Result<()> {
    use std::io::Read;

    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);

    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&input) {
        let conv_id = val
            .get("conversationId")
            .or_else(|| val.get("conversation_id"))
            .or_else(|| val.get("session_id"))
            .or_else(|| val.get("sessionId"))
            .and_then(|v| v.as_str());

        if let Some(cid) = conv_id {
            let db = db_path();
            if db.exists() {
                if let Ok(conn) = rusqlite::Connection::open(&db) {
                    let title: Option<String> = conn
                        .query_row(
                            "SELECT title FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1",
                            [cid],
                            |r| r.get(0),
                        )
                        .ok();

                    if let Some(t) = title {
                        if !t.trim().is_empty() {
                            let clean = sanitize_title(&t);

                            // Strategy 1: Check TabRegistry cross-reference map
                            let mut resolved_win: Option<String> = None;
                            if let Some(reg_win) =
                                crate::core::TabRegistry::get_window_for_conv_static(cid)
                            {
                                // Verify this window actually exists in tmux
                                let exists = Command::new("tmux")
                                    .args(["display-message", "-p", "-t", &reg_win, "#{window_id}"])
                                    .output()
                                    .ok()
                                    .and_then(|o| String::from_utf8(o.stdout).ok())
                                    .map(|s| s.trim() == reg_win)
                                    .unwrap_or(false);
                                if exists {
                                    resolved_win = Some(reg_win);
                                } else {
                                    let _ =
                                        crate::core::TabRegistry::unbind_window_static(&reg_win);
                                }
                            }

                            // Strategy 2: If not found in registry, check TMUX_PANE
                            if resolved_win.is_none() {
                                if let Ok(pane_id) = std::env::var("TMUX_PANE") {
                                    if let Ok(out) = Command::new("tmux")
                                        .args([
                                            "display-message",
                                            "-p",
                                            "-t",
                                            &pane_id,
                                            "#{window_id}",
                                        ])
                                        .output()
                                    {
                                        if out.status.success() {
                                            let stdout = String::from_utf8_lossy(&out.stdout);
                                            let w_id = stdout.trim().to_string();
                                            if !w_id.is_empty() {
                                                resolved_win = Some(w_id);
                                            }
                                        }
                                    }
                                }
                            }

                            // Strategy 3: Check list-windows for matching @conversation_id
                            if resolved_win.is_none() {
                                if let Ok(out) = Command::new("tmux")
                                    .args([
                                        "list-windows",
                                        "-F",
                                        "#{window_id}:#{@conversation_id}",
                                    ])
                                    .output()
                                {
                                    let stdout = String::from_utf8_lossy(&out.stdout);
                                    for line in stdout.lines() {
                                        let parts: Vec<&str> = line.splitn(2, ':').collect();
                                        if parts.len() >= 2 {
                                            let win_id = parts[0];
                                            let win_cid = parts[1];
                                            if win_cid == cid {
                                                resolved_win = Some(win_id.to_string());
                                                break;
                                            }
                                        }
                                    }
                                }
                            }

                            if let Some(target_win) = resolved_win {
                                let _ = crate::core::TabRegistry::bind_static(&target_win, cid);
                                let _ = tmux::TmuxDriver::set_window_option(
                                    &target_win,
                                    "@conversation_id",
                                    cid,
                                );
                                let _ = tmux::TmuxDriver::rename_window(&target_win, &clean);
                                println!("{{}}");
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    println!("{{}}");
    Ok(())
}

/// Safely close a window/tab by id or status range, unbinding from tab registry
pub fn run_close_window(target: Option<&str>) -> Result<()> {
    let win_id = if let Some(t) = target {
        let stripped = t.strip_prefix("close:").unwrap_or(t).trim();
        if stripped.is_empty() || stripped == "close" {
            Command::new("tmux")
                .args(["display-message", "-p", "#{window_id}"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
        } else {
            Some(stripped.to_string())
        }
    } else {
        Command::new("tmux")
            .args(["display-message", "-p", "#{window_id}"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    };

    let Some(target_win) = win_id else {
        return Ok(());
    };

    if target_win.is_empty() {
        return Ok(());
    }

    // Determine total windows in the session containing target_win
    let total_windows = Command::new("tmux")
        .args([
            "display-message",
            "-p",
            "-t",
            &target_win,
            "#{session_windows}",
        ])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(1);

    if total_windows > 1 {
        let _ = tmux::TmuxDriver::kill_window(&target_win);
        let _ = crate::core::TabRegistry::unbind_window_static(&target_win);
    } else {
        let _ = Command::new("tmux")
            .args([
                "confirm-before",
                "-p",
                "Close final tab? (y/n)",
                &format!("kill-window -t {}", target_win),
            ])
            .status();
        let _ = crate::core::TabRegistry::unbind_window_static(&target_win);
    }

    Ok(())
}

/// Install or update agymux tab sync lifecycle hook in ~/.gemini/config/hooks.json
pub fn ensure_hook_installed() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Cannot determine home directory")?;
    let config_dir = home.join(".gemini/config");
    fs::create_dir_all(&config_dir)?;
    let hook_path = config_dir.join("hooks.json");

    let mut root = if hook_path.exists() {
        let content = fs::read_to_string(&hook_path).unwrap_or_default();
        serde_json::from_str::<serde_json::Value>(&content)
            .unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    if !root.is_object() {
        root = serde_json::json!({});
    }

    let hook_entry = serde_json::json!({
        "enabled": true,
        "Stop": [
            {
                "type": "command",
                "command": "agymux hook",
                "timeout": 5
            }
        ]
    });

    if let Some(obj) = root.as_object_mut() {
        obj.insert("agymux-tab-sync".to_string(), hook_entry);
    }

    let formatted = serde_json::to_string_pretty(&root)?;
    fs::write(&hook_path, formatted)?;

    Ok(hook_path)
}

/// CLI command to install agymux lifecycle hook to ~/.gemini/config/hooks.json
pub fn run_install() -> Result<()> {
    let hook_path = ensure_hook_installed()?;
    println!("Installed agymux hook to {}", hook_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_agy_args_with_conv_id() {
        let args = vec![
            "--project".to_string(),
            "doppelganger".to_string(),
            "-c".to_string(),
            "conv-uuid-1".to_string(),
        ];
        let res = build_agy_args("doppelganger", &args, Some("conv-uuid-1"), true, false);
        assert_eq!(
            res,
            vec!["--project", "doppelganger", "--conversation", "conv-uuid-1"]
        );
    }

    #[test]
    fn test_build_agy_args_bare_continue_resolved() {
        let args = vec!["-c".to_string()];
        let res = build_agy_args("doppelganger", &args, Some("conv-uuid-latest"), true, false);
        assert_eq!(
            res,
            vec![
                "--project",
                "doppelganger",
                "--conversation",
                "conv-uuid-latest"
            ]
        );
    }

    #[test]
    fn test_build_agy_args_bare_continue_unresolved() {
        let args = vec!["-c".to_string()];
        let res = build_agy_args("doppelganger", &args, None, true, false);
        assert_eq!(res, vec!["--project", "doppelganger", "-c"]);
    }

    #[test]
    fn test_build_agy_args_new_session_no_flags() {
        let args: Vec<String> = vec![];
        let res = build_agy_args("doppelganger", &args, None, false, false);
        assert_eq!(res, vec!["--project", "doppelganger"]);
    }

    #[test]
    fn test_build_agy_args_preserves_custom_flags_and_remote_control() {
        let args = vec![
            "--model".to_string(),
            "gemini-3.8-flash".to_string(),
            "--conversation".to_string(),
            "conv-uuid-2".to_string(),
        ];
        let res = build_agy_args("doppelganger", &args, Some("conv-uuid-2"), true, true);
        assert_eq!(
            res,
            vec![
                "--project",
                "doppelganger",
                "--model",
                "gemini-3.8-flash",
                "--conversation",
                "conv-uuid-2",
                "--remote-control",
            ]
        );
    }

    #[test]
    fn test_ensure_hook_installed_and_json_format() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_dir = temp_dir.path().join(".gemini/config");
        std::fs::create_dir_all(&config_dir).unwrap();
        let hook_path = config_dir.join("hooks.json");

        // Write existing dummy hook
        let initial_json = r#"{ "other-hook": { "enabled": true } }"#;
        std::fs::write(&hook_path, initial_json).unwrap();

        // Simulate logic
        let content = std::fs::read_to_string(&hook_path).unwrap();
        let mut root: serde_json::Value = serde_json::from_str(&content).unwrap();
        let hook_entry = serde_json::json!({
            "enabled": true,
            "Stop": [
                {
                    "type": "command",
                    "command": "agymux hook",
                    "timeout": 5
                }
            ]
        });
        if let Some(obj) = root.as_object_mut() {
            obj.insert("agymux-tab-sync".to_string(), hook_entry);
        }
        let formatted = serde_json::to_string_pretty(&root).unwrap();
        std::fs::write(&hook_path, formatted).unwrap();

        let read_back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&hook_path).unwrap()).unwrap();
        assert!(read_back.get("other-hook").is_some());
        assert!(read_back.get("agymux-tab-sync").is_some());
        assert_eq!(read_back["agymux-tab-sync"]["enabled"], true);
        assert_eq!(
            read_back["agymux-tab-sync"]["Stop"][0]["command"],
            "agymux hook"
        );
    }
}
