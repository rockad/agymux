#!/usr/bin/env bash
# watcher.sh - Instant prompt-based tab naming & background SQLite title sync
# Part of agymux v0.2 daemon helpers.

set -euo pipefail

DB_PATH="${AGYMUX_DB_PATH:-${HOME}/.gemini/antigravity-cli/conversation_summaries.db}"
PROJECT=""
CONV_ID=""
PROMPT=""
TARGET_WIN=""
TIMEOUT=300
POLL_INTERVAL=3
ONESHOT=0

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --prompt|-p)
            PROMPT="$2"
            shift 2
            ;;
        --project)
            PROJECT="$2"
            shift 2
            ;;
        --conversation|-c)
            CONV_ID="$2"
            shift 2
            ;;
        --window|-w)
            TARGET_WIN="$2"
            shift 2
            ;;
        --db)
            DB_PATH="$2"
            shift 2
            ;;
        --timeout|-t)
            TIMEOUT="$2"
            shift 2
            ;;
        --interval|-i)
            POLL_INTERVAL="$2"
            shift 2
            ;;
        --oneshot)
            ONESHOT=1
            shift
            ;;
        *)
            shift
            ;;
    esac
done

# If not running in tmux, nothing to rename
if ! command -v tmux >/dev/null 2>&1 || [[ -z "${TMUX:-}" ]]; then
    exit 0
fi

# Detect target tmux window
if [[ -z "$TARGET_WIN" ]]; then
    TARGET_WIN=$(tmux display-message -p '#{window_id}' 2>/dev/null || echo "")
fi
[[ -z "$TARGET_WIN" ]] && exit 0

PROJECT="${PROJECT:-${AGYMUX_PROJECT:-$(basename "$PWD")}}"
START_TIME=$(date -u +"%Y-%m-%d %H:%M:%S")

# Sanitize title for tmux window name (max 18-20 chars)
sanitize_title() {
    local raw="$1"
    local clean
    clean=$(echo "$raw" | sed -E "s/<[^>]+>//g" | tr "\n\r\t" " ")
    clean=$(echo "$clean" | sed -E "s/[^[:alnum:] _\.\+-]//g" | sed -E "s/ +/ /g" | sed -E "s/^ //; s/ $//")
    clean="${clean:0:18}"
    echo "${clean:-agy}"
}

# 1. Instant prompt-based tab naming: If prompt is provided, rename immediately
if [[ -n "$PROMPT" ]]; then
    prompt_snippet=$(sanitize_title "$PROMPT")
    tmux rename-window -t "$TARGET_WIN" "$prompt_snippet" 2>/dev/null || true
    tmux set-option -w -t "$TARGET_WIN" @agymux_prompt "$PROMPT" 2>/dev/null || true
fi

# 2. If conversation ID is already provided, check if title already exists in SQLite
if [[ -n "$CONV_ID" ]]; then
    tmux set-option -w -t "$TARGET_WIN" @conversation_id "$CONV_ID" 2>/dev/null || true
    if [[ -f "$DB_PATH" ]] && command -v sqlite3 >/dev/null 2>&1; then
        existing_title=$(sqlite3 "$DB_PATH" "SELECT title FROM conversation_summaries WHERE conversation_id = '${CONV_ID}' LIMIT 1;" 2>/dev/null || true)
        if [[ -n "$existing_title" && "$existing_title" != "Untitled" ]]; then
            tmux rename-window -t "$TARGET_WIN" "$(sanitize_title "$existing_title")" 2>/dev/null || true
            exit 0
        fi
    fi
fi

# If oneshot was requested, exit after instant naming
if [[ $ONESHOT -eq 1 ]]; then
    exit 0
fi

# 3. Background watcher loop: Watch SQLite conversation_summaries.db for updates
if [[ ! -f "$DB_PATH" ]] || ! command -v sqlite3 >/dev/null 2>&1; then
    exit 0
fi

max_iterations=$(( TIMEOUT / POLL_INTERVAL ))
for (( i=1; i<=max_iterations; i++ )); do
    sleep "$POLL_INTERVAL"

    # Verify target window still exists
    if ! tmux list-windows -F "#{window_id}" 2>/dev/null | grep -q "^${TARGET_WIN}$"; then
        exit 0
    fi

    # Query for updated conversation title
    if [[ -n "$CONV_ID" ]]; then
        sql="
        SELECT conversation_id, title 
        FROM conversation_summaries 
        WHERE conversation_id = '${CONV_ID}'
          AND title IS NOT NULL AND title != '' AND title != 'Untitled'
        LIMIT 1;
        "
    else
        sql="
        SELECT conversation_id, title 
        FROM conversation_summaries 
        WHERE (workspace_uris LIKE '%${PROJECT}%' OR project_id = '${PROJECT}' OR '${PROJECT}' = '')
          AND last_modified_time >= '${START_TIME}'
          AND title IS NOT NULL AND title != '' AND title != 'Untitled'
        ORDER BY last_modified_time DESC 
        LIMIT 1;
        "
    fi

    found=$(sqlite3 "$DB_PATH" "$sql" 2>/dev/null || true)
    if [[ -n "$found" ]]; then
        cid=$(echo "$found" | cut -d'|' -f1)
        ctitle=$(echo "$found" | cut -d'|' -f2-)
        clean_title=$(sanitize_title "$ctitle")

        tmux set-option -w -t "$TARGET_WIN" @conversation_id "$cid" 2>/dev/null || true
        tmux rename-window -t "$TARGET_WIN" "$clean_title" 2>/dev/null || true
        exit 0
    fi
done

exit 0
