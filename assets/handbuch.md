# Handbuch der Werkbank

Die Werkbank ist ein Schreibwerkzeug für technische Bücher. Sie liest und schreibt gewöhnliche Markdown-Dateien; dein Buch bleibt ein Ordner voller Dateien, den du jederzeit auch mit anderen Programmen und mit Git bearbeiten kannst.

## Erste Schritte

Starte die Werkbank und öffne mit `Strg+O` den Ordner deines Buches. Das kann ein einzelnes Buch sein, eine Reihe mit Unterordnern wie `Band 1`, `Band 2` oder ein einzelner Bandordner. Die Band-Auswahl oben links in der Kopfleiste wechselt zwischen den Bänden; in der Mitte der Kopfleiste wählst du den Bereich (Schreiben, Bauen, Prüfen, Zahlen, Cover). Beim nächsten Start öffnet sich der zuletzt benutzte Ordner wieder.

Die Werkbank schreibt in deinen Ordner nur dann, wenn du etwas änderst. Alles, was sie selbst anlegt, liegt im Unterordner `.werkbank/` (Sicherungen, Statistik, Einstellungen des Bandes).

**Arbeite beim ersten Ausprobieren an einer Kopie deines Buches.**

## Schreiben

Links stehen in der Bibliothek die Teile deines Buches, daneben die Kapitel des gewählten Teils mit einer kurzen Vorschau, ihrem Bearbeitungsstand und dem Wortziel, in der Mitte der Text. Rechts zeigt der Inspektor Wortziel, Stand, Zahlen, Gliederung, Notizen, Querverweise und Hinweise zur Sprache. Die Schalter oben rechts blenden Seitenleisten, Vorschau und Inspektor ein und aus.

- **Speichern:** `Strg+S` oder der Knopf „● Speichern“ oben rechts, der erscheint, sobald es Ungespeichertes gibt. Außerdem speichert die Werkbank nach acht Sekunden Pause von selbst. Eine Datei wird nur geschrieben, wenn sich ihr Text wirklich geändert hat, und zwar atomar: Es entsteht nie eine halbe Datei.
- **Sicherungen:** Vor dem Überschreiben legt die Werkbank einen Stand in `.werkbank/snapshots/` ab. Mit `Strg+H` holst du ein Kapitel aus einem früheren Stand zurück. Der aktuelle Stand bleibt dabei ebenfalls erhalten.
- **Kapitel und Teile:** Neues Kapitel `Strg+Umschalt+N`, Umbenennen `F2`, Verschieben `Alt+Umschalt+↑/↓`. Ein **Rechtsklick** auf ein Kapitel oder einen Teil öffnet ein Menü mit Umbenennen, Verlauf, Verweis einfügen, Bearbeitungsstand, Verschieben und Löschen. Alles davon findest du auch in der Befehlspalette. Entfernte Teile landen im Papierkorb `.werkbank/papierkorb/`.
- **Suchen und Ersetzen:** `Strg+F` im Kapitel, `Strg+Umschalt+F` in allen Bänden des Buchs, `F3` zum nächsten Treffer. Oben wählst du den Umfang (Kapitel, ganzer Band, alle Bände), dazu Groß-/Kleinschreibung, ganzes Wort und reguläre Ausdrücke (`$1` im Ersatz setzt Gruppen ein). Die Trefferliste zeigt jede Zeile vorher und nachher. „Alle ersetzen“ nennt zuerst die Anzahl und führt erst beim zweiten Klick aus. Die alte Fassung bleibt als nummerierte Sicherung erhalten.
- **Verweise:** Mit `@kap:kennung` verweist du auf ein Kapitel. Fehlt das Ziel, erscheint der Verweis rot im Inspektor. Die Befehlspalette fügt Verweise für dich ein.
- **Schreibmaschinen-Modus:** In den Einstellungen einschaltbar; die Cursorzeile bleibt in der Fenstermitte.
- **Fokus-Modus:** `F11` blendet alles außer dem Text aus.
- **Markdown bleibt leise:** Auszeichnungen wie `##`, `**`, Backticks, Listen- und Zitatzeichen sowie Link-Adressen erscheinen blass, damit der Text im Vordergrund steht.

## Details, Tempo und Stil

Oben im Inspektor wählst du eine von vier Ansichten:

