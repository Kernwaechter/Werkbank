#!/usr/bin/env bash
# Baut das RPM der Werkbank neu und signiert es mit einem PGP-Schlüssel.
# Aufruf: packaging/rpm-signieren.sh /pfad/zu/werkbank-rpm-signatur.secret.asc
# Voraussetzung: das Release-Binary ist gebaut (packaging/pakete-bauen.sh) und der Schlüssel ist eine
# exportierte, passwortlose RSA-4096-Datei (siehe SIGNIEREN.md, Abschnitt „RPM signieren“).
set -euo pipefail
cd "$(dirname "$0")/.."
schluessel="${1:?Aufruf: rpm-signieren.sh <geheime-schluesseldatei.asc>}"
[ -f "$schluessel" ] || { echo "$schluessel nicht gefunden." >&2; exit 1; }
[ -x target/release/werkbank-app ] || { echo "target/release/werkbank-app fehlt: erst packaging/pakete-bauen.sh ausführen." >&2; exit 1; }
cargo generate-rpm -p crates/werkbank-app --signing-key "$schluessel"
rpm=$(ls -t target/generate-rpm/werkbank-[0-9]*.rpm | head -1)
echo
echo "Signiertes RPM: $rpm"
sha256sum "$rpm"
echo
echo "Prüfen (auf Fedora/openSUSE): sudo rpm --import werkbank-rpm-signatur.asc && rpm --checksig $(basename "$rpm")"
