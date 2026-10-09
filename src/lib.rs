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

    // Direct execution without tmux
    if cli.direct {
        let mut cmd = Command::new("agy");
        cmd.args(["--project", &project]);
        let status = cmd.status()?;
        std::process::exit(status.code().unwrap_or(0));
    }

    match cli.command {
        None => {
            handle_attach(&project, &dir, &session_name, &config, &[])?;
        }
        Some(Commands::Attach { args }) => {
            handle_attach(&project, &dir, &session_name, &config, &args)?;
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

        Some(Commands::New { name, args }) => {
            let tab_name = name.unwrap_or_else(|| "+ new".to_string());
            let mut run_args = format!("agymux run --project '{}'", project);
            if !args.is_empty() {
                run_args.push(' ');
                run_args.push_str(&args.join(" "));
            }

            if std::env::var("TMUX").is_ok() {
                tmux::TmuxDriver::new_window(Some(&tab_name), Some(&run_args))?;
            } else {
                if !tmux::TmuxDriver::session_exists(&session_name) {
                    tmux::TmuxDriver::ensure_session(&session_name, &dir, &config)?;
                }
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
                tmux::TmuxDriver::attach_session(&session_name)?;
            }
        }

        Some(Commands::Run { args }) => {
            run_runner(&project, args)?;
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
        }) => {
            run_watcher(&window, &project, &start_time, exclude_id.as_deref())?;
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
) -> Result<()> {
    if std::env::var("TMUX").is_ok() {
        let mut cmd = Command::new("agy");
        cmd.args(["--project", project]);
        if !args.is_empty() {
            cmd.args(args);
        } else {
            cmd.arg("-c");
        }
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let err = cmd.exec();
            bail!("Failed to execute agy: {}", err);
        }
        #[cfg(not(unix))]
        {
            let status = cmd.status()?;
            std::process::exit(status.code().unwrap_or(0));
        }
    }

    if tmux::TmuxDriver::session_exists(session_name) {
        tmux::TmuxDriver::attach_session(session_name)?;
    } else {
        tmux::TmuxDriver::ensure_session(session_name, dir, config)?;
        tmux::TmuxDriver::attach_session(session_name)?;
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
        trimmed.chars().take(20).collect()
    }
}

/// Internal process runner with instant prompt-based window naming and title watcher
fn run_runner(project: &str, args: Vec<String>) -> Result<()> {
    let in_tmux = std::env::var("TMUX").is_ok();
    let mut conv_id = None;
    let mut initial_prompt = None;

    // Check args for conversation id and prompt
    let mut iter = args.iter().peekable();
    while let Some(arg) = iter.next() {
        if arg == "--conversation" || arg == "-c" {
            if let Some(next) = iter.peek() {
                if !next.starts_with('-') {
                    conv_id = Some(next.to_string());
                }
            }
        } else if !arg.starts_with('-') && initial_prompt.is_none() {
            initial_prompt = Some(arg.to_string());
        }
    }

    let start_time = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%.6f")
        .to_string();

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

    let latest_existing_id = {
        let db = db_path();
        if db.exists() {
            rusqlite::Connection::open(&db)
                .ok()
                .and_then(|conn| {
                    let pattern = format!("%{}%", project);
                    conn.query_row(
                        "SELECT conversation_id FROM conversation_summaries WHERE (workspace_uris LIKE ?1 OR project_id = ?2) ORDER BY last_modified_time DESC LIMIT 1",
                        rusqlite::params![pattern, project],
                        |r| r.get::<_, String>(0),
                    ).or_else(|_| {
                        conn.query_row(
                            "SELECT conversation_id FROM conversation_summaries ORDER BY last_modified_time DESC LIMIT 1",
                            [],
                            |r| r.get::<_, String>(0),
                        )
                    }).ok()
                })
        } else {
            None
        }
    };

    // 2. Background detached watcher process for automatic title sync on new conversations
    if in_tmux && conv_id.is_none() {
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
    if !args.is_empty() {
        cmd.args(&args);
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
        stop_watcher.store(true, Ordering::Relaxed);
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

/// Detached background watcher that polls SQLite for a newly created conversation and renames the window
fn run_watcher(
    target_window: &str,
    project: &str,
    start_time: &str,
    exclude_id: Option<&str>,
) -> Result<()> {
    let db = db_path();

    let proj = project.to_string();
    let pattern = format!("%{}%", proj);

    for _ in 0..120 {
        thread::sleep(Duration::from_millis(500));
        if !db.exists() {
            continue;
        }

        if let Ok(conn) = rusqlite::Connection::open(&db) {
            let sql = "
                SELECT conversation_id, title 
                FROM conversation_summaries 
                WHERE (workspace_uris LIKE ?1 OR project_id = ?2)
                  AND last_modified_time >= ?3
                  AND (?4 IS NULL OR conversation_id != ?4)
                  AND title IS NOT NULL AND title != ''
                ORDER BY last_modified_time DESC 
                LIMIT 1;
            ";
            if let Ok(mut stmt) = conn.prepare(sql) {
                if let Ok((cid, title)) = stmt.query_row(
                    rusqlite::params![pattern, proj, start_time, exclude_id],
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
                    break;
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
