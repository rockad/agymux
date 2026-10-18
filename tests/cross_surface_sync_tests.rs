use agymux::cli::{Cli, Commands};
use agymux::config::Config;
use agymux::core::session::{get_conversation_mtime_and_steps, latest_conversation_id};
use agymux::ui::palette::{PaletteAction, PaletteItem, PaletteState};
use clap::Parser;
use rusqlite::Connection;

#[test]
fn test_config_remote_control_default_and_custom() {
    let default_config = Config::default();
    assert!(!default_config.general.remote_control);

    let toml_str = r#"
        [general]
        remote_control = true
    "#;
    let config: Config = toml::from_str(toml_str).expect("toml parsing succeeds");
    assert!(config.general.remote_control);
}

#[test]
fn test_cli_remote_control_flags() {
    // Global flag
    let cli = Cli::try_parse_from(["agymux", "--remote-control"]).expect("parsing succeeds");
    assert!(cli.remote_control);

    // Run command with global --remote-control
    let cli_run = Cli::try_parse_from(["agymux", "--remote-control", "run", "-c", "cid-123"])
        .expect("parsing succeeds");
    assert!(cli_run.remote_control);
    match cli_run.command {
        Some(Commands::Run { args }) => {
            assert_eq!(args, vec!["-c", "cid-123"]);
        }
        other => panic!("expected Commands::Run, got {:?}", other),
    }

    // Run command with forwarded trailing flag after --
    let cli_run_forwarded =
        Cli::try_parse_from(["agymux", "run", "--", "--remote-control", "-c", "cid-123"])
            .expect("parsing succeeds");
    match cli_run_forwarded.command {
        Some(Commands::Run { args }) => {
            assert!(args.contains(&"--remote-control".to_string()));
            assert!(args.contains(&"cid-123".to_string()));
        }
        other => panic!("expected Commands::Run, got {:?}", other),
    }

    // Attach command with forwarded --remote-control
    let cli_attach = Cli::try_parse_from(["agymux", "attach", "--", "--remote-control"])
        .expect("parsing succeeds");
    match cli_attach.command {
        Some(Commands::Attach { args }) => {
            assert!(args.contains(&"--remote-control".to_string()));
        }
        other => panic!("expected Commands::Attach, got {:?}", other),
    }
}

#[test]
fn test_palette_reload_active_session() {
    let items = PaletteItem::all();
    let reload_item = items
        .iter()
        .find(|item| item.action == PaletteAction::ReloadActiveSession);
    assert!(
        reload_item.is_some(),
        "ReloadActiveSession must be in PaletteItem::all()"
    );
    let item = reload_item.unwrap();
    assert!(item.title.contains("Reload active session"));
    assert!(item.shortcut.contains("R / К"));

    // Verify filter matching
    let mut state = PaletteState::new();
    state.query = "reload".to_string();
    state.filter();
    assert_eq!(
        state.selected_action(),
        Some(PaletteAction::ReloadActiveSession)
    );

    state.query = "sync".to_string();
    state.filter();
    assert_eq!(
        state.selected_action(),
        Some(PaletteAction::ReloadActiveSession)
    );

    state.query = "refresh".to_string();
    state.filter();
    assert_eq!(
        state.selected_action(),
        Some(PaletteAction::ReloadActiveSession)
    );

    state.query = "r".to_string();
    state.filter();
    assert_eq!(
        state.selected_action(),
        Some(PaletteAction::ReloadActiveSession)
    );
}

