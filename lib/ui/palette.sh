#!/usr/bin/env bash
# palette.sh: Quick Command Palette Modal for agymux
# Aesthetics: Monokai Pro Octagon themed action menu

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
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
    ANSI_YELLOW=$'\033[38;2;255;216;102m'
    ANSI_PURPLE=$'\033[38;2;171;157;242m'
    AGYMUX_FZF_THEME=()
fi

ACTION="${1:-}"

# -----------------------------------------------------------------------------
# Action Handlers
# -----------------------------------------------------------------------------

action_rename() {
    local curr_name
    curr_name=$(tmux display-message -p '#W' 2>/dev/null || echo "")
    printf "\n  ${ANSI_BOLD}${ANSI_PURPLE}📝 Rename Active Tab${ANSI_RESET}\n"
    if [[ -n "$curr_name" ]]; then
        printf "  Current title: ${ANSI_MUTED}%s${ANSI_RESET}\n" "$curr_name"
    fi
    printf "  Enter new tab title: "
    read -r new_title
    if [[ -n "$new_title" ]]; then
        tmux rename-window "$new_title"
        printf "  ${ANSI_ACTIVE_GREEN}✓ Tab renamed to:%s %s${ANSI_RESET}\n" "$ANSI_BOLD" "$new_title"
        sleep 0.5
    fi
}

action_new() {
    local project
    project="${AGYMUX_PROJECT:-$(basename "$PWD")}"
    if [[ -n "${TMUX:-}" ]]; then
        tmux new-window -n "+ new" "agymux-tab-runner --project '${project}'"
    else
        if command -v agymux >/dev/null 2>&1; then
            agymux new
        else
            echo "Opening new tab outside tmux is not supported."
        fi
    fi
}

action_fork() {
    local conv_id curr_win project
    conv_id=$(tmux display-message -p '#{@conversation_id}' 2>/dev/null || echo "")
    curr_win=$(tmux display-message -p '#W' 2>/dev/null || echo "fork")
    project="${AGYMUX_PROJECT:-$(basename "$PWD")}"

    if [[ -z "$conv_id" ]]; then
        printf "\n  ${ANSI_BOLD}${ANSI_PURPLE}🌿 Fork Conversation${ANSI_RESET}\n"
        printf "  ${ANSI_YELLOW}No conversation ID is attached to the current tab.${ANSI_RESET}\n"
        printf "  Enter conversation ID to fork: "
        read -r conv_id
    fi

    if [[ -n "$conv_id" ]]; then
        local fork_title="[fork] ${curr_win:0:12}"
        if [[ -n "${TMUX:-}" ]]; then
            tmux new-window -n "$fork_title" "agymux-tab-runner --project '${project}' --conversation '${conv_id}'"
        else
            agy --project "$project" -c "$conv_id"
        fi
    fi
}

action_worktree() {
    printf "\n  ${ANSI_BOLD}${ANSI_PURPLE}📁 New Git Worktree Tab${ANSI_RESET}\n"
    local repo_root
    if ! repo_root=$(git rev-parse --show-toplevel 2>/dev/null); then
        printf "  ${ANSI_PREFIX_PINK}Error: Working directory is not inside a git repository.${ANSI_RESET}\n"
        sleep 1.5
        exit 1
    fi

    printf "  Repository: ${ANSI_MUTED}%s${ANSI_RESET}\n" "$repo_root"
    printf "  Enter branch name for new worktree: "
    read -r branch
    if [[ -z "$branch" ]]; then
        exit 0
    fi

    branch="${branch// /_}"
    local wt_dir="${repo_root}/.worktrees/${branch}"
    mkdir -p "${repo_root}/.worktrees"

    if git show-ref --verify --quiet "refs/heads/${branch}"; then
        printf "  ${ANSI_MUTED}Branch exists, creating worktree...${ANSI_RESET}\n"
        git worktree add "${wt_dir}" "${branch}"
    else
        printf "  ${ANSI_MUTED}Branching and creating worktree...${ANSI_RESET}\n"
        git worktree add -b "${branch}" "${wt_dir}"
    fi

    local project
    project="${AGYMUX_PROJECT:-$(basename "$repo_root")}"
    local tab_name="[${branch}] new"

    if [[ -n "${TMUX:-}" ]]; then
        tmux new-window -c "${wt_dir}" -n "$tab_name" "agymux-tab-runner --project '${project}'"
    else
        printf "  ${ANSI_ACTIVE_GREEN}✓ Worktree created at %s${ANSI_RESET}\n" "$wt_dir"
    fi
}

