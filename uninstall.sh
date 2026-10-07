#!/usr/bin/env bash
# uninstall.sh: Clean uninstaller for agymux

set -euo pipefail

BIN_DIR="${HOME}/.local/bin"
TMUX_CONF="${HOME}/.config/tmux/tmux.conf"
TMUX_TABS_CONF="${HOME}/.config/tmux/agymux-tabs.conf"

echo "==> Uninstalling agymux..."

# 1. Remove binaries from ~/.local/bin
for b in agymux agymux-tab-picker agymux-tab-runner; do
    if [[ -L "$BIN_DIR/$b" || -f "$BIN_DIR/$b" ]]; then
        echo "  -> Removing $BIN_DIR/$b"
        rm -f "$BIN_DIR/$b"
    fi
done

# 2. Remove tmux config link
if [[ -L "$TMUX_TABS_CONF" || -f "$TMUX_TABS_CONF" ]]; then
    echo "  -> Removing $TMUX_TABS_CONF"
    rm -f "$TMUX_TABS_CONF"
fi

# 3. Strip directive from tmux.conf
if [[ -f "$TMUX_CONF" ]] && grep -q "agymux-tabs.conf" "$TMUX_CONF"; then
    echo "  -> Removing source directive from $TMUX_CONF"
    # Portable in-place removal
    tmp=$(mktemp)
    grep -v "agymux-tabs.conf" "$TMUX_CONF" | grep -v "# Source agymux" > "$tmp" || true
    mv "$tmp" "$TMUX_CONF"
fi

# 4. Reload tmux configuration if server is active
if command -v tmux >/dev/null 2>&1 && tmux has-session 2>/dev/null; then
    echo "  -> Reloading active tmux configuration"
    tmux source-file "$TMUX_CONF" 2>/dev/null || true
fi

echo "==> agymux uninstalled cleanly."
