# Werkbank – Das Schreibwerkzeug der Rust-Werkstatt

Die **Werkbank** ist ein Desktop-Werkzeug für Autoren technischer Bücher: schreiben, prüfen, Cover gestalten und die vorhandenen Bauskripte starten, ohne ein Terminal zu öffnen. Rust mit **Iced 0.14**.

Stand und Umsetzungsplan: [PLAN-umsetzung.md](PLAN-umsetzung.md). Konzept: [KONZEPT-werkbank.md](KONZEPT-werkbank.md).

## Aufbau

- **`crates/werkbank-kern`**: Datenmodell und Logik ohne Oberfläche: verlustfreier Markdown-Parser, Speichern mit Snapshots, Struktur, Suche, Statistik, Code-Prüfer, Cover-Vorlage, Portalregeln, Sprachprüfung (Markdown → LanguageTool).
- **`crates/werkbank-app`**: die Anwendung (Iced, `cosmic-text`, `wgpu`).

## Bücher öffnen

`Strg+O` (oder „Öffnen …“): Ordner mit einem Buch, einer Reihe (Unterordner `Band 1`, `Band_2`, …) oder einem einzelnen Band. Zwischen Bänden wechselt die Auswahlliste oben links. Der letzte Ordner wird gemerkt.

## Schreiben

- Dreispalter: Bibliothek, Blattliste, Editor, dazu Inspektor (Status, Wortziel, Metriken, Gliederung, Notizen, Verweise, Hinweise).
- **Speichern ist verlustfrei**: Eine Datei wird nur geschrieben, wenn sich ihr Text ändert, atomar und mit Snapshot des alten Stands in `.werkbank/snapshots/`. Autosave nach 8 s Pause, Warnung beim Beenden. **Verlauf** (`Strg+H`) holt ein Kapitel aus einem früheren Stand zurück.
- Kapitel und Teile anlegen, umbenennen (`F2`), verschieben (`Alt+Umschalt+↑/↓`), löschen; `reihenfolge.txt` wird nachgeführt, entfernte Teile landen im Papierkorb `.werkbank/papierkorb/`.
- **Suchen und Ersetzen** im Kapitel (`Strg+F`) und im Buch (`Strg+Umschalt+F`), `F3` weiter.
- Markierungen im Editor: Überschriften, Code, Zitate, `@kap:`-Verweise, Fettdruck; unbekannte Wörter rot (Hunspell, offline), LanguageTool-Hinweise orange.
- Schreibmaschinen-Modus (Cursorzeile bleibt in der Mitte) und Wellenlinien unter Rechtschreib- und Sprachhinweisen; dafür enthält die App eine erweiterte Kopie von Iceds `text_editor` (`editor.rs`).
- Live-Vorschau (`Strg+P`), Fokus-Modus (`F11`), Seitenleisten (`Strg+B`), Details (`Strg+I`).

## Prüfen

- **Rechtschreibung offline** mit dem Hunspell-Wörterbuch des Systems (`de_DE`), Fachwörter und eigene Wörter pro Band.
- **LanguageTool** (`Strg+L`, automatisch nach Schreibpause): Hinweise im Inspektor, Klick markiert, Vorschlag ersetzt. Einstellungen und Installationsanleitung (Linux, Windows, macOS) über das Zahnrad (`Strg+,` oder `F10`).
- Bereich **Prüfen**: Code-Prüfer (`rustc` je Rust-Beispiel, parallel; Dart/Flutter mit einem `dart analyze`-Durchgang über ein gemeinsames Analyseprojekt, Ausschnitte in drei Einbettungen, Kapitelimporte, Pakete per `pub get`; unvollständige Ausschnitte und externe Crates/Pakete werden gesondert gezählt; `rust,compile_fail` bzw. `dart,compile_fail` kennzeichnet absichtliche Fehler), Sprachprüfung über das ganze Buch, Druckregeln (Codezeilen ≤ 70 Zeichen).

## Zahlen

Netto-Wörter (neu minus gestrichen), Schreibzeit und Sitzungen, Tagespensum, Tempo, Streak, Heatmap, Prognose, Fortschritt je Teil, Buchzahlen, Flesch-Amstad, Satzlängen und Floskeln. Gespeichert in `.werkbank/statistik.json`.

## Bauen und Cover

