//! # Werkbank Kern
//!
//! Dieses Crate enthält die zentralen Datenstrukturen und die Geschäftslogik
//! der Werkbank – unabhängig von jeder grafischen Oberfläche.
//!
//! Hier verwalten wir Buchprojekte, Bände, Teile und Kapitel, lesen die
//! Konfigurationsdatei `band.yaml` und stellen die Schnittstellen für
//! spätere Prüf-, Statistik- und Bauprozesse bereit.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod codepruefung;
pub mod skriptpruefung;
pub mod prozess;
pub mod dartpruefung;
pub mod cover;
pub mod io;
pub mod markdown;
pub mod speicher;
pub mod verlauf;
pub mod diff;
pub mod statistik;
pub mod stil;
pub mod suche;
pub mod metriken;
pub mod portale;
pub mod sprache;

/// Status eines Textabschnitts im redaktionellen Ablauf.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum KapitelStatus {
    /// Erste Rohfassung oder Stichpunkte.
    #[default]
    Entwurf,
    /// Aktiver Schreibprozess im Gange.
    InArbeit,
    /// Inhaltlich vollständig, wartet auf sprachlichen Feinschliff.
    Ueberarbeitet,
    /// Vollständig geprüft, bereit für den Buchsatz.
    Fertig,
}

impl KapitelStatus {
    /// Liefert eine lesbare deutsche Bezeichnung für die Benutzeroberfläche.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Entwurf => "Entwurf",
            Self::InArbeit => "In Arbeit",
            Self::Ueberarbeitet => "Überarbeitet",
            Self::Fertig => "Fertig",
        }
    }
}

/// Repräsentiert ein einzelnes Kapitel oder einen Textabschnitt.
///
/// Ein Kapitel ist die kleinste redaktionelle Einheit, an der du im
/// Editor arbeitest. Es besitzt einen Dateinamen, einen Titel, seinen Textinhalt
/// und Metadaten wie das persönliche Wortziel und den Bearbeitungsstatus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kapitel {
    /// Eindeutige Kennung des Kapitels (z. B. "kap-01-hallo-rust").
    pub id: String,
    /// Anzeigename in der Seitenleiste und als Überschrift.
    pub titel: String,
    /// Relativer Pfad zur Markdown-Datei im Projekt.
    pub dateipfad: PathBuf,
    /// Wortziel für dieses Kapitel (z. B. 2000 Wörter).
    pub wortziel: usize,
    /// Redaktioneller Bearbeitungsstand.
    pub status: KapitelStatus,
    /// Notizen oder Schreibabsicht für dieses Kapitel.
    pub notizen: String,
    /// Aktueller Textinhalt (wird für schnelle Bearbeitung im Speicher gehalten).
    pub inhalt: String,
}

