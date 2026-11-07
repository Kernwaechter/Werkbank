#!/usr/bin/env bash
# Baut den Snap der Werkbank (strict, für den Snap Store) aus den versionierten Dateien.
# Braucht: snapcraft (sudo snap install snapcraft --classic) und lxd oder multipass.
# Hochladen: snapcraft upload --release=edge jz-werkbank_<version>_amd64.snap (nach `snapcraft register jz-werkbank`).
set -euo pipefail
cd "$(dirname "$0")/../.."
arbeit="$PWD/target/snap-build/app"   # fester Ort im Projekt: snapcraft packt nur dort, und der Container bleibt wiederverwendbar
rm -rf "$arbeit"; mkdir -p "$arbeit"
# Ohne target/ (mehrere GB), sonst kopiert snapcraft es mit
git ls-files -z --cached --others --exclude-standard | grep -zv -e "my-key" | xargs -0 cp -P --parents -t "$arbeit"
(cd "$arbeit" && snapcraft pack)
mkdir -p target/snap
mv "$arbeit"/*.snap target/snap/
ls -l target/snap/*.snap
