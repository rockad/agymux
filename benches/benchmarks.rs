use agymux::cli::Cli;
use agymux::core::session::SessionDb;
use agymux::core::transcript::TranscriptParser;
use clap::Parser;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rusqlite::Connection;
use std::io::Cursor;

fn bench_cli_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("cli_dispatch");
    group.bench_function("parse_default_args", |b| {
        b.iter(|| {
            let args = ["agymux", "attach", "--", "-c"];
            Cli::try_parse_from(black_box(args)).unwrap()
        });
    });

    group.bench_function("parse_switch_args", |b| {
        b.iter(|| {
            let args = ["agymux", "switch", "-s", "global"];
            Cli::try_parse_from(black_box(args)).unwrap()
        });
    });
    group.finish();
}

fn bench_transcript_parser_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("transcript_parser");

    // Generate simulated 1,000 turn NDJSON
    let mut data_1000 = String::with_capacity(1024 * 500);
    for i in 0..1000 {
        if i % 2 == 0 {
            data_1000.push_str(&format!(
                r#"{{"step_index":{},"source":"USER_EXPLICIT","type":"USER_INPUT","content":"<USER_REQUEST>Turn {} prompt</USER_REQUEST>"}}"#,
                i, i
            ));
        } else {
            data_1000.push_str(&format!(
                r#"{{"step_index":{},"source":"MODEL","type":"PLANNER_RESPONSE","input_tokens":150,"output_tokens":40,"content":"Turn {} response","tool_calls":[{{"name":"view_file"}},{{"name":"run_command"}}]}}"#,
                i, i
            ));
        }
        data_1000.push('\n');
    }

    group.throughput(criterion::Throughput::Bytes(data_1000.len() as u64));
    group.bench_function("parse_1000_turns_stream", |b| {
        b.iter(|| {
            let cursor = Cursor::new(black_box(&data_1000));
            TranscriptParser::parse_reader(cursor).unwrap()
        });
    });
    group.finish();
}

fn bench_sqlite_query_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("sqlite_queries");

    // Seed in-memory database with 1,000 conversation records
    let conn = Connection::open_in_memory().unwrap();
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

    {
        let mut stmt = conn
            .prepare(
                "INSERT INTO conversation_summaries (conversation_id, project_id, workspace_uris, title, preview, last_modified_time)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .unwrap();

        for i in 0..1000 {
            let cid = format!("conv-uuid-{}", i);
            let pid = if i % 2 == 0 { "agymux" } else { "doppelganger" };
            let uris = format!(r#"["file:///home/rockad/projects/{}"]"#, pid);
            let title = format!("Task {} summary details", i);
            let time = format!("2026-10-08 12:{:02}:{:02}", (i / 60) % 60, i % 60);
            stmt.execute(rusqlite::params![cid, pid, uris, title, "preview", time])
                .unwrap();
        }
    }

    let db = SessionDb::from_connection(conn);

    group.bench_function("query_global_1000_records", |b| {
        b.iter(|| db.get_global_conversations().unwrap());
    });

    group.bench_function("query_local_project_records", |b| {
        let p = std::path::Path::new("/home/rockad/projects/agymux");
        b.iter(|| db.get_local_conversations(black_box(p)).unwrap());
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_cli_parsing,
    bench_transcript_parser_throughput,
    bench_sqlite_query_latency
);
criterion_main!(benches);
