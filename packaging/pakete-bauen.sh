#!/usr/bin/env bash
# Baut Release-Binärdatei, Debian-Paket (.deb) und RPM (.rpm); Ergebnis in target/debian/ und target/generate-rpm/.
# Braucht: cargo-deb und cargo-generate-rpm (cargo install cargo-deb cargo-generate-rpm).
# Flatpak: packaging/flatpak/bauen.sh
set -euo pipefail
cd "$(dirname "$0")/.."
# Der Testlink von Stripe (…/test_…) darf nicht in ausgelieferte Pakete
if grep -q '/test_' assets/kauf-url.txt && [ "${ALLOW_TESTLINK:-}" != "1" ]; then
  echo "assets/kauf-url.txt enthält noch den Stripe-Testlink. Live-Link eintragen oder ALLOW_TESTLINK=1 setzen." >&2
  exit 1
fi
cargo build --release -p werkbank-app
cargo deb -p werkbank-app --no-build
cargo generate-rpm -p crates/werkbank-app
echo
ls -1 target/debian/*.deb target/generate-rpm/*.rpm
