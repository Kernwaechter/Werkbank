//! # Dateisystem-Ein- und Ausgabe für Buchprojekte
//!
//! Liest Buchverzeichnisse ein (mit `reihenfolge.txt` oder Standard-Teil-Dateien),
//! zerlegt mehrkapitelige Teil-Dateien sauber an den `## `-Überschriften und speichert
//! Änderungen atomar mit Zeitstempel-Snapshots in `.werkbank/snapshots/`.

use crate::{BandMetadaten, Kapitel, Projekt, Teil};
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};

impl Teil {
    /// Liefert den gesamten rekonstruierten Dateiinhalt dieses Teils zurück.
    ///
    /// Bei unveränderten Teilen entspricht das byte-genau der Datei auf der Platte.
    pub fn rekonstruiere_datei_inhalt(&self) -> String {
        let mut puffer = self.praeambel.clone();
        for (idx, kap) in self.kapitel.iter().enumerate() {
            // Zwischen zwei Kapiteln muss ein Zeilenumbruch stehen, sonst verschmelzen die Überschriften.
            if idx > 0 && !puffer.is_empty() && !puffer.ends_with('\n') {
                puffer.push('\n');
            }
            puffer.push_str(&kap.inhalt);
        }
        puffer
    }
}

impl Projekt {
    /// Lädt ein Buchprojekt aus einem Verzeichnis auf der Festplatte.
    ///
    /// Erkennt automatisch:
    /// - `reihenfolge.txt` (wie in der Rust-Werkstatt)
    /// - Standardaufbau: `titelei.md`, `vorwort.md`, `teil-*.md`, `anhaenge.md`, `anmerkungen.md`
    pub fn aus_verzeichnis(ordner: &Path) -> io::Result<Self> {
        if !ordner.is_dir() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Verzeichnis existiert nicht: {}", ordner.display()),
            ));
        }

        let mut dateien_in_reihenfolge = Vec::new();
        let reihenfolge_datei = ordner.join("reihenfolge.txt");

        if reihenfolge_datei.is_file() {
            let inhalt = fs::read_to_string(&reihenfolge_datei)?;
            for zeile in inhalt.lines() {
                let getrimmt = zeile.trim();
                if !getrimmt.is_empty() && !getrimmt.starts_with('#') {
                    dateien_in_reihenfolge.push(getrimmt.to_string());
                }
            }
        } else {
            // Standarddateien absuchen
            let standard_kandidaten = [
                "titelei.md",
                "vorwort.md",
                "teil-1.md",
                "teil-2.md",
                "teil-3.md",
                "teil-4.md",
                "teil-5.md",
                "teil-6.md",
                "teil-7.md",
                "teil-8.md",
                "teil-9.md",
                "teil-10.md",
                "teil-11.md",
                "teil-12.md",
                "anhaenge.md",
                "anhang-loesungen.md",
                "anmerkungen.md",
            ];

            for kandidat in &standard_kandidaten {
                if ordner.join(kandidat).is_file() {
                    dateien_in_reihenfolge.push(kandidat.to_string());
                }
            }

            // Falls keine Standardkandidaten existieren, alle .md-Dateien alphabetisch listen
            if dateien_in_reihenfolge.is_empty() {
                let mut eintrags_liste = Vec::new();
                for eintrag in fs::read_dir(ordner)? {
                    let eintrag = eintrag?;
                    let pfad = eintrag.path();
                    if pfad.is_file()
                        && pfad.extension().is_some_and(|ext| ext == "md")
                        && let Some(dateiname) = pfad.file_name().and_then(|n| n.to_str())
                            && !dateiname.starts_with('.') && dateiname != "AGENTS.md" {
                                eintrags_liste.push(dateiname.to_string());
                            }
                }
                eintrags_liste.sort();
                dateien_in_reihenfolge = eintrags_liste;
            }
        }

        if dateien_in_reihenfolge.is_empty() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Keine Markdown-Dateien in {} gefunden", ordner.display()),
            ));
        }

        let mut teile = Vec::new();
        let mut fortlaufende_kapitel_nr = 1;

        for (teil_idx, relativer_dateiname) in dateien_in_reihenfolge.iter().enumerate() {
            let voller_pfad = ordner.join(relativer_dateiname);
            if !voller_pfad.is_file() {
                continue;
            }

            let inhalt = fs::read_to_string(&voller_pfad)?;
            let teil_id = format!("teil-{}", teil_idx + 1);

            let teil = teil_aus_datei_inhalt(
                &teil_id,
                relativer_dateiname,
                &inhalt,
                &mut fortlaufende_kapitel_nr,
            );
            teile.push(teil);
        }

        let metadaten = metadaten_aus_ordner(ordner);

        let mut projekt = Self {
            wurzelverzeichnis: ordner.to_path_buf(),
            metadaten,
            teile,
        };
        projekt.kapitel_meta_laden();
        projekt.metadaten_aus_band_yaml();
        Ok(projekt)
    }
}

