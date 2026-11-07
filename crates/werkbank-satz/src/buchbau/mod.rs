//! # Buchbau ohne Skripte
//!
//! Ersetzt das bisherige `build_book.py` (Python, Pygments, Headless-Chrome, Ghostscript):
//! Markdown wird nach denselben Regeln übersetzt ([`md`]), die PDFs setzt das eingebaute Typst mit
//! einer Vorlage, die das bisherige Layout nachbildet, das EPUB entsteht wie bisher aus XHTML ([`epub`]).
//! Die Angaben, die im Skript fest standen (Dateien, Format, Bände, Rubriken …), stehen im
//! Buchprofil `buchbau.yaml` im Buchordner ([`profil`]).

pub mod css;
pub mod epub;
pub mod barcode;
pub mod farben;
pub mod hervorhebung;
pub mod klappentext;
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

/// Anzahl der Schritte, die [`setzen`] über `fortschritt` meldet.
pub const SCHRITTE: usize = 5;

/// Ein Satzauftrag: welcher Band, mit welchem Profil und welcher Gestaltung.
pub struct Auftrag<'a> {
    /// Ordner, in dem Ausgabe (`profil.ausgabe`) und Verlagsordner (tolino/, kdp/, webpublishing/) entstehen.
    pub buchordner: &'a Path,
    /// Ordner mit den Markdown-Dateien des Bandes.
    pub bandordner: &'a Path,
    pub profil: &'a profil::Profil,
    pub band: &'a profil::Band,
    /// Gestaltung der Elemente (leer = Werkstatt-Layout).
    pub stil: &'a crate::gestaltung::Satzstil,
    /// Frontcover als PNG, falls im Bandordner kein `cover.png` liegt (z. B. aus dem Cover-Designer).
    pub ersatz_cover: Option<Vec<u8>>,
    /// ISBN der Druckausgabe (für den Barcode auf dem Umschlag), z. B. aus `band.yaml`.
    pub isbn: Option<String>,
}

/// ISBN (Druck) des Bandes aus `band.yaml` und `.werkbank/metadaten.yaml` (Letztere gewinnt, wie in der App).
pub fn isbn_aus_ordner(bandordner: &Path) -> Option<String> {
    let mut isbn = None;
    for datei in [bandordner.join("band.yaml"), bandordner.join(".werkbank").join("metadaten.yaml")] {
        if let Ok(text) = std::fs::read_to_string(datei)
            && let Ok(m) = serde_yaml::from_str::<werkbank_kern::BandMetadaten>(&text)
        {
            isbn = m.isbn_print.filter(|i| !i.trim().is_empty());
        }
    }
    isbn
}

/// Baut einen Band nach `buchbau.yaml` mit der Gestaltung aus seinem `.werkbank/satzstil.json`.
pub fn bauen(buchordner: &Path, nummer: u32, fortschritt: &dyn Fn(&str)) -> Result<Bericht, String> {
    let profil = profil::Profil::laden(buchordner)?;
    let band = profil.band(nummer).ok_or_else(|| format!("Band {nummer} steht nicht in {}", profil::DATEI))?.clone();
    let bandordner = buchordner.join(&band.ordner);
    let stil = crate::gestaltung::Satzstil::laden(&bandordner);
    setzen(&Auftrag { buchordner, bandordner: &bandordner, profil: &profil, band: &band, stil: &stil, ersatz_cover: None, isbn: isbn_aus_ordner(&bandordner) }, fortschritt)
}

