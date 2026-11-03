# Werkbank: Plan für das Schreibwerkzeug der Rust-Werkstatt

Stand: Entwurf 0.4, 27. Oktober 2026. Arbeitstitel „Werkbank“.

**Entschieden:** Die Werkbank wird mit **Rust und Iced** umgesetzt. Sie wird zuerst dein eigenes Werkzeug, aber so gebaut, dass sich ein Vertrieb später offen hält (Abschnitt 11). Die Marke passt zum JURZEL Verlag.

## 1. Ziel in einem Satz

Ein Desktop-Programm in reinem Rust mit Iced, in dem du die Rust-Werkstatt-Bücher **schreibst, prüfst, setzt und veröffentlichst**, ohne ein Terminal zu öffnen: Editor im Ulysses-Stil, alle Build-Funktionen (Tolino und Amazon, eBook und Print), ein Cover-Designer, Rechtschreib- und Grammatikprüfung, Statistik zum Schreibtempo.

## 2. Was JZ Writer schon kann, und was hier anders ist

| | JZ Writer (Swift, iPad und Mac) | Werkbank (Rust + Iced: Linux, macOS, Windows) |
|---|---|---|
| Editor | Markdown, Fokus-Modus, Typewriter-Scroll | **übernehmen**, mit Ulysses-Dreispalter |
| Satz | Typst lokal, PDF in Sekunden | **übernehmen**, dazu die Prüfungen aus deinen Skripten |
| Gruppen, Kapitel | Gruppen, Drag und Drop, Corkboard (Pro) | Teile und Kapitel als Baum, Corkboard später |
| Zahlen | Wortziel, Streaks, Heatmap, Flesch-Amstad | **ausbauen**: Schreibtempo, Prognose, Buchstatistik |
| Versionen | Snapshots alle 5 Minuten | **übernehmen** |
| Export | PDF, DOCX, EPUB 3, RTF | **Verlagsfassungen**: Tolino, KDP, Lese-PDF, mit Prüfbericht |
| Zusatz | BibTeX, Zotero, Hochschul-Deckblatt, Widgets | **weglassen** (braucht ein Buch über Rust nicht) |
| Neu | – | **Cover-Designer**, **Sprachprüfung**, **Code-Prüfer** für die Beispiele |

## 3. Fünf Bereiche

**Schreiben.** Drei Spalten wie bei Ulysses: Bibliothek (Bände, Teile, Kapitel), Blattliste (Abschnitte mit Vorschau und Wortzahl), Editor. Markdown mit Hervorhebung, Fokus-Modus, Typewriter-Scroll, Schreibziel pro Kapitel. Querverweise `@kap:id` werden beim Tippen aufgelöst und bei Fehlern markiert. Optionale vierte Spalte: Vorschau (PDF oder EPUB).

**Bauen.** Ein Knopf pro Fassung, eine Fortschrittsanzeige, der Prüfbericht als Liste mit „OK, Warnung, Fehler“. Rücken, Barcode und ISBN werden in einem Dialog eingetragen. Ein Veröffentlichungs-Assistent führt durch die Portale (die Texte aus `portal-angaben.txt`, die Checkliste).

**Cover.** Siehe Abschnitt 7.

**Prüfen.** Zwei Prüfer. Der **Sprachprüfer** (Abschnitt 5) für Rechtschreibung, Grammatik, Stil. Der **Code-Prüfer**, der nur für dieses Buch gilt: Er zieht jeden Rust-Codeblock aus den Kapiteln, übersetzt ihn, startet ihn und vergleicht die Ausgabe mit dem Textblock darunter. Das ersetzt das, was bisher von Hand lief, und der Prüfstand pro Block (Rust-Version, Datum, System) wandert in die `Anmerkungen`, nicht in den Buchtext.

**Zahlen.** Siehe Abschnitt 6.

## 4. Technik

**Oberfläche: Iced (The Elm Architecture, 100 % Rust).**
Wir verzichten vollständig auf WebViews, eingebettete Browser-Engines und JavaScript. Iced zeichnet direkt über die Grafikkarte (wgpu) oder bei Bedarf per Software-Renderer (softbuffer / tiny-skia). Unter Linux entfällt damit die schwerfällige Abhängigkeit von WebKitGTK, unter Windows gibt es keinen Ärger mit WebView2-Runtimes. Das Ergebnis: schlanke Binärdateien, schneller Programmstart, geringer Speicherverbrauch und kein IPC-Kanal (keine JSON-Serialisierung von Tastaturanschlägen zwischen Rust und JavaScript).

