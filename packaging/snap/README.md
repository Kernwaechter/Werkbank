# Snap-Anleitung: JZ Werkbank und Lizenzgeber

Zwei Snaps, bewusst getrennt:

| | JZ Werkbank | Werkbank Lizenzgeber |
|---|---|---|
| Snap-Name | `jz-werkbank` | `werkbank-lizenzgeber` |
| Manifest | `snap/snapcraft.yaml` | `packaging/snap/lizenzgeber/snapcraft.yaml` |
| Bauen mit | `packaging/snap/bauen.sh` | `packaging/snap/lizenzgeber-bauen.sh` |
| Ziel | Snap Store | **nur lokal**, enthält den privaten Schlüssel |
| Ergebnis | `target/snap/jz-werkbank_1.5.0_amd64.snap` | `~/Lizenzgeber-Pakete/werkbank-lizenzgeber_1.5.0_amd64.snap` (Rechte 600) |

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
sudo snap install --dangerous target/snap/jz-werkbank_1.5.0_amd64.snap
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
sudo snap install --dangerous ~/Lizenzgeber-Pakete/werkbank-lizenzgeber_1.5.0_amd64.snap
werkbank-lizenzgeber        # Oberfläche
werkbank-lizenzgeber.cli    # Kommandozeile
```

- **Nie in den Snap Store hochladen und nie weitergeben.** Der private Schlüssel liegt im Paket und ist für lokale Benutzer lesbar, wie bei deb und rpm.
- Register und Schlüssel liegen im Snap unter `~/snap/werkbank-lizenzgeber/common/lizenzgeber` und überleben Aktualisierungen. Beim ersten Start kopiert der Snap das Schlüsselpaar dorthin. Ein bestehendes Kundenregister (`register.jsonl`) musst du von `~/.local/share/werkbank-lizenzgeber/` dorthin kopieren, weil der Snap versteckte Ordner im Home nicht lesen darf.
- Der Webhook-Mailbefehl (`STRIPE_GEHEIMNIS`, Mailprogramm) braucht ein Programm, das im Snap nicht liegt. Für die Weboberfläche und Schlüssel ausstellen spielt das keine Rolle.

## 4. In den Snap Store (nur JZ Werkbank), Schritt für Schritt

Gemeint ist der **Snap Store** von Canonical (snapcraft.io), der Software-Laden von Ubuntu und vielen anderen Linux-Systemen. Für Apples App Store (macOS) gilt diese Anleitung nicht.

### Vorher

1. **Testen:** Den Snap lokal installieren und die Prüfliste aus Abschnitt 2 durcharbeiten (`sudo snap install --dangerous …`).
2. **Rechtliches klären:** Die Lizenzvereinbarung (`LIZENZVEREINBARUNG.md`, trägt noch den Vermerk „Entwurf, vor dem Verkauf rechtlich prüfen lassen“) liegt im Snap. Dazu die Quellcode-Zusage in `DRITTANBIETER.md` (Java, Pandoc, Wörterbücher), die du hier verbindlich eingehst.
3. **Kauf und Testzeit:** Der Store verkauft keine Programme für dich. Der Snap ist kostenlos installierbar, die Werkbank läuft 28 Tage zur Probe, danach trägst du den Lizenzschlüssel aus deinem Kauf (Stripe, Webseite) ein. Das gehört in die Beschreibung im Store, damit niemand überrascht wird.

### Einmalig einrichten

4. **Konto:** Auf https://snapcraft.io mit einem Ubuntu-One-Konto anmelden (kostenlos) und die Entwicklervereinbarung annehmen.
5. **Im Terminal anmelden:** `snapcraft login` (öffnet den Browser; die Anmeldung gilt eine Weile).
6. **Namen reservieren:**
   ```bash
   snapcraft register jz-werkbank
   ```
   Der Name muss mit `name:` in `snap/snapcraft.yaml` übereinstimmen. Er lässt sich später **nicht umbenennen**. Ist er vergeben, einen anderen wählen und `name:` sowie den Dateinamen in `snap/gui/` anpassen. Mit `--private` registrierst du ihn zuerst nur für dich sichtbar: `snapcraft register jz-werkbank --private`. Der Titel „JZ Werkbank“ ist vom Namen unabhängig.

### Hochladen und freigeben

7. **Aktuellen Snap bauen** (Terminal, nicht aus einem Skript heraus, siehe Abschnitt 2):
   ```bash
   packaging/snap/bauen.sh        # erzeugt target/snap/jz-werkbank_1.5.0_amd64.snap
   ```
8. **Hochladen auf den Kanal `edge`:**
   ```bash
   snapcraft upload --release=edge target/snap/jz-werkbank_1.5.0_amd64.snap
   ```
   Der Store prüft den Snap automatisch (Struktur, Schnittstellen, Sicherheitsregeln). Das Ergebnis steht im Terminal und im Dashboard. Zwei Ausgänge:
   - *bestanden*: der Snap ist auf `edge` verfügbar.
   - *manuelle Prüfung nötig*: Bei Schnittstellen wie `removable-media` oder `network-bind` und bei neuen Namen kann das Review-Team Fragen stellen. Antworte im Dashboard bzw. im Forum (https://forum.snapcraft.io, Kategorie „store-requests“). Im Zweifel die Schnittstelle in `snap/snapcraft.yaml` weglassen und neu hochladen.
9. **Aus dem Store testen** (am besten auf einem frischen Rechner oder einer sauberen Sitzung):
   ```bash
   sudo snap install jz-werkbank --edge
   jz-werkbank
   ```
   Die Prüfliste aus Abschnitt 2 noch einmal durchgehen.
10. **Store-Eintrag ausfüllen** (https://snapcraft.io/jz-werkbank/listing, Menü „Listing“):
    - mindestens **ein Bildschirmfoto** (PNG oder JPG, am besten 3 bis 5: Schreiben, Prüfen, Cover, Bauen)
    - Kategorie (zum Beispiel „Productivity“ oder „Education“)
    - Lizenz **Proprietary**, Webseite `https://werkbank.jurzel-apps.de`, Kontakt und Support-Adresse
    - Beschreibung (steht schon im Manifest, hier kannst du sie erweitern, auch mit dem Hinweis auf Testzeit und Lizenzschlüssel)
