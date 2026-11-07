#!/usr/bin/env bash
# Baut den lokalen Snap des Lizenzgebers MIT privatem Schlüssel (nur für eigene Rechner).
# Der Schlüssel kommt aus $WERKBANK_LIZENZGEBER bzw. ~/.local/share/werkbank-lizenzgeber/.
# Ergebnis: ~/Lizenzgeber-Pakete/werkbank-lizenzgeber_<version>_amd64.snap (Rechte 600).
# Installieren: sudo snap install --dangerous <datei>.snap
# Braucht: snapcraft (sudo snap install snapcraft --classic) und lxd oder multipass; mit
# SNAPCRAFT_BUILD_ENVIRONMENT=host (nur Ubuntu 24.04) baut es auf dem Rechner selbst.
set -euo pipefail
cd "$(dirname "$0")/../.."
quelle="${WERKBANK_LIZENZGEBER:-$HOME/.local/share/werkbank-lizenzgeber}"
ziel="${LIZENZGEBER_PAKETE:-$HOME/Lizenzgeber-Pakete}"
[ -f "$quelle/privat.key" ] || { echo "$quelle/privat.key fehlt." >&2; exit 1; }
cmp -s "$quelle/oeffentlich.key" assets/lizenz-oeffentlich.key || { echo "oeffentlich.key passt nicht zu assets/lizenz-oeffentlich.key." >&2; exit 1; }

arbeit="$PWD/target/snap-build/lizenzgeber"   # snapcraft packt nur innerhalb des Projekts
rm -rf "$arbeit"; mkdir -p "$arbeit"
# Der Arbeitsordner und der Build-Container enthalten den privaten Schlüssel: beides am Ende löschen
trap '(cd "$arbeit" && snapcraft clean >/dev/null 2>&1); rm -rf "$arbeit"' EXIT
# Nur versionierte Dateien (kein target/); der Schlüssel liegt nur im Arbeitsordner
git ls-files -z --cached --others --exclude-standard | grep -zv -e "my-key" -e "^snap/" -e "^werkstatt-web/" | while IFS= read -r -d "" f; do if [ -e "$f" ]; then printf "%s\0" "$f"; fi; done | xargs -0 cp -P --parents -t "$arbeit"
install -d -m700 "$arbeit/schluessel" "$arbeit/snap/gui"
install -m600 "$quelle/privat.key" "$quelle/oeffentlich.key" "$arbeit/schluessel/"
install -m644 packaging/snap/lizenzgeber/snapcraft.yaml "$arbeit/snap/snapcraft.yaml"
install -m644 assets/icons/lizenzgeber/hicolor/256x256/apps/de.daehn.Werkbank.Lizenzgeber.png "$arbeit/snap/gui/werkbank-lizenzgeber.png"
sed -e 's|^Exec=.*|Exec=werkbank-lizenzgeber|' -e 's|^Icon=.*|Icon=${SNAP}/meta/gui/werkbank-lizenzgeber.png|' \
    assets/de.daehn.Werkbank.Lizenzgeber.desktop > "$arbeit/snap/gui/werkbank-lizenzgeber.desktop"

(cd "$arbeit" && snapcraft pack)
version=$(sed -n "s/^version: '\(.*\)'/\1/p" packaging/snap/lizenzgeber/snapcraft.yaml)
mkdir -p "$ziel"; chmod 700 "$ziel"
install -m600 "$arbeit"/werkbank-lizenzgeber_*.snap "$ziel/werkbank-lizenzgeber_${version}_amd64.snap"
echo "Fertig: $ziel/werkbank-lizenzgeber_${version}_amd64.snap"
