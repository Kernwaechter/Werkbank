# Snap-Anleitung: JZ Werkbank und Lizenzgeber

Zwei Snaps, bewusst getrennt:

| | JZ Werkbank | Werkbank Lizenzgeber |
|---|---|---|
| Snap-Name | `jz-werkbank` | `werkbank-lizenzgeber` |
| Manifest | `snap/snapcraft.yaml` | `packaging/snap/lizenzgeber/snapcraft.yaml` |
| Bauen mit | `packaging/snap/bauen.sh` | `packaging/snap/lizenzgeber-bauen.sh` |
| Ziel | Snap Store | **nur lokal**, enthält den privaten Schlüssel |
| Ergebnis | `target/snap/jz-werkbank_1.3.0_amd64.snap` | `~/Lizenzgeber-Pakete/werkbank-lizenzgeber_1.3.0_amd64.snap` (Rechte 600) |

Stand 7.11.2026: Beide Snaps **gebaut** (App 666 MB, Lizenzgeber 17 MB) und der Inhalt mit `unsquashfs -l` geprüft. **Installiert und gestartet wurde noch keiner**: die Prüfliste in Abschnitt 2 steht aus.

## 1. Einmal vorbereiten

Beide Snaps bauen in einem Container (Basis `core24`, Ubuntu 24.04), nicht auf dem Rechner selbst.

```bash
sudo snap install snapcraft --classic   # ist installiert (9.2.1)
sudo snap install lxd
sudo lxd init --auto
sudo usermod -aG lxd "$USER"            # danach neu anmelden
```

Der erste Build lädt das Container-Image, Rust 1.99.1, LanguageTool (250 MB), Typst und die Ubuntu-Pakete. Das dauert 15 bis 30 Minuten und braucht Netz. Folgebuilds sind schneller.

## 2. JZ Werkbank bauen und testen

```bash
packaging/snap/bauen.sh
sudo snap install --dangerous target/snap/jz-werkbank_1.3.0_amd64.snap
jz-werkbank
```

Die Skripte müssen in einem **Terminal** laufen: ohne Terminal bricht snapcraft nach etwa einer Minute ohne Meldung mit Exit-Code 120 ab. Für Läufe aus Skripten: `script -qefc packaging/snap/bauen.sh build.log`.

`--dangerous` ist nötig, solange der Snap nicht aus dem Store kommt. Das Skript kopiert nur versionierte Dateien in einen Arbeitsordner (neue, noch nicht committete Dateien zählen mit, `target/` und Schlüssel nicht).

**Prüfliste vor dem Hochladen**

1. Das Fenster öffnet sich (Wayland und X11), Dateidialoge funktionieren.
2. Einen Buchordner im Home öffnen und speichern. Einstellungen liegen unter `~/snap/jz-werkbank/current/.config/werkbank/`.
3. Sprachprüfung: Der LanguageTool-Server startet von selbst (Einstellungen: Ordner `/snap/jz-werkbank/current/opt/languagetool`, Java `java`). Der erste Start braucht einige Sekunden.
4. Rechtschreibung: Hunspell-Wörterbuch `de-DE` wird gefunden.
5. Einstellungen → Externe Werkzeuge → „Erneut prüfen“: Pandoc, Typst und rustc erscheinen mit Version, Ghostscript, Dart und Flutter fehlen (gewollt).
6. Buch setzen: Druck-PDF, Lese-PDF, EPUB, Cover.
7. Code-Prüfer mit einem Rust-Beispiel.
8. Eine erzeugte Datei „im Programm öffnen“ (`xdg-open` über das Portal). Das ist der unsicherste Punkt.
9. Lizenzschlüssel einlösen.

Fehler suchen:

```bash
snap run --shell jz-werkbank        # Shell in der Sandbox
snap logs jz-werkbank
sudo journalctl -k | grep -i denied # AppArmor-Verbote (fehlende plugs)
```

Falls etwas in der Sandbox verboten ist, fehlt meist ein `plug` unter `apps:` in `snap/snapcraft.yaml`.

## 3. Lizenzgeber bauen (lokal)

Voraussetzung: Schlüsselpaar in `~/.local/share/werkbank-lizenzgeber/` (oder `$WERKBANK_LIZENZGEBER`). Das Skript bricht ab, wenn `oeffentlich.key` nicht zu `assets/lizenz-oeffentlich.key` passt.

```bash
packaging/snap/lizenzgeber-bauen.sh
sudo snap install --dangerous ~/Lizenzgeber-Pakete/werkbank-lizenzgeber_1.3.0_amd64.snap
werkbank-lizenzgeber        # Oberfläche
werkbank-lizenzgeber.cli    # Kommandozeile
```