- **Details:** Wortziel (groß, mit Regler), Bearbeitungsstand (ein Klick auf Entwurf, In Arbeit, Überarbeitet oder Fertig), Textzahlen, Querverweise.
- **Tempo:** Fortschritt zum Wortziel, **Schreibzeit und Tempo in diesem Kapitel** (netto, Pausen über fünf Minuten zählen nicht), geschätzte Zeit bis zum Ziel, Lesedauer und der **Satzrhythmus** als Säulendiagramm.
- **Stil:** Ein Wert von 0 bis 100, **wie lebendig oder steril der Text klingt**: Satz- und Absatzrhythmus, Füllformeln, Verbindungswörter, Wortschatz, Satzanfänge, persönliche Stimme, Gedankenstriche, Dreierreihen. Schwache Merkmale kommen mit einem Rat zum Überarbeiten, gefundene Füllformeln lassen sich per Klick suchen. **Das ist kein KI-Detektor**, es gibt kein verlässliches Verfahren dafür; der Wert zeigt nur, wie abwechslungsreich und persönlich geschrieben wird.
- **Text:** Gliederung, Notizen, Rechtschreibung, Sprachprüfung.

## Die Befehlspalette

`Strg+K` oder „Suchen“ oben rechts öffnet ein Suchfeld für alle Funktionen, Kapitel, Überschriften, Bände und Einstellungsseiten (z. B. „Einstellungen: Editor“). Tippe ein paar Buchstaben, wähle mit den Pfeiltasten und bestätige mit Enter. Hinter jedem Befehl steht sein Tastenkürzel.

## Tastatur statt Maus

- `F6` (und in Fenstern `Tab`) wandert über alle Knöpfe, Schalter, Regler und Felder; der grüne Rahmen zeigt, wo du bist. `Enter` oder die Leertaste betätigt, die Pfeiltasten stellen Regler und Listen ein.
- Alle Kürzel lassen sich ändern: Einstellungen (`Strg+,` oder `F10`), Seite „Tastenkürzel“, **Ändern**, gewünschte Tasten drücken. Hat ein anderer Befehl das Kürzel schon, verliert er es.

## Vorschau

`Strg+P` zeigt das Kapitel **so, wie es gesetzt wird**: Markdown → Pandoc → Typst → Seiten (wie die Bauskripte), mit B5-Format, Libertinus und farbigem Code. Sie erneuert sich kurz nach dem Tippen. Ohne Pandoc setzt der eingebaute Übersetzer. Ob Pandoc und Ghostscript installiert sind, siehst du unter Einstellungen → Werkzeuge; das Symbol ⓘ erklärt die Installation für Windows, macOS, Debian/Ubuntu, Fedora und Arch. „Text“ schaltet auf die schnelle Text-Vorschau um: Sie zeigt Überschriften, Listen, Tabellen und Bilder (mit Bildunterschrift, Pfade relativ zum Bandordner) und folgt beim Schreiben dem Cursor. Codeblöcke haben in beiden Vorschauen und im nativen Satz Zeilennummern, je Block ab 1. Den Pandoc-Weg geht die Satzvorschau nur bei Bänden mit einer `pdf-header.typ`; alle anderen setzt der eingebaute Satz.

## Sprache prüfen

Unbekannte Wörter markiert die Werkbank sofort rot und unterschlängelt, mit einem Hunspell-Wörterbuch auf deinem Rechner (ohne Internet). Code, Verweise, Adressen und HTML-Tags prüft sie nicht. **Fährst du mit der Maus über eine markierte Stelle**, erklärt eine Blase, was nicht stimmt, mit Vorschlag. Ein **Rechtsklick** auf eine markierte Stelle öffnet ein kleines Menü: oben die Korrekturvorschläge (ein Klick ersetzt), darunter **Merken** (das Wort gilt im ganzen Band als richtig) und **Ignorieren** (bis zum nächsten Start nicht mehr markiert). Bei Hinweisen der Sprachprüfung kannst du die Stelle ignorieren oder die Regel für den Band ausschalten.

Für Grammatik und Stil nutzt die Werkbank **LanguageTool**, das du selbst betreibst (Anleitung: Einstellungen → Prüfung, Symbol ⓘ). Code, Verweise und Adressen werden dabei nicht geprüft. `Strg+L` prüft das Kapitel; die Hinweise erscheinen orange im Text und in den Details.

## Prüfen

