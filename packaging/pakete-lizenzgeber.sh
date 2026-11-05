#!/usr/bin/env bash
# Baut deb, rpm und Flatpak des Lizenzgebers (werkbank-lizenz) MIT privatem Schlüssel.
# Der Schlüssel kommt aus $WERKBANK_LIZENZGEBER bzw. ~/.local/share/werkbank-lizenzgeber/.
# Ergebnis: ~/Lizenzgeber-Pakete/ (Rechte 700). NIEMALS auf die Webseite oder in ein Repository legen.
# Braucht: cargo-deb, cargo-generate-rpm, flatpak (mit org.freedesktop.Platform//25.08).
# OHNE_FLATPAK=1 baut nur deb und rpm.
set -euo pipefail
cd "$(dirname "$0")/.."
quelle="${WERKBANK_LIZENZGEBER:-$HOME/.local/share/werkbank-lizenzgeber}"
ziel="${LIZENZGEBER_PAKETE:-$HOME/Lizenzgeber-Pakete}"
stage=target/lizenzgeber-stage
[ -f "$quelle/privat.key" ] || { echo "$quelle/privat.key fehlt." >&2; exit 1; }
# Öffentlicher Schlüssel im Datenordner muss zu dem in der Werkbank passen
cmp -s "$quelle/oeffentlich.key" assets/lizenz-oeffentlich.key || { echo "oeffentlich.key passt nicht zu assets/lizenz-oeffentlich.key." >&2; exit 1; }

rm -rf "$stage"; mkdir -p "$stage"
trap 'rm -rf "$stage"' EXIT
install -m644 "$quelle/privat.key" "$quelle/oeffentlich.key" "$stage/"

cargo build --release -p werkbank-lizenz
version=$(cargo pkgid -p werkbank-lizenz | sed 's/.*[#@]//')
mkdir -p "$ziel"; chmod 700 "$ziel"
cargo deb -p werkbank-lizenz --no-build --output "$ziel/werkbank-lizenzgeber_${version}_amd64.deb"
cargo generate-rpm -p crates/werkbank-lizenz --output "$ziel/werkbank-lizenzgeber-${version}-1.x86_64.rpm"

# Flatpak: Binary + Schlüssel in ein Build-Verzeichnis, dann als Bundle verpacken
if [ "${OHNE_FLATPAK:-}" != "1" ]; then
fb=target/flatpak-lizenzgeber
rm -rf "$fb" target/flatpak-lizenzgeber-repo
flatpak build-init "$fb" de.daehn.Werkbank.Lizenzgeber org.freedesktop.Platform org.freedesktop.Platform 25.08
install -Dm755 target/release/werkbank-lizenz "$fb/files/bin/werkbank-lizenz"
install -Dm755 target/release/werkbank-lizenz-gui "$fb/files/bin/werkbank-lizenz-gui"
install -Dm644 "$stage/privat.key" "$stage/oeffentlich.key" -t "$fb/files/share/werkbank-lizenzgeber"
flatpak build-finish "$fb" --command=werkbank-lizenz --share=network --filesystem=home
flatpak build-export target/flatpak-lizenzgeber-repo "$fb"
flatpak build-bundle target/flatpak-lizenzgeber-repo "$ziel/werkbank-lizenzgeber.flatpak" de.daehn.Werkbank.Lizenzgeber
rm -rf "$fb" target/flatpak-lizenzgeber-repo
fi
shopt -s nullglob
chmod 600 "$ziel"/*.deb "$ziel"/*.rpm "$ziel"/*.flatpak
(cd "$ziel" && rm -f SHA256SUMS.txt && sha256sum *.deb *.rpm *.flatpak > SHA256SUMS.txt)
ls -l "$ziel"