**Der Editor in Iced:**
Der Editor setzt auf Iceds `text_editor`-Widget auf, das intern `cosmic-text` für präzises Textlayout und Schrift-Rendering nutzt. Markdown-Syntaxhervorhebung wird über die Highlighter-Schnittstelle von Iced angebunden.
- **Tippgefühl und Eingabe:** Tastaturkürzel, Cursorsteuerung, Umlaute und Textauswahl laufen nativ im Rust-Event-Loop.
- **Spezialfunktionen:** Typewriter-Scrolling (automatische Zentrierung der aktiven Zeile), gedimmte Absätze im Fokus-Modus und farbige Markierungen für Sprachfehler werden direkt im Widget bzw. über maßgeschneiderte Highlighter- und Layout-Erweiterungen gesteuert.
- **Kein Web-Overhead:** Alles bleibt im selben Rust-Speicherbereich. Jede Textänderung ist unmittelbar im `State` verfügbar, ohne asynchrones Hin- und Herschicken über einen internen Web-Port.

**Kern als Cargo-Workspace**, damit die Logik ohne Oberfläche testbar bleibt:

- `werkbank-kern`: Projekt, Band, Teile, Kapitel, Verweise, `band.yaml`
- `werkbank-bau`: die Verlagsfassungen (Abschnitt 8)
- `werkbank-pruefung`: Sprachprüfer und Code-Prüfer
- `werkbank-statistik`: Sitzungen, Tempo, Prognose
- `werkbank-cover`: Cover-Vorlagen und Ausgabe
- `werkbank-app`: Iced-Oberfläche (State, Message, Update, View, Subscription)

**Daten bleiben Dateien.** Markdown und `band.yaml` auf der Platte, im selben Aufbau wie jetzt. Dann bleiben Git, dein Editor von nebenan und die alten Skripte nutzbar. Die App legt nur `.werkbank/` dazu (Snapshots, Statistik, Prüfstände). Statistik in SQLite, Snapshots als gepackte Dateien. Eine Datenbank für den Text selbst: nein.

**Satz: Typst als Bibliothek** (die Crate `typst`), nicht als Fremdprogramm. Das spart die Installation und macht eine direkte Vorschau möglich. **Pandoc** bleibt zunächst ein externer Prozess und ist in einem späteren Schritt ablösbar.

## 5. Rechtschreibung und Grammatik

Der Plan ist zweistufig:

1. **Sofort und offline: Hunspell-Wörterbuch** (deutsch, Rechtschreibung). Das ist schnell genug für jede Tastatureingabe und verlässt nie den Rechner.
2. **Grammatik und Stil: LanguageTool.** Die Prüfung läuft absatzweise, erst nach einer Pause im Tippen, und nur für geänderte Absätze. Über Iced-Subscriptions wird das Ergebnis asynchron zurück an die Oberfläche gemeldet, ohne das Tippen zu blockieren.

**Wichtig:** Die öffentliche LanguageTool-Schnittstelle erlaubt höchstens 20 Anfragen pro Minute und IP, 75 KB Text pro Minute und 20 KB pro Anfrage, und sie verbietet **automatische** Anfragen ausdrücklich („set up your own instance“). Ein Editor, der mitprüft, ist automatisch. Das heißt: **LanguageTool selbst betreiben.** Das passt zu dir: Es ist freie Software (LGPL-2.1), braucht etwa 2 bis 4 GB Arbeitsspeicher, und du hast den IONOS-Server schon. Dann gibt es keine Grenzen, und der Text geht nicht an Dritte. In der App ist die Adresse einstellbar (auch `localhost` für einen lokalen Docker-Container).

Eigene Wörterbuchregeln gehören dazu: Fachwörter wie „Borrow Checker“, „Trait“, „Closure“, Programmnamen und Code im Fließtext werden nicht geprüft (Codeblöcke und `Inline-Code` sind ausgenommen).

## 6. Statistik und Schreibtempo