/// Zerlegt den Inhalt einer Markdown-Datei in einen Teil und seine Kapitel.
///
/// Die Zerlegung ist verlustfrei: Präambel und Kapitelinhalte hintereinander
/// ergeben exakt den Dateiinhalt. `## ` in Codeblöcken beginnt kein Kapitel.
pub(crate) fn teil_aus_datei_inhalt(
    teil_id: &str,
    dateipfad: &str,
    inhalt: &str,
    fortlaufende_kapitel_nr: &mut usize,
) -> Teil {
    let mut teil_titel = dateipfad
        .strip_suffix(".md")
        .unwrap_or(dateipfad)
        .replace('-', " ");

    let ueberschriften = crate::markdown::ueberschriften(inhalt);
    if teil_titel.contains("teil")
        && let Some(h1) = ueberschriften.iter().find(|u| u.ebene == 1)
    {
        // Titel aus "# Teil [id]: Titel {-}" extrahieren
        teil_titel = h1.text.split("{-}").next().unwrap_or(&h1.text).trim().to_string();
    }

    let starts: Vec<usize> = ueberschriften
        .iter()
        .filter(|u| u.ebene == 2)
        .map(|u| u.byte)
        .collect();

    let mut praeambel = String::new();
    let mut kapitel = Vec::new();

    if starts.is_empty() {
        // Keine ##-Unterabschnitte: Die gesamte Datei ist ein einziges Kapitel
        let k_id = format!("kap-{:02}", *fortlaufende_kapitel_nr);
        *fortlaufende_kapitel_nr += 1;
        kapitel.push(Kapitel::neu(k_id, teil_titel.clone(), dateipfad, inhalt));
    } else {
        // Text vor der ersten ##-Zeile ist die Präambel des Teils
        praeambel = inhalt[..starts[0]].to_string();

        for (i, &start) in starts.iter().enumerate() {
            let ende = starts.get(i + 1).copied().unwrap_or(inhalt.len());
            let abschnitt = &inhalt[start..ende];
            let kopfzeile = abschnitt.lines().next().unwrap_or("").trim_start_matches("## ").trim();

            // ID und Titel aus "Kapitel [werkzeug]: Werkzeug auspacken" extrahieren
            let (k_id, k_titel) = parse_kapitel_kopf(kopfzeile, *fortlaufende_kapitel_nr);
            *fortlaufende_kapitel_nr += 1;

            kapitel.push(Kapitel::neu(k_id, k_titel, dateipfad, abschnitt));
        }
    }

    Teil {
        id: teil_id.to_string(),
        titel: teil_titel,
        praeambel,
        dateipfad: PathBuf::from(dateipfad),
        kapitel,
    }
}