impl Kapitel {
    /// Erstellt ein neues Kapitel mit Standardwerten.
    pub fn neu(
        id: impl Into<String>,
        titel: impl Into<String>,
        dateipfad: impl Into<PathBuf>,
        inhalt: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            titel: titel.into(),
            dateipfad: dateipfad.into(),
            wortziel: 2000,
            status: KapitelStatus::InArbeit,
            notizen: String::new(),
            inhalt: inhalt.into(),
        }
    }

    /// Ermittelt die Anzahl der Wörter im aktuellen Inhalt.
    pub fn wortzahl(&self) -> usize {
        self.inhalt.split_whitespace().count()
    }

    /// Zählt alle Zeichen inklusive Leerzeichen und Zeilenumbrüchen.
    pub fn zeichenzahl(&self) -> usize {
        self.inhalt.chars().count()
    }

    /// Zählt alle Zeichen ohne Leerzeichen und Zeilenumbrüche.
    pub fn zeichen_ohne_leerzeichen(&self) -> usize {
        self.inhalt.chars().filter(|c| !c.is_whitespace()).count()
    }

    /// Zählt die Anzahl nicht-leerer Absätze.
    pub fn absatz_anzahl(&self) -> usize {
        self.inhalt
            .split("\n\n")
            .filter(|absatz| !absatz.trim().is_empty())
            .count()
    }

    /// Schätzt die durchschnittliche Lesezeit in Minuten bei 200 Wörtern pro Minute.
    pub fn lesedauer_minuten(&self) -> usize {
        let w = self.wortzahl();
        w.div_ceil(200)
    }

    /// Berechnet den erreichten Fortschritt bezogen auf das Wortziel in Prozent (0 bis 100).
    pub fn ziel_fortschritt(&self) -> f32 {
        if self.wortziel == 0 {
            return 100.0;
        }
        ((self.wortzahl() as f32 / self.wortziel as f32) * 100.0).clamp(0.0, 100.0)
    }

    /// Liefert die ersten Zeichen als Vorschautext für die Blattliste.
    pub fn vorschautext(&self, max_zeichen: usize) -> String {
        let mut teile: Vec<String> = Vec::new();
        let mut im_code = false;
        for zeile in self.inhalt.lines() {
            let t = zeile.trim();
            if t.starts_with("```") || t.starts_with("~~~") {
                im_code = !im_code;
                continue;
            }
            if im_code || t.is_empty() || t.starts_with('#') || t.starts_with('|') || t.starts_with("---") {
                continue;
            }
            let rein = lesetext(t);
            if !rein.is_empty() {
                teile.push(rein);
            }
            if teile.iter().map(|s| s.chars().count()).sum::<usize>() > max_zeichen {
                break;
            }
        }
        let flach = teile.join(" ");
        if flach.chars().count() <= max_zeichen {
            flach
        } else {
            let gekuerzt: String = flach.chars().take(max_zeichen).collect();
            format!("{}…", gekuerzt.trim_end())
        }
    }

    /// Sucht alle Querverweise im Format `@kap:id` im aktuellen Kapiteltext.
    pub fn querverweise(&self) -> Vec<String> {
        let mut treffer = Vec::new();
        for wort in self.inhalt.split_whitespace() {
            if let Some(rest) = wort.strip_prefix("@kap:") {
                let bereinigt: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                if !bereinigt.is_empty() && !treffer.contains(&bereinigt) {
                    treffer.push(bereinigt);
                }
            }
        }
        treffer
    }

    /// Zählt alle Rust-Codeblöcke (```rust).
    pub fn rust_codebloecke_anzahl(&self) -> usize {
        self.inhalt.lines().filter(|l| l.trim().starts_with("```rust")).count()
    }

    /// Anzahl der Shell-Skript-Codeblöcke (bash, sh, zsh, fish, powershell).
    pub fn skript_codebloecke_anzahl(&self) -> usize {
        self.inhalt
            .lines()
            .filter(|l| {
                let t = l.trim().trim_start_matches('`');
                l.trim().starts_with("```") && crate::skriptpruefung::ist_skript(t.split([',', ' ', '{']).next().unwrap_or(""))
            })
            .count()
    }

    /// Anzahl der Dart-/Flutter-Codeblöcke.
    pub fn dart_codebloecke_anzahl(&self) -> usize {
        self.inhalt
            .lines()
            .filter(|l| {
                let t = l.trim();
                t.starts_with("```dart") || t.starts_with("```flutter")
            })
            .count()
    }

    /// Codezeilen, die länger als 70 Zeichen sind (Standardgrenze für den Druck im Buchformat).
    pub fn ueberlange_codezeilen(&self) -> Vec<(usize, String)> {
        self.ueberlange_codezeilen_ab(70)
    }

    /// Codezeilen mit mehr als `grenze` Zeichen; Zeilennummern zählen ab 1 im Kapiteltext.
    pub fn ueberlange_codezeilen_ab(&self, grenze: usize) -> Vec<(usize, String)> {
        crate::markdown::zeilen(&self.inhalt)
            .into_iter()
            .enumerate()
            .filter(|(_, z)| z.im_code && !z.text.trim_start().starts_with("```") && !z.text.trim_start().starts_with("~~~"))
            .filter(|(_, z)| z.text.chars().count() > grenze)
            .map(|(nr, z)| (nr + 1, z.text.to_string()))
            .collect()
    }
}

/// Gliedert Kapitel in einen übergeordneten Buchteil (z. B. „Teil 1: Die Werkbank“).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Teil {
    /// Eindeutige Kennung des Buchteils.
    pub id: String,
    /// Ausgeschriebener Titel des Teils.
    pub titel: String,
    /// Text oder Überschriften vor dem ersten Kapitel (Präambel).
    pub praeambel: String,
    /// Zugehörige Datei auf der Festplatte.
    pub dateipfad: PathBuf,
    /// Liste aller zugehörigen Kapitel in Leserichtung.
    pub kapitel: Vec<Kapitel>,
}

impl Teil {
    /// Erzeugt einen neuen leeren Buchteil.
    pub fn neu(id: impl Into<String>, titel: impl Into<String>) -> Self {
        let id_str = id.into();
        let fallback_pfad = PathBuf::from(format!("{}.md", id_str));
        Self {
            id: id_str,
            titel: titel.into(),
            praeambel: String::new(),
            dateipfad: fallback_pfad,
            kapitel: Vec::new(),
        }
    }

