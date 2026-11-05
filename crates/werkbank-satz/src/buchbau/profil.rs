//! Buchprofil `buchbau.yaml` im Buchordner: alles, was im bisherigen `build_book.py` fest stand.
//!
//! Der Klappentext eines Bandes steht in `<Bandordner>/klappentext.md` (wie beim Cover-Designer).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Dateiname des Profils im Buchordner.
pub const DATEI: &str = "buchbau.yaml";

/// Seitenränder in Millimetern (innen = Bundsteg, gegengleich gespiegelt).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Raender {
    pub innen: f32,
    pub aussen: f32,
    pub oben: f32,
    pub unten: f32,
}

/// Werkstatt-Rubrik: Überschriften der Ebene 3, die mit `titel` beginnen, bekommen die Gestaltung `art`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rubrik {
    pub titel: String,
    pub art: String,
}

/// Texte und Farben des Umschlags eines Bandes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Umschlag {
    /// Bandbezeichnung auf der Vorderseite, z. B. „Band 1“.
    pub band_etikett: String,
    /// Bandbezeichnung auf dem Rücken, z. B. „BAND 1“.
    pub band_ruecken: String,
    /// Zeile unter dem Abzeichen (HTML-Auszeichnung wie `&amp;` erlaubt).
    pub abzeichen_text: String,
    /// Hintergrund- und Schriftfarbe des Band-Abzeichens.
    pub abzeichen_grund: String,
    pub abzeichen_schrift: String,
    /// Untertitel auf der Vorderseite und seine Farbe.
    pub untertitel: String,
    pub untertitel_farbe: String,
    /// Kurzer Werbetext auf der Vorderseite.
    pub lead: String,
    /// Rückseite: Dachzeile, Überschrift (mit `<br/>`), Fließtext.
    pub rueckseite_dachzeile: String,
    pub rueckseite_titel: String,
    pub rueckseite_text: String,
    /// Merkmalskasten auf der Rückseite.
    pub merkmale_titel: String,
    pub merkmale: Vec<String>,
}

/// Gemeinsame Texte des Komplettumschlags aller Bände.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reihe {
    /// Dachzeile über dem Titel auf der Vorderseite.
    pub dachzeile: String,
    /// Titel der Vorderseite: `[eckige Klammern]` = Akzentfarbe, `\n` = Zeilenumbruch.
    pub titel: String,
    /// Titel auf dem Buchrücken.
    pub ruecken_titel: String,
    /// Kurzbiografie auf der Rückseite (Überschrift und Text; `<strong>` erlaubt).
    pub autor_bio_titel: String,
    pub autor_bio: String,
    /// Zeile mit den Webseiten unten auf der Rückseite und Zeile darunter (die Seitenzahl wird angehängt).
    pub webseiten_zeile: String,
    pub download_zeile: String,
    /// Webadresse und Systemhinweis unten auf der Vorderseite.
    pub domain: String,
    pub system_hinweis: String,
}

/// Ein Band des Buchs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Band {
    pub nummer: u32,
    /// Ordner des Bandes relativ zum Buchordner (z. B. `Band_1`).
    pub ordner: String,
    /// Untertitel, z. B. „Band 1: Vom ersten Widget zur eigenen App“.
    pub untertitel: String,
    /// Eindeutige Kennung des E-Books (`urn:uuid:…`).
    pub uuid: String,
    /// Kurzname für Dateinamen (z. B. `flutter-werkstatt-band1`).
    pub kurzname: String,
    #[serde(default)]
    pub umschlag: Umschlag,
}

fn standard_sprache() -> String {
    "de".to_string()
}
fn standard_ausgabe() -> String {
    "build".to_string()
}

/// Das Buchprofil.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profil {
    pub titel: String,
    pub autor: String,
    #[serde(default = "standard_sprache")]
    pub sprache: String,
    /// Verlagsangabe für die E-Book-Metadaten.
    pub verlag: String,
    /// Seitenformat (Breite, Höhe) in Millimetern.
    pub format_mm: [f32; 2],
    pub raender_mm: Raender,
    /// Quelldateien je Band in Buchreihenfolge (die erste ist die Titelei).
    pub quellen: Vec<String>,
    pub rubriken: Vec<Rubrik>,
    /// Ordner für die Zwischenergebnisse (relativ zum Buchordner).
    #[serde(default = "standard_ausgabe")]
    pub ausgabe: String,
    /// Webseiten für den Direktvertrieb (erscheinen in den Hinweisdateien und auf dem Umschlag).
    #[serde(default)]
    pub webseiten: Vec<String>,
    /// Zeilennummern in Codeblöcken der PDFs.
    #[serde(default)]
    pub zeilennummern: bool,
    pub baende: Vec<Band>,
    /// Gemeinsame Texte des Komplettumschlags.
    #[serde(default)]
    pub reihe: Reihe,
}

