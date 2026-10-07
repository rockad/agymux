#!/usr/bin/env bash
# project.sh - Git repository discovery and active tmux agymux session tracker
# Part of agymux v0.2 core engine.

set -euo pipefail

PROJECTS_DIR="${AGYMUX_PROJECTS_DIR:-${HOME}/projects}"

# Discover git repositories under a base directory (default: ~/projects/)
# Output (tab-separated):
#   repo_name \t repo_path
discover_git_projects() {
    local base="${1:-$PROJECTS_DIR}"
    [[ ! -d "$base" ]] && return 0

    if command -v fd >/dev/null 2>&1; then
        fd -t d -t f -d 2 --hidden --no-ignore '^\.git$' "$base" 2>/dev/null | while read -r git_path; do
            local repo_dir
            repo_dir="$(dirname "$git_path")"
            local repo_name
            repo_name="$(basename "$repo_dir")"
            printf "%s\t%s\n" "$repo_name" "$repo_dir"
        done | sort -u
    else
        for dir in "$base"/*; do
            if [[ -d "$dir/.git" || -f "$dir/.git" ]]; then
                printf "%s\t%s\n" "$(basename "$dir")" "$dir"
            fi
        done | sort -u
    fi
}

# Query tmux for active agy-* sessions
# Output (tab-separated):
#   session_name \t project_name \t window_count \t attached_clients
list_active_agy_sessions() {
    if ! command -v tmux >/dev/null 2>&1; then
        return 0
    fi

    tmux list-sessions -F "#{session_name}	#{session_windows}	#{session_attached}" 2>/dev/null | while IFS=$'\t' read -r sname nwins natt; do
        if [[ "$sname" =~ ^agy-(.+)$ ]]; then
            local pname="${BASH_REMATCH[1]}"
            printf "%s\t%s\t%s\t%s\n" "$sname" "$pname" "$nwins" "$natt"
        fi
    done || true
}

# Get active status for a specific project
# Output:
#   "active:<tabs_count>" or "inactive:0"
get_project_session_status() {
    local target_project="${1:-}"
    [[ -z "$target_project" ]] && { echo "inactive:0"; return 0; }

    local session_name="agy-${target_project}"
    if ! command -v tmux >/dev/null 2>&1; then
        echo "inactive:0"
        return 0
    fi

    local wins
    wins=$(tmux list-windows -t "$session_name" -F "#{window_id}" 2>/dev/null | wc -l || echo "0")
    if [[ "$wins" -gt 0 ]]; then
        echo "active:${wins}"
    else
        echo "inactive:0"
    fi
}

# List all projects with combined tmux activity status
# Output (tab-separated):
#   project_name \t repo_path \t status (active/inactive) \t tabs_count
list_all_projects_status() {
    local base="${1:-$PROJECTS_DIR}"

    declare -A ACTIVE_WINS
    declare -A ACTIVE_ATTS

    if command -v tmux >/dev/null 2>&1; then
        while IFS=$'\t' read -r sname pname nwins natt; do
            [[ -z "$pname" ]] && continue
            ACTIVE_WINS["$pname"]="$nwins"
            ACTIVE_ATTS["$pname"]="$natt"
        done < <(list_active_agy_sessions)
    fi

    # 1. Output discovered projects from filesystem
    while IFS=$'\t' read -r repo_name repo_path; do
        [[ -z "$repo_name" ]] && continue
        if [[ -n "${ACTIVE_WINS[$repo_name]:-}" ]]; then
            printf "%s\t%s\tactive\t%s\n" "$repo_name" "$repo_path" "${ACTIVE_WINS[$repo_name]}"
            unset "ACTIVE_WINS[$repo_name]"
        else
            printf "%s\t%s\tinactive\t0\n" "$repo_name" "$repo_path"
        fi
    done < <(discover_git_projects "$base")

    # 2. Output any active sessions not discovered under base directory
    for pname in "${!ACTIVE_WINS[@]}"; do
        printf "%s\t\tactive\t%s\n" "$pname" "${ACTIVE_WINS[$pname]}"
    done
}

# CLI entry point if run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    subcmd="${1:-list}"
    case "$subcmd" in
        discover)
            discover_git_projects "${2:-$PROJECTS_DIR}"
            ;;
        active)
            list_active_agy_sessions
            ;;
        status)
            get_project_session_status "${2:-}"
            ;;
        list|"")
            list_all_projects_status "${2:-$PROJECTS_DIR}"
            ;;
        *)
            echo "Usage: $0 {discover [dir] | active | status <project> | list [dir]}" >&2
            exit 1
            ;;
    esac
fi