11. **Version freigeben:** Die Revisionsnummer zeigt `snapcraft status jz-werkbank`. Dann nach oben schieben:
    ```bash
    snapcraft release jz-werkbank <revision> candidate
    snapcraft release jz-werkbank <revision> stable
    ```
    Normale Nutzer bekommen nur `stable`; `edge` und `candidate` sind für Tests.
12. **Öffentlich machen:** Im Dashboard unter „Settings“ den Snap von „privat“ bzw. „nicht gelistet“ auf **öffentlich** stellen. Erst danach erscheint er in der Suche und auf snapcraft.io/jz-werkbank.
13. **Prüfen, ob es klappt:** `snap info jz-werkbank` auf einem fremden Rechner. Der Installationsbefehl `sudo snap install jz-werkbank` kommt dann auf die Webseite (Abschnitt „Download“) und in die Anleitung.

*Hinweis:* Die Namen der Menüpunkte im Dashboard (Listing, Settings, Releases) kann Canonical ändern; die Befehle (`register`, `upload`, `release`, `status`) sind stabil. Quelle: https://documentation.ubuntu.com/snapcraft/ (Abschnitte „Publish“ und „Releasing“).

### Wenn etwas schiefgeht

| Meldung oder Lage | Was tun |
|---|---|
| `Registrierung abgelehnt: Name bereits vergeben` | anderen Namen wählen (Schritt 6) |
| Upload bleibt bei „needs manual review“ | im Forum nachfragen, Schnittstelle prüfen (Schritt 8) |
| `cannot find snap` nach dem Hochladen | der Snap ist noch nicht auf einem Kanal veröffentlicht (Schritt 11) oder noch privat (Schritt 12) |
| Programm startet im Store-Snap nicht, lokal schon | `snap run --shell jz-werkbank` und `sudo journalctl -k | grep -i denied` (fehlende Schnittstelle) |
| Anmeldung abgelaufen | `snapcraft login` wiederholen |

### Vor jeder neuen Version

Siehe Abschnitt 5. Kurz: Version erhöhen, bauen, mit `snapcraft upload --release=edge` hochladen, testen, dann mit `snapcraft release` auf `candidate` und `stable` schieben.

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
