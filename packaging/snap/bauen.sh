#!/usr/bin/env bash
# Baut das Snap-Paket aus snap/snapcraft.yaml in einem sauberen Arbeitsordner.
# Braucht: snapcraft und lxd (siehe packaging/snap/README.md).
# Hinweis: Muss in einem interaktiven Terminal laufen (oder mit `script -qefc`).
set -euo pipefail
cd "$(dirname "$0")/../.."

if ! command -v snapcraft >/dev/null 2>&1; then
  echo "Fehler: snapcraft ist nicht installiert (sudo snap install snapcraft --classic)." >&2
  exit 1
fi

python3 packaging/lizenzen.py
python3 packaging/lizenztexte.py

VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n1)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

rsync -a \
  --exclude='/target' \
  --exclude='/.git' \
  --exclude='/distribute' \
  --exclude='/werkstatt-web' \
  --exclude='/Transfer' \
  --exclude='/packaging/flatpak/vendor' \
  --exclude='/packaging/flatpak/build' \
  --exclude='/packaging/flatpak/.flatpak-builder' \
  --exclude='/packaging/flatpak/repo' \
  --exclude='/packaging/flatpak/*.flatpak' \
  --exclude='/my-key-export' \
  --exclude='/packaging/my-key-export' \
  --exclude='/my-keys.md' \
  ./ "$TMP/"

(
  cd "$TMP"
  snapcraft pack
)

mkdir -p target/snap
mv "$TMP"/*.snap "target/snap/"
echo "Snap erzeugt in target/snap/:"
ls -1 target/snap/*.snap
