#!/usr/bin/env bash

APP_NAME="javalens-manager"
BIN_DIR="$HOME/.local/bin"
APP_DIR="$HOME/.local/share/applications"

echo "Uninstalling $APP_NAME..."

if [ -f "$BIN_DIR/$APP_NAME" ]; then
    rm "$BIN_DIR/$APP_NAME"
    echo "Removed executable from $BIN_DIR"
fi

if [ -f "$APP_DIR/$APP_NAME.desktop" ]; then
    rm "$APP_DIR/$APP_NAME.desktop"
    echo "Removed desktop entry from $APP_DIR"
fi

echo "Uninstallation complete. Note: Configuration and data files in ~/.config/javalens-manager and ~/.local/state/javalens-manager were kept."
