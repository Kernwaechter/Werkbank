# Snap-Anleitung: JZ Werkbank

| | JZ Werkbank |
|---|---|
| Snap-Name | `jz-werkbank` |
| Manifest | `snap/snapcraft.yaml` |
| Bauen mit | `packaging/snap/bauen.sh` |
| Ziel | Snap Store |
| Ergebnis | `target/snap/jz-werkbank_1.5.0_amd64.snap` |

Stand 7.11.2026: Der Snap ist **gebaut** (666 MB in der alten Fassung; nach der Verschlankung am 8.11.2026 rund 50 MB) und der Inhalt mit `unsquashfs -l` geprüft. **Installiert und gestartet wurde er noch nicht**: die Prüfliste in Abschnitt 2 steht aus.

## 1. Einmal vorbereiten

Beide Snaps bauen in einem Container (Basis `core24`, Ubuntu 24.04), nicht auf dem Rechner selbst.

```bash
sudo snap install snapcraft --classic   # ist installiert (9.2.1)
sudo snap install lxd
sudo lxd init --auto
sudo usermod -aG lxd "$USER"            # danach neu anmelden
```

Der erste Build lädt das Container-Image, Rust 1.99.1 und die Ubuntu-Pakete. Das dauert 15 bis 30 Minuten und braucht Netz. Folgebuilds sind schneller.

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
3. Sprachprüfung: Der Snap enthält LanguageTool nicht. Java und LanguageTool auf dem Rechner installieren und den Server im Terminal starten (`java -cp languagetool-server.jar org.languagetool.server.HTTPServer --port 8081`). In den Einstellungen (Seite „Prüfung“) steht die Anleitung unter ⓘ; „Verbindung testen“ muss auf `http://localhost:8081` „Erreichbar“ zeigen.
4. Rechtschreibung: Hunspell-Wörterbuch `de-DE` wird gefunden.
5. Einstellungen → Werkzeuge → „Erneut prüfen“: Pandoc, Typst-Programm, Ghostscript, Rust, Dart und Flutter fehlen (gewollt), bei „Skripte“ stehen Bash und sh; oben steht der Snap-Hinweis.
6. Buch setzen: Druck-PDF, Lese-PDF, EPUB, Cover.
7. Skript-Prüfer mit einem Bash-Beispiel (der Rust- und Dart-Prüfer gehen im Snap nicht).
8. Eine erzeugte Datei „im Programm öffnen“ (`xdg-open` über das Portal). Das ist der unsicherste Punkt.
9. Hilfe → „Buy me a coffee“ öffnet die Spendenseite im Browser.

Fehler suchen:

```bash
snap run --shell jz-werkbank        # Shell in der Sandbox
snap logs jz-werkbank
sudo journalctl -k | grep -i denied # AppArmor-Verbote (fehlende plugs)
```

Falls etwas in der Sandbox verboten ist, fehlt meist ein `plug` unter `apps:` in `snap/snapcraft.yaml`.

## 3. In den Snap Store, Schritt für Schritt

Gemeint ist der **Snap Store** von Canonical (snapcraft.io), der Software-Laden von Ubuntu und vielen anderen Linux-Systemen. Für Apples App Store (macOS) gilt diese Anleitung nicht.

### Vorher