Stephen King beschreibt in *On Writing* ein festes Tagespensum (etwa 2.000 Wörter) und eine Erstfassung in ungefähr drei Monaten. Das Prinzip dahinter ist Regelmäßigkeit, und die kann ein Programm sichtbar machen:

- **Sitzungen:** erkannt an Pausen (zum Beispiel mehr als 5 Minuten). Daraus **Schreibzeit** und **Wörter pro Stunde**.
- **Netto statt brutto:** neue Wörter minus gestrichene. Beim Umschreiben ist das die ehrliche Zahl.
- **Tag, Woche, Monat, Streak, Heatmap.**
- **Prognose:** „Bei diesem Tempo ist Teil 7 am … fertig.“ Dazu der **Fortschritt pro Teil** (Entwurf, überarbeitet, geprüft, korrigiert) als Ampel.
- **Buchstatistik:** Wörter, Zeichen, Normseiten (1.800 Zeichen), Seiten im Druck, Kapitellängen im Vergleich, Codeanteil gegen Fließtext.
- **Textqualität:** Lesbarkeit (Flesch-Amstad, wie in JZ Writer), Satzlängen-Verteilung, Füllwörter, Wiederholungen („stell dir vor“, „darüber hinaus“ und andere Floskeln deiner Stilregeln als eigene Liste).

## 7. Cover-Designer

Start klein und nützlich: **Vorlage plus Eingabefelder**, kein freies Zeichenprogramm. Du wählst eine Vorlage (die heutige `cover.typ` ist die erste), tippst Titel, Untertitel, Bandzeile und Autor, wählst Farben und Bild und siehst die Vorderseite live (über Typst gerendert und direkt als Bild in Iced angezeigt). Daraus entstehen **alle Formate**: Vorderseite für das EPUB, Tolino-Cover (1600 Pixel breit), Amazon-Cover (1600 x 2560), Umschlag für Print mit Rücken (aus der Seitenzahl), Rückseite aus dem Klappentext, Barcode-Fläche. Die Prüfregeln der Portale (Mindestmaße, Beschnitt) laufen mit.

Später, wenn der Rest steht: eigene Ebenen, Textrahmen, Schriften wählen. Das ist ein eigenes großes Projekt und gehört nicht in die erste Fassung.

## 8. Wie kommen deine Skripte in die App?

Die Python-Skripte enthalten rund 150 KB Logik, vor allem die Prüfungen für Tolino und Amazon. Alles auf einmal neu zu schreiben, wäre das größte Risiko des Projekts. Darum **schrittweise**:

1. **Umhüllen:** Die App ruft die vorhandenen Skripte asynchron über Hintergrundprozesse auf und zeigt Fortschritt und Prüfbericht sauber in Iced an. Python und Pandoc bleiben Voraussetzung.
2. **Nachbauen mit Maßstab:** Stück für Stück in Rust neu schreiben (Verweise auflösen, Cheat-Sheet, EPUB, Print, Prüfungen). Jedes Stück wird gegen die Ausgabe des Skripts getestet: Seitenzahl, Dateien, Prüfbericht, Pixelvergleich beim Cover.
3. **Regeln als Daten:** Die Regeln der Portale (Seitengrenzen, Rückenformel, Mindestmaße) stehen in einer Konfigurationsdatei mit Quelle und Datum, nicht fest im Code. Portale ändern sich.

## 9. Meilensteine

