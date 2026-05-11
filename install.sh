#!/usr/bin/env bash
set -e

REPO="hw1964/javalens-manager"
APP_NAME="javalens-manager"
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/128x128/apps"

MACHINE=$(uname -m)
case "$MACHINE" in
    x86_64)         APPIMAGE_ARCH="amd64" ;;
    aarch64|arm64)  APPIMAGE_ARCH="aarch64" ;;
    *)
        echo "Error: unsupported architecture '$MACHINE'."
        echo "Supported: x86_64 (amd64) and aarch64/arm64."
        exit 1
        ;;
esac
echo "Detected architecture: $MACHINE -> $APPIMAGE_ARCH"

echo "Fetching latest release from GitHub..."
LATEST_RELEASE=$(curl -sSL "https://api.github.com/repos/$REPO/releases/latest")
APPIMAGE_URL=$(echo "$LATEST_RELEASE" | grep -oP "\"browser_download_url\": \"\K(.*_${APPIMAGE_ARCH}\.AppImage)(?=\")")

if [ -z "$APPIMAGE_URL" ]; then
    echo "Error: Could not find an .AppImage for architecture '$APPIMAGE_ARCH' in the latest release."
    echo "Check https://github.com/$REPO/releases/latest for available assets."
    exit 1
fi

echo "Downloading $APP_NAME..."
mkdir -p "$BIN_DIR"
curl -sSL -o "$BIN_DIR/$APP_NAME" "$APPIMAGE_URL"
chmod +x "$BIN_DIR/$APP_NAME"

echo "Setting up desktop entry..."
mkdir -p "$APP_DIR"
mkdir -p "$ICON_DIR"

# Download icon from the repository
curl -sSL -o "$ICON_DIR/$APP_NAME.png" "https://raw.githubusercontent.com/$REPO/main/src-tauri/icons/128x128.png"

cat > "$APP_DIR/$APP_NAME.desktop" <<EOF
[Desktop Entry]
Name=javalens-manager
Exec=$BIN_DIR/$APP_NAME
Icon=$ICON_DIR/$APP_NAME.png
Type=Application
Categories=Development;
Terminal=false
EOF

# Try to update desktop database and icon cache silently if the tools are available
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" || true
fi

echo "Installation complete! You can now launch $APP_NAME from your application menu."
echo "Note: Make sure $BIN_DIR is in your PATH."
