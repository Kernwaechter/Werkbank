#!/usr/bin/env bash
# Baut und installiert das Flatpak lokal.
# Braucht: flatpak, flatpak-builder, Laufzeit und SDK (werden bei Bedarf angeboten):
#   flatpak install flathub org.freedesktop.Platform//26.08 org.freedesktop.Sdk//26.08 org.freedesktop.Sdk.Extension.rust-stable//26.08
set -euo pipefail
cd "$(dirname "$0")"
# Lizenzlisten und -texte der Fremdbausteine neu erzeugen
(cd ../.. && python3 packaging/lizenzen.py && python3 packaging/lizenztexte.py)
# Alle Abhängigkeiten in vendor/ ablegen (flatpak-builder baut ohne Netz)
(cd ../.. && cargo vendor --locked packaging/flatpak/vendor > /dev/null)
flatpak-builder --user --install --force-clean build ../de.daehn.Werkbank.yaml
echo "Fertig. Starten mit: flatpak run de.daehn.Werkbank"
