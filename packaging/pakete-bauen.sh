#!/usr/bin/env bash
# Baut Release-Binärdatei, Debian-Paket (.deb) und RPM (.rpm); Ergebnis in target/debian/ und target/generate-rpm/.
# Braucht: cargo-deb und cargo-generate-rpm (cargo install cargo-deb cargo-generate-rpm).
# Flatpak: packaging/flatpak/bauen.sh
set -euo pipefail
cd "$(dirname "$0")/.."
# Lizenzlisten und -texte der Fremdbausteine neu erzeugen (lizenzen.txt steckt im Programm, beide kommen ins Paket)
python3 packaging/lizenzen.py
python3 packaging/lizenztexte.py
cargo build --release -p werkbank-app
cargo deb -p werkbank-app --no-build
cargo generate-rpm -p crates/werkbank-app
echo
ls -1 target/debian/*.deb target/generate-rpm/*.rpm
