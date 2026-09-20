#!/bin/bash
set -e

APP_NAME="RixLauncher"
EXECUTABLE_NAME="rix_launcher"
APPLICATION_ID="rix_launcher"
DESKTOP_ENTRY_NAME="${APPLICATION_ID}.desktop"
APP_COMMENT="Modern Minecraft Launcher"

INSTALL_BIN_DIR="${HOME}/.local/bin"
ICON_DIR="${HOME}/.local/share/icons/hicolor/256x256/apps"
APPLICATIONS_DIR="${HOME}/.local/share/applications"
BINARY_SOURCE="target/release/${EXECUTABLE_NAME}"
BINARY_TARGET="${INSTALL_BIN_DIR}/${EXECUTABLE_NAME}"
ICON_TARGET="${ICON_DIR}/${APPLICATION_ID}.png"
DESKTOP_TARGET="${APPLICATIONS_DIR}/${DESKTOP_ENTRY_NAME}"

echo "[${APP_NAME} Installer] Building release binary..."
cargo build --release

# Ensure local directories exist.
mkdir -p "$INSTALL_BIN_DIR" "$ICON_DIR" "$APPLICATIONS_DIR"

echo "[${APP_NAME} Installer] Copying binary..."
install -m 755 "$BINARY_SOURCE" "$BINARY_TARGET"

echo "[${APP_NAME} Installer] Copying assets..."
# Convert SVG to PNG if magick exists, otherwise copy pre-generated PNG.
if command -v magick &> /dev/null; then
    magick -background none icon.svg -resize 256x256 -depth 8 "$ICON_TARGET"
elif [ -f src/icon_256.png ]; then
    install -m 644 src/icon_256.png "$ICON_TARGET"
else
    echo "[${APP_NAME} Installer] Warning: no icon asset was found; continuing without one."
fi

echo "[${APP_NAME} Installer] Installing desktop entry..."
desktop_tmp="$(mktemp "${APPLICATIONS_DIR}/.${DESKTOP_ENTRY_NAME}.XXXXXX")"
trap 'rm -f "$desktop_tmp"' EXIT
printf '%s\n' \
    "[Desktop Entry]" \
    "Name=${APP_NAME}" \
    "Comment=${APP_COMMENT}" \
    "Type=Application" \
    "Icon=${APPLICATION_ID}" \
    "Exec=\"${BINARY_TARGET}\"" \
    "StartupWMClass=${APPLICATION_ID}" \
    "Categories=Game;" \
    "Terminal=false" > "$desktop_tmp"
chmod 644 "$desktop_tmp"
mv -f "$desktop_tmp" "$DESKTOP_TARGET"
trap - EXIT

echo "[${APP_NAME} Installer] Installation complete! You can now start ${APP_NAME} from your application menu."
