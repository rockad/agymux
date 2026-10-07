#!/usr/bin/env bash
# picker.sh: Two-Pane Interactive Tab & Conversation Picker for agymux
# Aesthetics: Monokai Pro Octagon themed dual-pane switcher (45% list / 55% preview)
# Scope: Supports Tab / Ctrl+A to toggle between Local project and Global machine scope

set -euo pipefail

SCRIPT_PATH="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/$(basename "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(dirname "${SCRIPT_PATH}")"

# Source Octagon theme tokens if available
if [[ -f "${SCRIPT_DIR}/theme.sh" ]]; then
    # shellcheck source=/dev/null
    source "${SCRIPT_DIR}/theme.sh"
else
    ANSI_RESET=$'\033[0m'
    ANSI_BOLD=$'\033[1m'
    ANSI_MUTED=$'\033[38;2;114;112;114m'
    ANSI_ACTIVE_GREEN=$'\033[38;2;169;220;118m'
    ANSI_ACTIVE_CYAN=$'\033[38;2;120;220;232m'
    ANSI_PREFIX_PINK=$'\033[38;2;255;97;136m'
    ANSI_RUNNING_ORANGE=$'\033[38;2;252;152;103m'
    ANSI_YELLOW=$'\033[38;2;255;216;102m'
    ANSI_PURPLE=$'\033[38;2;171;157;242m'
    AGYMUX_FZF_THEME=()
fi

DB_PATH="${HOME}/.gemini/antigravity-cli/conversation_summaries.db"
BRAIN_DIR="${HOME}/.gemini/antigravity-cli/brain"

# -----------------------------------------------------------------------------
# Subcommand: Render Rich Preview (Right Pane: 55%)
# -----------------------------------------------------------------------------
render_preview() {
    local p_type="${1:-}"
    local p_id="${2:-}"
    local p_title="${3:-}"
    local p_scope="${4:-}"
    local p_project="${5:-}"
    local p_conv="${6:-}"

    # If agymux-preview command exists and we have a conversation ID, prefer it
    if [[ -n "$p_conv" && "$p_conv" != "__NEW__" ]] && command -v agymux-preview >/dev/null 2>&1; then
        agymux-preview "$p_conv" 2>/dev/null && return 0
    fi

    # If lib/core/transcript.sh exists, delegate to it
    if [[ -n "$p_conv" && "$p_conv" != "__NEW__" && -f "${SCRIPT_DIR}/../core/transcript.sh" ]]; then
        "${SCRIPT_DIR}/../core/transcript.sh" preview "$p_conv" 2>/dev/null && return 0
    fi

    case "$p_type" in
        NEW)
            printf "\n  ${ANSI_BOLD}${ANSI_PURPLE}╭──────────────────────────────────────────────────────────╮${ANSI_RESET}\n"
            printf "  ${ANSI_BOLD}${ANSI_PURPLE}│ ➕ Launch New Conversation                               │${ANSI_RESET}\n"
            printf "  ${ANSI_BOLD}${ANSI_PURPLE}╰──────────────────────────────────────────────────────────╯${ANSI_RESET}\n\n"
            printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Project:" "${p_project:-current}"
            printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Working Dir:" "${PWD}"
            printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Engine:" "Google Antigravity CLI (agy)"
            printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Shortcut:" "Ctrl+Space c  /  с"
            printf "\n"
            printf "  ${ANSI_BOLD}${ANSI_MUTED}┌── BEHAVIOR ──────────────────────────────────────────────┐${ANSI_RESET}\n"
            printf "  ${ANSI_MUTED}│  Spawns a fresh conversation in a new top tab.           │${ANSI_RESET}\n"
            printf "  ${ANSI_MUTED}│  Title names immediately from your initial prompt.       │${ANSI_RESET}\n"
            printf "  ${ANSI_MUTED}│  Running turn status displays live in status bar (●).     │${ANSI_RESET}\n"
            printf "  ${ANSI_BOLD}${ANSI_MUTED}└──────────────────────────────────────────────────────────┘${ANSI_RESET}\n"
            ;;

        TAB)
            printf "\n  ${ANSI_BOLD}${ANSI_ACTIVE_GREEN}╭──────────────────────────────────────────────────────────╮${ANSI_RESET}\n"
            printf "  ${ANSI_BOLD}${ANSI_ACTIVE_GREEN}│ 🗂️  Tab #%-4s %-43s│${ANSI_RESET}\n" "$p_id" "${p_title:0:43}"
            printf "  ${ANSI_BOLD}${ANSI_ACTIVE_GREEN}╰──────────────────────────────────────────────────────────╯${ANSI_RESET}\n\n"
            printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Project:" "$p_project"
            printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Window Index:" "$p_id"
            if [[ -n "$p_conv" && "$p_conv" != "-" ]]; then
                printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Conversation:" "$p_conv"
                # Render conversation preview below
                render_conv_transcript "$p_conv" "$p_title" "$p_project"
            else
                printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Status:" "Active Tab (no bound conversation)"
            fi
            ;;

        HIST)
            local target_conv="${p_conv:-$p_id}"
            render_conv_transcript "$target_conv" "$p_title" "$p_project"
            ;;

        *)
            printf "\n  ${ANSI_MUTED}Select an item from the left pane to view details.${ANSI_RESET}\n"
            ;;
    esac
}

