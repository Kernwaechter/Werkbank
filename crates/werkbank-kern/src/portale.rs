//! # Portalregeln als Daten
//!
//! Maße und Grenzen der Verkaufsportale stehen nicht im Code, sondern in
//! `.werkbank/portale.json` (mit Quelle und Stand). Ohne Datei gelten die
//! Voreinstellungen, die der Autor gegen die Portale prüfen und anpassen soll.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// Regeln eines Portals.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portal {
    /// Anzeigename.
    pub name: String,
    /// Kurzbeschreibung der Fassungen.
    pub beschreibung: String,
    /// Cover für das E-Book in Pixeln (Breite, Höhe).
    pub cover_ebook_px: (u32, u32),
    /// Buchformat im Druck in Millimetern (Breite, Höhe).
    pub druck_format_mm: (f32, f32),
    /// Rückenstärke pro Seite in Millimetern.
    pub ruecken_mm_pro_seite: f32,
    /// Beschnittzugabe in Millimetern.
    pub beschnitt_mm: f32,
    /// Kleinste zulässige Seitenzahl im Druck.
    pub seiten_min: u32,
    /// Größte zulässige Seitenzahl im Druck.
    pub seiten_max: u32,
    /// Woher die Zahlen stammen.
    pub quelle: String,
    /// Wann sie zuletzt geprüft wurden.
    pub stand: String,
}

/// Alle Regeln eines Projekts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portalregeln {
    /// Höchste Zeichenzahl einer Codezeile im Druck.
    pub code_zeilenlaenge: usize,
    /// Rust-Edition für die Prüfung der Beispiele.
    pub edition: String,
    /// Die Portale.
    pub portale: Vec<Portal>,
}

impl Default for Portalregeln {
    fn default() -> Self {
        let hinweis = "Voreinstellung der Werkbank – gegen Portal und eigene Skripte prüfen".to_string();
        Self {
            code_zeilenlaenge: 70,
            edition: "2024".to_string(),
            portale: vec![
                Portal {
                    name: "Tolino Media".to_string(),
                    beschreibung: "E-Book (EPUB 3) und Taschenbuch, Druck in Schwarzweiß".to_string(),
                    cover_ebook_px: (1600, 2560),
                    druck_format_mm: (190.0, 270.0),
                    ruecken_mm_pro_seite: 0.0575,
                    beschnitt_mm: 3.0,
                    seiten_min: 24,
                    seiten_max: 800,
                    quelle: hinweis.clone(),
                    stand: "2026-10".to_string(),
                },
                Portal {
                    name: "Amazon KDP".to_string(),
                    beschreibung: "Kindle-E-Book und Taschenbuch, Druck in Schwarzweiß".to_string(),
                    cover_ebook_px: (1600, 2560),
                    druck_format_mm: (190.0, 270.0),
                    ruecken_mm_pro_seite: 0.0572,
                    beschnitt_mm: 3.2,
                    seiten_min: 24,
                    seiten_max: 828,
                    quelle: hinweis,
                    stand: "2026-10".to_string(),
                },
            ],
        }
    }
}

impl Portalregeln {
    /// Lädt `.werkbank/portale.json` oder liefert die Voreinstellung.
    pub fn laden(bandordner: &Path) -> Self {
        fs::read_to_string(bandordner.join(".werkbank").join("portale.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Schreibt die Regeln (zum Anpassen) in `.werkbank/portale.json`.
    pub fn speichern(&self, bandordner: &Path) -> io::Result<()> {
        let ordner = bandordner.join(".werkbank");
        fs::create_dir_all(&ordner)?;
        fs::write(ordner.join("portale.json"), serde_json::to_string_pretty(self).map_err(io::Error::other)?)
    }
}

impl Portal {
    /// Rückenbreite in Millimetern bei gegebener Seitenzahl.
    pub fn ruecken_mm(&self, seiten: u32) -> f32 {
        seiten as f32 * self.ruecken_mm_pro_seite
    }

    /// Gesamtmaß des Umschlags (Rückseite + Rücken + Vorderseite, mit Beschnitt) in Millimetern.
    pub fn umschlag_mm(&self, seiten: u32) -> (f32, f32) {
        (
            self.druck_format_mm.0 * 2.0 + self.ruecken_mm(seiten) + self.beschnitt_mm * 2.0,
            self.druck_format_mm.1 + self.beschnitt_mm * 2.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruecken_und_umschlag() {
        let p = &Portalregeln::default().portale[1];
        assert!((p.ruecken_mm(400) - 22.88).abs() < 0.01);
        let (b, h) = p.umschlag_mm(400);
        assert!((b - (380.0 + 22.88 + 6.4)).abs() < 0.01);
        assert!((h - 276.4).abs() < 0.01);
    }

    #[test]
    fn roundtrip_in_datei() {
        let o = std::env::temp_dir().join("werkbank-test-portale");
        let _ = fs::remove_dir_all(&o);
        let r = Portalregeln {
            code_zeilenlaenge: 66,
            ..Default::default()
        };
        r.speichern(&o).unwrap();
        assert_eq!(Portalregeln::laden(&o).code_zeilenlaenge, 66);
        assert_eq!(Portalregeln::laden(&o.join("nix")).code_zeilenlaenge, 70);
    }
}