    /// Fügt dem Teil ein Kapitel hinzu.
    pub fn kapitel_hinzufuegen(&mut self, k: Kapitel) {
        self.kapitel.push(k);
    }

    /// Sucht ein Kapitel anhand seiner Kennung.
    pub fn kapitel_finden(&self, id: &str) -> Option<&Kapitel> {
        self.kapitel.iter().find(|k| k.id == id)
    }

    /// Sucht ein Kapitel veränderbar anhand seiner Kennung.
    pub fn kapitel_finden_mut(&mut self, id: &str) -> Option<&mut Kapitel> {
        self.kapitel.iter_mut().find(|k| k.id == id)
    }

    /// Summiert alle Wörter sämtlicher Kapitel dieses Teils.
    pub fn gesamtwortzahl(&self) -> usize {
        self.kapitel.iter().map(|k| k.wortzahl()).sum()
    }
}

/// Enthält alle Metadaten eines Bandes, typischerweise aus `band.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BandMetadaten {
    /// Buchtitel (z. B. „Die Rust-Werkstatt“).
    pub titel: String,
    /// Untertitel (z. B. „Praktische Softwareentwicklung mit System“).
    pub untertitel: String,
    /// Bandnummer oder Reihenbezeichnung (z. B. „Band 1: Vom Lehrling zum Gesellen“).
    pub band_bezeichnung: String,
    /// Name des Autors.
    pub autor: String,
    /// Erscheinungsjahr oder Versionsstand.
    pub jahr: String,
    /// ISBN für Tolino / Printausgabe (sofern schon vergeben).
    pub isbn_print: Option<String>,
}

impl Default for BandMetadaten {
    fn default() -> Self {
        Self {
            titel: "Die Rust-Werkstatt".to_string(),
            untertitel: "Vom Lehrling zum Gesellen".to_string(),
            band_bezeichnung: "Band 1".to_string(),
            autor: "Volker Dähn".to_string(),
            jahr: "2026".to_string(),
            isbn_print: Some("978-3-9825412-0-4".to_string()),
        }
    }
}

/// Das vollständige Buchprojekt, an dem in der Werkbank gearbeitet wird.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Projekt {
    /// Projektordner auf der Festplatte.
    pub wurzelverzeichnis: PathBuf,
    /// Metadaten des aktuellen Bandes.
    pub metadaten: BandMetadaten,
    /// Die einzelnen Hauptteile des Buches.
    pub teile: Vec<Teil>,
}

impl Projekt {
    /// Erzeugt ein leeres Projekt mit vorgegebenem Pfad und Standardmetadaten.
    pub fn neu(wurzelverzeichnis: impl Into<PathBuf>) -> Self {
        Self {
            wurzelverzeichnis: wurzelverzeichnis.into(),
            metadaten: BandMetadaten::default(),
            teile: Vec::new(),
        }
    }