#[test]
fn test_latest_conversation_id_and_mtime_steps() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE conversation_summaries (
            conversation_id TEXT PRIMARY KEY,
            project_id TEXT,
            workspace_uris TEXT,
            title TEXT,
            preview TEXT,
            last_modified_time TEXT,
            parent_conversation_id TEXT,
            nesting_depth INTEGER,
            step_count INTEGER
        )",
        [],
    )
    .unwrap();

    // Insert root session
    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('cid-100', 'proj-alpha', '[\"file:///home/user/projects/proj-alpha\"]', 'First Session', '', '2026-10-13 12:00:00', '', 0, 5)",
        [],
    )
    .unwrap();

    // Insert subagent session (must be skipped by latest_conversation_id)
    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('cid-sub', 'proj-alpha', '[\"file:///home/user/projects/proj-alpha\"]', 'Subagent Session', '', '2026-10-13 12:10:00', 'cid-100', 1, 2)",
        [],
    )
    .unwrap();

    // Insert newer root session
    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('cid-200', 'proj-alpha', '[\"file:///home/user/projects/proj-alpha\"]', 'Second Session', '', '2026-10-13 12:15:00', '', 0, 12)",
        [],
    )
    .unwrap();

    let latest_project = latest_conversation_id(&conn, Some("proj-alpha"));
    assert_eq!(latest_project.as_deref(), Some("cid-200"));

    let latest_global = latest_conversation_id(&conn, None);
    assert_eq!(latest_global.as_deref(), Some("cid-200"));

    // Check step count and mtime retrieval
    let (steps, epoch) = get_conversation_mtime_and_steps(Some(&conn), "cid-200");
    assert_eq!(steps, Some(12));
    assert!(epoch.is_some());
}

#[test]
fn test_watcher_phase1_bound_id_exclusion() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE conversation_summaries (
            conversation_id TEXT PRIMARY KEY,
            project_id TEXT,
            workspace_uris TEXT,
            title TEXT,
            preview TEXT,
            last_modified_time TEXT,
            parent_conversation_id TEXT,
            nesting_depth INTEGER,
            step_count INTEGER
        )",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('cid-tab1', 'my-proj', '[\"file:///home/user/my-proj\"]', 'Existing Tab 1 Title', '', '2026-10-13 14:00:00', '', 0, 5)",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('cid-tab2', 'my-proj', '[\"file:///home/user/my-proj\"]', 'New Tab 2 Title', '', '2026-10-13 14:05:00', '', 0, 1)",
        [],
    )
    .unwrap();

    // Query mimicking Phase 1 with bound_ids containing cid-tab2:
    let bound_ids: std::collections::HashSet<String> =
        vec!["cid-tab2".to_string()].into_iter().collect();
    let placeholders: Vec<String> = (0..bound_ids.len())
        .map(|i| format!("?{}", i + 4))
        .collect();
    let bound_clause = format!(" AND conversation_id NOT IN ({})", placeholders.join(","));

    let sql = format!(
        "
        SELECT conversation_id, title 
        FROM conversation_summaries 
        WHERE (workspace_uris LIKE ?1 OR project_id = ?2)
          AND (last_modified_time >= ?3 OR substr(last_modified_time, 1, 19) >= substr(?3, 1, 19))
          AND title IS NOT NULL AND title != ''{}
        ORDER BY last_modified_time DESC 
        LIMIT 1;
    ",
        bound_clause
    );

    let mut stmt = conn.prepare(&sql).unwrap();
    let pattern = "%my-proj%".to_string();
    let proj = "my-proj".to_string();
    let start_time = "2026-10-13 13:50:00".to_string();
    let mut params: Vec<rusqlite::types::Value> =
        vec![pattern.into(), proj.into(), start_time.into()];
    for id in bound_ids {
        params.push(id.into());
    }

    let (cid, title): (String, String) = stmt
        .query_row(rusqlite::params_from_iter(params.iter()), |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();

    // Because cid-tab2 was in bound_ids, it selected cid-tab1 instead!
    assert_eq!(cid, "cid-tab1");
    assert_eq!(title, "Existing Tab 1 Title");
}

#[test]
fn test_sanitize_title_length_and_characters() {
    assert_eq!(agymux::sanitize_title("Short"), "Short");
    assert_eq!(
        agymux::sanitize_title("This is a title that is quite long and descriptive"),
        "This is a title that is quite"
    );
    assert_eq!(agymux::sanitize_title("   "), "agy");
    assert_eq!(
        agymux::sanitize_title("Hello / World : Test !"),
        "Hello  World  Test"
    );
}
