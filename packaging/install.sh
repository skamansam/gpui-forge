#!/bin/sh
# Installs the Forge Studio desktop entry + icon for the current user.
set -e
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

mkdir -p "$HOME/.local/share/icons/hicolor/scalable/apps"
mkdir -p "$HOME/.local/share/applications"

cp "$ROOT/gpui_forge_icon_medallion.svg" \
   "$HOME/.local/share/icons/hicolor/scalable/apps/gpui-forge.svg"
cp "$ROOT/packaging/gpui-forge.desktop" \
   "$HOME/.local/share/applications/gpui-forge.desktop"

command -v update-desktop-database >/dev/null && \
    update-desktop-database "$HOME/.local/share/applications" || true
command -v gtk-update-icon-cache >/dev/null && \
    gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" || true

echo "installed gpui-forge.desktop + gpui-forge.svg"
