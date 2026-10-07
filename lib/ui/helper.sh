#!/usr/bin/env bash
# helper.sh: Interactive Keyboard Shortcuts & Cheatsheet Modal for agymux
# Aesthetics: Monokai Pro Octagon themed cheatsheet

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
fi

clear 2>/dev/null || true

printf "\n"
printf "  ${ANSI_BOLD}${ANSI_PURPLE}╭──────────────────────────────────────────────────────────────────────────╮${ANSI_RESET}\n"
printf "  ${ANSI_BOLD}${ANSI_PURPLE}│                  agymux v0.2 — Keyboard Shortcuts & Commands             │${ANSI_RESET}\n"
printf "  ${ANSI_BOLD}${ANSI_PURPLE}╰──────────────────────────────────────────────────────────────────────────╯${ANSI_RESET}\n\n"

printf "  ${ANSI_BOLD}${ANSI_ACTIVE_CYAN}▌ TAB NAVIGATION & MOVEMENT${ANSI_RESET}\n"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Alt + 1..9" "Jump directly to tab 1..9 (layout-independent)"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Alt + Left / Right" "Cycle to previous / next tab"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space s  /  ы" "Two-pane tab & conversation switcher"
printf "\n"

printf "  ${ANSI_BOLD}${ANSI_ACTIVE_CYAN}▌ CONVERSATION & WORKTREE LIFECYCLE${ANSI_RESET}\n"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space c  /  с" "New conversation tab in current workspace"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space w  /  ц" "New git worktree tab (.worktrees/<branch>)"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space r  /  к" "Rename active tab"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space f  /  а" "Fork current conversation into new tab"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space l  /  д" "Launch new tab with custom model / effort flags"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space x  /  ч" "Close active tab (with confirmation)"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space d  /  в" "Detach tmux session (leaves agents running)"
printf "\n"

printf "  ${ANSI_BOLD}${ANSI_ACTIVE_CYAN}▌ MODALS & GLOBAL CONTROLS${ANSI_RESET}\n"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space p  /  з" "Quick Command Palette (all actions menu)"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space P  /  З" "Global Project Switcher"
printf "    ${ANSI_ACTIVE_GREEN}%-26s${ANSI_RESET} %s\n" "Ctrl+Space ?  /  h  /  р" "Display this shortcuts cheatsheet"
printf "\n"

printf "  ${ANSI_BOLD}${ANSI_ACTIVE_CYAN}▌ INSIDE TWO-PANE PICKER${ANSI_RESET}\n"
printf "    ${ANSI_YELLOW}%-26s${ANSI_RESET} %s\n" "Tab  or  Ctrl+A" "Toggle search scope (Local Project ⇄ Global Machine)"
printf "    ${ANSI_YELLOW}%-26s${ANSI_RESET} %s\n" "Enter" "Select tab / resume historical conversation"
printf "    ${ANSI_YELLOW}%-26s${ANSI_RESET} %s\n" "Esc" "Cancel / close modal"
printf "\n"

printf "  ${ANSI_MUTED}──────────────────────────────────────────────────────────────────────────${ANSI_RESET}\n"
printf "  ${ANSI_MUTED}All letter shortcuts feature paired English & Cyrillic (RussianWin) keys.${ANSI_RESET}\n"
printf "  ${ANSI_BOLD}${ANSI_PREFIX_PINK}Press any key, Enter, or Esc to close...${ANSI_RESET} "

# Read single character or keypress without echo
read -r -s -n 1 _ || true
printf "\n"
