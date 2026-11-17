#!/usr/bin/env bash
# Baut das Flatpak-Bündel aus packaging/de.daehn.Werkbank.yaml.
# Braucht: flatpak, flatpak-builder sowie Runtime/SDK (org.freedesktop.Platform//26.08,
# org.freedesktop.Sdk//26.08 und org.freedesktop.Sdk.Extension.rust-stable//26.08).
set -euo pipefail
cd "$(dirname "$0")/../.."

if ! command -v flatpak-builder >/dev/null 2>&1; then
  echo "Fehler: flatpak-builder ist nicht installiert (z. B. sudo apt install flatpak-builder)." >&2
  exit 1
fi

VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)
OUT="packaging/flatpak/werkbank-${VERSION}.flatpak"

python3 packaging/lizenzen.py
python3 packaging/lizenztexte.py

rm -rf packaging/flatpak/vendor
cargo vendor --locked packaging/flatpak/vendor > /dev/null

flatpak-builder \
  --force-clean \
  --repo=packaging/flatpak/repo \
  --state-dir=packaging/flatpak/.flatpak-builder \
  packaging/flatpak/build \
  packaging/de.daehn.Werkbank.yaml

flatpak build-bundle \
  packaging/flatpak/repo \
  "$OUT" \
  de.daehn.Werkbank

echo "Flatpak erzeugt: $OUT"
