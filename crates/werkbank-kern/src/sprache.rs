//! # Sprachprüfung (LanguageTool)
//!
//! Zerlegt Markdown in prüfbaren Text und Auszeichnung (`markup`), damit
//! Codeblöcke, Inline-Code, Link-Adressen und `@kap:`-Verweise nicht als
//! Sprachfehler gemeldet werden. Das Ergebnis passt zum Annotationsformat
//! des LanguageTool-Servers (`data`-Parameter von `/v2/check`). Zusätzlich
//! werden die Treffer des Servers wieder auf Zeilen und Spalten im
//! Markdown-Quelltext abgebildet und gegen Projekt-Ausnahmen gefiltert.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::io;
use std::ops::Range;
use std::path::Path;

/// Ein Abschnitt des Quelltextes: entweder prüfbarer Text oder Auszeichnung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stueck {
    /// Der Quelltext dieses Abschnitts (unverändert aus der Markdown-Datei).
    pub text: String,
    /// `true` = Auszeichnung, wird nicht geprüft.
    pub markup: bool,
    /// Wofür LanguageTool die Auszeichnung beim Prüfen halten soll (z. B. Absatzende).
    pub als: Option<&'static str>,
}

/// Teilt Markdown in prüfbare Text- und nicht zu prüfende Auszeichnungsstücke.
///
/// Zusammengesetzt ergeben alle Stücke wieder exakt den Eingabetext.
pub fn annotation_aus_markdown(md: &str) -> Vec<Stueck> {
    let mut texte: Vec<Range<usize>> = Vec::new();
    let mut im_codeblock = false;

    for (ereignis, bereich) in Parser::new_ext(md, Options::ENABLE_STRIKETHROUGH).into_offset_iter() {
        match ereignis {
            Event::Start(Tag::CodeBlock(_)) => im_codeblock = true,
            Event::End(TagEnd::CodeBlock) => im_codeblock = false,
            Event::Text(_) | Event::SoftBreak if !im_codeblock => {
                match texte.last_mut() {
                    Some(letzter) if bereich.start <= letzter.end => {
                        letzter.end = letzter.end.max(bereich.end);
                    }
                    _ => texte.push(bereich),
                }
            }
            _ => {}
        }
    }

    let mut stuecke = Vec::new();
    let mut position = 0;
    for bereich in texte {
        if bereich.start < position || bereich.end > md.len() {
            continue;
        }
        if bereich.start > position {
            stuecke.push(auszeichnung(&md[position..bereich.start]));
        }
        text_mit_verweisen(&md[bereich.clone()], &mut stuecke);
        position = bereich.end;
    }
    if position < md.len() {
        stuecke.push(auszeichnung(&md[position..]));
    }
    stuecke
}

/// Klassifiziert Auszeichnung: Zeilenwechsel gelten als Absatzgrenze, Inline-Code als Platzhalter.
fn auszeichnung(s: &str) -> Stueck {
    let als = if s.contains('\n') {
        Some("\n\n")
    } else if s.contains('`') {
        Some("Code")
    } else {
        None
    };
    Stueck {
        text: s.to_string(),
        markup: true,
        als,
    }
}

/// Fügt Fließtext ein und spaltet `@kap:id`-Querverweise als Auszeichnung ab.
fn text_mit_verweisen(s: &str, ziel: &mut Vec<Stueck>) {
    let mut rest = s;
    while let Some(start) = rest.find("@kap:") {
        let nach = &rest[start + 5..];
        let laenge = nach
            .char_indices()
            .find(|(_, c)| !(c.is_alphanumeric() || *c == '-' || *c == '_'))
            .map_or(nach.len(), |(i, _)| i);
        if start > 0 {
            ziel.push(text_stueck(&rest[..start]));
        }
        ziel.push(Stueck {
            text: rest[start..start + 5 + laenge].to_string(),
            markup: true,
            als: Some("Kapitel"),
        });
        rest = &rest[start + 5 + laenge..];
    }
    if !rest.is_empty() {
        ziel.push(text_stueck(rest));
    }
}

fn text_stueck(s: &str) -> Stueck {
    Stueck {
        text: s.to_string(),
        markup: false,
        als: None,
    }
}