1. **Testen:** Den Snap lokal installieren und die Prüfliste aus Abschnitt 2 durcharbeiten (`sudo snap install --dangerous …`).
2. **Rechtliches klären:** Der MIT-Lizenztext (`LICENSE`) liegt im Snap. Dazu die Quellcode-Zusage in `DRITTANBIETER.md` (Hunspell-Wörterbücher und Abhängigkeiten), die du hier verbindlich eingehst.
3. **Spenden:** Der Snap ist kostenlos. Der Hinweis auf „Buy me a coffee“ steht in der Beschreibung im Store.

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
   - *manuelle Prüfung nötig*: Bei Schnittstellen und bei neuen Namen kann das Review-Team Fragen stellen. Antworte im Dashboard bzw. im Forum (https://forum.snapcraft.io, Kategorie „store-requests“). Im Zweifel die Schnittstelle in `snap/snapcraft.yaml` weglassen und neu hochladen.
9. **Aus dem Store testen** (am besten auf einem frischen Rechner oder einer sauberen Sitzung):
   ```bash
   sudo snap install jz-werkbank --edge
   jz-werkbank
   ```
   Die Prüfliste aus Abschnitt 2 noch einmal durchgehen.
10. **Store-Eintrag ausfüllen** (https://snapcraft.io/jz-werkbank/listing, Menü „Listing“):
    - mindestens **ein Bildschirmfoto** (PNG oder JPG, am besten 3 bis 5: Schreiben, Prüfen, Cover, Bauen)
    - Kategorie (zum Beispiel „Productivity“ oder „Education“)
    - Lizenz **MIT**, Webseite `https://werkbank.jurzel-apps.de`, Kontakt und Support-Adresse
    - Beschreibung (steht schon im Manifest, hier kannst du sie erweitern, auch mit dem Hinweis auf die Spendenseite)
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

0. Bei geänderten Abhängigkeiten: `python3 packaging/lizenzen.py`, `python3 packaging/lizenztexte.py` und `cargo deny check licenses` ausführen.
1. Version in `Cargo.toml` (`[workspace.package]`) und in beiden `snapcraft.yaml` (`version:`) erhöhen.
2. `packaging/snap/bauen.sh`, testen, hochladen.
3. Rust-Version für den Bau ändern: In `snap/snapcraft.yaml` die Version bei `RUST=` ändern.

## 6. Was im Snap steckt

- **Dabei:** xdg-utils und Hunspell (de, en-US, en-GB, fr, es). Lizenzen: `DRITTANBIETER.md`.
- **Nicht dabei, weil man sie selbst installieren kann:** LanguageTool und Java, Pandoc, das Typst-Programm, Python, Ghostscript, Rust, Dart und Flutter. Ein strikter Snap kann Programme des Systems nicht starten. Deshalb funktionieren diese im Snap auch nach eigener Installation nicht; nur LanguageTool geht, weil es als Server auf `localhost` läuft (Anleitung in der App). Der Buchsatz ist eingebaut. Wer den Rust-/Dart-Code-Prüfer, PDF/X-3, die Pandoc-Vorschau oder eigene Bauskripte braucht, nimmt DEB, RPM oder Flatpak. Der Skript-Prüfer funktioniert im Snap für Bash und sh (beide stammen aus der Snap-Basis); zsh, fish und PowerShell des Systems sieht der Snap nicht.
- **Größe:** Ohne LanguageTool, Java, Pandoc und rustc ist der Snap nur noch rund 50 MB groß (die erste Fassung hatte 666 MB).
- **Sandbox:** `strict` mit `home`, `network` und der GNOME-Erweiterung (liefert Grafik, GTK-Dialoge, Portale). Nicht lesbar sind versteckte Ordner im Home (`~/.cargo`, `~/.config` …) und Wechseldatenträger (`/media`, `/mnt`): `removable-media` ist bewusst nicht eingetragen. Bücher auf einem USB-Stick oder externen Laufwerk öffnet man über den Ordner-Dialog; die Portale geben der App dann Zugriff auf genau diesen Ordner. **Das muss vor der Veröffentlichung mit einem echten Stick getestet werden** (Ordner öffnen, bearbeiten, speichern, neu starten). Klappt es nicht, `- removable-media` unter `plugs:` wieder eintragen und im Forum um Auto-Verbindung bitten.
- **Name in der Oberfläche:** Der Snap, die Desktop-Datei und der Store-Eintrag heißen „JZ Werkbank“. Fenstertitel, Handbuch, Deb/RPM/Flatpak heißen weiter „Werkbank“.
