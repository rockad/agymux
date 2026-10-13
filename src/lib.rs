pub mod cli;
pub mod config;
pub mod core;
pub mod tmux;
pub mod ui;

use anyhow::{bail, Result};
use clap::Parser;
use cli::{Cli, Commands};
use config::Config;
use std::{
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::Duration,
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
        let mut cmd = Command::new("agy");
        cmd.args(["--project", &project]);
        if remote_control {
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
                    "bind" => ui::palette::PaletteAction::BindConversation,
                    "term" | "terminal" => ui::palette::PaletteAction::NewTerminalTab,
                    "main" => ui::palette::PaletteAction::SwitchToMain,
                    "reload" | "sync" | "refresh" => ui::palette::PaletteAction::ReloadActiveSession,
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
            let mut run_args = format!("agymux run --project '{}'", project);
            if remote_control {
                run_args.push_str(" --remote-control");
            }
            if !args.is_empty() {
                run_args.push(' ');
                run_args.push_str(&args.join(" "));
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
            run_runner(&project, args, rc)?;
        }

        Some(Commands::Preview { conversation_id }) => {
            run_preview(&conversation_id)?;
        }

        Some(Commands::Ls) => {
            run_ls()?;
        }

        Some(Commands::Watch {
            window,
            project,
            start_time,
            exclude_id,
            conversation_id,
        }) => {
            run_watcher(
                &window,
                &project,
                &start_time,
                exclude_id.as_deref(),
                conversation_id.as_deref(),
            )?;
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

    let _ = heal_unbound_windows(project);

    // If explicit arguments were passed, forward them directly
    if !args.is_empty() {
        if std::env::var("TMUX").is_ok() {
            if let Some(curr_session) = tmux::TmuxDriver::current_session() {
                if curr_session == session_name {
                    return run_runner(project, args.to_vec(), remote_control);
                }
            }
            if !tmux::TmuxDriver::session_exists(session_name) {
                let mut run_cmd = format!("agymux run --project '{}'", project);
                if remote_control {
                    run_cmd.push_str(" --remote-control");
                }
                run_cmd.push(' ');
                run_cmd.push_str(&args.join(" "));
                tmux::TmuxDriver::ensure_session(
                    session_name,
                    dir,
                    config,
                    Some(&run_cmd),
                    Some("+ new"),
                )?;
            }
            return tmux::TmuxDriver::switch_client(session_name);
        }

        if !tmux::TmuxDriver::session_exists(session_name) {
            let mut run_cmd = format!("agymux run --project '{}'", project);
            if remote_control {
                run_cmd.push_str(" --remote-control");
            }
            run_cmd.push(' ');
            run_cmd.push_str(&args.join(" "));
            tmux::TmuxDriver::ensure_session(
                session_name,
                dir,
                config,
                Some(&run_cmd),
                Some("+ new"),
            )?;
        }

        if std::io::stdin().is_terminal() || std::io::stdout().is_terminal() {
            return tmux::TmuxDriver::attach_session(session_name);
        } else {
            eprintln!(
                "agymux: Session '{}' is running in the background. (attach with: tmux attach -t {})",
                session_name, session_name
            );
            return Ok(());
        }
    }

    // If session already exists, attach or switch directly
    if tmux::TmuxDriver::session_exists(session_name) {
        if std::env::var("TMUX").is_ok() {
            if let Some(curr_session) = tmux::TmuxDriver::current_session() {
                if curr_session == session_name {
                    // Already inside: run agy -c in this window
                    return run_runner(project, vec!["-c".to_string()], remote_control);
                }
            }
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
            if remote_control {
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
                format!("agymux run --project '{}' -c '{}'", project, lid)
            } else {
                format!("agymux run --project '{}'", project)
            };
            if remote_control {
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

/// Heal any window with title "+ new" or "agy" where @conversation_id is empty by checking SQLite
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
    let mut unbound_windows = Vec::new();
    let mut bound_ids = std::collections::HashSet::new();

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
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

    if unbound_windows.is_empty() {
        return Ok(());
    }

    let db = db_path();
    if !db.exists() {
        return Ok(());
    }

    if let Ok(conn) = rusqlite::Connection::open(&db) {
        let hygiene = crate::core::session::session_hygiene_conditions(&conn, "conversation_summaries");
        let hygiene_and = if hygiene.is_empty() {
            String::new()
        } else {
            format!(" AND {}", hygiene.join(" AND "))
        };
        let pattern = format!("%{}%", project);
        let sql = format!(
            "SELECT conversation_id, title FROM conversation_summaries 
             WHERE (workspace_uris LIKE ?1 OR project_id = ?2) 
               AND title IS NOT NULL AND title != ''{} 
             ORDER BY last_modified_time DESC LIMIT 10",
            hygiene_and
        );
        let mut stmt = match conn.prepare(&sql) {
            Ok(s) => s,
            Err(_) => return Ok(()),
        };

        let rows = stmt.query_map(rusqlite::params![pattern, project], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        });

        if let Ok(candidates) = rows {
            for (cid, title) in candidates.flatten() {
                if !bound_ids.contains(&cid) {
                    if let Some(win_id) = unbound_windows.pop() {
                        let _ = Command::new("tmux")
                            .args(["set-option", "-w", "-t", &win_id, "@conversation_id", &cid])
                            .status();
                        let clean = sanitize_title(&title);
                        let _ = Command::new("tmux")
                            .args(["rename-window", "-t", &win_id, &clean])
                            .status();
                        bound_ids.insert(cid);
                    }
                }
                if unbound_windows.is_empty() {
                    break;
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
        let _ = Command::new("tmux")
            .args(["set-option", "-w", "@conversation_id", &target_cid])
            .status();
        let _ = Command::new("tmux")
            .args(["rename-window", &clean])
            .status();
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

/// Internal process runner with instant prompt-based window naming and title watcher
fn run_runner(project: &str, args: Vec<String>, remote_control: bool) -> Result<()> {
    let in_tmux = std::env::var("TMUX").is_ok();
    let mut conv_id = None;
    let mut initial_prompt = None;
    let mut has_continue_flag = false;

    // Check args for conversation id and prompt
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--conversation" || arg == "-c" {
            has_continue_flag = true;
            if let Some(next) = iter.peek() {
                if !next.starts_with('-') {
                    conv_id = Some(next.to_string());
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
                .and_then(|conn| {
                    crate::core::session::latest_conversation_id(&conn, Some(project))
                })
        } else {
            None
        }
    };

    // If -c was supplied without an argument, resolve to latest_existing_id
    if has_continue_flag && conv_id.is_none() {
        conv_id = latest_existing_id.clone();
    }

    let start_time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

    // 1. Instant window naming and capture window id
    let target_window = if in_tmux {
        Command::new("tmux")
            .args(["display-message", "-p", "#{window_id}"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    } else {
        None
    };

    if in_tmux {
        if let Some(ref cid) = conv_id {
            let _ = Command::new("tmux")
                .args(["set-option", "-w", "@conversation_id", cid])
                .status();

            let db = db_path();
            if db.exists() {
                if let Ok(conn) = rusqlite::Connection::open(&db) {
                    let stmt = conn
                        .prepare("SELECT title FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1")
                        .ok();
                    if let Some(mut s) = stmt {
                        if let Ok(title) = s.query_row([cid], |r| r.get::<_, String>(0)) {
                            let _ = tmux::TmuxDriver::rename_window(&sanitize_title(&title));
                        }
                    }
                }
            }
        } else if let Some(ref prompt) = initial_prompt {
            let _ = tmux::TmuxDriver::rename_window(&sanitize_title(prompt));
        } else {
            let _ = tmux::TmuxDriver::rename_window("+ new");
        }
    }

    // 2. Background detached watcher process for title sync and external updates
    if in_tmux {
        if let Some(ref win) = target_window {
            let mut watch_cmd = Command::new("agymux");
            watch_cmd.args([
                "watch",
                "--window",
                win,
                "--project",
                project,
                "--start-time",
                &start_time,
            ]);
            if let Some(ref ex_id) = latest_existing_id {
                watch_cmd.args(["--exclude-id", ex_id]);
            }
            if let Some(ref cid) = conv_id {
                watch_cmd.args(["--conversation-id", cid]);
            }
            let _ = watch_cmd
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }

    // 3. Assemble and execute agy
    let mut cmd = Command::new("agy");
    cmd.args(["--project", project]);

    // Build args forwarding, ensuring -c <resolved_id> and --remote-control are passed appropriately
    let mut forward_args = Vec::new();
    let mut i = 0;
    let mut injected_cid = false;
    let mut has_remote_control = false;
    while i < args.len() {
        let a = &args[i];
        if a == "--remote-control" {
            has_remote_control = true;
            forward_args.push(a.clone());
            i += 1;
            continue;
        }
        if a == "-c" || a == "--conversation" {
            forward_args.push(a.clone());
            if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                forward_args.push(args[i + 1].clone());
                i += 2;
                injected_cid = true;
                continue;
            } else if let Some(ref cid) = conv_id {
                forward_args.push(cid.clone());
                injected_cid = true;
                i += 1;
                continue;
            }
        }
        forward_args.push(a.clone());
        i += 1;
    }
    if has_continue_flag && !injected_cid {
        if let Some(ref cid) = conv_id {
            forward_args.push("-c".to_string());
            forward_args.push(cid.clone());
        }
    }
    if remote_control && !has_remote_control {
        forward_args.push("--remote-control".to_string());
    }

    if !forward_args.is_empty() {
        cmd.args(&forward_args);
    }

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

/// Detached background watcher that polls SQLite for a newly created conversation, renames the window,
/// and continuously monitors for external turn / step count / mtime updates.
fn run_watcher(
    target_window: &str,
    project: &str,
    start_time: &str,
    exclude_id: Option<&str>,
    conversation_id: Option<&str>,
) -> Result<()> {
    let db = db_path();
    let proj = project.to_string();
    let pattern = format!("%{}%", proj);

    let mut active_cid: Option<String> = conversation_id.map(|s| s.to_string());

    // Phase 1: If no conversation ID was provided, poll for the newly created conversation
    if active_cid.is_none() {
        for _ in 0..300 {
            thread::sleep(Duration::from_millis(500));

            // Verify window still exists
            let window_alive = Command::new("tmux")
                .args(["display-message", "-p", "-t", target_window, "#{window_id}"])
                .output()
                .map(|o| o.status.success() && !o.stdout.is_empty())
                .unwrap_or(false);
            if !window_alive {
                return Ok(());
            }

            if !db.exists() {
                continue;
            }

            if let Ok(conn) = rusqlite::Connection::open(&db) {
                let hygiene = crate::core::session::session_hygiene_conditions(&conn, "conversation_summaries");
                let hygiene_and = if hygiene.is_empty() {
                    String::new()
                } else {
                    format!(" AND {}", hygiene.join(" AND "))
                };

                // Exclude any conversation ID already bound to a window in tmux
                let mut bound_ids = get_bound_conversation_ids(None);
                if let Some(ex) = exclude_id {
                    bound_ids.insert(ex.to_string());
                }

                let bound_clause = if bound_ids.is_empty() {
                    String::new()
                } else {
                    let placeholders: Vec<String> = (0..bound_ids.len()).map(|i| format!("?{}", i + 4)).collect();
                    format!(" AND conversation_id NOT IN ({})", placeholders.join(","))
                };

                let sql = format!("
                    SELECT conversation_id, title 
                    FROM conversation_summaries 
                    WHERE (workspace_uris LIKE ?1 OR project_id = ?2)
                      AND (last_modified_time >= ?3 OR substr(last_modified_time, 1, 19) >= substr(?3, 1, 19))
                      AND title IS NOT NULL AND title != ''{}{bound_clause}
                    ORDER BY last_modified_time DESC 
                    LIMIT 1;
                ", hygiene_and);

                if let Ok(mut stmt) = conn.prepare(&sql) {
                    let mut params: Vec<rusqlite::types::Value> = vec![
                        pattern.clone().into(),
                        proj.clone().into(),
                        start_time.to_string().into(),
                    ];
                    for id in bound_ids {
                        params.push(id.into());
                    }

                    if let Ok((cid, title)) = stmt.query_row(
                        rusqlite::params_from_iter(params.iter()),
                        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
                    ) {
                        let _ = Command::new("tmux")
                            .args([
                                "set-option",
                                "-w",
                                "-t",
                                target_window,
                                "@conversation_id",
                                &cid,
                            ])
                            .status();
                        let clean = sanitize_title(&title);
                        let _ = Command::new("tmux")
                            .args(["rename-window", "-t", target_window, &clean])
                            .status();
                        active_cid = Some(cid);
                        break;
                    }
                }
            }
        }
    }

    // Phase 2: External sync loop for active conversation (monitoring step count, mtime, and title)
    if let Some(cid) = active_cid {
        let mut last_steps: Option<i64> = None;
        let mut last_mtime: Option<i64> = None;
        let mut last_title: Option<String> = None;

        // Baseline initialization
        if db.exists() {
            if let Ok(conn) = rusqlite::Connection::open(&db) {
                let _ = conn.query_row(
                    "SELECT title, step_count, strftime('%s', last_modified_time) FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1",
                    [&cid],
                    |r| {
                        last_title = r.get::<_, Option<String>>(0).ok().flatten();
                        last_steps = r.get::<_, Option<i64>>(1).ok().flatten();
                        let epoch_str: Option<String> = r.get(2).ok();
                        if let Some(ep) = epoch_str {
                            last_mtime = ep.parse::<i64>().ok();
                        }
                        Ok(())
                    },
                );
            }
        }

        if let Some(steps) = last_steps {
            let _ = Command::new("tmux")
                .args(["set-option", "-w", "-t", target_window, "@step_count", &steps.to_string()])
                .status();
        }

        // Poll for external updates while window exists (every 1s, up to 600 iterations)
        for _ in 0..600 {
            thread::sleep(Duration::from_secs(1));

            // Verify window still exists in tmux
            let window_alive = Command::new("tmux")
                .args(["display-message", "-p", "-t", target_window, "#{window_id}"])
                .output()
                .map(|o| o.status.success() && !o.stdout.is_empty())
                .unwrap_or(false);
            if !window_alive {
                break;
            }

            if !db.exists() {
                continue;
            }

            if let Ok(conn) = rusqlite::Connection::open(&db) {
                let row_res = conn.query_row(
                    "SELECT title, step_count, strftime('%s', last_modified_time) FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1",
                    [&cid],
                    |r| {
                        let t: Option<String> = r.get(0).ok();
                        let s: Option<i64> = r.get(1).ok();
                        let ep: Option<String> = r.get(2).ok();
                        Ok((t, s, ep.and_then(|e| e.parse::<i64>().ok())))
                    },
                );

                if let Ok((curr_title, curr_steps, curr_mtime)) = row_res {
                    let mut changed = false;

                    if let Some(steps) = curr_steps {
                        if last_steps.is_none_or(|prev| steps > prev) {
                            last_steps = Some(steps);
                            changed = true;
                            let _ = Command::new("tmux")
                                .args(["set-option", "-w", "-t", target_window, "@step_count", &steps.to_string()])
                                .status();
                        }
                    }

                    if let Some(mtime) = curr_mtime {
                        if last_mtime.is_none_or(|prev| mtime > prev) {
                            last_mtime = Some(mtime);
                            changed = true;
                        }
                    }

                    // Direct file mtime check
                    let (_, file_mtime) = crate::core::session::get_conversation_mtime_and_steps(None, &cid);
                    if let Some(fmtime) = file_mtime {
                        if last_mtime.is_some_and(|prev| fmtime > prev) {
                            last_mtime = Some(fmtime);
                            changed = true;
                        }
                    }

                    if changed {
                        let _ = Command::new("tmux")
                            .args(["set-option", "-w", "-t", target_window, "@external_sync", "1"])
                            .status();
                    }

                    if let Some(ref t) = curr_title {
                        if !t.is_empty() && last_title.as_ref() != Some(t) {
                            let clean = sanitize_title(t);
                            let _ = Command::new("tmux")
                                .args(["rename-window", "-t", target_window, &clean])
                                .status();
                            last_title = Some(t.clone());
                        }
                    }
                }
            }
        }
    }

    Ok(())
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