render_conv_transcript() {
    local conv_id="$1"
    local fallback_title="${2:-Conversation}"
    local project="${3:-}"

    local db_title=""
    local db_time=""
    local db_steps=""
    local db_workspace=""

    # 1. Fetch metadata from SQLite database (< 3 ms)
    if [[ -f "$DB_PATH" ]] && command -v sqlite3 >/dev/null 2>&1; then
        local db_row
        db_row=$(sqlite3 "$DB_PATH" "
            SELECT COALESCE(NULLIF(title, ''), '${fallback_title}'),
                   strftime('%Y-%m-%d %H:%M', last_modified_time),
                   step_count,
                   COALESCE(workspace_uris, '')
            FROM conversation_summaries
            WHERE conversation_id = '${conv_id}'
            LIMIT 1;
        " 2>/dev/null || true)

        if [[ -n "$db_row" ]]; then
            IFS='|' read -r db_title db_time db_steps db_workspace <<< "$db_row"
        fi
    fi

    db_title="${db_title:-$fallback_title}"

    # Header Box
    printf "\n  ${ANSI_BOLD}${ANSI_PURPLE}╭──────────────────────────────────────────────────────────╮${ANSI_RESET}\n"
    printf "  ${ANSI_BOLD}${ANSI_PURPLE}│ %-56s │${ANSI_RESET}\n" "${db_title:0:56}"
    printf "  ${ANSI_BOLD}${ANSI_PURPLE}╰──────────────────────────────────────────────────────────╯${ANSI_RESET}\n\n"

    printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Conversation ID:" "$conv_id"
    [[ -n "$project" ]] && printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Project:" "$project"
    [[ -n "$db_time" ]] && printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s\n" "Last Active:" "$db_time"
    [[ -n "$db_steps" ]] && printf "    ${ANSI_ACTIVE_CYAN}%-18s${ANSI_RESET} %s steps\n" "History Steps:" "$db_steps"
    printf "\n"

    # 2. Extract Transcript details with jq (< 4 ms)
    local transcript_file="${BRAIN_DIR}/${conv_id}/.system_generated/logs/transcript.jsonl"
    if [[ -f "$transcript_file" ]] && command -v jq >/dev/null 2>&1; then
        local parsed
        parsed=$(jq -s -r '
            def clean_text: gsub("<[^>]+>"; "") | gsub("^[ \t\n]+|[ \t\n]+$"; "");
            ( [ .[] | select(.type=="USER_INPUT") | .content | clean_text ] | first // "" ) as $init_prompt |
            ( .[-1] // {} ) as $last |
            ( [ ($last.tool_calls // [])[] | .name // .toolAction // "tool" ] | join(", ") ) as $tools |
            {
                prompt: ($init_prompt | .[0:260]),
                last_type: ($last.type // "DONE"),
                last_thought: (($last.thinking // "") | clean_text | .[0:220]),
                last_content: (($last.content // "") | clean_text | .[0:220]),
                tools: $tools
            }
        ' "$transcript_file" 2>/dev/null || true)

        if [[ -n "$parsed" ]]; then
            local prompt_text last_type last_content last_tools
            prompt_text=$(echo "$parsed" | jq -r '.prompt // ""')
            last_type=$(echo "$parsed" | jq -r '.last_type // ""')
            last_content=$(echo "$parsed" | jq -r '.last_content // ""')
            last_tools=$(echo "$parsed" | jq -r '.tools // ""')

            # Initial Prompt Block
            if [[ -n "$prompt_text" ]]; then
                printf "  ${ANSI_BOLD}${ANSI_MUTED}┌── INITIAL PROMPT ────────────────────────────────────────┐${ANSI_RESET}\n"
                while IFS= read -r pline; do
                    printf "  ${ANSI_FG}│ %-56s │${ANSI_RESET}\n" "${pline:0:56}"
                done <<< "$prompt_text"
                printf "  ${ANSI_BOLD}${ANSI_MUTED}└──────────────────────────────────────────────────────────┘${ANSI_RESET}\n\n"
            fi

            # Latest Activity Block
            if [[ -n "$last_content" || -n "$last_tools" ]]; then
                printf "  ${ANSI_BOLD}${ANSI_MUTED}┌── LATEST ACTIVITY [%s] ───────────────────┐${ANSI_RESET}\n" "${last_type:0:15}"
                if [[ -n "$last_tools" ]]; then
                    printf "  ${ANSI_RUNNING_ORANGE}│ 🛠️  Tools: %-47s │${ANSI_RESET}\n" "${last_tools:0:47}"
                fi
                if [[ -n "$last_content" ]]; then
                    while IFS= read -r cline; do
                        printf "  ${ANSI_FG}│ %-56s │${ANSI_RESET}\n" "${cline:0:56}"
                    done <<< "$last_content"
                fi
                printf "  ${ANSI_BOLD}${ANSI_MUTED}└──────────────────────────────────────────────────────────┘${ANSI_RESET}\n"
            fi
        fi
    fi
}

# -----------------------------------------------------------------------------
# Subcommand: List Candidates (Left Pane: 45%)
# -----------------------------------------------------------------------------
list_candidates() {
    local scope="${1:-local}"
    local project="${2:-}"
    local workdir="${3:-$PWD}"

    declare -A OPEN_TABS_BY_CONV

    # 1. New Tab Entry
    printf "➕ [New Conversation Tab]\tNEW\t__NEW__\tNew Conversation\t%s\t%s\t-\n" "$scope" "$project"

    # 2. Active tmux tabs
    if [[ -n "${TMUX:-}" ]]; then
        if [[ "$scope" == "global" ]]; then
            # Query all sessions starting with agy-
            while IFS=$'\t' read -r sess win_idx win_name conv_id win_active pane_path; do
                [[ -z "$sess" || -z "$win_idx" ]] && continue
                sess_proj="${sess#agy-}"
                marker="  "
                [[ "$win_active" == "1" ]] && marker="▶ "
                
                # Running turn indicator
                status_icon=" "
                [[ "$win_name" =~ [●] ]] && status_icon="●"

                # Git dirty indicator
                dirty_icon=" "
                if [[ -d "$pane_path" ]]; then
                    (cd "$pane_path" && git status --porcelain 2>/dev/null | grep -q . && dirty_icon="*") || true
                fi

                label=$(printf "%s[%s:%s] %s%s %-24s (open)" "$marker" "$sess_proj" "$win_idx" "$status_icon" "$dirty_icon" "${win_name:0:24}")
                printf "%s\tTAB\t%s:%s\t%s\tglobal\t%s\t%s\n" "$label" "$sess" "$win_idx" "$win_name" "$sess_proj" "${conv_id:--}"

                if [[ -n "$conv_id" ]]; then
                    OPEN_TABS_BY_CONV["$conv_id"]=1
                fi
            done < <(tmux list-windows -a -F "#{session_name}	#{window_index}	#{window_name}	#{@conversation_id}	#{window_active}	#{pane_current_path}" 2>/dev/null | grep '^agy-' || true)
        else
            # Local session tabs
            while IFS=$'\t' read -r win_idx win_name conv_id win_active pane_path; do
                [[ -z "$win_idx" ]] && continue
                marker="  "
                [[ "$win_active" == "1" ]] && marker="▶ "

                status_icon=" "
                [[ "$win_name" =~ [●] ]] && status_icon="●"

                dirty_icon=" "
                if [[ -d "$pane_path" ]]; then
                    (cd "$pane_path" && git status --porcelain 2>/dev/null | grep -q . && dirty_icon="*") || true
                fi

                label=$(printf "%s[Tab %s] %s%s %-28s (open)" "$marker" "$win_idx" "$status_icon" "$dirty_icon" "${win_name:0:28}")
                printf "%s\tTAB\t%s\t%s\tlocal\t%s\t%s\n" "$label" "$win_idx" "$win_name" "$project" "${conv_id:--}"

                if [[ -n "$conv_id" ]]; then
                    OPEN_TABS_BY_CONV["$conv_id"]=1
                fi
            done < <(tmux list-windows -F "#{window_index}	#{window_name}	#{@conversation_id}	#{window_active}	#{pane_current_path}" 2>/dev/null || true)
        fi
    fi

    # 3. Historical conversations from SQLite
    if [[ -f "$DB_PATH" ]] && command -v sqlite3 >/dev/null 2>&1; then
        local sql_filter
        if [[ "$scope" == "global" ]]; then
            sql_filter="1=1"
        else
            sql_filter="(workspace_uris LIKE '%${project}%' OR project_id = '${project}' OR '${project}' = '')"
        fi

        local query="
            SELECT conversation_id,
                   COALESCE(NULLIF(title, ''), NULLIF(substr(preview, 1, 35), ''), 'Untitled'),
                   strftime('%m-%d %H:%M', last_modified_time),
                   COALESCE(NULLIF(project_id, ''), 'main')
            FROM conversation_summaries
            WHERE ${sql_filter}
            ORDER BY last_modified_time DESC
            LIMIT 45;
        "

        while IFS='|' read -r c_id c_title c_time c_proj; do
            [[ -z "$c_id" ]] && continue
            # Avoid duplicating already-opened tab conversations
            [[ -n "${OPEN_TABS_BY_CONV[$c_id]:-}" ]] && continue

            local clean_title="${c_title//$'\n'/ }"
            clean_title="${clean_title:0:30}"

            local label
            if [[ "$scope" == "global" ]]; then
                label=$(printf "   [%-10s] %-30s (%s)" "${c_proj:0:10}" "$clean_title" "$c_time")
            else
                label=$(printf "   [Hist]       %-30s (%s)" "$clean_title" "$c_time")
            fi

            printf "%s\tHIST\t%s\t%s\t%s\t%s\t%s\n" "$label" "$c_id" "$clean_title" "$scope" "$c_proj" "$c_id"
        done < <(sqlite3 "$DB_PATH" "$query" 2>/dev/null || true)
    fi
}

# -----------------------------------------------------------------------------
# Subcommand: Scope Toggle (Tab / Ctrl+A handler)
# -----------------------------------------------------------------------------
toggle_scope() {
    local state_file="$1"
    local project="$2"
    local workdir="$3"

    local current="local"
    if [[ -f "$state_file" ]]; then
        current=$(cat "$state_file" 2>/dev/null || echo "local")
    fi

    if [[ "$current" == "local" ]]; then
        echo "global" > "$state_file"
        echo "change-prompt(⚡ [Global Search] Tab/Conv > )+reload(\"${SCRIPT_PATH}\" --list global \"${project}\" \"${workdir}\")"
    else
        echo "local" > "$state_file"
        echo "change-prompt(⚡ [Local] Switch Tab > )+reload(\"${SCRIPT_PATH}\" --list local \"${project}\" \"${workdir}\")"
    fi
}

# -----------------------------------------------------------------------------
# Dispatch subcommands
# -----------------------------------------------------------------------------
if [[ "${1:-}" == "preview" ]]; then
    shift
    render_preview "$@"
    exit 0
elif [[ "${1:-}" == "--list" ]]; then
    shift
    list_candidates "$@"
    exit 0
elif [[ "${1:-}" == "--toggle-scope" ]]; then
    shift
    toggle_scope "$@"
    exit 0
fi

# -----------------------------------------------------------------------------
# Main Interactive Picker Entry Point
# -----------------------------------------------------------------------------
PROJECT="${1:-${AGYMUX_PROJECT:-}}"
WORKDIR="${2:-${PWD}}"

if [[ -z "$PROJECT" && -n "${TMUX:-}" ]]; then
    sess_name=$(tmux display-message -p '#S' 2>/dev/null || echo "")
    if [[ "$sess_name" =~ ^agy-(.*)$ ]]; then
        PROJECT="${BASH_REMATCH[1]}"
    fi
fi
PROJECT="${PROJECT:-$(basename "$WORKDIR")}"

STATE_FILE=$(mktemp "/tmp/agymux-scope-XXXXXX")
echo "local" > "$STATE_FILE"
trap 'rm -f "$STATE_FILE"' EXIT

if ! command -v fzf >/dev/null 2>&1; then
    echo "Error: fzf is required for agymux two-pane picker." >&2
    exit 1
fi

INITIAL_CANDIDATES=$(list_candidates local "$PROJECT" "$WORKDIR")

SELECTED=$(printf "%s\n" "$INITIAL_CANDIDATES" | fzf \
    "${AGYMUX_FZF_THEME[@]}" \
    --delimiter=$'\t' \
    --with-nth=1 \
    --prompt="⚡ [Local] Switch Tab > " \
    --reverse \
    --no-info \
    --header="Enter: select | Tab / Ctrl+A: toggle scope (Local/Global) | Esc: cancel" \
    --preview="\"${SCRIPT_PATH}\" preview {2} {3} {4} {5} {6} {7}" \
    --preview-window="right,55%,border-left" \
    --bind "tab:transform(\"${SCRIPT_PATH}\" --toggle-scope \"${STATE_FILE}\" \"${PROJECT}\" \"${WORKDIR}\")" \
    --bind "ctrl-a:transform(\"${SCRIPT_PATH}\" --toggle-scope \"${STATE_FILE}\" \"${PROJECT}\" \"${WORKDIR}\")" \
    --height=100% || true)

[[ -z "$SELECTED" ]] && exit 0

IFS=$'\t' read -r _sel_disp sel_type sel_id sel_title sel_scope sel_proj sel_conv <<< "$SELECTED"

case "$sel_type" in
    NEW)
        if [[ -n "${TMUX:-}" ]]; then
            tmux new-window -c "$WORKDIR" -n "+ new" "agymux-tab-runner --project '${PROJECT}'"
        else
            command -v agymux >/dev/null 2>&1 && agymux new || agy --project "$PROJECT"
        fi
        ;;

    TAB)
        if [[ "$sel_id" =~ ^(.*):([0-9]+)$ ]]; then
            # Global tab selection across sessions
            target_sess="${BASH_REMATCH[1]}"
            target_win="${BASH_REMATCH[2]}"
            tmux switch-client -t "${target_sess}:${target_win}" 2>/dev/null || tmux select-window -t "${target_sess}:${target_win}"
        else
            tmux select-window -t ":${sel_id}"
        fi
        ;;

    HIST)
        target_project="${sel_proj:-$PROJECT}"
        if [[ "$sel_scope" == "global" && "$target_project" != "$PROJECT" && -n "${TMUX:-}" ]]; then
            target_sess="agy-${target_project}"
            if ! tmux has-session -t "$target_sess" 2>/dev/null; then
                target_dir="${HOME}/projects/${target_project}"
                [[ ! -d "$target_dir" ]] && target_dir="$WORKDIR"
                tmux new-session -d -s "$target_sess" -c "$target_dir" -n "conv" \
                    "agymux-tab-runner --project '${target_project}' --conversation '${sel_conv}'"
            else
                tmux new-window -t "$target_sess" -n "conv" \
                    "agymux-tab-runner --project '${target_project}' --conversation '${sel_conv}'"
            fi
            tmux switch-client -t "$target_sess"
        else
            if [[ -n "${TMUX:-}" ]]; then
                tmux new-window -c "$WORKDIR" -n "conv" \
                    "agymux-tab-runner --project '${PROJECT}' --conversation '${sel_conv}'"
            else
                agy --project "$PROJECT" -c "$sel_conv"
            fi
        fi
        ;;
esac