/// Setzt einen Band: Druck-PDF in Farbe (mit Cover) und in Graustufen, EPUB 3, Verlagsordner mit Umschlägen.
/// `fortschritt(text)` meldet die [`SCHRITTE`] Arbeitsschritte.
pub fn setzen(a: &Auftrag, fortschritt: &dyn Fn(&str)) -> Result<Bericht, String> {
    let (profil, band, buchordner, bandordner) = (a.profil, a.band, a.buchordner, a.bandordner);
    if !bandordner.is_dir() {
        return Err(format!("Das Verzeichnis „{}“ existiert noch nicht.", bandordner.display()));
    }
    let ausgabe = match profil.ausgabe.trim() {
        "" | "." => buchordner.to_path_buf(),
        a => buchordner.join(a),
    };
    std::fs::create_dir_all(&ausgabe).map_err(|e| format!("{}: {e}", ausgabe.display()))?;

    let mut bericht = Bericht::default();
    fortschritt("Texte lesen und übersetzen");
    let abschnitte = abschnitte_lesen(bandordner, profil, &mut bericht.hinweise);
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
        css_epub: format!("{}{}", css::css(true, true, profil.format_mm[0], profil.format_mm[1]), css::ZUSATZ_EPUB),
    };

    // Gesamtes Markdown als Referenz
    let gesamt = ausgabe.join(format!("{}-gesamt.md", band.kurzname));
    let text = abschnitte.iter().map(|a| a.markdown.as_str()).collect::<Vec<_>>().join("\n\n");
    std::fs::write(&gesamt, text).map_err(|e| format!("{}: {e}", gesamt.display()))?;
    bericht.notieren(&gesamt);

    // Frontcover: cover.png im Bandordner, sonst das mitgegebene Bild (Cover-Designer)
    let cover_png = match (bandordner.join("cover.png"), &a.ersatz_cover) {
        (p, _) if p.is_file() => Some(p),
        (_, Some(daten)) => {
            let p = ausgabe.join(format!("{}-cover.png", band.kurzname));
            std::fs::write(&p, daten).map_err(|e| format!("{}: {e}", p.display()))?;
            Some(p)
        }
        _ => {
            bericht.hinweise.push("Kein Cover gefunden (cover.png im Bandordner oder Bereich „Cover“) – PDF und EPUB ohne Titelbild.".to_string());
            None
        }
    };
    for fassung in [pdf::Fassung::Farbe, pdf::Fassung::Grau] {
        let (endung, cover) = match fassung {
            pdf::Fassung::Farbe => ("print-farbe", cover_png.as_ref().map(|_| "cover.png")),
            pdf::Fassung::Grau => ("print-sw", None),
        };
        let dateien = dateien_fuer_typst(&abschnitte, bandordner, buchordner, fassung, cover.and(cover_png.as_deref()), &mut bericht.hinweise);
        fortschritt(match fassung {
            pdf::Fassung::Farbe => "Druck-PDF in Farbe setzen",
            pdf::Fassung::Grau => "Druck-PDF in Graustufen setzen",
        });
        let quelle = pdf::dokument(profil, &band.untertitel, &abschnitte, fassung, cover, 0, a.stil);
        let mut ergebnis = crate::welt::typst_zu_pdf(quelle, Some(bandordner.to_path_buf()), dateien.clone())?;
        // Graudruck: wie bisher mit Notizseiten auffüllen, bis die Seitenzahl durch 4 teilbar ist
        if fassung == pdf::Fassung::Grau && ergebnis.seiten % 4 != 0 {
            let fehlen = 4 - ergebnis.seiten % 4;
            let quelle = pdf::dokument(profil, &band.untertitel, &abschnitte, fassung, cover, fehlen, a.stil);
            ergebnis = crate::welt::typst_zu_pdf(quelle, Some(bandordner.to_path_buf()), dateien)?;
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
    let cover = cover_finden(bandordner, buchordner).or_else(|| cover_png.clone());
    epub::schreiben(&epub_pfad, &info, &abschnitte, bilder.as_deref(), cover.as_deref(), &jetzt_iso())?;
    bericht.notieren(&epub_pfad);

    fortschritt("Verlagsordner und Umschläge (tolino/, kdp/, webpublishing/)");
    let seiten = bericht.seiten_grau;
    let umschlag_hinweise = std::cell::RefCell::new(Vec::<String>::new());
    let umschlag = |plattform: pakete::Plattform, ziel: &Path| {
        let b = umschlag::schreiben(profil, band, bandordner, plattform, seiten, ziel, a.isbn.as_deref())?;
        for h in &b.hinweise {
            umschlag_hinweise.borrow_mut().push(format!("{}-Umschlag: {h}", plattform.name));
        }
        if !b.passt {
            umschlag_hinweise.borrow_mut().push(format!(
                "{}-Umschlag: Der Text aus klappentext.md ist {:.0} mm zu hoch für die Rückseite (auch bei {:.1} pt). Bitte kürzen.",
                plattform.name, b.ueberhang_mm, b.groesse_pt
            ));
        }
        Ok(())
    };
    let eingaben = pakete::Eingaben {
        buchordner,
        bandordner,
        profil,
        band,
        farb_pdf: &ausgabe.join(format!("{}-print-farbe.pdf", band.kurzname)),
        grau_pdf: &ausgabe.join(format!("{}-print-sw.pdf", band.kurzname)),
        epub: &epub_pfad,
        seiten,
        cover: cover_png.as_deref(),
        umschlag: &umschlag,
    };
    for d in pakete::schreiben(&eingaben, &mut bericht.hinweise)? {
        bericht.notieren(&d);
    }
    bericht.hinweise.extend(umschlag_hinweise.into_inner());

    Ok(bericht)
}

/// Satz eines einzelnen Kapitels für die Vorschau – genau wie im Buch: Profil aus `buchbau.yaml`
/// (falls der Band dazugehört), Gestaltung aus `.werkbank/satzstil.json`, farbig, Abbildungen eingebettet.
pub fn vorschau_typst(md_text: &str, bandordner: &Path) -> (String, std::collections::HashMap<String, Vec<u8>>, Vec<String>) {
    let (buchordner, profil) = profil::Profil::suchen(bandordner).unwrap_or_else(|| (bandordner.to_path_buf(), muster_profil()));
    let stil = crate::gestaltung::Satzstil::laden(bandordner);
    let abschnitt = md::zerlegen(md_text, "v_", &profil.rubriken_paare());
    let daten = vec![Abschnittsdaten { datei: String::new(), markdown: String::new(), html: String::new(), abschnitt, ist_titelei: false }];
    let mut hinweise = Vec::new();
    let dateien = dateien_fuer_typst(&daten, bandordner, &buchordner, pdf::Fassung::Farbe, None, &mut hinweise);
    let mut t = pdf::vorlage(&profil, pdf::Fassung::Farbe, "", &stil);
    t.push_str(&pdf::knoten_typst(&daten[0].abschnitt.knoten, 38));
    (t, dateien, hinweise)
}

/// Profil ohne `buchbau.yaml`: Werkstatt-Format 170 × 240 mm.
fn muster_profil() -> profil::Profil {
    profil::Profil {
        titel: String::new(),
        autor: String::new(),
        sprache: "de".into(),
        verlag: String::new(),
        format_mm: [170.0, 240.0],
        raender_mm: profil::Raender { innen: 22.0, aussen: 18.0, oben: 20.0, unten: 22.0 },
        quellen: Vec::new(),
        rubriken: Vec::new(),
        ausgabe: String::new(),
        webseiten: Vec::new(),
        zeilennummern: false,
        baende: Vec::new(),
        reihe: Default::default(),
    }
}

/// Musterseiten der Gestaltung im Werkstatt-Satz: Typst-Quelltext und virtuelle Dateien.
pub fn muster(stil: &crate::gestaltung::Satzstil, farbig: bool) -> (String, std::collections::HashMap<String, Vec<u8>>) {
    const MD: &str = "## Kapitel 1: Muster\n\nFließtext mit **fett**, *kursiv* und `Code im Text`. Ein zweiter Satz, damit der Absatz etwas länger wird und man den Blocksatz sieht.\n\n### Ein Abschnitt\n\n### Bughunter: Eine Rubrik\n\n#### Ein Unterabschnitt\n\n> Ein Zitat über mehrere Wörter, damit man den Kasten und die Schrift sieht.\n\n```dart\nvoid main() {\n  print('Hallo Werkstatt');\n}\n```\n\n| Spalte A | Spalte B |\n|---|---|\n| eins | zwei |\n\n::: tipp\nEin Tipp im Kasten.\n:::\n\n::: warnung\nEine Warnung.\n:::\n";
    let profil = profil::Profil {
        titel: "Muster".into(),
        raender_mm: profil::Raender { innen: 16.0, aussen: 16.0, oben: 14.0, unten: 16.0 },
        rubriken: vec![profil::Rubrik { titel: "Bughunter".into(), art: "bughunter".into() }],
        ..muster_profil()
    };
    let fassung = if farbig { pdf::Fassung::Farbe } else { pdf::Fassung::Grau };
    let abschnitt = md::zerlegen(MD, "m_", &profil.rubriken_paare());
    let mut t = pdf::vorlage(&profil, fassung, "", stil);
    // Ohne Seitenwechsel vor der Kapitelüberschrift, damit alles auf eine Seite passt
    t.push_str("\n#show heading.where(level: 2): it => block(sticky: true, width: 100%, below: 12.45pt, h2text(it.body))\n");
    t.push_str(&pdf::knoten_typst(&abschnitt.knoten, 38));
    let mut dateien = std::collections::HashMap::new();
    dateien.insert("werkstatt.tmTheme".to_string(), hervorhebung::typst_thema(farbig).into_bytes());
    (t, dateien)
}

#[cfg(test)]
mod tests {
    /// Ein Band ohne `buchbau.yaml`: Profil aus den Angaben, Ersatzcover, Abbildung, Zitat, Kasten, eigene Gestaltung.
    #[test]
    fn band_ohne_profil_wird_gesetzt() {
        use super::profil::*;
        let o = std::env::temp_dir().join("werkbank-test-setzen");
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(o.join("images")).unwrap();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(40, 30, image::Rgb([200, 30, 30]))).write_to(&mut png, image::ImageFormat::Png).unwrap();
        let png = png.into_inner();
        std::fs::write(o.join("images/bild.png"), &png).unwrap();
        std::fs::write(o.join("titelei.md"), "<div class=\"half-title\">\n\n# Testbuch\n\n</div>\n").unwrap();
        std::fs::write(
            o.join("teil-1.md"),
            "# Teil 1\n\n## Kapitel 1\n\nText mit `code`.\n\n> Ein Zitat\n\n::: tipp\nTipp\n:::\n\n![Ein Bild](images/bild.png)\n\n```rust\nfn main() {}\n```\n",
        )
        .unwrap();
        let band = Band { nummer: 1, ordner: ".".into(), untertitel: "Band 1".into(), uuid: "urn:uuid:1".into(), kurzname: "test".into(), umschlag: Default::default() };
        let profil = Profil {
            titel: "Testbuch".into(),
            autor: "A".into(),
            sprache: "de".into(),
            verlag: "A".into(),
            format_mm: [150.0, 210.0],
            raender_mm: Raender { innen: 20.0, aussen: 15.0, oben: 15.0, unten: 18.0 },
            quellen: vec!["titelei.md".into(), "teil-1.md".into()],
            rubriken: Vec::new(),
            ausgabe: ".".into(),
            webseiten: Vec::new(),
            zeilennummern: false,
            baende: vec![band.clone()],
            reihe: Default::default(),
        };
        let mut stil = crate::gestaltung::Satzstil::default();
        stil.setze("h1", "rand_staerke", "0".into());
        stil.setze("codeblock", "kopf", "nein".into());
        stil.setze("zitat", "fuellung", "#ffeecc".into());
        let ziel = o.join(".werkbank").join("satz");
        let a = super::Auftrag { buchordner: &ziel, bandordner: &o, profil: &profil, band: &band, stil: &stil, ersatz_cover: Some(png), isbn: Some("978-3-9825412-0-4".into()) };
        let b = super::setzen(&a, &|_| {}).unwrap_or_else(|f| panic!("{f}"));
        assert_eq!(b.seiten_grau % 4, 0);
        for d in ["test-print-farbe.pdf", "test-print-sw.pdf", "test-ebook.epub", "test-cover.png", "kdp/test-kdp-kindle-cover-1600x2560.jpg", "webpublishing/test-cover.png"] {
            assert!(ziel.join(d).is_file(), "{d} fehlt: {:?}", b.dateien);
        }
        // Ohne klappentext.md: Hinweis statt Fehler, kein Umschlag
        assert!(b.hinweise.iter().any(|h| h.contains("klappentext.md")), "{:?}", b.hinweise);
        assert!(!ziel.join("tolino/test-tolino-taschenbuch-umschlag.pdf").exists());
        assert!(!b.hinweise.iter().any(|h| h.contains("Abbildung nicht gefunden")), "{:?}", b.hinweise);

        // Mit klappentext.md entstehen beide Umschläge (Tolino mit Barcode aus der ISBN)
        std::fs::write(o.join("klappentext.md"), "Dachzeile\n\n# Schlagzeile\n\nText.\n\n- Punkt\n\n---\nweb.de\nDownload · {seiten} Seiten\n").unwrap();
        let b = super::setzen(&a, &|_| {}).unwrap_or_else(|f| panic!("{f}"));
        for d in ["tolino/test-tolino-taschenbuch-umschlag.pdf", "kdp/test-kdp-taschenbuch-umschlag.pdf"] {
            let pfad = ziel.join(d);
            assert!(std::fs::read(&pfad).unwrap_or_default().starts_with(b"%PDF"), "{d} fehlt: {:?}", b.hinweise);
        }
        assert!(!b.hinweise.iter().any(|h| h.contains("Umschlag")), "{:?}", b.hinweise);

        // Ohne gültige ISBN: Tolino-Umschlag mit weißem Feld und Hinweis, KDP ohne Hinweis
        for isbn in [None, Some("978-3-9825412-0-5".to_string())] {
            let a2 = super::Auftrag { isbn, ersatz_cover: None, ..a };
            let b = super::setzen(&a2, &|_| {}).unwrap_or_else(|f| panic!("{f}"));
            let h: Vec<_> = b.hinweise.iter().filter(|h| h.contains("Umschlag")).collect();
            assert!(h.len() == 1 && h[0].starts_with("Tolino-Umschlag") && h[0].contains("Barcode"), "{h:?}");
        }
    }

    #[test]
    fn zeitstempel_hat_das_richtige_format() {
        let t = super::jetzt_iso();
        assert_eq!(t.len(), 20, "{t}");
        assert!(t.starts_with("20") && t.ends_with('Z') && t.as_bytes()[10] == b'T');
    }
}