    /// Erzeugt ein Demoprojekt mit typischen Rust-Werkstatt-Inhalten,
    /// damit die Benutzeroberfläche sofort realitätsnah getestet werden kann.
    pub fn beispiel_projekt() -> Self {
        let mut projekt = Self::neu(Path::new("/run/media/volker/Daten/Werkbank"));

        let mut teil1 = Teil::neu("teil-1", "Teil 1: An der Hobelbank");
        teil1.kapitel_hinzufuegen(Kapitel::neu(
            "kap-01",
            "Kapitel 1: Werkzeugkiste aufklappen",
            "teil-1/kapitel-1.md",
            "# Kapitel 1: Werkzeugkiste aufklappen\n\n\
            Willkommen an der Werkbank. Wenn du mit Rust beginnst, lernst du nicht bloß eine neue Syntax, \
            sondern eine andere Art, über Speicher, Besitz und Fehler nachzudenken.\n\n\
            In diesem Band zeige ich dir Schritt für Schritt, wie du saubere und sichere Werkzeuge baust. \
            Wir starten klein an der Hobelbank, bevor wir zu komplexeren Konstruktionen übergehen.\n\n\
            ### Ran an die Tastatur\n\n\
            Tippe folgenden Code in dein Projekt ein:\n\n\
            ```rust\n\
            fn main() {\n    \
                println!(\"Hallo aus der Rust-Werkstatt!\");\n\
            }\n\
            ```\n\n\
            Speichere die Datei und starte das Programm. Im Terminal siehst du sofort die Begrüßungszeile.\n\n\
            ### Was passiert hier?\n\n\
            Das Makro `println!` schreibt den formatierten Text auf die Standardausgabe. Das Ausrufezeichen verrät \
            dir, dass es sich um ein Makro handelt – mehr dazu sehen wir in den späteren Kapiteln (@kap:kap-02).\n\n\
            ### Bughunter\n\n\
            Was passiert, wenn du das Semikolon am Zeilenende vergisst? Der Rust-Compiler hält dich sofort \
            freundlich, aber bestimmt an und zeigt dir die genaue Stelle.\n"
        ));

        teil1.kapitel_hinzufuegen(Kapitel::neu(
            "kap-02",
            "Kapitel 2: Variablen und Bindungen",
            "teil-1/kapitel-2.md",
            "# Kapitel 2: Variablen und Bindungen\n\n\
            In Rust sind Variablen standardmäßig unveränderlich (immutable). Das mag auf den ersten Blick \
            ungewohnt wirken, schützt dich in der Praxis aber vor einer riesigen Klasse von Fehlern.\n\n\
            ### Ran an die Tastatur\n\n\
            ```rust\n\
            fn main() {\n    \
                let spanbreite = 42;\n    \
                println!(\"Die Spanbreite beträgt {} mm\", spanbreite);\n\
            }\n\
            ```\n\n\
            Wenn du versuchst, `spanbreite = 50;` anzufügen, weist dich der Compiler ruhig und bestimmt ab. \
            Möchtest du eine Variable ändern, musst du das Schlüsselwort `mut` explizit voranstellen.\n\n\
            Siehe dazu auch die Erläuterungen im Speicher-Kapitel @kap:kap-03.\n"
        ));

        let mut teil2 = Teil::neu("teil-2", "Teil 2: Das Kernholz verstehen");
        teil2.kapitel_hinzufuegen(Kapitel::neu(
            "kap-03",
            "Kapitel 3: Ownership und Borrowing",
            "teil-2/kapitel-3.md",
            "# Kapitel 3: Ownership und Borrowing\n\n\
            Jetzt widmen wir uns dem Herzstück von Rust: dem Besitzmodell. Jedes Stück Daten im Speicher hat \
            genau einen Besitzer. Verlässt dieser den Gültigkeitsbereich, räumt Rust den Speicher automatisch ab.\n\n\
            Kein Garbage Collector, keine manuellen `free`-Aufrufe und keine Speicherlecks zur Laufzeit.\n\n\
            ### Ran an die Tastatur\n\n\
            ```rust\n\
            fn main() {\n    \
                let s1 = String::from(\"Rust\");\n    \
                let s2 = s1;\n    \
                // println!(\"{}\", s1); // Fehler: Wert wurde an s2 übertragen\n    \
                println!(\"Besitzer ist jetzt s2: {}\", s2);\n\
            }\n\
            ```\n\n\
            Das Ownership-Prinzip garantiert dir Speichersicherheit ganz ohne Performance-Einbußen.\n"
        ));

        projekt.teile.push(teil1);
        projekt.teile.push(teil2);

        projekt
    }

    /// Ermittelt die Gesamtwortzahl aller Kapitel im Buch.
    pub fn gesamtwortzahl(&self) -> usize {
        self.teile.iter().map(|t| t.gesamtwortzahl()).sum()
    }

    /// Ermittelt die Gesamtzahl aller Zeichen im Buch.
    pub fn gesamtzeichenzahl(&self) -> usize {
        self.teile
            .iter()
            .flat_map(|t| &t.kapitel)
            .map(|k| k.zeichenzahl())
            .sum()
    }

    /// Berechnet die Anzahl der Normseiten (1.800 Zeichen inkl. Leerzeichen).
    pub fn normseiten(&self) -> f32 {
        self.gesamtzeichenzahl() as f32 / 1800.0
    }

    /// Schätzt die Anzahl der Buchseiten im Druckformat 170 x 240 mm (ca. 280 Wörter pro Seite).
    pub fn buchseiten_schaetzung(&self) -> usize {
        let w = self.gesamtwortzahl();
        w.div_ceil(280)
    }

    /// Zählt die Gesamtzahl aller Kapitel im Projekt.
    pub fn kapitel_anzahl(&self) -> usize {
        self.teile.iter().map(|t| t.kapitel.len()).sum()
    }

    /// Sucht ein Kapitel über alle Teile hinweg anhand seiner Kennung.
    pub fn kapitel_finden(&self, id: &str) -> Option<&Kapitel> {
        for teil in &self.teile {
            if let Some(k) = teil.kapitel_finden(id) {
                return Some(k);
            }
        }
        None
    }

