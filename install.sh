#!/usr/bin/env bash
set -e

REPO="hw1964/javalens-manager"
APP_NAME="javalens-manager"
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"
ICON_DIR="$HOME/.local/share/icons/hicolor/128x128/apps"

echo "Fetching latest release from GitHub..."
LATEST_RELEASE=$(curl -sSL "https://api.github.com/repos/$REPO/releases/latest")
APPIMAGE_URL=$(echo "$LATEST_RELEASE" | grep -oP '"browser_download_url": "\K(.*\.AppImage)(?=")')

if [ -z "$APPIMAGE_URL" ]; then
    echo "Error: Could not find .AppImage in the latest release."
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
Icon=$APP_NAME
Type=Application
Categories=Development;
Terminal=false
EOF

echo "Installation complete! You can now launch $APP_NAME from your application menu."
echo "Note: Make sure $BIN_DIR is in your PATH."
