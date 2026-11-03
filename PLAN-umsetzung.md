# Umsetzungsplan Werkbank

Stand der Planung: 2026-10-29. Reihenfolge nach Risiko für das Manuskript, dann nach Nutzen.
Status: ☐ offen · ◐ teilweise · ☑ umgesetzt (siehe Ende der Datei, wird laufend nachgeführt).

## Befund vor Beginn
- Echte Bände wurden **nicht verlustfrei** eingelesen: Der Teil-Parser erkannte `## ` auch in Codeblöcken
  (`anhang-loesungen.md`) und verlor abschließende Leerzeilen (5 weitere Dateien). Speichern wäre ohne Reparatur zerstörerisch gewesen.
- Status, Wortziel und Notizen der Kapitel wurden nirgends gespeichert.

## Phase 1 – Verlustfreie Ein-/Ausgabe und Speichern (Kern + App)
1. Markdown-Hilfen (Codezäune erkennen, Überschriften, Codeblöcke) – `kern::markdown`.
2. Parser neu: Teil = Präambel + Kapitel, Zusammensetzen ergibt **byte-identisch** die Datei. Test gegen echte Bände.
3. Kapitel-Metadaten (Status, Ziel, Notizen) in `.werkbank/kapitel.json`.
4. Speichern: nur geänderte Teile, Snapshot (gedrosselt, begrenzt), atomar. Demo-Buch schreibt nie.
5. Struktur: Kapitel anlegen/umbenennen/löschen/verschieben, Teil anlegen/umbenennen/verschieben/entfernen (Papierkorb), `reihenfolge.txt` nachführen.
6. App: Autosave, Beenden-Dialog, Verlauf (Snapshot-Wiederherstellung je Kapitel), Eingabe- und Bestätigungsdialoge.

## Phase 2 – Schreibwerkzeuge
Suchen/Ersetzen (Kapitel, Buch), Gliederung, Notizen, `@kap:`-Prüfung und Einfügen. (Typewriter-Scroll: mit Iceds `text_editor` nicht umsetzbar, keine Scroll-API.)

## Phase 3 – Statistik (M3)
Sitzungen, Netto-Wörter, Tempo, Tagespensum, Streak, Heatmap, Prognose, Textqualität (Flesch-Amstad, Satzlängen, Füllwörter/Floskeln). Speicherung in `.werkbank/statistik.json` (kein SQLite nötig).

## Phase 4 – Prüfen (M7)
Code-Prüfer (rustc je Rust-Block, Hintergrund), Buchprüflauf mit LanguageTool, Portalregeln als Daten (`.werkbank/portale.json`).

## Phase 5 – Bauen (M2)
Bauskripte erkennen/konfigurieren, asynchron starten, Ausgabe live, Abbrechen, Ergebnisdateien.

## Phase 6 – Einstellungen und `band.yaml`
Schriftgröße, Spaltenbreite, Theme hell/dunkel, Autosave; Metadaten-Editor für `band.yaml`.

## Phase 7 – Rechtschreibung im Text
Offline-Wörterbuch (Hunspell-Format) und Markierung von Fehlern im Editor über eigenen Highlighter (nur Farbe, keine Wellenlinie möglich).

## Phase 8 – Cover (M5)
Vorlage mit Eingabefeldern, SVG-Vorschau, Export SVG/PNG in Portalformaten.

## Phase 9 – Pakete (M8)
Desktop-Datei, Symbol, Release-Workflow für Linux/Windows/macOS (nicht ausführbar getestet).

## Nicht umsetzbar / bewusst offen
Typewriter-Scroll, Wellenlinien, Tab-Fokus auf Knöpfe (Iced 0.14), native Satz-Pipeline in Rust (M6) und Vertriebsreife (M9).