| Nr. | Inhalt | Abnahme |
|---|---|---|
| M0 | Spike mit Iced: Fenster, `text_editor` mit Markdown-Hervorhebung, Projektordner öffnen, Datei speichern | Läuft flüssig unter Linux, macOS, Windows |
| M1 | Schreiben: Dreispalter in Iced (Bibliothek, Blattliste, Editor), Fokus-Modus, Wortzähler, Snapshots | Du schreibst ein ganzes Kapitel darin |
| M2 | Bauen (Umhüllung der Skripte): Knopf pro Fassung, Prüfbericht | Band 1 baut aus der App, Ergebnis gleich wie im Terminal |
| M3 | Statistik: Sitzungen, Tempo, Heatmap, Prognose | Zahlen stimmen mit einer Stichprobe von Hand |
| M4 | Sprachprüfung: Hunspell offline, eigenes LanguageTool per Hintergrund-Task | Markierungen im Text ohne Ruckeln beim Tippen |
| M5 | Cover-Designer, erste Vorlage, alle Formate | Cover von Band 2 entsteht daraus |
| M6 | Pipeline nativ in Rust, Skripte abgelöst | Ergebnis gleich wie die Skripte (Golden-Tests) |
| M7 | Code-Prüfer für die Buchbeispiele | Alle Blöcke eines Bandes werden geprüft |
| M8 | Politur, native Pakete für die drei Systeme | Direkt installierbar ohne WebKitGTK-Hürden |
| M9 | Vertriebsreife (nur bei bewusster Entscheidung nach M3): Lizenzprüfung, Signatur, Handbuch, Seite, Verkauf | Zahlender Fremdnutzer kann kaufen, installieren und veröffentlichen |

Die Reihenfolge ist verhandelbar. M0 zeigt direkt, wie sich Iceds `text_editor` beim Schreiben längerer Kapitel anfühlt und wie gut das Anpassen klappt.

## 10. Risiken

- **Editor-Ausbau in Iced:** `text_editor` liefert eine solide Basis mit `cosmic-text`. Spezielle Features wie Typewriter-Scroll (aktive Zeile immer vertikal zentriert halten) und wellenförmige Squiggles für Sprachfehler erfordern Anpassungen am Widget oder Highlighter. Das klären wir direkt im M0-Spike.
- **Pflegeaufwand der Portale:** Tolino und Amazon ändern Regeln. Gegenmittel: Regeln als Daten, Datum und Quelle dabei.
- **LanguageTool im Betrieb:** Ein Server will gewartet werden. Gegenmittel: Die App funktioniert auch ohne (nur Rechtschreibung offline).
- **Umfang:** „Ulysses plus Cover-Programm plus Verlagswerkzeug“ sind drei Produkte. Gegenmittel: strikte Meilensteine, wobei M1 bis M3 bereits ein vollwertiges Schreib- und Bauwerkzeug ergeben.
- **Paketierung und Signaturen:** macOS (Notarisierung), Windows (Signaturzertifikat), Linux (Flatpak). Bei Iced entfällt wenigstens das Mitschleppen von Browser-Bibliotheken.

## 11. Vertriebsoption: Was „vertreibbar“ von Anfang an kostet

Die Regel: **Produktfähig bauen, intern zuerst ausliefern.** Das kostet im Alltag wenig, wenn man es von Tag 1 beachtet, und viel, wenn man es später nachrüstet.

**Von Anfang an beachten:**

1. **Lizenzen sauber halten.** `cargo deny` im Projekt, eine Liste aller Fremdbausteine mit Lizenz. Die Kernbausteine sind unkritisch: Iced (MIT), cosmic-text (Apache-2.0 / MIT), Typst als Bibliothek (Apache-2.0), Libertinus (OFL), DejaVu (freie Lizenz), Ferris (CC0). Keine JavaScript-Bibliotheken, keine Web-Lizenzen.
2. **Nichts Festes im Code.** Titel, Reihe, Farben, Portalregeln, Texte stehen in Dateien (wie `band.yaml`), nicht im Programm. Dann ist es kein „Rust-Werkstatt-Programm“, sondern ein Buchprogramm, das die Rust-Werkstatt als erstes Projekt nutzt.
3. **Oberflächentexte in einer Sprachdatei.** Deutsch zuerst, Englisch später ohne Umbau.
4. **Projektformat mit Versionsnummer** (`werkbank.yaml` mit `format: 1`), damit spätere Versionen alte Projekte öffnen können.
5. **Tests und Bau auf allen drei Systemen** von M1 an (automatisierter GitHub-Actions-Ablauf).
6. **Kein Server, auf den die App angewiesen ist.** Das passt zu deinem Grundsatz bei JZ Writer („Texte verlassen das Rechnergehäuse nicht“) und erspart dir Betrieb, Datenschutz-Verträge und laufende Kosten.

**Zwei Dinge, die den Vertrieb wirklich verändern:**

