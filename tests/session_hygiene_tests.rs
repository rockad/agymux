use ax::core::session::SessionDb;
use rusqlite::Connection;
use std::path::Path;

#[test]
fn test_integration_subagent_exclusion_in_global_conversations() {
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
         VALUES ('root-alpha', 'agymux', '[\"file:///home/user/projects/agymux\"]', 'Root Main Session', 'Working on hygiene', '2026-10-13 12:00:00', '', 0, 15)",
        [],
    )
    .unwrap();

    // Insert subagent session with parent_conversation_id
    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('subagent-child-1', 'agymux', '[\"file:///home/user/projects/agymux\"]', 'Code Engineer Subagent', 'Researching files', '2026-10-13 12:05:00', 'root-alpha', 1, 8)",
        [],
    )
    .unwrap();

    // Insert nested subagent session with nesting_depth = 2
    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('subagent-grandchild', 'agymux', '[\"file:///home/user/projects/agymux\"]', 'Deep Nested Subagent', 'Sub-task execution', '2026-10-13 12:06:00', 'subagent-child-1', 2, 4)",
        [],
    )
    .unwrap();

    // Insert abandoned 0-step session (no title, no preview, step_count = 0)
    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('abandoned-ghost', 'agymux', '[\"file:///home/user/projects/agymux\"]', '', '', '2026-10-13 12:07:00', '', 0, 0)",
        [],
    )
    .unwrap();

    // Insert active 0-step session with title (e.g. freshly launched turn)
    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('active-new-turn', 'agymux', '[\"file:///home/user/projects/agymux\"]', 'Implement Phase 1', '', '2026-10-13 12:08:00', '', 0, 0)",
        [],
    )
    .unwrap();

    let db = SessionDb::from_connection(conn);
    let global = db.get_global_conversations().unwrap();
    let ids: Vec<String> = global.into_iter().map(|item| item.id).collect();

    // Subagents must NOT be returned
    assert!(
        !ids.contains(&"subagent-child-1".to_string()),
        "Subagent with parent_conversation_id must be excluded"
    );
    assert!(
        !ids.contains(&"subagent-grandchild".to_string()),
        "Nested subagent must be excluded"
    );
    assert!(
        !ids.contains(&"abandoned-ghost".to_string()),
        "Abandoned 0-step session must be excluded"
    );

    // Root sessions and active new turns must be included
    assert!(
        ids.contains(&"root-alpha".to_string()),
        "Root session must be returned"
    );
    assert!(
        ids.contains(&"active-new-turn".to_string()),
        "Active session with prompt title must be returned"
    );
    assert_eq!(ids.len(), 2);
}

#[test]
fn test_integration_subagent_exclusion_in_local_conversations() {
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

    // Insert root for project A
    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('root-proj-a', 'proj-a', '[\"file:///home/user/projects/proj-a\"]', 'Proj A Main', 'summary', '2026-10-13 12:00:00', '', 0, 10)",
        [],
    )
    .unwrap();

    // Insert subagent for project A
    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time, parent_conversation_id, nesting_depth, step_count)
         VALUES ('sub-proj-a', 'proj-a', '[\"file:///home/user/projects/proj-a\"]', 'Proj A Subagent', 'summary', '2026-10-13 12:01:00', 'root-proj-a', 1, 5)",
        [],
    )
    .unwrap();

    let db = SessionDb::from_connection(conn);
    let local = db
        .get_local_conversations(Path::new("/home/user/projects/proj-a"))
        .unwrap();
    let local_ids: Vec<String> = local.into_iter().map(|item| item.id).collect();

    assert_eq!(local_ids, vec!["root-proj-a"]);
    assert!(!local_ids.contains(&"sub-proj-a".to_string()));
}

#[test]
fn test_integration_graceful_missing_columns() {
    let conn = Connection::open_in_memory().unwrap();
    // Older schema without parent_conversation_id, nesting_depth, step_count
    conn.execute(
        "CREATE TABLE conversation_summaries (
            conversation_id TEXT PRIMARY KEY,
            project_id TEXT,
            workspace_uris TEXT,
            title TEXT,
            preview TEXT,
            last_modified_time TEXT
        )",
        [],
    )
    .unwrap();

    conn.execute(
        "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time)
         VALUES ('legacy-conv-1', 'agymux', '[\"file:///home/user/projects/agymux\"]', 'Legacy conversation', 'preview', '2026-10-01 10:00:00')",
        [],
    )
    .unwrap();

    let db = SessionDb::from_connection(conn);
    let global = db.get_global_conversations().unwrap();
    assert_eq!(global.len(), 1);
    assert_eq!(global[0].id, "legacy-conv-1");

    let local = db
        .get_local_conversations(Path::new("/home/user/projects/agymux"))
        .unwrap();
    assert_eq!(local.len(), 1);
    assert_eq!(local[0].id, "legacy-conv-1");
}
