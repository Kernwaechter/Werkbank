//! # Buchbau ohne Skripte
//!
//! Ersetzt das bisherige `build_book.py` (Python, Pygments, Headless-Chrome, Ghostscript):
//! Markdown wird nach denselben Regeln übersetzt ([`md`]), die PDFs setzt das eingebaute Typst mit
//! einer Vorlage, die das bisherige Layout nachbildet, das EPUB entsteht wie bisher aus XHTML ([`epub`]).
//! Die Angaben, die im Skript fest standen (Dateien, Format, Bände, Rubriken …), stehen im
//! Buchprofil `buchbau.yaml` im Buchordner ([`profil`]).

pub mod css;
pub mod epub;
pub mod hervorhebung;
pub mod md;
pub mod pakete;
pub mod pdf;
pub mod profil;
pub mod umschlag;

use std::path::{Path, PathBuf};

/// Ein hervorgehobener Codeblock als HTML (wie `highlight_code_block` im Original).
pub fn code_html(code: &str, sprache: &str) -> String {
    let rein = if sprache.trim().is_empty() { "text".to_string() } else { sprache.trim().to_lowercase() };
    let etikett = md::sprach_etikett(&rein);
    format!(
        "<div class=\"code-block\" data-lang=\"{etikett}\"><div class=\"code-header\">{etikett}</div><pre class=\"highlight\"><code>{}</code></pre></div>",
        hervorhebung::html_spans(code, &rein)
    )
}

/// Angaben eines Bandes für die Ausgaben (aus Profil und Band zusammengeführt).
#[derive(Debug, Clone)]
pub struct BandInfo {
    pub titel: String,
    pub untertitel: String,
    pub autor: String,
    pub sprache: String,
    pub verlag: String,
    pub uuid: String,
    pub kurzname: String,
    pub css_epub: String,
}

/// Eine übersetzte Quelldatei.
#[derive(Debug, Clone)]
pub struct Abschnittsdaten {
    pub datei: String,
    pub markdown: String,
    pub abschnitt: md::Abschnitt,
    pub html: String,
    /// Die Titelei erscheint nicht im Inhaltsverzeichnis.
    pub ist_titelei: bool,
}

/// Liest und übersetzt die Quelldateien eines Bandes in Buchreihenfolge.
/// Fehlende Dateien werden wie im Original übersprungen (mit Hinweis), die Nummerierung der Anker
/// (`s1_`, `s2_` …) folgt aber der Liste im Profil.
pub fn abschnitte_lesen(bandordner: &Path, profil: &profil::Profil, hinweise: &mut Vec<String>) -> Vec<Abschnittsdaten> {
    let rubriken = profil.rubriken_paare();
    let mut aus = Vec::new();
    for (idx, datei) in profil.quellen.iter().enumerate() {
        let pfad = bandordner.join(datei);
        let Ok(markdown) = std::fs::read_to_string(&pfad) else {
            hinweise.push(format!("Datei {datei} nicht gefunden, wird übersprungen."));
            continue;
        };
        let abschnitt = md::zerlegen(&markdown, &format!("s{}_", idx + 1), &rubriken);
        let html = md::html(&abschnitt, &|c, s| code_html(c, s));
        aus.push(Abschnittsdaten { datei: datei.clone(), markdown, abschnitt, html, ist_titelei: datei == "titelei.md" });
    }
    aus
}