- **Die Sprachprüfung.** Dein eigener LanguageTool-Server auf dem IONOS-Server ist für dich passend, für fremde Kunden aber ungeeignet: Ihre Texte gingen an deinen Server (Datenschutzvertrag, Betrieb, Kosten). Für den Vertrieb bleibt: Die Rechtschreibung läuft **offline** in der App, und die Grammatikprüfung ist **optional** und verbindet sich mit einem Server, den der Nutzer selbst angibt (eigener Docker-Container oder persönlicher Zugang bei LanguageTool). Die App funktioniert auch ganz ohne.
- **Das Wörterbuch.** Die üblichen deutschen Hunspell-Wörterbücher (igerman98 und „frami“) stehen unter **GPL v2 oder v3**. Mitliefern in einem verkauften, geschlossenen Programm ist damit heikel. Wege: das Wörterbuch beim ersten Start **herunterladen lassen** (als Daten, nicht fest eingebaut), ein Wörterbuch mit passender Lizenz nutzen oder die Rechtschreibung über LanguageTool abwickeln.

**Später, wenn die Entscheidung zum Vertrieb fällt (M9):**

- **Pandoc** (GPL) nur als getrenntes Programm aufrufen oder ganz durch Rust-Lösungen ersetzen.
- **Pakete und Signatur:** macOS (Apple-Entwicklerkonto, Notarisierung), Windows (Signaturzertifikat), Linux (Flatpak).
- **Verkauf:** Einmalkauf wie bei JZ Writer, Lizenzschlüssel oder App-Store, Handbuch, Webauftritt, Impressum und AGB.
- **Portalregeln und Haftung:** Tolino und Amazon ändern ihre Vorgaben. Dem Nutzer muss klar sein, dass der Prüfbericht eine Hilfe ist und die Portale das letzte Wort haben.

**Entscheidungspunkt nach M3:** Zwei Monate selbst benutzen, dann entscheiden: bleibt es intern, oder geht es in den Vertrieb. Bis dahin entsteht nichts, was den Vertrieb verbaut.

**Marke:** Das Werkzeug wäre ein Programm der **JURZEL Apps** mit Bezug zum **JURZEL Verlag**: zum Beispiel „JZ Werkbank“, mit dem Satz „Gebaut, um die Bücher des JURZEL Verlags zu machen“.

## 12. Offene Entscheidungen

1. ~~Intern oder Produkt?~~ **Entschieden:** zuerst intern, Vertrieb als Option (Abschnitt 11).
2. ~~GUI-Framework?~~ **Entschieden:** Rust mit Iced.
3. **Marke und Name:** „JZ Werkbank“ unter JURZEL Apps, mit Bezug zum Verlag? Und das Design der App: die JURZEL-Designwerte (Grün `#00875A`, Dunkel `#0B1810`, Cloud `#F5F4F0`), nicht das Dunkelblau und Gelb der Rust-Werkstatt-Bücher?
4. **Umfang der Erstfassung:** nur Rust-Werkstatt-Bände oder beliebige Bücher (dann braucht es allgemeine Vorlagen)?
5. **LanguageTool für dich:** auf dem IONOS-Server oder lokal im Docker-Container?
6. **Editor-Bibliothek:** Teilen mit dem „Reißbrett“ (deinem geplanten Website-Editor)? Ein gemeinsames Editor-Crate in Iced nutzt beiden Projekten.

## 13. Erfahrungen aus dem Dioxus-Spike und Wechsel zu Iced

Im ersten Entwurf (0.3) wurde ein Vorab-Spike mit Dioxus und CodeMirror 6 erprobt. Dabei haben sich wesentliche Erkenntnisse gezeigt:

1. **WebView bringt unnötigen Ballast mit:** Die Abhängigkeit von WebKitGTK unter Linux sorgte sofort für Reibereien bei Bibliotheksversionen auf Entwicklungs- und Produktivsystemen. Unter Windows verhält sich WebView2 wieder anders als WebKit auf Linux und macOS.
2. **Zwei Welten im selben Programm:** CodeMirror läuft in JavaScript, der Rest in Rust. Nachrichten wie Textänderungen, Tastaturereignisse und Cursorpositionen müssen serialisiert und hin- und hergeschickt werden. Das verkompliziert Tests und Zustandsverwaltung unnötig.
3. **Der Schnitt zu Iced:** Iced bietet mit The Elm Architecture (TEA) ein klares, rein in Rust geschriebenes Architekturmodell. Mit `iced::widget::text_editor` steht auf Basis von `cosmic-text` eine native Textkomponente zur Verfügung, die wir direkt kontrollieren – ohne Browser-Engine dazwischen.