/// Baut den `data`-Parameter für `/v2/check`.
pub fn annotation_json(stuecke: &[Stueck]) -> String {
    let liste: Vec<Value> = stuecke
        .iter()
        .filter(|s| !s.text.is_empty())
        .map(|s| match (s.markup, s.als) {
            (false, _) => json!({ "text": s.text }),
            (true, Some(als)) => json!({ "markup": s.text, "interpretAs": als }),
            (true, None) => json!({ "markup": s.text }),
        })
        .collect();
    json!({ "annotation": liste }).to_string()
}

/// Ein von LanguageTool gemeldeter Hinweis, bezogen auf den Markdown-Quelltext.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Treffer {
    /// Kennung der Regel (z. B. `GERMAN_SPELLER_RULE`).
    pub regel_id: String,
    /// Kategorie (z. B. `TYPOS`, `GRAMMAR`, `STYLE`).
    pub kategorie_id: String,
    /// Ausführliche Meldung.
    pub meldung: String,
    /// Der betroffene Text.
    pub fundstelle: String,
    /// Byte-Offset des Anfangs im Quelltext.
    pub von: usize,
    /// Byte-Offset des Endes im Quelltext.
    pub bis: usize,
    /// Zeile (ab 0) des Anfangs.
    pub zeile: usize,
    /// Spalte (Byte-Index in der Zeile, ab 0) des Anfangs.
    pub spalte: usize,
    /// Zeile (ab 0) des Endes.
    pub ende_zeile: usize,
    /// Spalte (Byte-Index in der Zeile, ab 0) des Endes.
    pub ende_spalte: usize,
    /// Vorgeschlagene Ersetzungen (höchstens fünf).
    pub ersetzungen: Vec<String>,
}

/// Byte-Index im Quelltext für einen UTF-16-Offset (so zählt Java).
fn utf16_zu_byte(s: &str, utf16: usize) -> Option<usize> {
    let mut zaehler = 0;
    for (byte, c) in s.char_indices() {
        if zaehler == utf16 {
            return Some(byte);
        }
        zaehler += c.len_utf16();
        if zaehler > utf16 {
            return None;
        }
    }
    (zaehler == utf16).then_some(s.len())
}

/// Wie [`utf16_zu_byte`], aber nur über die prüfbaren Stücke gezählt (Auszeichnung zählt nicht).
fn text_offset_zu_byte(stuecke: &[Stueck], text_utf16: usize) -> Option<usize> {
    let mut text_pos = 0;
    let mut md_byte = 0;
    for s in stuecke {
        if !s.markup {
            let laenge: usize = s.text.chars().map(char::len_utf16).sum();
            if text_utf16 < text_pos + laenge || (text_utf16 == text_pos + laenge && text_utf16 > text_pos) {
                return utf16_zu_byte(&s.text, text_utf16 - text_pos).map(|b| md_byte + b);
            }
            text_pos += laenge;
        }
        md_byte += s.text.len();
    }
    None
}

fn zeile_spalte(md: &str, byte: usize) -> (usize, usize) {
    let davor = &md[..byte];
    let zeile = davor.matches('\n').count();
    let zeilenanfang = davor.rfind('\n').map_or(0, |i| i + 1);
    (zeile, byte - zeilenanfang)
}