    /// Sucht ein Kapitel veränderbar über alle Teile hinweg.
    pub fn kapitel_finden_mut(&mut self, id: &str) -> Option<&mut Kapitel> {
        for teil in &mut self.teile {
            if let Some(k) = teil.kapitel_finden_mut(id) {
                return Some(k);
            }
        }
        None
    }
}

/// Macht aus einer Markdown-Zeile lesbaren Text für Vorschauen: ohne HTML-Tags, Auszeichnungen,
/// Listenzeichen, Zitatzeichen und Link-Ziele (`[Text](url)` → `Text`).
pub fn lesetext(zeile: &str) -> String {
    let mut t = zeile.trim_start();
    for p in ["> ", ">", "- ", "* ", "+ "] {
        if let Some(r) = t.strip_prefix(p) {
            t = r.trim_start();
        }
    }
    // Nummerierte Listen: „1. “
    if let Some(pos) = t.find(". ")
        && pos > 0
        && pos <= 3
        && t[..pos].chars().all(|c| c.is_ascii_digit())
    {
        t = &t[pos + 2..];
    }
    let mut aus = String::with_capacity(t.len());
    let zeichen: Vec<char> = t.chars().collect();
    let mut i = 0;
    while i < zeichen.len() {
        let c = zeichen[i];
        match c {
            // HTML-Tags überspringen
            '<' => {
                if let Some(ende) = zeichen[i..].iter().position(|&z| z == '>') {
                    i += ende + 1;
                    continue;
                }
                aus.push(c);
            }
            // Bilder ganz weglassen, Links auf ihren Text kürzen
            '!' if zeichen.get(i + 1) == Some(&'[') => {
                if let Some(ende) = zeichen[i..].iter().position(|&z| z == ')') {
                    i += ende + 1;
                    continue;
                }
            }
            ']' if zeichen.get(i + 1) == Some(&'(') => {
                if let Some(ende) = zeichen[i..].iter().position(|&z| z == ')') {
                    i += ende + 1;
                    continue;
                }
            }
            '*' | '_' | '`' | '[' | ']' | '~' => {}
            _ => aus.push(c),
        }
        i += 1;
    }
    let aus = aus.split_whitespace().collect::<Vec<_>>().join(" ");
    // Leerzeichen vor Satzzeichen, die durch weggelassene Bilder entstehen
    aus.replace(" .", ".").replace(" ,", ",").replace(" :", ":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vorschau_ohne_auszeichnungen() {
        let k = Kapitel::neu(
            "k",
            "K",
            "k.md",
            "## Titel\n\n**Volker Dähn** </div> <div class=\"x\">\n\n```dart\nfinal x = 1;\n```\n\n- Ein `Text`-Widget mit [Link](https://x.de) und ![Bild](a.png).\n",
        );
        assert_eq!(k.vorschautext(200), "Volker Dähn Ein Text-Widget mit Link und.");
        assert_eq!(lesetext("1. Schritt *eins*"), "Schritt eins");
    }

    #[test]
    fn test_kapitel_wortzaehlung() {
        let k = Kapitel::neu("k1", "Test", "t.md", "Drei kleine Wörter");
        assert_eq!(k.wortzahl(), 3);
        assert_eq!(k.zeichenzahl(), 18);
    }

    #[test]
    fn test_kapitel_querverweise() {
        let k = Kapitel::neu(
            "k1",
            "Test",
            "t.md",
            "Siehe @kap:kap-02 und auch @kap:kap-03 im Text.",
        );
        let refs = k.querverweise();
        assert_eq!(refs.len(), 2);
        assert_eq!(refs[0], "kap-02");
        assert_eq!(refs[1], "kap-03");
    }

    #[test]
    fn test_ueberlange_codezeilen() {
        let kurz = "```rust\nlet a = 1;\n```";
        let lang = format!("```rust\n{}\n```", "x".repeat(75));

        let k1 = Kapitel::neu("k1", "Kurz", "k1.md", kurz);
        assert!(k1.ueberlange_codezeilen().is_empty());

        let k2 = Kapitel::neu("k2", "Lang", "k2.md", lang);
        assert_eq!(k2.ueberlange_codezeilen().len(), 1);
    }

    #[test]
    fn test_beispiel_projekt_enthaelt_kapitel() {
        let p = Projekt::beispiel_projekt();
        assert_eq!(p.teile.len(), 2);
        assert_eq!(p.kapitel_anzahl(), 3);
        assert!(p.gesamtwortzahl() > 100);
        assert!(p.normseiten() > 0.5);
    }
}