## 14. Konventionen für den Quelltext

- **Dokumentation immer ausführlich auf Deutsch.** Jede Datei beginnt mit einer Beschreibung (was sie tut, wie sie ins Ganze passt), jede Funktion, jedes Struct, jedes Feld und jedes Enum hat einen Kommentar. Kommentare erklären das **Warum** (Entwurfsentscheidungen, Stolperfallen), nicht nur das Was.
- **Kern ohne Oberfläche.** Alles, was Dateien, Regeln und Zahlen betrifft, steht in Modulen ohne GUI-Abhängigkeiten und hat automatische Tests.
- **Tests vor Vertrauen.** Jede neue Kernfunktion bekommt Unit-Tests.
- **Installationshinweise gehören ins README**, inklusive minimaler Systempakete für die Grafikbeschleunigung (z. B. Vulkan/OpenGL-Treiber).
- **`cargo doc` ohne Warnungen.** Verweise in der Dokumentation müssen auflösen.
- **Kein JavaScript, kein HTML, kein CSS:** Die gesamte Oberfläche entsteht in idiomatischem Rust über Iced-Widgets.

## 15. Entscheidung: Warum Iced der Weg ist

Die Wahl fällt auf **Iced** aus vier greifbaren Gründen:

1. **Reines Rust statt Web-Stack:** Keine WebView, kein JavaScript-Bundle, kein CSS-Gefrickel. Das Programm baut mit reinem `cargo build` und läuft ohne externe Browser-Laufzeitumgebungen. Das passt exakt zu deiner Philosophie und dem Anspruch der Rust-Werkstatt.
2. **The Elm Architecture (TEA):** Das Architekturmuster (`State`, `Message`, `update`, `view`, `subscription`) sorgt für glasklare Datenflüsse. UI-Logik lässt sich hervorragend trennen, reproduzieren und mit Unit-Tests absichern. Asynchrone Aufgaben (LanguageTool-Abfragen, Typst-Kompilierung im Hintergrund, Dateisystem-Watcher) fügen sich nahtlos über Iced-Subscriptions ein.
3. **Plattformunabhängig ohne Systembibliotheken-Chaos:** Iced nutzt modernes wgpu für die Hardwarebeschleunigung. Unter Linux entfällt der Albtraum mit WebKitGTK-Paketen; unter Windows läuft die Ausgabe direkt über DirectX/Vulkan; unter macOS über Metal.
4. **Volle Kontrolle über den Texteditor:** Statt eine Blackbox im Browserfenster über JavaScript-PostMessages fernzusteuern, greift Iceds `text_editor` direkt auf `cosmic-text` zu. Markdown-Highlighting, Zeilennummerierung, Wortzähler und Textmanipulationen sind direkter Bestandteil des Rust-Zustands.

## 16. Stand und nächste Schritte

**Stand:** Plan Entwurf 0.4. Architektur verbindlich auf Rust und Iced ausgerichtet.

**Als Nächstes (M0-Spike mit Iced):**

1. **Cargo-Workspace aufsetzen:**
   - `werkbank-kern` (Projektstrukturen, `band.yaml`, Dateihandling)
   - `werkbank-app` (Iced-Anwendung)
2. **Iced-Fenster mit `text_editor` bereitstellen:**
   - Projektordner der Rust-Werkstatt öffnen.
   - Kapitel-Datei laden und im `text_editor`-Widget darstellen.
   - Markdown-Syntaxhervorhebung anbinden.
   - Atomares Speichern auf Tastendruck (`Ctrl+S` / `Cmd+S`) und bei Fokusverlust.
3. **Tipp- und Schreibgefühl evaluieren:**
   - Cursorsteuerung, Zeilenumbrüche, Schriftdarstellung.
   - Typewriter-Zentrierung und Scrollverhalten prüfen.
4. **Danach direkt in M1 einsteigen:** Dreispalter (Bibliothek, Blattliste, Editor), Schreibziel und automatische Snapshots.
