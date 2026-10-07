#!/usr/bin/env bash
# session.sh - SQLite queries and metadata helpers for Antigravity conversations
# Part of agymux v0.2 core engine.

set -euo pipefail

DB_PATH="${AGYMUX_DB_PATH:-${HOME}/.gemini/antigravity-cli/conversation_summaries.db}"

# Return database path or fail if not found
get_db_path() {
    if [[ ! -f "$DB_PATH" ]]; then
        echo "Error: Conversation database not found at $DB_PATH" >&2
        return 1
    fi
    echo "$DB_PATH"
}

# Convert elapsed seconds into human-readable relative timestamp (e.g. '5m ago', '2h ago', '3d ago')
format_seconds_relative() {
    local diff="${1:-0}"
    if [[ "$diff" -lt 0 ]]; then
        echo "just now"
    elif [[ "$diff" -lt 60 ]]; then
        echo "${diff}s ago"
    elif [[ "$diff" -lt 3600 ]]; then
        echo "$(( diff / 60 ))m ago"
    elif [[ "$diff" -lt 86400 ]]; then
        echo "$(( diff / 3600 ))h ago"
    elif [[ "$diff" -lt 2592000 ]]; then
        echo "$(( diff / 86400 ))d ago"
    elif [[ "$diff" -lt 31536000 ]]; then
        echo "$(( diff / 2592000 ))mo ago"
    else
        echo "$(( diff / 31536000 ))y ago"
    fi
}

# Convert any ISO/SQLite timestamp or epoch seconds to relative string
format_relative_time() {
    local input="${1:-}"
    [[ -z "$input" ]] && echo "" && return 0

    local target_epoch
    if [[ "$input" =~ ^[0-9]+$ ]]; then
        target_epoch="$input"
    else
        target_epoch=$(date -d "$input" +%s 2>/dev/null || echo "")
        if [[ -z "$target_epoch" ]]; then
            echo "$input"
            return 0
        fi
    fi

    local now_epoch
    now_epoch=$(date +%s)
    local diff=$(( now_epoch - target_epoch ))
    format_seconds_relative "$diff"
}

# Extract friendly project name from workspace_uris and project_id
extract_project_name() {
    local uris="${1:-}"
    local pid="${2:-}"
    local re='file://([^",]+)'

    if [[ "$uris" =~ $re ]]; then
        local path="${BASH_REMATCH[1]}"
        local p_base
        p_base=$(basename "$path")
        if [[ "$p_base" == "vault" ]]; then
            local parent
            parent=$(basename "$(dirname "$path")")
            echo "$parent"
            return 0
        elif [[ -n "$p_base" && "$p_base" != "rockad" && "$p_base" != "home" ]]; then
            echo "$p_base"
            return 0
        fi
    fi

    if [[ -n "$pid" && "$pid" != "default-cli-project" ]]; then
        echo "$pid"
    else
        echo "default"
    fi
}

# List conversations for a specific project directory or project ID
# Arguments:
#   $1: Project name, directory basename, or project_id
#   $2: Limit (optional, default 50)
# Output (tab-separated):
#   conversation_id \t summary \t relative_time \t updated_at
list_project_conversations() {
    local project="${1:-}"
    local limit="${2:-50}"
    local db
    db=$(get_db_path) || return 1

    # Normalize project query
    local proj_query="${project}"
    if [[ "$proj_query" == /* ]]; then
        proj_query=$(basename "$proj_query")
    fi

    local sql="
    SELECT conversation_id,
           COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled') AS summary,
           last_modified_time,
           (strftime('%s', 'now') - strftime('%s', last_modified_time)) AS diff_sec
    FROM conversation_summaries
    WHERE (workspace_uris LIKE '%${proj_query}%' OR project_id = '${proj_query}' OR '${proj_query}' = '')
    ORDER BY last_modified_time DESC
    LIMIT ${limit};
    "

    sqlite3 -separator $'\t' "$db" "$sql" 2>/dev/null | while IFS=$'\t' read -r cid summary updated_at diff_sec; do
        [[ -z "$cid" ]] && continue
        # Clean newlines from summary
        local clean_summary="${summary//$'\n'/ }"
        clean_summary="${clean_summary//$'\r'/}"
        local rel_time
        rel_time=$(format_seconds_relative "${diff_sec:-0}")
        printf "%s\t%s\t%s\t%s\n" "$cid" "$clean_summary" "$rel_time" "$updated_at"
    done
}

# List global conversations across all projects
# Arguments:
#   $1: Limit (optional, default 50)
# Output (tab-separated):
#   project_name \t summary \t conversation_id \t relative_time \t updated_at
list_global_conversations() {
    local limit="${1:-50}"
    local db
    db=$(get_db_path) || return 1

    local sql="
    SELECT conversation_id,
           COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled') AS summary,
           last_modified_time,
           (strftime('%s', 'now') - strftime('%s', last_modified_time)) AS diff_sec,
           workspace_uris,
           project_id
    FROM conversation_summaries
    ORDER BY last_modified_time DESC
    LIMIT ${limit};
    "

    sqlite3 -separator $'\t' "$db" "$sql" 2>/dev/null | while IFS=$'\t' read -r cid summary updated_at diff_sec uris pid; do
        [[ -z "$cid" ]] && continue
        local clean_summary="${summary//$'\n'/ }"
        clean_summary="${clean_summary//$'\r'/}"
        local proj_name
        proj_name=$(extract_project_name "$uris" "$pid")
        local rel_time
        rel_time=$(format_seconds_relative "${diff_sec:-0}")
        printf "%s\t%s\t%s\t%s\t%s\n" "$proj_name" "$clean_summary" "$cid" "$rel_time" "$updated_at"
    done
}

# Get a single conversation summary by conversation_id
# Output (tab-separated):
#   conversation_id \t summary \t project_name \t relative_time \t updated_at \t step_count
get_conversation_summary() {
    local cid="${1:-}"
    [[ -z "$cid" ]] && return 1
    local db
    db=$(get_db_path) || return 1

    local sql="
    SELECT conversation_id,
           COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled') AS summary,
           last_modified_time,
           (strftime('%s', 'now') - strftime('%s', last_modified_time)) AS diff_sec,
           workspace_uris,
           project_id,
           step_count
    FROM conversation_summaries
    WHERE conversation_id = '${cid}'
    LIMIT 1;
    "

    local row
    row=$(sqlite3 -separator $'\t' "$db" "$sql" 2>/dev/null || true)
    [[ -z "$row" ]] && return 1

    IFS=$'\t' read -r r_cid summary updated_at diff_sec uris pid steps <<< "$row"
    local clean_summary="${summary//$'\n'/ }"
    clean_summary="${clean_summary//$'\r'/}"
    local proj_name
    proj_name=$(extract_project_name "$uris" "$pid")
    local rel_time
    rel_time=$(format_seconds_relative "${diff_sec:-0}")
    printf "%s\t%s\t%s\t%s\t%s\t%s\n" "$r_cid" "$clean_summary" "$proj_name" "$rel_time" "$updated_at" "$steps"
}

# CLI entry point if run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    subcmd="${1:-list-global}"
    shift || true
    case "$subcmd" in
        list-project)
            list_project_conversations "${1:-}" "${2:-50}"
            ;;
        list-global)
            list_global_conversations "${1:-50}"
            ;;
        get)
            get_conversation_summary "${1:-}"
            ;;
        relative-time)
            format_relative_time "${1:-}"
            ;;
        *)
            echo "Usage: $0 {list-project <project> [limit] | list-global [limit] | get <id> | relative-time <ts>}" >&2
            exit 1
            ;;
    esac
fi
