#!/usr/bin/env bash
# uninstall.sh: Clean uninstaller for agymux

set -euo pipefail

BIN_DIR="${HOME}/.local/bin"
TMUX_CONF="${HOME}/.config/tmux/tmux.conf"
TMUX_AGY_CONF="${HOME}/.config/tmux/agymux.conf"
TMUX_TABS_CONF="${HOME}/.config/tmux/agymux-tabs.conf"

echo "==> Uninstalling agymux..."

# 1. Remove binaries from ~/.local/bin
for b in agymux ax agymux-tab-picker agymux-tab-runner agymux-palette agymux-project-picker agymux-helper agymux-preview; do
    if [[ -L "$BIN_DIR/$b" || -f "$BIN_DIR/$b" ]]; then
        echo "  -> Removing $BIN_DIR/$b"
        rm -f "$BIN_DIR/$b"
    fi
done

# 2. Remove tmux config links
for cfg in "$TMUX_AGY_CONF" "$TMUX_TABS_CONF"; do
    if [[ -L "$cfg" || -f "$cfg" ]]; then
        echo "  -> Removing $cfg"
        rm -f "$cfg"
    fi
done

# 3. Strip directive from tmux.conf
if [[ -f "$TMUX_CONF" ]]; then
    if grep -q "agymux" "$TMUX_CONF"; then
        echo "  -> Removing source directive from $TMUX_CONF"
        tmp=$(mktemp)
        grep -v "agymux.conf" "$TMUX_CONF" | grep -v "agymux-tabs.conf" | grep -v "# Source agymux" > "$tmp" || true
        mv "$tmp" "$TMUX_CONF"
    fi
fi

# 4. Reload tmux configuration if server is active
if command -v tmux >/dev/null 2>&1 && tmux has-session 2>/dev/null; then
    echo "  -> Reloading active tmux configuration"
    tmux source-file "$TMUX_CONF" 2>/dev/null || true
fi

echo "==> agymux uninstalled cleanly."
