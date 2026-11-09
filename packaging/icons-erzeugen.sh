#!/usr/bin/env bash
# Erzeugt alle Symbolgrößen aus assets/icons/werkbank.svg (einzige Quelle)
# PNG je Größe (Linux/hicolor), werkbank.ico (Windows), werkbank.icns (macOS).
# Braucht: typst, python3 mit Pillow. Aufruf im Projektordner: packaging/icons-erzeugen.sh
set -euo pipefail
cd "$(dirname "$0")/.."
ID=de.daehn.Werkbank
QUELLE=assets/icons/werkbank.svg
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT
cp "$QUELLE" "$TMP/werkbank.svg"

png() { # Größe → Datei (transparent; ein Punkt = ein Pixel bei 72 ppi)
  printf '#set page(width: %spt, height: %spt, margin: 0pt, fill: none)\n#image("werkbank.svg", width: 100%%, height: 100%%)\n' "$1" "$1" > "$TMP/i$1.typ"
  typst compile --format png --ppi 72 "$TMP/i$1.typ" "$2"
}

for g in 16 22 24 32 48 64 128 256 512; do
  mkdir -p "assets/icons/hicolor/${g}x${g}/apps"
  png "$g" "assets/icons/hicolor/${g}x${g}/apps/$ID.png"
done
mkdir -p assets/icons/hicolor/scalable/apps
cp "$QUELLE" "assets/icons/hicolor/scalable/apps/$ID.svg"
cp "assets/icons/hicolor/256x256/apps/$ID.png" assets/icons/256.png
png 1024 "$TMP/1024.png"

python3 - "$TMP" <<'PY'
import sys
from PIL import Image
tmp = sys.argv[1]
gross = Image.open(f"{tmp}/1024.png").convert("RGBA")
# Windows: mehrere Größen in einer .ico
gross.save("assets/icons/werkbank.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
# macOS: .icns mit allen üblichen Größen
gross.save("assets/icons/werkbank.icns", format="ICNS", sizes=[(s, s) for s in (16, 32, 64, 128, 256, 512, 1024)])
PY
echo "Symbole in assets/icons/ erzeugt."
