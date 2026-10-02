#!/bin/sh
set -e

# Agent-Radar One-Line Installer
# Zero-privilege, static binary deployment for Linux

OS="$(uname -s)"
ARCH="$(uname -m)"

if [ "$OS" != "Linux" ]; then
    echo "[ERROR] Agent-Radar requires Linux (inotify + procfs). Unsupported OS: $OS" >&2
    exit 1
fi

case "$ARCH" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        ;;
    aarch64|arm64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        echo "[ERROR] Unsupported architecture: $ARCH. Agent-Radar supports x86_64 and aarch64." >&2
        exit 1
        ;;
esac

echo "[*] Detecting installation destination..."
if [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
else
    INSTALL_DIR="$HOME/.local/bin"
    mkdir -p "$INSTALL_DIR"
fi

BINARY_NAME="agent-radar"
DEST="$INSTALL_DIR/$BINARY_NAME"

echo "[*] Installing Agent-Radar ($TARGET_ARCH) to $DEST..."

# If running from local build repository, install directly
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
LOCAL_BIN="$SCRIPT_DIR/target/${TARGET_ARCH}-unknown-linux-musl/release/$BINARY_NAME"
if [ ! -f "$LOCAL_BIN" ]; then
    LOCAL_BIN="$SCRIPT_DIR/target/release/$BINARY_NAME"
fi

if [ -f "$LOCAL_BIN" ]; then
    echo "[*] Copying local binary from $LOCAL_BIN..."
    cp "$LOCAL_BIN" "$DEST"
    chmod 755 "$DEST"
else
    # Fetch from GitHub release
    LATEST_URL="https://github.com/agent-radar/agent-radar/releases/latest/download/agent-radar-${TARGET_ARCH}-unknown-linux-musl"
    echo "[*] Downloading static binary from $LATEST_URL..."
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$LATEST_URL" -o "$DEST"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$DEST" "$LATEST_URL"
    else
        echo "[ERROR] Neither curl nor wget found in PATH." >&2
        exit 1
    fi
    chmod 755 "$DEST"
fi

echo "[✓] Successfully installed Agent-Radar at $DEST"
"$DEST" --version

case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *)
        echo ""
        echo "[NOTE] $INSTALL_DIR is not in your current PATH."
        echo "Add it with: export PATH=\"\$PATH:$INSTALL_DIR\""
        ;;
esac

echo ""
echo "Run 'agent-radar' in your project directory or in a tmux pane next to your AI agent!"