## Fortschritt
- ☑ Phase 1 (Kern): verlustfreier Parser (Test gegen echte Bände: 19/19, 9/9 Dateien byte-identisch), Sidecar `kapitel.json`, atomares Speichern mit gedrosselten Snapshots, Struktur-Operationen, `reihenfolge.txt`, Papierkorb, `band.yaml`.
- ☑ Phase 1 (App): Änderungsverfolgung je Teil, Autosave (8 s Pause), Beenden-Dialog, Verlauf/Wiederherstellung, Eingabe- und Bestätigungsdialoge, Palette-Befehle.
- ☑ Phase 2: Suchen/Ersetzen (Kapitel, Buch, F3), Gliederung, Notizen je Kapitel, `@kap:`-Prüfung (rot bei fehlendem Ziel) und Einfügen per Palette.
- ☑ Phase 3: Statistik mit Netto-Wörtern, Sitzungen, Tempo, Streak, Heatmap, Prognose, Teil-Ampel, Buchzahlen, Flesch/Satzlängen/Floskeln.
- ☑ Phase 4: Code-Prüfer (rustc, 4 Threads, Band 1: 555 Beispiele in 17 s), Buch-Sprachprüfung, Druckregeln; Portalregeln als Daten.
- ☑ Phase 5: Bauen über vorhandene Skripte mit Live-Ausgabe, Abbrechen, Ergebnisdateien, Werkzeugprüfung.
- ☑ Phase 6: Einstellungen (Autosave, Tagespensum, hell/dunkel, Schriftgröße, Spaltenbreite), Buchdaten-Dialog (fremde `band.yaml` wird nie überschrieben).
- ☑ Phase 7: Hunspell offline (spellbook, de_DE aus dem System), eigener Markierer im Editor (Überschriften, Code, Zitat, Verweise, Fett, Rechtschreibung rot, LanguageTool orange), Fachwörter, „Wort merken“.
- ☑ Phase 8: Cover-Designer (Vorlage, Farben, Bild, Rücken aus Seitenzahl, Barcode-Fläche, Portalprüfung, Export PNG/SVG; als Bild geprüft).
- ◐ Phase 9: Desktop-Datei, Symbol, Fenstertitel/-symbol, Release-Profil, `cargo-deb`/`cargo-bundle`-Metadaten, Release-Workflow – **nichts davon gebaut oder ausgeführt**.

## Endstand und Grenzen
Getestet: 63 automatische Tests (Kern 45, App 18), Round-Trip aller Dateien von Band 1/Flutter Band 1/2 byte-identisch, Speichern auf einer Kopie von Band 1 (nur die geänderte Datei ändert sich, ein Snapshot), Code-Prüfer auf Band 1 (555 Beispiele, 17 s), Cover als Bild.
Nicht getestet: die Oberfläche selbst (kein Bildschirmfoto möglich), echter LanguageTool-Server, Windows/macOS, Pakete.
Nicht umgesetzt: Typewriter-Scroll und Wellenlinien (Iced-Grenzen), Tab-Fokus auf Knöpfe, native Satz-Pipeline in Rust (M6), Vertriebsreife (M9), eigene Tastenkürzel, PDF-Export des Covers.

