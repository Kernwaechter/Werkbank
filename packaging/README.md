# Pakete

Die Werkbank ist eine einzelne ausführbare Datei (`werkbank-app`); Schriften, Wörterbücher und LanguageTool werden nicht mitgeliefert. Anwendungskennung: `de.daehn.Werkbank` (Desktop-Datei, Symbole, Metadaten und Flatpak heißen danach).

Das **Symbol** (`assets/icons/werkbank.svg`: aufgeschlagenes Buch auf einer Werkbank in Grün) ist die einzige Quelle. `packaging/icons-erzeugen.sh` erzeugt daraus alle Größen für Linux (16–512 px und SVG), `werkbank.ico` (Windows) und `werkbank.icns` (macOS); `crates/werkbank-app/build.rs` bettet das ico unter Windows in die .exe ein.

| System | Weg | Stand |
|---|---|---|
| Debian/Ubuntu (.deb) | `packaging/pakete-bauen.sh` (cargo-deb), Metadaten in `crates/werkbank-app/Cargo.toml` | **gebaut** (20 MB), Inhalt mit `dpkg-deb` geprüft, nicht installiert |
| Fedora/openSUSE (.rpm) | `packaging/pakete-bauen.sh` (cargo-generate-rpm) | **gebaut** (22 MB), Dateiliste und Abhängigkeiten gelesen, nicht installiert |
| Flatpak | `packaging/flatpak/bauen.sh`, Manifest `packaging/de.daehn.Werkbank.yaml` | Manifest geprüft, **nicht gebaut** (flatpak-builder fehlt) |
| macOS | `cargo install cargo-bundle && cargo bundle --release -p werkbank-app` erzeugt eine `.app` mit dem icns-Symbol | **nicht gebaut, nicht signiert/notarisiert** |
| Windows | `packaging\windows\windows-bauen.ps1` (unter Windows, MSVC) oder `windows-bauen.sh` (Cross-Build unter Linux) liefert `werkbank-app.exe` mit Symbol und ein ZIP; `werkbank.iss` (Inno Setup) den Installer | Release-Build läuft unter Windows 11; Installer und Signatur **nicht geprüft** |

`.github/workflows/release.yml` baut alle Systeme bei einem Tag `v*` (inklusive Flatpak). Er wurde nicht ausgeführt. Signieren: [SIGNIEREN.md](SIGNIEREN.md).

**Windows: was eingebaut ist und was nicht.** Im Programm selbst stecken (ohne Python, Bash, Pandoc, Typst-Programm): der Satz der Bücher (Typst-Bibliothek mit Schriften Libertinus Serif und DejaVu Sans Mono), Druck-PDFs je Portal in Schwarzweiß, Lese-/Web-PDF in Farbe, EPUB 3, Cover (PNG/SVG/PDF), Kästen (`::: info` … `:::`), Code-Prüfung für Rust und Dart (nur diese Compiler müssen installiert sein). Der Stil lässt sich pro Band in `.werkbank/satz-stil.typ` ändern (Knopf „Stil bearbeiten“ im Bereich Bauen). Nicht eingebaut, aber **Ghostscript** für die PDF/X-3-Graustufen-Druckdatei (AGPL, deshalb nicht mitgeliefert; nicht in winget: Installer von ghostscript.com, `choco install ghostscript` oder `scoop install ghostscript`); per winget-Knopf (Einstellungen → Externe Werkzeuge) nachinstallierbar: **Pandoc** nur für die Kapitelvorschau im Stil der Skripte, Rust bzw. Dart/Flutter für den Code-Prüfer. Vorhandene Bauskripte (`build.sh`, `alle.py` …) laufen unter Windows, wenn Python (`py`/`python`) und Git für Windows (`bash`) installiert sind; sie sind optional. Rechtschreibung: Hunspell-Wörterbücher im Ordner `woerterbuecher` neben der .exe (oder `%APPDATA%\werkbank\woerterbuecher`, LibreOffice-Ordner); legt man sie vor dem Bauen nach `packaging\windows\woerterbuecher`, nimmt sie der Installer mit.

Zur Laufzeit gebraucht (je nach Funktion): Java 17+ und LanguageTool (Sprachprüfung), Hunspell-Wörterbücher (Rechtschreibung), `rustc` (Code-Prüfer), Python, Typst und Pandoc (Bauskripte). Die Pakete empfehlen/schlagen sie vor, erzwingen sie nicht.

**Hinweis zur Verträglichkeit:** Die Binärdatei hängt von der glibc des Rechners ab, auf dem sie gebaut wurde (hier Ubuntu 26.04, glibc 2.43). Für ältere Systeme (Debian stable, Fedora, LTS-Versionen) in einem Container mit älterer glibc bauen; das Flatpak ist davon unabhängig.
