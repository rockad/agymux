#!/usr/bin/env bash
# theme.sh: Monokai Pro Octagon styling tokens and helpers for agymux
# Workstation UI standards reference: ~/.agents/memory/environment-and-ui-preferences.md

set -euo pipefail

# 1. Monokai Pro Octagon Color Tokens (Hex)
export AGYMUX_THEME_BG="#282a3a"          # Base warm dark grey background
export AGYMUX_THEME_BG_ALT="#221f22"      # Inactive tab background
export AGYMUX_THEME_FG="#eaf2f1"          # Primary text foreground
export AGYMUX_THEME_MUTED="#727072"       # Inactive tab text / dimmed metadata
export AGYMUX_THEME_ACTIVE_BG="#a9dc76"   # Active tab pill background (green)
export AGYMUX_THEME_ACTIVE_CYAN="#78dce8" # Alternative active pill (cyan)
export AGYMUX_THEME_ACTIVE_FG="#282a3a"   # Active tab pill text
export AGYMUX_THEME_PREFIX="#ff6188"      # Prefix indicator (pink)
export AGYMUX_THEME_RUNNING="#fc9867"     # Running turn / agent working (orange ●)
export AGYMUX_THEME_YELLOW="#ffd866"      # Yellow warning / highlight
export AGYMUX_THEME_PURPLE="#ab9df2"      # Purple accent / project badges

# 2. Powerline Glyphs
export AGYMUX_PL_LEFT=""
export AGYMUX_PL_RIGHT=""
export AGYMUX_ICON_RUNNING="●"
export AGYMUX_ICON_READY="✓"
export AGYMUX_ICON_DIRTY="*"

# 3. ANSI 24-bit TrueColor Sequences
export ANSI_RESET=$'\033[0m'
export ANSI_BOLD=$'\033[1m'
export ANSI_DIM=$'\033[2m'
export ANSI_ITALIC=$'\033[3m'
export ANSI_UNDERLINE=$'\033[4m'

# Text Foregrounds
export ANSI_FG=$'\033[38;2;234;242;241m'       # #eaf2f1
export ANSI_MUTED=$'\033[38;2;114;112;114m'    # #727072
export ANSI_ACTIVE_GREEN=$'\033[38;2;169;220;118m' # #a9dc76
export ANSI_ACTIVE_CYAN=$'\033[38;2;120;220;232m'  # #78dce8
export ANSI_PREFIX_PINK=$'\033[38;2;255;97;136m'   # #ff6188
export ANSI_RUNNING_ORANGE=$'\033[38;2;252;152;103m' # #fc9867
export ANSI_YELLOW=$'\033[38;2;255;216;102m'   # #ffd866
export ANSI_PURPLE=$'\033[38;2;171;157;242m'   # #ab9df2
export ANSI_DARK_TEXT=$'\033[38;2;40;42;58m'   # #282a3a

# Backgrounds
export ANSI_BG_BASE=$'\033[48;2;40;42;58m'     # #282a3a
export ANSI_BG_INACTIVE=$'\033[48;2;34;31;34m' # #221f22
export ANSI_BG_GREEN=$'\033[48;2;169;220;118m' # #a9dc76
export ANSI_BG_CYAN=$'\033[48;2;120;220;232m'  # #78dce8
export ANSI_BG_PINK=$'\033[48;2;255;97;136m'   # #ff6188
export ANSI_BG_ORANGE=$'\033[48;2;252;152;103m'# #fc9867
export ANSI_BG_PURPLE=$'\033[48;2;171;157;242m'# #ab9df2

# 4. Standard FZF Color Flags for Monokai Pro Octagon
export AGYMUX_FZF_THEME=(
    --color="bg+:${AGYMUX_THEME_BG_ALT},bg:${AGYMUX_THEME_BG},fg:${AGYMUX_THEME_FG}"
    --color="fg+:${AGYMUX_THEME_ACTIVE_BG},hl:${AGYMUX_THEME_YELLOW},hl+:${AGYMUX_THEME_ACTIVE_BG}"
    --color="pointer:${AGYMUX_THEME_PREFIX},prompt:${AGYMUX_THEME_ACTIVE_CYAN},header:${AGYMUX_THEME_PURPLE}"
    --color="info:${AGYMUX_THEME_RUNNING},border:${AGYMUX_THEME_MUTED}"
)

# 5. CLI Test & Inspection Output
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    printf "${ANSI_BOLD}${ANSI_PURPLE}agymux v0.2 Monokai Pro Octagon Color Palette${ANSI_RESET}\n\n"
    printf "  Base Background:    %s#282a3a%s  [%s  ████  %s]\n" "$ANSI_MUTED" "$ANSI_RESET" "$ANSI_BG_BASE" "$ANSI_RESET"
    printf "  Inactive Tab:       %s#727072 on #221f22%s  [%s%s  Inactive Tab  %s]\n" "$ANSI_MUTED" "$ANSI_RESET" "$ANSI_BG_INACTIVE" "$ANSI_MUTED" "$ANSI_RESET"
    printf "  Active Tab Pill:    %s#282a3a on #a9dc76%s  [%s%s%s %sActive Tab%s %s%s]\n" "$ANSI_MUTED" "$ANSI_RESET" "$ANSI_ACTIVE_GREEN" "$AGYMUX_PL_LEFT" "$ANSI_BG_GREEN$ANSI_DARK_TEXT$ANSI_BOLD" "$ANSI_RESET$ANSI_ACTIVE_GREEN" "$AGYMUX_PL_RIGHT" "$ANSI_RESET"
    printf "  Active Cyan Pill:   %s#282a3a on #78dce8%s  [%s%s%s %sActive Cyan%s %s%s]\n" "$ANSI_MUTED" "$ANSI_RESET" "$ANSI_ACTIVE_CYAN" "$AGYMUX_PL_LEFT" "$ANSI_BG_CYAN$ANSI_DARK_TEXT$ANSI_BOLD" "$ANSI_RESET$ANSI_ACTIVE_CYAN" "$AGYMUX_PL_RIGHT" "$ANSI_RESET"
    printf "  Prefix Indicator:   %s#ff6188%s  [%s%s 󰘳 PREFIX %s]\n" "$ANSI_MUTED" "$ANSI_RESET" "$ANSI_BG_PINK$ANSI_DARK_TEXT$ANSI_BOLD" "$ANSI_RESET" "$ANSI_RESET"
    printf "  Running Turn:       %s#fc9867%s  [%s %s RUNNING%s ]\n" "$ANSI_MUTED" "$ANSI_RESET" "$ANSI_RUNNING_ORANGE" "$AGYMUX_ICON_RUNNING" "$ANSI_RESET"
    printf "  Purple Accent:      %s#ab9df2%s  [%s  ████  %s]\n" "$ANSI_MUTED" "$ANSI_RESET" "$ANSI_PURPLE" "$ANSI_RESET"
    printf "  Yellow Warning:     %s#ffd866%s  [%s  ████  %s]\n" "$ANSI_MUTED" "$ANSI_RESET" "$ANSI_YELLOW" "$ANSI_RESET"
    echo ""
fi