action_flags() {
    printf "\n  ${ANSI_BOLD}${ANSI_PURPLE}⚙️  Launch with Custom Flags${ANSI_RESET}\n"

    local models=(
        "Gemini 3.6 Flash (Medium)"
        "Gemini 3.8 Flash"
        "Gemini Pro"
        "Flash Lite"
    )

    local selected_model
    selected_model=$(printf "%s\n" "${models[@]}" | fzf "${AGYMUX_FZF_THEME[@]}" \
        --prompt="Select Model > " \
        --height=40% \
        --reverse \
        --header="Pick model tier" || echo "")

    [[ -z "$selected_model" ]] && exit 0

    local efforts=(
        "default"
        "low"
        "medium"
        "high"
        "max"
        "none"
    )

    local selected_effort
    selected_effort=$(printf "%s\n" "${efforts[@]}" | fzf "${AGYMUX_FZF_THEME[@]}" \
        --prompt="Select Thinking Effort > " \
        --height=40% \
        --reverse \
        --header="Pick thinking effort" || echo "")

    [[ -z "$selected_effort" ]] && exit 0

    local project
    project="${AGYMUX_PROJECT:-$(basename "$PWD")}"
    local short_model="${selected_model%% *}"
    local tab_name="agy [${short_model,,}]"

    local flag_args=()
    [[ "$selected_model" != "Gemini 3.6 Flash (Medium)" ]] && flag_args+=(--model "$selected_model")
    [[ "$selected_effort" != "default" ]] && flag_args+=(--effort "$selected_effort")

    if [[ -n "${TMUX:-}" ]]; then
        tmux new-window -n "$tab_name" "agymux-tab-runner --project '${project}' ${flag_args[*]:-}"
    else
        agy --project "$project" "${flag_args[@]}"
    fi
}

action_close() {
    local curr_win
    curr_win=$(tmux display-message -p '#W' 2>/dev/null || echo "active tab")
    printf "\n  ${ANSI_BOLD}${ANSI_PREFIX_PINK}Close tab '%s'? [y/N]: ${ANSI_RESET}" "$curr_win"
    read -r -n 1 confirm
    printf "\n"
    if [[ "$confirm" =~ ^[Yy]$ ]]; then
        tmux kill-window
    fi
}

action_switch() {
    if [[ -f "${SCRIPT_DIR}/picker.sh" ]]; then
        exec "${SCRIPT_DIR}/picker.sh"
    elif command -v agymux-tab-picker >/dev/null 2>&1; then
        exec agymux-tab-picker
    fi
}

action_project() {
    if command -v agymux >/dev/null 2>&1; then
        agymux project
    fi
}

action_helper() {
    if [[ -f "${SCRIPT_DIR}/helper.sh" ]]; then
        exec "${SCRIPT_DIR}/helper.sh"
    fi
}

action_detach() {
    tmux detach-client
}

# -----------------------------------------------------------------------------
# Dispatch direct subcommand or launch interactive palette
# -----------------------------------------------------------------------------

case "$ACTION" in
    rename)   action_rename; exit 0 ;;
    new)      action_new; exit 0 ;;
    fork)     action_fork; exit 0 ;;
    worktree) action_worktree; exit 0 ;;
    flags)    action_flags; exit 0 ;;
    close)    action_close; exit 0 ;;
    switch)   action_switch; exit 0 ;;
    project)  action_project; exit 0 ;;
    helper)   action_helper; exit 0 ;;
    detach)   action_detach; exit 0 ;;
    "")       ;; # Fallthrough to interactive picker
    *)        echo "Unknown palette action: $ACTION" >&2; exit 1 ;;
esac

if ! command -v fzf >/dev/null 2>&1; then
    echo "Error: fzf is required for agymux palette." >&2
    exit 1
fi

MENU_DATA=(
    "📝  Rename Tab                (Ctrl+Space r / к)	rename"
    "➕  New Tab                   (Ctrl+Space c / с)	new"
    "🌿  Fork Conversation         (Ctrl+Space f / а)	fork"
    "📁  New Worktree Tab          (Ctrl+Space w / ц)	worktree"
    "⚙️   Launch with Custom Flags  (Ctrl+Space l / д)	flags"
    "🗂️   Switch Tab / Session      (Ctrl+Space s / ы)	switch"
    "🌐  Switch Project            (Ctrl+Space P / З)	project"
    "❓  Keyboard Shortcuts        (Ctrl+Space ? / h / р)	helper"
    "🚪  Detach Session            (Ctrl+Space d / в)	detach"
    "🧹  Close Tab                 (Ctrl+Space x / ч)	close"
)

SELECTED=$(printf "%s\n" "${MENU_DATA[@]}" | fzf "${AGYMUX_FZF_THEME[@]}" \
    --delimiter=$'\t' \
    --with-nth=1 \
    --prompt="⚡ Action > " \
    --reverse \
    --no-info \
    --header="agymux Command Palette | Enter: run | Esc: cancel" \
    --height=100% || true)

[[ -z "$SELECTED" ]] && exit 0

DISPATCH_ACTION=$(echo "$SELECTED" | cut -f2)

case "$DISPATCH_ACTION" in
    rename)   action_rename ;;
    new)      action_new ;;
    fork)     action_fork ;;
    worktree) action_worktree ;;
    flags)    action_flags ;;
    switch)   action_switch ;;
    project)  action_project ;;
    helper)   action_helper ;;
    detach)   action_detach ;;
    close)    action_close ;;
esac