impl Profil {
    /// Liest `buchbau.yaml` aus dem Buchordner.
    pub fn laden(buchordner: &Path) -> Result<Self, String> {
        let pfad = buchordner.join(DATEI);
        let text = std::fs::read_to_string(&pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
        serde_yaml::from_str(&text).map_err(|e| format!("{}: {e}", pfad.display()))
    }

    /// Gibt es im Buchordner ein Profil?
    pub fn vorhanden(buchordner: &Path) -> bool {
        buchordner.join(DATEI).is_file()
    }

    /// Sucht das Profil vom Bandordner aus (im Bandordner selbst oder eine Ebene darüber).
    /// Liefert Buchordner und Profil.
    pub fn suchen(bandordner: &Path) -> Option<(PathBuf, Self)> {
        [Some(bandordner), bandordner.parent()]
            .into_iter()
            .flatten()
            .find(|d| Self::vorhanden(d))
            .and_then(|d| Self::laden(d).ok().map(|p| (d.to_path_buf(), p)))
    }

    /// Der Band mit dieser Nummer.
    pub fn band(&self, nummer: u32) -> Option<&Band> {
        self.baende.iter().find(|b| b.nummer == nummer)
    }

    /// Der Band, dessen Ordner `ordner` ist.
    pub fn band_zu_ordner(&self, buchordner: &Path, ordner: &Path) -> Option<&Band> {
        let ziel = ordner.canonicalize().ok();
        self.baende.iter().find(|b| buchordner.join(&b.ordner).canonicalize().ok() == ziel)
    }

    /// Rubriken als (Titelanfang, Klasse) für den Übersetzer.
    pub fn rubriken_paare(&self) -> Vec<(String, String)> {
        self.rubriken.iter().map(|r| (r.titel.clone(), r.art.clone())).collect()
    }

    /// Als YAML (für das Anlegen des Profils).
    pub fn als_yaml(&self) -> String {
        let mut s = String::from(
            "# Buchbau der Werkbank: Angaben, die bisher im Bauskript standen.\n\
             # Der Klappentext eines Bandes steht in <Bandordner>/klappentext.md.\n",
        );
        s.push_str(&serde_yaml::to_string(self).unwrap_or_default());
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hin_und_zurueck() {
        let p = Profil {
            titel: "Buch".into(),
            autor: "A".into(),
            sprache: "de".into(),
            verlag: "V".into(),
            format_mm: [170.0, 240.0],
            raender_mm: Raender { innen: 22.0, aussen: 18.0, oben: 20.0, unten: 22.0 },
            quellen: vec!["titelei.md".into(), "teil-1.md".into()],
            rubriken: vec![Rubrik { titel: "Bughunter".into(), art: "bughunter".into() }],
            ausgabe: "build".into(),
            webseiten: vec!["x.de".into()],
            zeilennummern: false,
            baende: vec![Band {
                nummer: 1,
                ordner: "Band_1".into(),
                untertitel: "Band 1".into(),
                uuid: "urn:uuid:1".into(),
                kurzname: "buch-band1".into(),
                umschlag: Umschlag { merkmale: vec!["a".into()], ..Default::default() },
            }],
            reihe: Reihe { titel: "Die [A] B".into(), ..Default::default() },
        };
        let y = p.als_yaml();
        let zurueck: Profil = serde_yaml::from_str(&y).unwrap();
        assert_eq!(p, zurueck);
        // Fehlende Felder mit Voreinstellung
        let knapp: Profil = serde_yaml::from_str(
            "titel: B\nautor: A\nverlag: V\nformat_mm: [170, 240]\nraender_mm: {innen: 22, aussen: 18, oben: 20, unten: 22}\nquellen: [a.md]\nrubriken: []\nbaende: []\n",
        )
        .unwrap();
        assert_eq!(knapp.ausgabe, "build");
        assert_eq!(knapp.sprache, "de");
    }
}