/// Parst eine "## "-Kopfzeile in ID und lesbaren Titel.
pub(crate) fn parse_kapitel_kopf(kopfzeile: &str, standard_nr: usize) -> (String, String) {
    // Beispiel: "Kapitel [werkzeug]: Werkzeug auspacken"
    if let Some(open) = kopfzeile.find('[')
        && let Some(close) = kopfzeile.find(']')
        && close > open
    {
        let id = kopfzeile[open + 1..close].trim().to_string();
        let rest = &kopfzeile[close + 1..];
        let sauber_titel = rest.trim_start_matches(':').trim();
        let titel = if sauber_titel.is_empty() {
            kopfzeile.to_string()
        } else {
            sauber_titel.to_string()
        };
        (id, titel)
    } else {
        let fallback_id = format!("kap-{:02}", standard_nr);
        (fallback_id, kopfzeile.to_string())
    }
}

/// Liest Metadaten aus dem Ordner (z. B. aus `preisempfehlung.md` oder Verzeichnisname).
fn metadaten_aus_ordner(ordner: &Path) -> BandMetadaten {
    let ordner_name = ordner
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Band 1");

    let eltern_name = ordner
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Werkstatt");

    let mut titel = if eltern_name.contains("Flutter") {
        "Die Flutter- & Dart-Werkstatt".to_string()
    } else {
        "Die Rust-Werkstatt".to_string()
    };

    let mut untertitel = "Vom Lehrling zum Gesellen".to_string();
    let band_bezeichnung = ordner_name.to_string();

    // Versuche preisempfehlung.md zu parsen
    let preis_datei = ordner.join("preisempfehlung.md");
    if preis_datei.is_file()
        && let Ok(inhalt) = fs::read_to_string(preis_datei)
    {
        for zeile in inhalt.lines() {
            if let Some(rest) = zeile.strip_prefix("Projekt:") {
                let p = rest.trim().trim_matches('*');
                if let Some((t, u)) = p.split_once(':') {
                    titel = t.trim().to_string();
                    untertitel = u.trim().to_string();
                } else {
                    titel = p.to_string();
                }
            }
        }
    }

    BandMetadaten {
        titel,
        untertitel,
        band_bezeichnung,
        autor: "Volker Dähn".to_string(),
        jahr: "2026".to_string(),
        isbn_print: Some("978-3-9825412-0-4".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kapitel_kopf() {
        let (id, titel) = parse_kapitel_kopf("Kapitel [werkzeug]: Werkzeug auspacken", 1);
        assert_eq!(id, "werkzeug");
        assert_eq!(titel, "Werkzeug auspacken");

        let (id2, titel2) = parse_kapitel_kopf("Bevor wir anfangen", 2);
        assert_eq!(id2, "kap-02");
        assert_eq!(titel2, "Bevor wir anfangen");
    }

    #[test]
    fn test_teil_rekonstruktion() {
        let text = "# Teil 1: Grundlagen\n\nPräambel-Text hier.\n\n## Kapitel [k1]: Erstes\nInhalt 1\n\n## Kapitel [k2]: Zweites\nInhalt 2";
        let mut nr = 1;
        let teil = teil_aus_datei_inhalt("teil-1", "teil-1.md", text, &mut nr);

        assert_eq!(teil.kapitel.len(), 2);
        assert_eq!(teil.kapitel[0].id, "k1");
        assert_eq!(teil.kapitel[1].id, "k2");

        let rekonstruiert = teil.rekonstruiere_datei_inhalt();
        assert!(rekonstruiert.contains("# Teil 1: Grundlagen"));
        assert!(rekonstruiert.contains("## Kapitel [k1]: Erstes"));
        assert!(rekonstruiert.contains("## Kapitel [k2]: Zweites"));
    }

    #[test]
    fn test_echter_band1_laden() {
        let pfad = Path::new("/run/media/volker/Daten/Bücher/Rust-Werkstatt/Band 1");
        if pfad.is_dir() {
            let projekt = Projekt::aus_verzeichnis(pfad).expect("Band 1 sollte lesbar sein");
            assert!(projekt.teile.len() >= 10);
            assert!(projekt.gesamtwortzahl() > 100_000);
            assert!(projekt.kapitel_anzahl() > 50);
        }
    }
}
