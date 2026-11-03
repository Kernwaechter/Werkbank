//! # Textmetriken & Lesbarkeitsanalyse
//!
//! Berechnet Lesbarkeitsindizes (Flesch-Amstad für deutsche Texte),
//! Satzlängen-Verteilungen und wacht über die Einhaltung der
//! 70-Zeichen-Grenze in Codeblöcken für den Buchdruck.

use serde::{Deserialize, Serialize};

/// Einstufung der Lesbarkeit nach dem Flesch-Amstad-Index für Deutsch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LesbarkeitGrad {
    /// Sehr leicht (Flesch >= 80): Kurze Sätze, alltäglicher Wortschatz.
    SehrLeicht,
    /// Leicht / Sehr gut verständlich (Flesch 65..80): Idealer Sachbuch-Ton für Einsteiger.
    Leicht,
    /// Mittelschwer (Flesch 50..65): Gehobene Allgemeinsprache.
    Mittel,
    /// Schwer (Flesch 35..50): Anspruchsvolle Fachliteratur.
    Schwer,
    /// Sehr schwer (Flesch < 35): Akademische Abhandlung, verschachtelte Sätze.
    SehrSchwer,
}

impl LesbarkeitGrad {
    /// Deutsche Bezeichnung für die Oberfläche.
    pub fn name(&self) -> &'static str {
        match self {
            Self::SehrLeicht => "Sehr leicht verständlich",
            Self::Leicht => "Sehr gut verständlich (Einsteiger-Ton)",
            Self::Mittel => "Mittelschwer",
            Self::Schwer => "Schwer (Fachchinesisch)",
            Self::SehrSchwer => "Sehr schwer (Verschachtelt)",
        }
    }
}

/// Detaillierte Analyse eines Textabschnitts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TextAnalyse {
    /// Gesamtwortzahl.
    pub woerter: usize,
    /// Gesamtzahl der Sätze.
    pub saetze: usize,
    /// Gesamtzahl der Silben (geschätzt).
    pub silben: usize,
    /// Durchschnittliche Satzlänge (Wörter pro Satz - ASL).
    pub durchschnittliche_satzlaenge: f32,
    /// Durchschnittliche Silbenzahl pro Wort (ASW).
    pub silben_pro_wort: f32,
    /// Flesch-Reading-Ease nach Amstad (angepasst für Deutsch): 180 - ASL - (58.5 * ASW).
    pub flesch_amstad: f32,
    /// Lesbarkeitsgrad.
    pub grad: Option<LesbarkeitGrad>,
    /// Anzahl kurzer Sätze (1 bis 12 Wörter).
    pub saetze_kurz: usize,
    /// Anzahl mittlerer Sätze (13 bis 24 Wörter).
    pub saetze_mittel: usize,
    /// Anzahl langer Sätze (25 bis 35 Wörter).
    pub saetze_lang: usize,
    /// Anzahl überlanger Bandwurmsätze (> 35 Wörter).
    pub saetze_ueberlang: usize,
}

/// Schätzt die Silbenzahl eines deutschen Wortes anhand von Vokalclustern.
pub fn silben_zaehlen(wort: &str) -> usize {
    let sauberes = wort.trim().to_lowercase();
    if sauberes.is_empty() {
        return 0;
    }

    let vokale = ['a', 'e', 'i', 'o', 'u', 'ä', 'ö', 'ü', 'y'];
    let mut anzahl = 0;
    let mut vorher_vokal = false;

    let chars: Vec<char> = sauberes.chars().collect();
    for &c in &chars {
        let ist_vokal = vokale.contains(&c);

        if ist_vokal {
            // Diphthonge zählen als eine Silbe: ei, eu, au, ie, äu, ai, etc.
            if vorher_vokal {
                // Vorheriger Buchstabe war auch ein Vokal -> Zusammenfassung
            } else {
                anzahl += 1;
            }
            vorher_vokal = true;
        } else {
            vorher_vokal = false;
        }
    }

    // Jedes Wort hat mindestens eine Silbe
    anzahl.max(1)
}