/// Zeitstempel wie `datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")`.
pub fn jetzt_iso() -> String {
    let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (tage, rest) = (s.div_euclid(86_400), s.rem_euclid(86_400));
    // Kalenderdatum aus Tagen seit 1970 (Algorithmus von H. Hinnant)
    let z = tage + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let tag = doy - (153 * mp + 2) / 5 + 1;
    let monat = if mp < 10 { mp + 3 } else { mp - 9 };
    let jahr = yoe + era * 400 + i64::from(monat <= 2);
    format!("{jahr:04}-{monat:02}-{tag:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// Das erste vorhandene Cover für das E-Book (Bandordner vor Buchordner, jpg vor png).
pub fn cover_finden(bandordner: &Path, buchordner: &Path) -> Option<PathBuf> {
    [bandordner, buchordner]
        .iter()
        .flat_map(|b| ["cover.jpg", "cover.jpeg", "cover.png"].map(|n| b.join(n)))
        .find(|p| p.is_file())
}

/// Dateien, die Typst als virtuelle Dateien bekommt: Farbthema für Code, die Abbildungen
/// (im Graudruck nativ in Graustufen umgerechnet) und das Cover.
fn dateien_fuer_typst(
    abschnitte: &[Abschnittsdaten],
    bandordner: &Path,
    buchordner: &Path,
    fassung: pdf::Fassung,
    cover: Option<&Path>,
    hinweise: &mut Vec<String>,
) -> std::collections::HashMap<String, Vec<u8>> {
    let mut dateien = std::collections::HashMap::new();
    dateien.insert("werkstatt.tmTheme".to_string(), hervorhebung::typst_thema(fassung == pdf::Fassung::Farbe).into_bytes());
    let mut pfade: Vec<String> = abschnitte
        .iter()
        .flat_map(|a| a.abschnitt.knoten.iter())
        .filter_map(|k| match k {
            md::Knoten::Bild { pfad, .. } => Some(pfad.clone()),
            _ => None,
        })
        .collect();
    pfade.sort();
    pfade.dedup();
    for pfad in pfade {
        // Wie im Original: erst im Bandordner, dann im Buchordner suchen
        let Some(datei) = [bandordner.join(&pfad), buchordner.join(&pfad)].into_iter().find(|p| p.is_file()) else {
            hinweise.push(format!("Abbildung nicht gefunden: {pfad}"));
            continue;
        };
        match std::fs::read(&datei) {
            Ok(daten) if fassung == pdf::Fassung::Grau => match in_graustufen(&daten, &pfad) {
                Ok(grau) => {
                    dateien.insert(pfad, grau);
                }
                Err(e) => {
                    hinweise.push(format!("{pfad}: nicht in Graustufen umgerechnet ({e})"));
                    dateien.insert(pfad, daten);
                }
            },
            Ok(daten) => {
                dateien.insert(pfad, daten);
            }
            Err(e) => hinweise.push(format!("{pfad}: {e}")),
        }
    }
    if let Some(c) = cover
        && let Ok(daten) = std::fs::read(c)
    {
        dateien.insert("cover.png".to_string(), daten);
    }
    dateien
}

/// Rechnet ein Bild in reine Graustufen um (gleiches Format wie die Datei, damit Typst es erkennt).
pub fn in_graustufen(daten: &[u8], pfad: &str) -> Result<Vec<u8>, String> {
    let bild = image::load_from_memory(daten).map_err(|e| e.to_string())?;
    let mut aus = std::io::Cursor::new(Vec::new());
    let jpeg = pfad.to_lowercase().ends_with(".jpg") || pfad.to_lowercase().ends_with(".jpeg");
    if jpeg {
        image::DynamicImage::ImageLuma8(bild.to_luma8()).write_to(&mut aus, image::ImageFormat::Jpeg).map_err(|e| e.to_string())?;
    } else if bild.color().has_alpha() {
        image::DynamicImage::ImageLumaA8(bild.to_luma_alpha8()).write_to(&mut aus, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    } else {
        image::DynamicImage::ImageLuma8(bild.to_luma8()).write_to(&mut aus, image::ImageFormat::Png).map_err(|e| e.to_string())?;
    }
    Ok(aus.into_inner())
}

/// Ein Programm im Suchpfad (für optionale Helfer wie Ghostscript).
pub fn programm_finden(namen: &[&str]) -> Option<PathBuf> {
    let endungen: &[&str] = if cfg!(windows) { &[".exe", ""] } else { &[""] };
    for d in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        for n in namen {
            for e in endungen {
                let p = d.join(format!("{n}{e}"));
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// Ergebnis eines Bauvorgangs.
#[derive(Debug, Clone, Default)]
pub struct Bericht {
    /// Erzeugte Dateien mit Größe in Bytes.
    pub dateien: Vec<(PathBuf, u64)>,
    pub hinweise: Vec<String>,
    /// Seitenzahl des Graudrucks (Buchblock für Tolino und KDP).
    pub seiten_grau: usize,
}

impl Bericht {
    fn notieren(&mut self, pfad: &Path) {
        let groesse = std::fs::metadata(pfad).map(|m| m.len()).unwrap_or(0);
        self.dateien.push((pfad.to_path_buf(), groesse));
    }
}

/// Anzahl der Schritte, die [`bauen`] über `fortschritt` meldet.
pub const SCHRITTE: usize = 5;

/// Baut einen Band: `fortschritt(text)` meldet die [`SCHRITTE`] Arbeitsschritte.
pub fn bauen(buchordner: &Path, nummer: u32, fortschritt: &dyn Fn(&str)) -> Result<Bericht, String> {
    let profil = profil::Profil::laden(buchordner)?;
    let band = profil.band(nummer).ok_or_else(|| format!("Band {nummer} steht nicht in {}", profil::DATEI))?.clone();
    let bandordner = buchordner.join(&band.ordner);
    if !bandordner.is_dir() {
        return Err(format!("Das Verzeichnis „{}“ existiert noch nicht.", bandordner.display()));
    }
    let ausgabe = buchordner.join(&profil.ausgabe);
    std::fs::create_dir_all(&ausgabe).map_err(|e| format!("{}: {e}", ausgabe.display()))?;

    let mut bericht = Bericht::default();
    fortschritt("Texte lesen und übersetzen");
    let abschnitte = abschnitte_lesen(&bandordner, &profil, &mut bericht.hinweise);
    if abschnitte.is_empty() {
        return Err(format!("Keine Markdown-Quelldateien in „{}“ gefunden.", bandordner.display()));
    }
    let info = BandInfo {
        titel: profil.titel.clone(),
        untertitel: band.untertitel.clone(),
        autor: profil.autor.clone(),
        sprache: profil.sprache.clone(),
        verlag: profil.verlag.clone(),
        uuid: band.uuid.clone(),
        kurzname: band.kurzname.clone(),
        css_epub: css::css(true, true, profil.format_mm[0], profil.format_mm[1]),
    };

    // Gesamtes Markdown als Referenz
    let gesamt = ausgabe.join(format!("{}-gesamt.md", band.kurzname));
    let text = abschnitte.iter().map(|a| a.markdown.as_str()).collect::<Vec<_>>().join("\n\n");
    std::fs::write(&gesamt, text).map_err(|e| format!("{}: {e}", gesamt.display()))?;
    bericht.notieren(&gesamt);


    let cover_png = bandordner.join("cover.png");
    for fassung in [pdf::Fassung::Farbe, pdf::Fassung::Grau] {
        let (endung, cover) = match fassung {
            pdf::Fassung::Farbe => ("print-farbe", cover_png.is_file().then_some("cover.png")),
            pdf::Fassung::Grau => ("print-sw", None),
        };
        let dateien = dateien_fuer_typst(&abschnitte, &bandordner, buchordner, fassung, cover.map(|_| cover_png.as_path()), &mut bericht.hinweise);
        fortschritt(match fassung {
            pdf::Fassung::Farbe => "Druck-PDF in Farbe setzen",
            pdf::Fassung::Grau => "Druck-PDF in Graustufen setzen",
        });
        let quelle = pdf::dokument(&profil, &band.untertitel, &abschnitte, fassung, cover, 0);
        let mut ergebnis = crate::welt::typst_zu_pdf(quelle, Some(bandordner.clone()), dateien.clone())?;
        // Graudruck: wie bisher mit Notizseiten auffüllen, bis die Seitenzahl durch 4 teilbar ist
        if fassung == pdf::Fassung::Grau && ergebnis.seiten % 4 != 0 {
            let fehlen = 4 - ergebnis.seiten % 4;
            let quelle = pdf::dokument(&profil, &band.untertitel, &abschnitte, fassung, cover, fehlen);
            ergebnis = crate::welt::typst_zu_pdf(quelle, Some(bandordner.clone()), dateien)?;
        }
        bericht.hinweise.extend(ergebnis.warnungen.iter().map(|w| format!("Satz ({endung}): {w}")));
        let ziel = ausgabe.join(format!("{}-{endung}.pdf", band.kurzname));
        std::fs::write(&ziel, &ergebnis.bytes).map_err(|e| format!("{}: {e}", ziel.display()))?;
        if fassung == pdf::Fassung::Grau && ergebnis.seiten % 4 != 0 {
            bericht.hinweise.push(format!("Graudruck hat {} Seiten (nicht durch 4 teilbar).", ergebnis.seiten));
        }
        bericht.notieren(&ziel);
        if fassung == pdf::Fassung::Grau {
            bericht.seiten_grau = ergebnis.seiten;
        }
    }

    fortschritt("EPUB 3 schreiben");
    let epub_pfad = ausgabe.join(format!("{}-ebook.epub", band.kurzname));
    let bilder = [bandordner.join("images"), buchordner.join("images")].into_iter().find(|p| p.is_dir());
    let cover = cover_finden(&bandordner, buchordner);
    epub::schreiben(&epub_pfad, &info, &abschnitte, bilder.as_deref(), cover.as_deref(), &jetzt_iso())?;
    bericht.notieren(&epub_pfad);

    fortschritt("Verlagsordner und Umschläge (tolino/, kdp/, webpublishing/)");
    let seiten = bericht.seiten_grau;
    let umschlag = |plattform: pakete::Plattform, ziel: &Path| umschlag::schreiben(&profil, &band, &bandordner, plattform, seiten, ziel);
    let eingaben = pakete::Eingaben {
        buchordner,
        bandordner: &bandordner,
        profil: &profil,
        band: &band,
        farb_pdf: &ausgabe.join(format!("{}-print-farbe.pdf", band.kurzname)),
        grau_pdf: &ausgabe.join(format!("{}-print-sw.pdf", band.kurzname)),
        epub: &epub_pfad,
        seiten,
        umschlag: &umschlag,
    };
    for d in pakete::schreiben(&eingaben, &mut bericht.hinweise)? {
        bericht.notieren(&d);
    }

    Ok(bericht)
}

#[cfg(test)]
mod tests {
    #[test]
    fn zeitstempel_hat_das_richtige_format() {
        let t = super::jetzt_iso();
        assert_eq!(t.len(), 20, "{t}");
        assert!(t.starts_with("20") && t.ends_with('Z') && t.as_bytes()[10] == b'T');
    }
}
