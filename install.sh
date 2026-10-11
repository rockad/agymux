#!/usr/bin/env bash
# install.sh: Idempotent installer for agymux

set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="${HOME}/.local/bin"
TMUX_CONF_DIR="${HOME}/.config/tmux"
TMUX_CONF="${TMUX_CONF_DIR}/tmux.conf"

echo "==> Installing agymux from ${REPO_DIR}..."

# 1. Install binaries into ~/.local/bin
mkdir -p "$BIN_DIR"
# Clean up any legacy secondary symlinks
for legacy in agymux-tab-picker agymux-tab-runner agymux-palette agymux-project-picker agymux-helper agymux-preview; do
    rm -f "${BIN_DIR}/$legacy"
done

if [[ -f "${REPO_DIR}/agymux" && -f "${REPO_DIR}/ax" ]]; then
    echo "  -> Installing pre-built binaries to $BIN_DIR"
    cp -f "${REPO_DIR}/agymux" "${BIN_DIR}/agymux"
    cp -f "${REPO_DIR}/ax" "${BIN_DIR}/ax"
    chmod +x "${BIN_DIR}/agymux" "${BIN_DIR}/ax"
elif [[ -f "${REPO_DIR}/target/release/agymux" && -f "${REPO_DIR}/target/release/ax" ]]; then
    echo "  -> Linking release binaries to $BIN_DIR"
    ln -sf "${REPO_DIR}/target/release/agymux" "${BIN_DIR}/agymux"
    ln -sf "${REPO_DIR}/target/release/ax" "${BIN_DIR}/ax"
elif command -v cargo >/dev/null 2>&1; then
    echo "  -> Building release binaries with cargo"
    (cd "${REPO_DIR}" && cargo build --release)
    ln -sf "${REPO_DIR}/target/release/agymux" "${BIN_DIR}/agymux"
    ln -sf "${REPO_DIR}/target/release/ax" "${BIN_DIR}/ax"
elif [[ -x "${HOME}/.cargo/bin/cargo" ]]; then
    echo "  -> Building release binaries with ~/.cargo/bin/cargo"
    (cd "${REPO_DIR}" && "${HOME}/.cargo/bin/cargo" build --release)
    ln -sf "${REPO_DIR}/target/release/agymux" "${BIN_DIR}/agymux"
    ln -sf "${REPO_DIR}/target/release/ax" "${BIN_DIR}/ax"
else
    echo "Error: No binaries found and cargo is not available." >&2
    exit 1
fi

# 2. Wire tmux configuration
mkdir -p "$TMUX_CONF_DIR"
if [[ -d "${REPO_DIR}/.git" ]]; then
    ln -sf "${REPO_DIR}/config/tmux/agymux-tabs.conf" "${TMUX_CONF_DIR}/agymux-tabs.conf"
elif [[ -f "${REPO_DIR}/config/tmux/agymux-tabs.conf" ]]; then
    cp -f "${REPO_DIR}/config/tmux/agymux-tabs.conf" "${TMUX_CONF_DIR}/agymux-tabs.conf"
fi

if [[ -f "$TMUX_CONF" ]]; then
    if ! grep -q "agymux-tabs.conf" "$TMUX_CONF"; then
        echo "  -> Adding agymux-tabs.conf source directive to $TMUX_CONF"
        cat << 'EOF' >> "$TMUX_CONF"

# Source agymux top-tab styling & keybindings
if-shell '[ -f ~/.config/tmux/agymux-tabs.conf ]' 'source-file ~/.config/tmux/agymux-tabs.conf'
EOF
    else
        echo "  -> agymux-tabs.conf directive already present in $TMUX_CONF"
    fi
fi

# 3. Reload tmux configuration if server is active
if command -v tmux >/dev/null 2>&1 && tmux has-session 2>/dev/null; then
    echo "  -> Reloading active tmux configuration"
    tmux source-file "$TMUX_CONF" 2>/dev/null || true
fi

echo "==> agymux installation complete!"
echo "    Run 'agymux' to start or 'agymux switch' for the tab switcher."