/// Wertet die JSON-Antwort von `/v2/check` aus und ordnet die Treffer dem Quelltext zu.
///
/// Treffer, die sich nicht eindeutig im Quelltext wiederfinden (etwa weil sie
/// ausschließlich Auszeichnung betreffen), werden verworfen.
pub fn treffer_aus_antwort(antwort: &str, md: &str, stuecke: &[Stueck]) -> Result<Vec<Treffer>, String> {
    let wert: Value = serde_json::from_str(antwort).map_err(|e| format!("Antwort nicht lesbar: {e}"))?;
    let liste = wert
        .get("matches")
        .and_then(Value::as_array)
        .ok_or("Antwort enthält keine „matches“")?;

    let mut ergebnis = Vec::new();
    for m in liste {
        let offset = m.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let laenge = m.get("length").and_then(Value::as_u64).unwrap_or(0) as usize;
        if laenge == 0 {
            continue;
        }

        // Was LanguageTool als betroffenen Text meldet (laut Kontext)
        let erwartet = m.get("context").and_then(|c| {
            let text = c.get("text")?.as_str()?;
            let o = c.get("offset")?.as_u64()? as usize;
            let l = c.get("length")?.as_u64()? as usize;
            let einheiten: Vec<u16> = text.encode_utf16().collect();
            einheiten.get(o..o + l).map(String::from_utf16_lossy)
        });

        // Offsets beziehen sich normalerweise auf den Gesamttext inklusive Auszeichnung;
        // ersatzweise wird der Offset als Position im reinen Text gedeutet.
        let kandidaten = [
            utf16_zu_byte(md, offset).zip(utf16_zu_byte(md, offset + laenge)),
            text_offset_zu_byte(stuecke, offset).zip(text_offset_zu_byte(stuecke, offset + laenge)),
        ];
        let Some((von, bis)) = kandidaten
            .into_iter()
            .flatten()
            .find(|&(v, b)| v < b && erwartet.as_deref().is_none_or(|e| md.get(v..b) == Some(e)))
        else {
            continue;
        };

        let regel = m.get("rule");
        let text_von = |wert: Option<&Value>, feld: &str| {
            wert.and_then(|r| r.get(feld))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let (zeile, spalte) = zeile_spalte(md, von);
        let (ende_zeile, ende_spalte) = zeile_spalte(md, bis);

        ergebnis.push(Treffer {
            regel_id: text_von(regel, "id"),
            kategorie_id: text_von(regel.and_then(|r| r.get("category")), "id"),
            meldung: m
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            fundstelle: md[von..bis].to_string(),
            von,
            bis,
            zeile,
            spalte,
            ende_zeile,
            ende_spalte,
            ersetzungen: m
                .get("replacements")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|r| r.get("value").and_then(Value::as_str))
                .take(5)
                .map(str::to_string)
                .collect(),
        });
    }
    Ok(ergebnis)
}

/// Vom Autor ausgeschaltete Regeln und eigene Wörter (pro Band in `.werkbank/sprache.json`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ausnahmen {
    /// Regel-Kennungen, die nie gemeldet werden.
    #[serde(default)]
    pub regeln: Vec<String>,
    /// Wörter, die bei Rechtschreibhinweisen als richtig gelten.
    #[serde(default)]
    pub woerter: Vec<String>,
}

impl Ausnahmen {
    /// Entfernt Treffer, die der Autor ausgeschlossen hat.
    pub fn filtern(&self, treffer: Vec<Treffer>) -> Vec<Treffer> {
        treffer
            .into_iter()
            .filter(|t| {
                !self.regeln.contains(&t.regel_id)
                    && !(t.kategorie_id == "TYPOS" && self.woerter.contains(&t.fundstelle))
            })
            .collect()
    }