- **Nie in den Snap Store hochladen und nie weitergeben.** Der private Schlüssel liegt im Paket und ist für lokale Benutzer lesbar, wie bei deb und rpm.
- Register und Schlüssel liegen im Snap unter `~/snap/werkbank-lizenzgeber/common/lizenzgeber` und überleben Aktualisierungen. Beim ersten Start kopiert der Snap das Schlüsselpaar dorthin. Ein bestehendes Kundenregister (`register.jsonl`) musst du von `~/.local/share/werkbank-lizenzgeber/` dorthin kopieren, weil der Snap versteckte Ordner im Home nicht lesen darf.
- Der Webhook-Mailbefehl (`STRIPE_GEHEIMNIS`, Mailprogramm) braucht ein Programm, das im Snap nicht liegt. Für die Weboberfläche und Schlüssel ausstellen spielt das keine Rolle.

## 4. In den Snap Store (nur JZ Werkbank)

Einmalig:

1. Konto auf https://snapcraft.io anlegen, `snapcraft login`.
2. Namen reservieren: `snapcraft register jz-werkbank`. Der Name kann vergeben sein, dann in `snap/snapcraft.yaml` ändern.
3. Im Dashboard Bildschirmfotos, Kategorie und Beschreibung ergänzen. Die Webseite (`werkbank.jurzel-apps.de`) und der Kontakt stehen schon im Manifest.

Hochladen:

```bash
snapcraft upload --release=edge target/snap/jz-werkbank_1.3.0_amd64.snap
```

Erst `edge`, testen mit `sudo snap install jz-werkbank --edge`, dann `candidate`, dann `stable`. Neue Namen und proprietäre Software kann der Store zur manuellen Prüfung zurückhalten.

**Vor dem Veröffentlichen klären**

- **Lizenzvereinbarung:** `LIZENZVEREINBARUNG.md` trägt noch den Vermerk „Entwurf, vor dem Verkauf rechtlich prüfen lassen“. Sie liegt im Snap.
- **GPL-Quellcode:** `packaging/snap/DRITTANBIETER.md` sagt zu, den Quellcode von Java, Pandoc und den Wörterbüchern auf Anfrage an volker@daehn.de zu schicken. Das gilt für dich verbindlich.
- **Ghostscript:** nicht enthalten (AGPL). Wer die PDF/X-3-Druckdatei braucht, installiert es selbst; im Snap ist das nur eingeschränkt möglich.

## 5. Neue Version veröffentlichen

1. Version in `Cargo.toml` (`[workspace.package]`) und in beiden `snapcraft.yaml` (`version:`) erhöhen.
2. `packaging/snap/bauen.sh`, testen, hochladen.
3. LanguageTool oder Typst aktualisieren: In `snap/snapcraft.yaml` die Version und die SHA-256-Summe im jeweiligen Teil ändern (`sha256sum datei`). Die Summe sorgt dafür, dass ein veränderter Download den Build abbricht.

## 6. Was im Snap steckt

- **Dabei:** LanguageTool 6.6, OpenJDK 21, Pandoc, Typst 0.15.1, Python 3, rustc 1.99.1, xdg-utils, Hunspell (de, en-US, en-GB, fr, es). Lizenzen: `DRITTANBIETER.md`.
- **Nicht dabei:** Ghostscript, Dart, Flutter, Pandoc-Filter, Python-Module (außer der Standardbibliothek). Eigene Bauskripte, die weitere Module oder Programme brauchen, laufen im Snap nicht.
- **Größe:** Erwartet werden 400 bis 600 MB. Wer sie senken will, kann in `languagetool` ungenutzte Sprachen löschen oder `rustc` weglassen.
- **Sandbox:** `strict` mit `home`, `network`, `network-bind` (lokaler LanguageTool-Server), `removable-media` und der GNOME-Erweiterung (liefert Grafik, GTK-Dialoge, Portale). Nicht lesbar sind versteckte Ordner im Home (`~/.cargo`, `~/.config` …). Programme, die dort liegen (zum Beispiel ein eigenes Rust oder Flutter), sieht die Werkbank im Snap nicht.
- **Name in der Oberfläche:** Der Snap, die Desktop-Datei und der Store-Eintrag heißen „JZ Werkbank“. Fenstertitel, Handbuch, Deb/RPM/Flatpak und der Lizenzgeber heißen weiter „Werkbank“.