## Zweite Runde (Iced-Grenzen, Kürzel, Satz, Vertrieb)
- ◐ Eigener Editor (`editor.rs`, Kopie von Iceds `text_editor` mit Erweiterungen): Schreibmaschinen-Modus und Wellenlinien unter Rechtschreib- und Sprachhinweisen. Kompiliert, **visuell ungeprüft**.
- ◐ Tastaturfokus (`ui/fokus.rs`): Hülle macht Knöpfe, Schalter, Regler und Auswahllisten per Tab (in Fenstern) und F6 (überall) erreichbar, mit grünem Fokusrahmen; Enter/Leertaste klickt, Pfeiltasten stellen Regler/Listen. Kompiliert, **nicht bedient**.
- ☑ Eigene Tastenkürzel (`kuerzel.rs`): 27 Befehle mit Standardkürzel, Änderung per Aufnahme in den Einstellungen, Konflikte entziehen das Kürzel dem alten Befehl, Palette zeigt die gültigen Kürzel. Fest bleiben Esc, Tab, F6, Enter und die Pfeile in der Palette.
- ☑ Nativer Satz (`crates/werkbank-satz`): Markdown → Typst → PDF (Druck je Portal in Schwarzweiß, farbiges Lese-PDF B5) und EPUB 3. Auf Band 1 geprüft: 497 Seiten in 5,3 s (Release), Seiten als Bild angesehen (Titel, Inhaltsverzeichnis, Kapitel, Code). **Kein Golden-Test gegen die Python-Skripte**; Satzbild und Umbrüche weichen ab.
- ☑ Cover als PDF (Vorderseite und Umschlag in exakten Millimetern, über Typst).
- ◐ Vertriebsreife: Lizenzprüfung mit Ed25519 (Werkzeug `werkbank-lizenz`, Ende-zu-Ende getestet, öffentlicher Schlüssel noch **nicht eingerichtet**), „Über“-Fenster mit allen 550 Fremdlizenzen (keine GPL/AGPL), `deny.toml`, Handbuch im Programm (`F1`), Signier-Schritte im Release-Workflow (**ohne Zertifikate, nie ausgeführt**). Seite, Verkauf und Support nicht angefasst.
- ☑ Seitenzahlen im nativen Satz: Stellung (Mitte / außen abwechselnd), Zählung (Titelei römisch + Haupttext ab 1 rechts / durchgehend), rechte Startseiten mit Leerseiten ohne Zahl, Zahl auf jeder Textseite. Inhaltsverzeichnis an Band 1 geprüft: 122/122 Einträge = gedruckte Seitenzahl; 15 Tests am fertigen PDF (`pdftotext`). Nicht stabiler Satz bricht ab.
- ☑ Schmutztitel, Leerseiten, Teiltitel: Schmutztitel (S. 1 rechts) → Leerseite → Titelblatt → Impressum → Inhalt, alle ohne Zahl bis zum Inhaltsverzeichnis; Teiltitelseiten (rechts, leere Rückseite, ohne Zahl) und gerade Gesamtseitenzahl mit Endleerseite sind neu. Band 1: 518 Seiten, 11 Teiltitel alle rechts mit leerer Rückseite, Seitenzahl gerade; 19 Tests.
- ☑ Programmsymbol (Buch auf Werkbank, SVG als Quelle; PNG 16–512, ico, icns) und Pakete: `.deb` und `.rpm` gebaut und im Inhalt geprüft, Flatpak-Manifest (`de.daehn.Werkbank`) und AppStream-Metainfo geschrieben (**Flatpak nicht gebaut**, flatpak-builder fehlt; Metainfo hat noch keine Homepage/Bildschirmfotos).
- ◐ Dritte Runde: neutrale, ergonomische Palette (Papier/Dämmerung, Kontraste im Test geprüft), Editorschrift Libertinus, Zeilenabstand; Stilprofil (menschlich/steril) im Kern mit Test an Band 1 (Median 94, Fehlalarm-frei, maschineller Mustertext ≤ 40); Satzvorschau über Pandoc → Typst (wie die Bauskripte), Werkzeugprüfung mit Installationshinweisen in den Einstellungen, PDF/X-3 über Ghostscript am Ende des Satzlaufs.
- ☑ Ergonomie: neutrale Flächen ohne Farbstich, ein zurückhaltender Stahlblau-Akzent, Kontraste im Test (Haupttext ≥ 10:1 auf Schreibfläche), Editor in Libertinus Serif (eingebettet) mit 1,55 Zeilenabstand und 700 px Spalte, eigenes Iced-Thema; Reiter im Inspektor (Details, Tempo, Stil, Text).
- ☑ Tempo je Kapitel (Schreibzeit, Wörter/Std., Restzeit, Lesedauer, Satzrhythmus) und Stilprofil „lebendig/steril“ (9 Merkmale, Tipps, Füllformeln; kein KI-Detektor). An Band 1: 99 Kapitel, Median 94, kein Kapitel unter 72.
- ☑ Satzvorschau über Pandoc → Typst (wie `build.py`, mit Verweis- und Nummernauflösung der Skripte), Rückfall auf den eingebauten Satz; Werkzeugprüfung (Pandoc, Ghostscript) mit Installationshinweisen; PDF/X-3 über Ghostscript (Test: erzeugt gültige Kennung).
- ☑ Cover: Bild über Umschlag oder Vorderseite (zugeschnitten skaliert, Ausrichtung, Abdunkeln), Bildgrößen je Portal mit Bewertung, Farbe je Text, Kästen mit Deckkraft hinter Rücken- und Rückseitentext, Vorschau in Farbe und Graustufen/E-Ink.