    /// Lädt die Ausnahmen eines Bandes (leer, wenn es noch keine gibt).
    pub fn laden(bandordner: &Path) -> Self {
        fs::read_to_string(Self::datei(bandordner))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Speichert die Ausnahmen im Bandordner.
    pub fn speichern(&self, bandordner: &Path) -> io::Result<()> {
        let datei = Self::datei(bandordner);
        if let Some(eltern) = datei.parent() {
            fs::create_dir_all(eltern)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        fs::write(datei, text)
    }

    fn datei(bandordner: &Path) -> std::path::PathBuf {
        bandordner.join(".werkbank").join("sprache.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEISPIEL: &str = "# Titel\n\nDas ist ein `Vec<u8>` im Text, siehe @kap:grundlagen.\nZweite Zeile.\n\n```rust\nfn main() { let x = 1; }\n```\n\nSchluss mit [Link](https://example.org/x).\n";

    #[test]
    fn stuecke_ergeben_den_originaltext() {
        let stuecke = annotation_aus_markdown(BEISPIEL);
        let zusammen: String = stuecke.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(zusammen, BEISPIEL);
    }

    #[test]
    fn code_und_verweise_werden_nicht_geprueft() {
        let stuecke = annotation_aus_markdown(BEISPIEL);
        let geprueft: String = stuecke.iter().filter(|s| !s.markup).map(|s| s.text.as_str()).collect();
        assert!(!geprueft.contains("fn main"));
        assert!(!geprueft.contains("Vec<u8>"));
        assert!(!geprueft.contains("@kap:"));
        assert!(!geprueft.contains("https://"));
        assert!(geprueft.contains("Das ist ein"));
        assert!(geprueft.contains("Zweite Zeile."));
        assert!(geprueft.contains("Link"));
        assert!(stuecke.iter().any(|s| s.text == "@kap:grundlagen" && s.als == Some("Kapitel")));
    }

    #[test]
    fn json_hat_annotationsformat() {
        let json = annotation_json(&annotation_aus_markdown("Hallo `x` Welt\n"));
        let wert: Value = serde_json::from_str(&json).unwrap();
        let liste = wert["annotation"].as_array().unwrap();
        assert_eq!(liste[0]["text"], "Hallo ");
        assert_eq!(liste[1]["markup"], "`x`");
        assert_eq!(liste[1]["interpretAs"], "Code");
    }

    fn antwort(offset: usize, laenge: usize, kontext: &str, ko: usize) -> String {
        json!({"matches":[{
            "message":"Tippfehler","offset":offset,"length":laenge,
            "replacements":[{"value":"Beispiel"}],
            "context":{"text":kontext,"offset":ko,"length":laenge},
            "rule":{"id":"GERMAN_SPELLER_RULE","category":{"id":"TYPOS"}}
        }]})
        .to_string()
    }

    #[test]
    fn treffer_mit_umlauten_und_zeilen() {
        let md = "# Ärger\n\nEin Beipsiel hier.\n";
        let stuecke = annotation_aus_markdown(md);
        // „Ärger“ hat 5 UTF-16-Einheiten wie Zeichen; „Beipsiel“ beginnt bei Zeichen 14
        let offset = md.chars().take_while(|_| true).collect::<String>().find("Beipsiel").unwrap();
        let offset_zeichen = md[..offset].chars().count();
        let json = antwort(offset_zeichen, 8, "Ein Beipsiel hier.", 4);
        let treffer = treffer_aus_antwort(&json, md, &stuecke).unwrap();
        assert_eq!(treffer.len(), 1);
        assert_eq!(treffer[0].fundstelle, "Beipsiel");
        assert_eq!((treffer[0].zeile, treffer[0].spalte), (2, 4));
        assert_eq!(treffer[0].ersetzungen, ["Beispiel"]);
    }

    #[test]
    fn offsets_im_reinen_text_werden_erkannt() {
        let md = "`code` und Beipsiel.\n";
        let stuecke = annotation_aus_markdown(md);
        // Server zählt hier nur den Text: " und Beipsiel." -> Offset 5
        let json = antwort(5, 8, " und Beipsiel.", 5);
        let treffer = treffer_aus_antwort(&json, md, &stuecke).unwrap();
        assert_eq!(treffer.len(), 1);
        assert_eq!(treffer[0].fundstelle, "Beipsiel");
    }

    #[test]
    fn unpassende_treffer_werden_verworfen() {
        let md = "Hallo Welt\n";
        let stuecke = annotation_aus_markdown(md);
        let json = antwort(0, 5, "ganz anderer Text", 0);
        assert!(treffer_aus_antwort(&json, md, &stuecke).unwrap().is_empty());
    }

    #[test]
    fn ausnahmen_filtern_regeln_und_woerter() {
        let md = "Ein Beipsiel.\n";
        let stuecke = annotation_aus_markdown(md);
        let treffer = treffer_aus_antwort(&antwort(4, 8, "Ein Beipsiel.", 4), md, &stuecke).unwrap();

        let nichts = Ausnahmen::default();
        assert_eq!(nichts.filtern(treffer.clone()).len(), 1);

        let wort = Ausnahmen { woerter: vec!["Beipsiel".into()], ..Default::default() };
        assert!(wort.filtern(treffer.clone()).is_empty());

        let regel = Ausnahmen { regeln: vec!["GERMAN_SPELLER_RULE".into()], ..Default::default() };
        assert!(regel.filtern(treffer).is_empty());
    }
}