/// Analysiert den Fließtext (ohne Codeblöcke und Überschriften) eines Kapitels.
pub fn text_analysieren(markdown: &str) -> TextAnalyse {
    let mut fliess_saetze = Vec::new();
    let mut im_codeblock = false;

    // Nur Fließtext betrachten: Codeblöcke ausblenden
    for zeile in markdown.lines() {
        let trimmed = zeile.trim();
        if trimmed.starts_with("```") {
            im_codeblock = !im_codeblock;
            continue;
        }
        if im_codeblock || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Satz-Endzeichen trennen (. ! ?)
        let mut satz_puffer = String::new();
        for c in zeile.chars() {
            satz_puffer.push(c);
            if c == '.' || c == '!' || c == '?' {
                let s = satz_puffer.trim().to_string();
                if !s.is_empty() && s.split_whitespace().count() > 0 {
                    fliess_saetze.push(s);
                }
                satz_puffer.clear();
            }
        }
        let rest = satz_puffer.trim().to_string();
        if !rest.is_empty() && rest.split_whitespace().count() > 0 {
            fliess_saetze.push(rest);
        }
    }

    if fliess_saetze.is_empty() {
        return TextAnalyse::default();
    }

    let saetze_anzahl = fliess_saetze.len();
    let mut gesamtwoerter = 0;
    let mut gesamtsilben = 0;
    let mut saetze_kurz = 0;
    let mut saetze_mittel = 0;
    let mut saetze_lang = 0;
    let mut saetze_ueberlang = 0;

    for satz in &fliess_saetze {
        let woerter: Vec<&str> = satz
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .collect();

        let w_count = woerter.len();
        gesamtwoerter += w_count;

        for w in &woerter {
            gesamtsilben += silben_zaehlen(w);
        }

        match w_count {
            0..=12 => saetze_kurz += 1,
            13..=24 => saetze_mittel += 1,
            25..=35 => saetze_lang += 1,
            _ => saetze_ueberlang += 1,
        }
    }

    if gesamtwoerter == 0 {
        return TextAnalyse::default();
    }

    let asl = gesamtwoerter as f32 / saetze_anzahl as f32;
    let asw = gesamtsilben as f32 / gesamtwoerter as f32;

    // Flesch-Amstad-Formel für Deutsch:
    // 180 - ASL - (58.5 * ASW)
    let flesch = (180.0 - asl - (58.5 * asw)).clamp(0.0, 100.0);

    let grad = if flesch >= 80.0 {
        LesbarkeitGrad::SehrLeicht
    } else if flesch >= 65.0 {
        LesbarkeitGrad::Leicht
    } else if flesch >= 50.0 {
        LesbarkeitGrad::Mittel
    } else if flesch >= 35.0 {
        LesbarkeitGrad::Schwer
    } else {
        LesbarkeitGrad::SehrSchwer
    };

    TextAnalyse {
        woerter: gesamtwoerter,
        saetze: saetze_anzahl,
        silben: gesamtsilben,
        durchschnittliche_satzlaenge: asl,
        silben_pro_wort: asw,
        flesch_amstad: flesch,
        grad: Some(grad),
        saetze_kurz,
        saetze_mittel,
        saetze_lang,
        saetze_ueberlang,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silben_zaehlen() {
        assert_eq!(silben_zaehlen("Rust"), 1);
        assert_eq!(silben_zaehlen("Werkbank"), 2);
        assert_eq!(silben_zaehlen("Programmieren"), 4);
        assert_eq!(silben_zaehlen("Leidenschaft"), 3);
        assert_eq!(silben_zaehlen("Hobelbank"), 3);
    }

    #[test]
    fn test_text_analysieren() {
        let text = "Wir sitzen nebeneinander an einer Werkbank. Du hast die Tastatur. Ich schaue dir über die Schulter.";
        let analyse = text_analysieren(text);
        assert_eq!(analyse.saetze, 3);
        assert!(analyse.woerter >= 13);
        assert!(analyse.flesch_amstad > 60.0);
    }
}

/// Typische Floskeln und Füllwörter, die beim Überarbeiten auffallen sollen.
pub const STANDARD_FLOSKELN: &[&str] = &[
    "stell dir vor",
    "darüber hinaus",
    "im grunde",
    "sozusagen",
    "quasi",
    "eigentlich",
    "irgendwie",
    "ziemlich",
    "einfach",
    "wirklich",
    "natürlich",
    "selbstverständlich",
    "grundsätzlich",
    "im Prinzip",
];

/// Zählt Begriffe im Fließtext (ohne Codeblöcke), unabhängig von Groß-/Kleinschreibung.
/// Einzelwörter gelten nur als ganze Wörter. Liefert nur Begriffe mit Treffern, häufigste zuerst.
pub fn begriffe_zaehlen(markdown: &str, begriffe: &[&str]) -> Vec<(String, usize)> {
    let fliesstext: String = crate::markdown::zeilen(markdown)
        .into_iter()
        .filter(|z| !z.im_code)
        .map(|z| z.text)
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase();

    let mut ergebnis: Vec<(String, usize)> = begriffe
        .iter()
        .map(|b| {
            let b = b.to_lowercase();
            let n = fliesstext
                .match_indices(&b)
                .filter(|(i, m)| {
                    let davor = fliesstext[..*i].chars().next_back();
                    let danach = fliesstext[i + m.len()..].chars().next();
                    !davor.is_some_and(char::is_alphanumeric) && !danach.is_some_and(char::is_alphanumeric)
                })
                .count();
            (b, n)
        })
        .filter(|(_, n)| *n > 0)
        .collect();
    ergebnis.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    ergebnis
}

#[cfg(test)]
mod begriffe_tests {
    use super::*;

    #[test]
    fn zaehlt_ganze_woerter_ohne_code() {
        let text = "Das ist einfach gut. Einfach so. Vereinfacht nicht.\n\n```text\neinfach einfach\n```\nStell dir vor, es geht.\n";
        let z = begriffe_zaehlen(text, &["einfach", "stell dir vor", "quasi"]);
        assert_eq!(z, vec![("einfach".to_string(), 2), ("stell dir vor".to_string(), 1)]);
    }
}