Der Bereich **Prüfen** übersetzt jedes Rust-Beispiel deines Buches mit `rustc` und prüft Dart- und Flutter-Beispiele mit `dart analyze` (dafür muss Flutter oder Dart installiert sein). Außerdem prüft er das ganze Buch auf Sprache und zeigt Codezeilen, die für den Druck zu lang sind. Dart-Ausschnitte bettet die Werkbank passend ein; Blöcke ohne eigene Importe bekommen die Importe ihres Kapitels. Verweist ein Ausschnitt nur auf Code, der woanders steht, zählt er als Ausschnitt statt als Fehler. Absichtlich fehlerhafte Beispiele kennzeichnest du mit ` ```rust,compile_fail ` bzw. ` ```dart,compile_fail `, nicht prüfbare mit `ignore`.

## Zahlen

Netto-Wörter (neu geschrieben minus gestrichen), Schreibzeit, Tempo, Tagespensum, Serie aufeinanderfolgender Schreibtage, Aktivität der letzten Wochen, Prognose bis zu den Wortzielen, Fortschritt je Teil und Textqualität (Lesbarkeit, Satzlängen, Floskeln).

## Bauen

Die Werkbank startet die Bauskripte deines Bandes (`build.sh`, `alle.py`, `build.py` oder `scripts/build_book.py`) und zeigt ihre Ausgabe. Vorher speichert sie alle Änderungen. Für Druckdateien erzeugt sie am Ende mit Ghostscript eine **PDF/X-3-Datei** (Graustufen, Weg von `tolino-print.py`). Zusätzlich kann sie das Buch **selbst setzen** (Typst und EPUB, ohne Skripte): Befehl „Buch nativ setzen“ in der Befehlspalette.

**Seitenzahlen im nativen Satz** (Bereich Bauen, Karte „Nativer Satz“):

- *Stellung:* außen abwechselnd (gerade Seiten links, ungerade rechts, wie bei Romanen und Sachbüchern) oder immer mittig.
- *Zählung:* Titelei mit römischen Ziffern und der Haupttext beginnt auf einer rechten Seite mit **1** (üblich), oder durchgehend arabisch ab dem Titelblatt.
- Teile (auf Wunsch auch Kapitel) beginnen auf rechten Seiten; die dafür nötigen Leerseiten tragen keine Zahl. Schmutztitel, Titelblatt und Impressum bleiben ohne Zahl, außer du schaltest das ein.
- *Schmutztitel, Leerseiten, Teiltitel:* Im Druck beginnt das Buch mit dem Schmutztitel (Seite 1, rechts), dahinter steht eine Leerseite. Es folgen Titelblatt (rechts) und Impressum (links), dann das Inhaltsverzeichnis. Jeder Teil bekommt auf Wunsch eine eigene Titelseite auf einer rechten Seite mit leerer Rückseite. Leerseiten zählen mit, bleiben aber ohne Zahl. Auf Wunsch endet das Buch auf einer geraden Seite, nötigenfalls mit einer Leerseite am Ende.
- Jede Textseite trägt eine Seitenzahl, auch Kapitelanfänge. Das Inhaltsverzeichnis übernimmt immer die gedruckten Zahlen; wird der Satz nicht stabil, bricht die Werkbank mit einer Meldung ab, statt falsche Zahlen auszugeben.

Die Maße der Portale stehen in `.werkbank/portale.json`; die Voreinstellungen musst du selbst gegen die Portale prüfen.

## Cover

Der Bereich **Cover** bietet eine Vorlage mit Titel, Untertitel, Bandzeile, Autor, **einer Farbe je Text**, Hintergrundbild, Rückentext und Barcode-Fläche. Das **Hintergrundbild** läuft über den ganzen Umschlag oder nur über die Vorderseite und wird zugeschnitten skaliert (nie verzerrt); die Werkbank nennt die nötige Pixelgröße je Portal (bei 300 dpi) und bewertet dein Bild. Hinter **Rückentext und Rückseitentext** steht auf Wunsch ein Kasten mit wählbarer Farbe und Deckkraft. Die **Vorschau zeigt Farbe und Graustufen** nebeneinander, auf Wunsch mit 16 E-Ink-Stufen. Die Rückenbreite folgt aus der Seitenzahl. Exportiert wird als PNG und PDF in `.werkbank/cover/`.

**Ein Cover, eine Stelle.** Den Umschlag setzt die Werkbank immer gleich, in der Vorschau, im Export und beim Bauen (`tolino/`, `kdp/`): Die **Vorderseite** ist das fertige `cover.png` des Bandes, solange du kein Hintergrundbild gewählt hast, sonst das Bild mit den Titeltexten aus dem Cover-Bereich. **Farben, Kästen, Schriftgröße und die Barcode-Fläche** kommen aus dem Cover-Bereich. Der **Text der Rückseite** steht nur in `klappentext.md` im Bandordner:

```markdown
Programmieren lernen an der Werkbank          ← Dachzeile (Absatz vor der ersten Überschrift)

# Keine Theorie auf Vorrat.<br/>Sofort loslegen.   ← Schlagzeile (<br/> = Zeilenumbruch)

Absätze mit **fett**, *kursiv* und `Code`.

## Deine 5 Praxis-Projekte:                    ← Überschrift mit Aufzählung = Kasten
- **Projekt 1:** Beschreibung
- **Projekt 2:** Beschreibung

## Über den Autor                              ← Überschrift mit Text = Infoblock unten
Jahrgang 1971 …

---
flutter-werkstatt.de                           ← Fußzeile: Zeilen nach dem letzten ---
Alle Begleitprogramme zum Download · {seiten} Seiten
```

Alles außer der Schlagzeile ist freiwillig. Ist der Text zu lang, wird die Schrift bis 7,5 pt kleiner; passt er dann noch nicht, meldet die Werkbank, um wie viele Millimeter er zu hoch ist. Ein langer **Shop-Text** für Buchbeschreibungen kann daneben in `klappentext-shop.md` stehen; er landet als `KLAPPENTEXT.md` in den Verlagsordnern. Alte Umschlagtexte aus `buchbau.yaml` überträgt `cargo run -p werkbank-satz --example klappentext_uebernehmen -- <Buchordner> --machen` in die `klappentext.md` jedes Bandes (der bisherige Text wird als `klappentext-shop.md` gesichert).

**Barcode und ISBN.** Die ISBN trägst du direkt im Cover-Bereich ein (Feld unter „Freifläche für den Barcode“; die Werkbank prüft sie beim Tippen), im Dialog „Buchdaten bearbeiten“ (Knopf „Buchdaten …“ im Cover- und im Bauen-Bereich, auch in der Befehlspalette `Strg+K`) oder in `band.yaml`. Beim Bauen meldet die Werkbank, wenn der Tolino-Umschlag ohne gültige ISBN entsteht. **Tolino:** Bei gültiger ISBN-13 setzt die Werkbank den EAN-13-Barcode selbst auf die Rückseite (33,4 × 18,1 mm wie von Tolino Media verlangt, 16 mm Abstand zu Rücken und Kante; Tolino verlangt mindestens 15 mm); ohne gültige ISBN bleibt ein weißes Feld dieser Größe für den Barcode aus dem Tolino-Dashboard. **KDP:** Es bleibt ein weißes Feld (50,8 × 30,5 mm, 8 mm Abstand), in das KDP den Barcode selbst druckt, auch bei eigener ISBN; das Feld nicht beschriften. Ohne Haken bei „Freifläche für den Barcode“ entfällt der Bereich.

## Dateien im Bandordner

| Datei | Inhalt |
|---|---|
| `.werkbank/snapshots/` | frühere Stände der Dateien |
| `.werkbank/papierkorb/` | entfernte Teile |
| `.werkbank/kapitel.json` | Status, Wortziel, Notizen |
| `.werkbank/statistik.json` | Tageswerte |
| `.werkbank/sprache.json` | gemerkte Wörter, ausgeschaltete Regeln |
| `.werkbank/portale.json` | Portalmaße |
| `.werkbank/cover.json`, `.werkbank/cover/` | Cover-Eingaben und Ausgaben |
| `band.yaml` | Buchdaten (eine fremde `band.yaml` wird nie überschrieben) |

## Lizenz

Die Werkbank läuft 28 Tage lang mit vollem Funktionsumfang. Danach trägst du unter Einstellungen (Zahnrad oder `Strg+,`) auf der Seite „Allgemein“ die E-Mail-Adresse deines Kaufs und den Lizenzschlüssel ein (oder wählst eine Lizenzdatei). Ohne Schlüssel bleibt nur „Speichern und beenden“, deine Dateien bleiben unverändert. Unter „Über die Werkbank“ siehst du den Lizenzstatus und die Lizenzen der verwendeten Bausteine.

## Hilfe bei Problemen

- **Nichts wird markiert:** Ist unter Einstellungen → Prüfung ein Wörterbuch geladen? Auf Linux hilft das Paket `hunspell-de-de`.
- **LanguageTool nicht erreichbar:** Einstellungen → Prüfung → Verbindung testen; die Anleitung steht hinter dem Symbol ⓘ.
- **Eine Datei hat sich unerwartet geändert:** Öffne den Verlauf (`Strg+H`) oder sieh in `.werkbank/snapshots/` nach.