- **Bauen**: erkennt `build.sh`, `alle.py`, `build.py` oder `scripts/build_book.py` (oder `.werkbank/bauen.json`), startet sie im Hintergrund mit Live-Ausgabe und Abbrechen und listet die erzeugten Dateien. Portalmaße stehen als Daten in `.werkbank/portale.json` (Voreinstellungen, bitte gegen die Portale prüfen).
- **Vorlagen**: zwei fest mitgelieferte Gestaltungen, „Klassisch – Buchdruck“ (ruhig, Serifen, Papiertöne, Streifenkästen) und „Modern – Technikbuch“ (Akzentfarbe Petrol, abgerundete Kästen mit Rahmen). Sie nutzen nur eingebaute Schriften (Libertinus Serif, Fira Sans, DejaVu Sans Mono) und sehen auf jedem Rechner gleich aus. Eine Vorlage ersetzt die aktuelle Gestaltung und lässt sich danach frei ändern.
- **Gestaltung ohne Code**: Bauen → „Gestaltung der Elemente“: je Element (Fließtext, Kapitel-/Abschnittsüberschriften, Zitat, Codeblock, Code im Text, Tabelle, Kästen) Schrift, Größe, Text-, Hintergrund- und Randfarbe, Randstärke und -seite, Innenabstand, Ecken, fett/kursiv, mit Musterseite (Farbe oder Graustufen). Gespeichert in `.werkbank/satzstil.json`; im Druck werden Farben zu Grau.
- **Stil und Kästen**: Pandoc-Divs (`::: info` … `:::`; eingebaut: info, hinweis, tipp, merke, wichtig, achtung, warnung, beispiel, weitere Namen erscheinen mit ihrem Namen als Titel) werden zu Kästen. Das Erscheinungsbild (Schrift, Ränder, Überschriften, Code- und Kästen, Tabellen) ändert `.werkbank/satz-stil.typ` (Typst, Knopf „Stil bearbeiten …“ legt eine Vorlage an); es gilt für Druck und Web-PDF.
- **Nativer Satz**: „Buch nativ setzen“ erzeugt ohne Skripte je Portal ein Schwarzweiß-Druck-PDF, ein farbiges Lese-PDF und ein EPUB 3 in `.werkbank/satz/` (Typst als Bibliothek, `epub-builder`). Das Ergebnis ist nicht mit der Skript-Ausgabe identisch. Seitenzahlen: außen abwechselnd oder mittig, Titelei römisch mit Haupttext ab 1 oder durchgehend, rechte Startseiten für Teile (und Kapitel) mit unnummerierten Leerseiten; das Inhaltsverzeichnis trägt immer die gedruckten Zahlen (an Band 1 geprüft: 122 von 122 Einträgen stimmen).
- **Cover** (`Strg+5`): Vorlage mit Titel, Untertitel, Bandzeile, Autor, Farben, Hintergrundbild, Rücken aus der Seitenzahl und Barcode-Fläche; Vorschau; Export als PNG und PDF in `.werkbank/cover/`. Der Umschlag wird überall gleich gesetzt (Typst): Vorderseite aus `cover.png` oder Designer, Rückseite aus `klappentext.md`, Farben aus dem Cover-Bereich, Tolino-Barcode aus der ISBN, KDP-Freifläche (siehe Handbuch).

## Dateien der Werkbank im Bandordner

`.werkbank/` enthält `snapshots/`, `papierkorb/`, `kapitel.json` (Status, Wortziel, Notizen), `statistik.json`, `sprache.json` (Ausnahmen), `portale.json`, `cover.json`, `cover/`. Buchdaten (Titel, Autor, ISBN) stehen in `band.yaml`, eine fremde `band.yaml` wird nie überschrieben (dann `.werkbank/metadaten.yaml`).

## Tastatur

Alles ist per Tastatur erreichbar: `F6` (in Fenstern `Tab`) wandert über Knöpfe, Schalter, Regler und Felder, `Enter`/Leertaste betätigt, Pfeile stellen Regler und Listen. **Alle Kürzel sind änderbar** (Einstellungen → Tastenkürzel). Standard:

`F1` Handbuch · `Strg+K` Befehle · `Strg+S` Speichern · `Strg+O` Ordner · `Strg+F`/`Strg+Umschalt+F` Suche · `Strg+H` Verlauf · `Strg+L` Sprachprüfung · `Strg+,`/`F10` Einstellungen · `Strg+1…5` Bereiche · `F2` Umbenennen · `Alt+↑/↓` Kapitel wechseln.

## Starten und prüfen

```bash
cargo run -p werkbank-app --release
cargo test --workspace
cargo clippy --workspace --all-targets
```

Hilfsprogramme im Kern: `cargo run -p werkbank-kern --example roundtrip -- <Bandordner>` (verlustfreies Einlesen prüfen), `--example codecheck -- <Bandordner>` (Beispiele kompilieren).

Pakete: siehe [packaging/README.md](packaging/README.md).

## Lizenz und Vertrieb

Die Werkbank ist kommerzielle Software ([LIZENZVEREINBARUNG.md](LIZENZVEREINBARUNG.md)): 28 Tage Testzeit mit vollem Funktionsumfang, danach ein Lizenzschlüssel zur E-Mail-Adresse des Kaufs (Ed25519, Prüfung offline). Verkäuferseite: [LIZENZ-vertrieb.md](LIZENZ-vertrieb.md). Alle Fremdbausteine stehen unter permissiven Lizenzen (Liste im Programm unter „Über die Werkbank“, erzeugt mit `python3 packaging/lizenzen.py`, prüfbar mit `cargo deny check`). Signieren und Prüfsummen: [packaging/SIGNIEREN.md](packaging/SIGNIEREN.md).
