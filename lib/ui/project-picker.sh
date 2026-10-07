#!/usr/bin/env bash
# project-picker.sh: Global Project Switcher Modal for agymux
# Aesthetics: Monokai Pro Octagon themed project hopper

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CORE_DIR="${SCRIPT_DIR}/../core"

if [[ -f "${SCRIPT_DIR}/theme.sh" ]]; then
    # shellcheck source=/dev/null
    source "${SCRIPT_DIR}/theme.sh"
else
    ANSI_RESET=$'\033[0m'
    ANSI_BOLD=$'\033[1m'
    ANSI_MUTED=$'\033[38;2;114;112;114m'
    ANSI_ACTIVE_GREEN=$'\033[38;2;169;220;118m'
    ANSI_ACTIVE_CYAN=$'\033[38;2;120;220;232m'
    ANSI_PURPLE=$'\033[38;2;171;157;242m'
    AGYMUX_FZF_THEME=()
fi

if ! command -v fzf >/dev/null 2>&1; then
    echo "Error: fzf is required for agymux project switcher." >&2
    exit 1
fi

PROJECTS_DATA=$(mktemp)
DATA_MAP=$(mktemp)
trap 'rm -f "$PROJECTS_DATA" "$DATA_MAP"' EXIT

# Source project listing helper
if [[ -f "${CORE_DIR}/project.sh" ]]; then
    # shellcheck source=/dev/null
    source "${CORE_DIR}/project.sh"
    RAW_PROJECTS=$(list_all_projects_status)
else
    echo "Error: project.sh core helper not found." >&2
    exit 1
fi

# Build formatted menu
while IFS=$'\t' read -r pname pdir pstatus ptabs; do
    [[ -z "$pname" ]] && continue
    if [[ "$pstatus" == "active" ]]; then
        status_badge="${ANSI_ACTIVE_GREEN}● active (${ptabs} tabs)${ANSI_RESET}"
        display_line=$(printf "%-28s %-25s %s" "${pname}" "${status_badge}" "${ANSI_MUTED}${pdir}${ANSI_RESET}")
    else
        status_badge="${ANSI_MUTED}○ inactive${ANSI_RESET}"
        display_line=$(printf "%-28s %-25s %s" "${pname}" "${status_badge}" "${ANSI_MUTED}${pdir}${ANSI_RESET}")
    fi
    echo "$display_line" >> "$PROJECTS_DATA"
    printf "%s\t%s\t%s\n" "$pname" "$pdir" "$pstatus" >> "$DATA_MAP"
done <<< "$RAW_PROJECTS"

SELECTED=$(fzf "${AGYMUX_FZF_THEME[@]}" \
    --ansi \
    --prompt="🌐 Switch Project > " \
    --header="Enter: switch to project | Esc: cancel" \
    --height=70% \
    --reverse \
    --no-info < "$PROJECTS_DATA" || true)

[[ -z "$SELECTED" ]] && exit 0

LINE_NUM=$(grep -n -F "$SELECTED" "$PROJECTS_DATA" | head -1 | cut -d: -f1)
[[ -z "$LINE_NUM" ]] && exit 0

RECORD=$(sed -n "${LINE_NUM}p" "$DATA_MAP")
P_NAME=$(echo "$RECORD" | cut -f1)
P_DIR=$(echo "$RECORD" | cut -f2)
P_STATUS=$(echo "$RECORD" | cut -f3)

SESS_NAME="agy-${P_NAME}"

# Switch or create project session
if [[ -n "${TMUX:-}" ]]; then
    if tmux has-session -t "=${SESS_NAME}" 2>/dev/null; then
        tmux switch-client -t "=${SESS_NAME}"
    else
        target_dir="${P_DIR:-${HOME}/projects/${P_NAME}}"
        [[ ! -d "$target_dir" ]] && target_dir="${PWD}"
        tmux new-session -d -s "${SESS_NAME}" -c "$target_dir" -n "main" \
            "agymux-tab-runner --project '${P_NAME}' -c"
        tmux switch-client -t "=${SESS_NAME}"
    fi
else
    if tmux has-session -t "=${SESS_NAME}" 2>/dev/null; then
        exec tmux attach-session -t "=${SESS_NAME}"
    else
        target_dir="${P_DIR:-${HOME}/projects/${P_NAME}}"
        [[ ! -d "$target_dir" ]] && target_dir="${PWD}"
        exec tmux new-session -s "${SESS_NAME}" -c "$target_dir" -n "main" \
            "agymux-tab-runner --project '${P_NAME}' -c"
    fi
fi
