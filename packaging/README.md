# Pakete

Die Werkbank ist eine einzelne ausführbare Datei (`werkbank-app`); Schriften, Wörterbücher und LanguageTool werden nicht mitgeliefert. Anwendungskennung: `de.daehn.Werkbank` (Desktop-Datei, Symbole, Metadaten und Flatpak heißen danach).

Das **Symbol** (`assets/icons/werkbank.svg`: aufgeschlagenes Buch auf einer Werkbank in Grün) ist die einzige Quelle. `packaging/icons-erzeugen.sh` erzeugt daraus alle Größen für Linux (16–512 px und SVG), `werkbank.ico` (Windows) und `werkbank.icns` (macOS); `crates/werkbank-app/build.rs` bettet das ico unter Windows in die .exe ein.

| System | Weg | Stand |
|---|---|---|
| Debian/Ubuntu (.deb) | `packaging/pakete-bauen.sh` (cargo-deb), Metadaten in `crates/werkbank-app/Cargo.toml` | **gebaut** (20 MB), Inhalt mit `dpkg-deb` geprüft, nicht installiert |
| Fedora/openSUSE (.rpm) | `packaging/pakete-bauen.sh` (cargo-generate-rpm) | **gebaut** (22 MB), Dateiliste und Abhängigkeiten gelesen, nicht installiert |
| Flatpak | `packaging/flatpak/bauen.sh`, Manifest `packaging/de.daehn.Werkbank.yaml` | Manifest geprüft, **nicht gebaut** (flatpak-builder fehlt) |
| macOS | `cargo install cargo-bundle && cargo bundle --release -p werkbank-app` erzeugt eine `.app` mit dem icns-Symbol | **nicht gebaut, nicht signiert/notarisiert** |
| Windows | `cargo build --release -p werkbank-app` liefert `werkbank-app.exe` mit Symbol | **kein Installer, nicht signiert, nicht gebaut** |

`.github/workflows/release.yml` baut alle Systeme bei einem Tag `v*` (inklusive Flatpak). Er wurde nicht ausgeführt. Signieren: [SIGNIEREN.md](SIGNIEREN.md).

Zur Laufzeit gebraucht (je nach Funktion): Java 17+ und LanguageTool (Sprachprüfung), Hunspell-Wörterbücher (Rechtschreibung), `rustc` (Code-Prüfer), Python, Typst und Pandoc (Bauskripte). Die Pakete empfehlen/schlagen sie vor, erzwingen sie nicht.

**Hinweis zur Verträglichkeit:** Die Binärdatei hängt von der glibc des Rechners ab, auf dem sie gebaut wurde (hier Ubuntu 26.04, glibc 2.43). Für ältere Systeme (Debian stable, Fedora, LTS-Versionen) in einem Container mit älterer glibc bauen; das Flatpak ist davon unabhängig.
