#!/bin/sh
# Installa eDEX-UI RS per l'utente corrente (senza root): ~/.local/bin + menu applicazioni.
set -e
cd "$(dirname "$0")"
install -Dm755 edex-ui-rs "$HOME/.local/bin/edex-ui-rs"
install -Dm644 edex-ui-rs.png "$HOME/.local/share/icons/hicolor/128x128/apps/edex-ui-rs.png"
install -Dm644 edex-ui-rs.desktop "$HOME/.local/share/applications/edex-ui-rs.desktop"
echo "Installato. Avvia con: edex-ui-rs (assicurati che ~/.local/bin sia nel PATH)"
