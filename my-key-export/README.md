# Lizenzgeber & Privater Schlüssel: Übertragung auf ein anderes System

Diese Übersicht zeigt dir, wo die Schlüssel und Daten für die Lizenzerzeugung der Werkbank liegen und wie du sie sicher auf einen anderen Rechner überträgst.

---

## 1. Speicherorte

Der Lizenzgeber (`werkbank-lizenz` CLI und GUI) speichert das Schlüsselpaar und das Kundenregister in einem zentralen Benutzerordner außerhalb des Quellcode-Repositorys:

* **Linux:**  
  `~/.local/share/werkbank-lizenzgeber/`  
  *(wird über `$WERKBANK_LIZENZGEBER` oder `$XDG_DATA_HOME/werkbank-lizenzgeber` gesteuert)*
* **Windows:**  
  `%APPDATA%\werkbank-lizenzgeber\` (z. B. `C:\Users\<Benutzer>\AppData\Roaming\werkbank-lizenzgeber\`)
* **macOS:**  
  `~/.local/share/werkbank-lizenzgeber/` *(bzw. `$WERKBANK_LIZENZGEBER`)*

---

## 2. Enthaltene Dateien

Im Ordner `werkbank-lizenzgeber` liegen vier wichtige Bestandteile:

| Datei / Ordner | Zweck | Vertraulichkeit |
|---|---|---|
| `privat.key` | Privater Ed25519-Schlüssel zur Signatur neuer Lizenzen | **Streng geheim** (Rechte `600`) |
| `oeffentlich.key` | Öffentlicher Schlüssel (identisch mit `assets/lizenz-oeffentlich.key`) | Öffentlich |
| `register.jsonl` | Register aller bisher an Kunden ausgegebenen Lizenzen | Vertraulich (Kundendaten) |
| `sicherung/` | Automatische fortlaufende Backups des Kundenregisters | Vertraulich |

---

## 3. Schlüssel und Register sichern (Altes System)

Du kannst den gesamten Ordner in ein verschlüsseltes Archiv oder auf einen gesicherten Datenträger (z. B. USB-Stick im Safe / Passwortmanager) exportieren:

```bash
# Archiv mit Zeitstempel im Home-Verzeichnis anlegen
tar -czvf ~/werkbank-lizenzgeber-backup-$(date +%Y%m%d).tar.gz -C ~/.local/share werkbank-lizenzgeber
```

---

## 4. Auf neuem System einspielen

### Unter Linux

1. Zielordner anlegen und Archiv entpacken (oder Dateien direkt kopieren):

```bash
mkdir -p ~/.local/share
tar -xzvf werkbank-lizenzgeber-backup-*.tar.gz -C ~/.local/share/
```

2. Dateirechte einschränken (Schutz vor unberechtigtem Zugriff anderer lokaler Benutzer):

```bash
chmod 700 ~/.local/share/werkbank-lizenzgeber
chmod 600 ~/.local/share/werkbank-lizenzgeber/privat.key
```

### Unter Windows

Kopiere den Ordner `werkbank-lizenzgeber` nach:
```text
%APPDATA%\werkbank-lizenzgeber
```

---

## 5. Funktionstest auf dem neuen System

Prüfe nach dem Kopieren, ob der Lizenzgeber Schlüssel und Kundenbestand sofort erkennt:

```bash
# Im Projektordner der Werkbank:
cargo run -p werkbank-lizenz -- liste
```

Oder falls du die GUI verwendest:
```bash
cargo run -p werkbank-lizenz-gui
```

Wenn das Register und der Status des Schlüsselpaars grün angezeigt werden, ist die Übertragung vollständig abgeschlossen.

