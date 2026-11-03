//! # Cover-Vorlage
//!
//! Eine Vorlage mit Eingabefeldern erzeugt SVG für die Vorderseite und den
//! Gesamtumschlag (Rückseite, Rücken, Vorderseite) in den Maßen der Portale.
//! Ein Hintergrundbild kann über den ganzen Umschlag oder nur über die Vorderseite laufen
//! und wird so skaliert, dass es die Fläche füllt (zugeschnitten, nie verzerrt). Jeder Text hat
//! eine eigene Farbe; Rücken und Rückseite können in einem Kasten mit wählbarer Deckkraft stehen.
//! Das SVG wird in der App mit `resvg` zu PNG gerendert.

use crate::portale::Portal;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

/// Eingaben der Cover-Vorlage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CoverDaten {
    pub titel: String,
    pub untertitel: String,
    pub bandzeile: String,
    pub autor: String,
    /// Hintergrundfarbe als `#rrggbb`.
    pub hintergrund: String,
    /// Standard-Textfarbe (für Texte ohne eigene Farbe).
    pub text: String,
    /// Akzentfarbe für Linien und (ohne eigene Farbe) die Bandzeile.
    pub akzent: String,
    /// Eigene Farben je Text; leer = Standard.
    pub farbe_bandzeile: String,
    pub farbe_titel: String,
    pub farbe_untertitel: String,
    pub farbe_autor: String,
    pub farbe_rueckseite: String,
    pub farbe_ruecken: String,
    /// Pfad eines Hintergrundbildes (PNG oder JPG), leer = keines.
    pub bild: String,
    /// `umschlag` (Bild über Rückseite, Rücken und Vorderseite) oder `vorderseite` (nur vorn).
    pub bild_modus: String,
    /// Welcher Teil des Bildes sichtbar bleibt, wenn es zugeschnitten wird: `links`, `mitte`, `rechts`.
    pub bild_ausrichtung: String,
    /// Wie stark das Bild zur Hintergrundfarbe hin abgedunkelt wird (0 bis 0,9).
    pub bild_abdunkeln: f32,
    /// Kasten hinter dem Rückentext: Farbe und Deckkraft (0 = unsichtbar, 1 = deckend).
    pub ruecken_kasten_farbe: String,
    pub ruecken_kasten_deckkraft: f32,
    /// Kasten hinter dem Text der Rückseite.
    pub rueckseite_kasten_farbe: String,
    pub rueckseite_kasten_deckkraft: f32,
    /// Seitenzahl für die Rückenbreite (0 = Schätzung aus dem Buch).
    pub seiten: u32,
    /// Freifläche für den Barcode auf der Rückseite.
    pub barcode: bool,
    /// Text der Rückseite (leer = `klappentext.md`).
    pub klappentext: String,
}

impl Default for CoverDaten {
    fn default() -> Self {
        Self {
            titel: "Die Rust-Werkstatt".to_string(),
            untertitel: "Vom Lehrling zum Gesellen".to_string(),
            bandzeile: "Band 1".to_string(),
            autor: "Volker Dähn".to_string(),
            hintergrund: "#1b1c1f".to_string(),
            text: "#f5f4f0".to_string(),
            akzent: "#7fa6c9".to_string(),
            farbe_bandzeile: String::new(),
            farbe_titel: String::new(),
            farbe_untertitel: String::new(),
            farbe_autor: String::new(),
            farbe_rueckseite: String::new(),
            farbe_ruecken: String::new(),
            bild: String::new(),
            bild_modus: "umschlag".to_string(),
            bild_ausrichtung: "mitte".to_string(),
            bild_abdunkeln: 0.35,
            ruecken_kasten_farbe: "#000000".to_string(),
            ruecken_kasten_deckkraft: 0.55,
            rueckseite_kasten_farbe: "#000000".to_string(),
            rueckseite_kasten_deckkraft: 0.45,
            seiten: 0,
            barcode: true,
            klappentext: String::new(),
        }
    }
}

impl CoverDaten {
    /// Lädt `.werkbank/cover.json`.
    pub fn laden(bandordner: &Path) -> Option<Self> {
        fs::read_to_string(bandordner.join(".werkbank").join("cover.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
    }

    /// Speichert nach `.werkbank/cover.json`.
    pub fn speichern(&self, bandordner: &Path) -> io::Result<()> {
        let ordner = bandordner.join(".werkbank");
        fs::create_dir_all(&ordner)?;
        fs::write(ordner.join("cover.json"), serde_json::to_string_pretty(self).map_err(io::Error::other)?)
    }

    fn bild_nur_vorn(&self) -> bool {
        self.bild_modus == "vorderseite"
    }

    fn ausrichtung(&self) -> &'static str {
        match self.bild_ausrichtung.as_str() {
            "links" => "xMinYMid slice",
            "rechts" => "xMaxYMid slice",
            _ => "xMidYMid slice",
        }
    }

    fn abdunkeln(&self) -> f32 {
        self.bild_abdunkeln.clamp(0.0, 0.9)
    }
}

/// Nur gültige Farben (`#rgb`/`#rrggbb`) gelangen ins SVG, alles andere wird zum Ersatzwert.
pub fn farbe(eingabe: &str, ersatz: &str) -> String {
    let e = eingabe.trim();
    let hex = e.strip_prefix('#').unwrap_or(e);
    if matches!(hex.len(), 3 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        format!("#{hex}")
    } else {
        ersatz.to_string()
    }
}

fn maskieren(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Bricht Text grob nach Zeichenbreite um.
pub fn umbrechen(text: &str, max_zeichen: usize) -> Vec<String> {
    let mut zeilen = Vec::new();
    for absatz in text.split('\n') {
        let mut zeile = String::new();
        for wort in absatz.split_whitespace() {
            if !zeile.is_empty() && zeile.chars().count() + 1 + wort.chars().count() > max_zeichen {
                zeilen.push(std::mem::take(&mut zeile));
            }
            if !zeile.is_empty() {
                zeile.push(' ');
            }
            zeile.push_str(wort);
        }
        zeilen.push(zeile);
    }
    zeilen
}

const SCHRIFT: &str = "Libertinus Serif, DejaVu Serif, Georgia, serif";
const SCHRIFT_GROTESK: &str = "DejaVu Sans, Liberation Sans, Arial, sans-serif";

#[allow(clippy::too_many_arguments)]
fn text_zeilen(zeilen: &[String], x: f32, y: f32, groesse: f32, farbe: &str, familie: &str, gewicht: &str, abstand: f32, anker: &str) -> String {
    zeilen
        .iter()
        .enumerate()
        .map(|(i, z)| {
            format!(
                r#"<text x="{x:.1}" y="{:.1}" font-size="{groesse:.1}" fill="{farbe}" font-family="{familie}" font-weight="{gewicht}" text-anchor="{anker}">{}</text>"#,
                y + i as f32 * groesse * abstand,
                maskieren(z)
            )
        })
        .collect()
}

/// Maße des Gesamtumschlags in Millimetern.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometrie {
    /// Gesamtbreite und -höhe inklusive Beschnitt.
    pub gesamt: (f32, f32),
    /// Beschnittzugabe.
    pub beschnitt: f32,
    /// Format einer Buchseite (Breite, Höhe).
    pub format: (f32, f32),
    /// Beginn und Breite des Rückens.
    pub ruecken_x: f32,
    pub ruecken_b: f32,
    /// Beginn und Breite der Vorderseite (mit rechtem Beschnitt).
    pub vorn_x: f32,
    pub vorn_b: f32,
}

/// Berechnet die Aufteilung des Umschlags.
pub fn geometrie(portal: &Portal, seiten: u32) -> Geometrie {
    let gesamt = portal.umschlag_mm(seiten);
    let (fb, _) = portal.druck_format_mm;
    let ruecken_b = portal.ruecken_mm(seiten);
    let ruecken_x = portal.beschnitt_mm + fb;
    Geometrie {
        gesamt,
        beschnitt: portal.beschnitt_mm,
        format: portal.druck_format_mm,
        ruecken_x,
        ruecken_b,
        vorn_x: ruecken_x + ruecken_b,
        vorn_b: fb + portal.beschnitt_mm,
    }
}

/// Nur die Texte der Vorderseite (ohne Hintergrund) in einer Zeichenfläche `b` × `h`.
fn vorderseite_texte(d: &CoverDaten, b: f32, h: f32) -> String {
    let (tx, ak) = (farbe(&d.text, "#f5f4f0"), farbe(&d.akzent, "#7fa6c9"));
    let f_band = farbe(&d.farbe_bandzeile, &ak);
    let f_titel = farbe(&d.farbe_titel, &tx);
    let f_unter = farbe(&d.farbe_untertitel, &tx);
    let f_autor = farbe(&d.farbe_autor, &tx);
    let mitte = b / 2.0;
    let mut s = text_zeilen(&[d.bandzeile.to_uppercase()], mitte, h * 0.09, b * 0.036, &f_band, SCHRIFT_GROTESK, "bold", 1.0, "middle");
    // Schriftgröße so wählen, dass auch das längste Wort in die Breite passt (Serifen fett: ~0.62 em je Zeichen)
    let laengstes = d.titel.split_whitespace().map(|w| w.chars().count()).max().unwrap_or(1).max(1) as f32;
    let titel_groesse = (b * 0.115).min(b * 0.84 / (laengstes * 0.62));
    let titel = umbrechen(&d.titel, ((b * 0.86) / (titel_groesse * 0.55)).max(6.0) as usize);
    let titel_y = h * 0.30;
    s.push_str(&text_zeilen(&titel, mitte, titel_y, titel_groesse, &f_titel, SCHRIFT, "bold", 1.1, "middle"));
    let unter_y = titel_y + titel.len() as f32 * titel_groesse * 1.1 + h * 0.03;
    s.push_str(&format!(
        r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{ak}"/>"#,
        mitte - b * 0.12,
        unter_y - titel_groesse * 0.35,
        b * 0.24,
        (b * 0.006).max(1.0)
    ));
    let unter_g = b * 0.045;
    let unter = umbrechen(&d.untertitel, ((b * 0.8) / (unter_g * 0.5)).max(8.0) as usize);
    s.push_str(&text_zeilen(&unter, mitte, unter_y + unter_g * 1.2, unter_g, &f_unter, SCHRIFT, "normal", 1.3, "middle"));
    s.push_str(&text_zeilen(std::slice::from_ref(&d.autor), mitte, h * 0.92, b * 0.05, &f_autor, SCHRIFT, "normal", 1.0, "middle"));
    s
}

/// Pixelmaße einer PNG- oder JPEG-Datei aus den Kopfdaten.
pub fn bildmasse(pfad: &str) -> Option<(u32, u32)> {
    use std::io::Read;
    // Die Kopfdaten genügen; große Bilder werden nicht ganz gelesen
    let mut d = Vec::new();
    fs::File::open(pfad).ok()?.take(4 << 20).read_to_end(&mut d).ok()?;
    if d.len() > 24 && d.starts_with(&[0x89, b'P', b'N', b'G']) {
        let b = u32::from_be_bytes(d[16..20].try_into().ok()?);
        let h = u32::from_be_bytes(d[20..24].try_into().ok()?);
        return Some((b, h));
    }
    if d.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2;
        while i + 9 < d.len() {
            if d[i] != 0xFF {
                i += 1;
                continue;
            }
            let marke = d[i + 1];
            if marke == 0xFF {
                i += 1;
                continue;
            }
            if (0xC0..=0xCF).contains(&marke) && !matches!(marke, 0xC4 | 0xC8 | 0xCC) {
                let h = u16::from_be_bytes([d[i + 5], d[i + 6]]) as u32;
                let b = u16::from_be_bytes([d[i + 7], d[i + 8]]) as u32;
                return Some((b, h));
            }
            let laenge = u16::from_be_bytes([d[i + 2], d[i + 3]]) as usize;
            i += 2 + laenge;
        }
    }
    None
}

fn bild_uri(pfad: &str) -> Option<String> {
    use std::fmt::Write;
    let daten = fs::read(pfad).ok()?;
    let mime = if pfad.to_lowercase().ends_with(".png") { "image/png" } else { "image/jpeg" };
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut aus = String::with_capacity(daten.len() * 4 / 3 + 32);
    write!(aus, "data:{mime};base64,").ok()?;
    for block in daten.chunks(3) {
        let n = block.iter().enumerate().fold(0u32, |a, (i, b)| a | (*b as u32) << (16 - 8 * i));
        for i in 0..4 {
            if i <= block.len() {
                aus.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                aus.push('=');
            }
        }
    }
    Some(aus)
}

/// Bildelement, das die Fläche `x, y, b, h` füllt (zugeschnitten, nicht verzerrt), plus Abdunklung.
fn bildflaeche(d: &CoverDaten, uri: &str, x: f32, y: f32, b: f32, h: f32, bg: &str) -> String {
    format!(
        r#"<image href="{uri}" x="{x:.2}" y="{y:.2}" width="{b:.2}" height="{h:.2}" preserveAspectRatio="{}"/><rect x="{x:.2}" y="{y:.2}" width="{b:.2}" height="{h:.2}" fill="{bg}" opacity="{:.2}"/>"#,
        d.ausrichtung(),
        d.abdunkeln()
    )
}

/// SVG der Vorderseite für das E-Book im Seitenverhältnis `breite` × `hoehe` (z. B. 1600 × 2560 Pixel).
pub fn svg_vorderseite(d: &CoverDaten, breite: f32, hoehe: f32) -> String {
    svg_ebook(d, breite, hoehe, None)
}

/// Wie [`svg_vorderseite`]. Mit `umschlag = Some((Portal, Seiten))` und Bildmodus „umschlag“ zeigt das E-Book-Cover
/// genau den Ausschnitt der Vorderseite aus dem Umschlagbild, passend zum Druck.
pub fn svg_ebook(d: &CoverDaten, breite: f32, hoehe: f32, umschlag: Option<(&Portal, u32)>) -> String {
    let bg = farbe(&d.hintergrund, "#1b1c1f");
    let mut s = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{breite}" height="{hoehe}" viewBox="0 0 {breite} {hoehe}"><rect width="{breite}" height="{hoehe}" fill="{bg}"/>"#
    );
    if let Some(uri) = (!d.bild.is_empty()).then(|| bild_uri(&d.bild)).flatten() {
        match umschlag {
            Some((p, seiten)) if !d.bild_nur_vorn() => {
                let g = geometrie(p, seiten);
                // Ausschnitt der Vorderseite aus dem Umschlagbild, auf die E-Book-Fläche eingepasst
                s.push_str(&format!(
                    r#"<svg x="0" y="0" width="{breite}" height="{hoehe}" viewBox="{:.2} 0 {:.2} {:.2}" preserveAspectRatio="xMidYMid slice">{}</svg>"#,
                    g.vorn_x,
                    g.vorn_b,
                    g.gesamt.1,
                    bildflaeche(d, &uri, 0.0, 0.0, g.gesamt.0, g.gesamt.1, &bg)
                ));
            }
            _ => s.push_str(&bildflaeche(d, &uri, 0.0, 0.0, breite, hoehe, &bg)),
        }
    }
    s.push_str(&vorderseite_texte(d, breite, hoehe));
    s.push_str("</svg>");
    s
}

/// SVG des Gesamtumschlags in Millimetern: Rückseite, Rücken und Vorderseite samt Beschnitt.
pub fn svg_umschlag(d: &CoverDaten, portal: &Portal, seiten: u32, klappentext: &str) -> String {
    let g = geometrie(portal, seiten);
    let (gb, gh) = g.gesamt;
    let (fb, fh) = g.format;
    let bg = farbe(&d.hintergrund, "#1b1c1f");
    let tx = farbe(&d.text, "#f5f4f0");
    let ak = farbe(&d.akzent, "#7fa6c9");
    let uri = (!d.bild.is_empty()).then(|| bild_uri(&d.bild)).flatten();

    let mut s = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="{gb:.2}mm" height="{gh:.2}mm" viewBox="0 0 {gb:.2} {gh:.2}"><rect width="{gb:.2}" height="{gh:.2}" fill="{bg}"/>"#
    );
    // Hintergrundbild: über den ganzen Umschlag oder nur über die Vorderseite
    if let Some(u) = &uri {
        if d.bild_nur_vorn() {
            s.push_str(&bildflaeche(d, u, g.vorn_x, 0.0, g.vorn_b, gh, &bg));
        } else {
            s.push_str(&bildflaeche(d, u, 0.0, 0.0, gb, gh, &bg));
        }
    }

    // Rückseite: Text in einem Kasten mit wählbarer Deckkraft
    let text_b = fb - 24.0;
    let roh = if klappentext.trim().is_empty() { &d.klappentext } else { klappentext };
    let zeilen: Vec<String> = umbrechen(roh, (text_b / 2.1).max(20.0) as usize).into_iter().take(40).collect();
    if !zeilen.is_empty() && !roh.trim().is_empty() {
        let kasten = farbe(&d.rueckseite_kasten_farbe, "#000000");
        s.push_str(&format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="2" fill="{kasten}" fill-opacity="{:.2}"/>"#,
            g.beschnitt + 6.0,
            g.beschnitt + 20.0,
            fb - 12.0,
            zeilen.len() as f32 * 4.0 * 1.45 + 10.0,
            d.rueckseite_kasten_deckkraft.clamp(0.0, 1.0)
        ));
    }
    s.push_str(&text_zeilen(&zeilen, g.beschnitt + 12.0, g.beschnitt + 30.0, 4.0, &farbe(&d.farbe_rueckseite, &tx), SCHRIFT, "normal", 1.45, "start"));
    if d.barcode {
        // Freifläche für den Barcode: 50 × 30 mm, 10 mm vom Rand
        let (bx, by) = (g.beschnitt + fb - 60.0, g.beschnitt + fh - 40.0);
        s.push_str(&format!(r##"<rect x="{bx:.1}" y="{by:.1}" width="50" height="30" fill="#ffffff"/>"##));
    }

    // Rücken: Kasten über die ganze Höhe mit wählbarer Deckkraft, darin der Text
    if g.ruecken_b >= 6.0 {
        let kasten = farbe(&d.ruecken_kasten_farbe, "#000000");
        s.push_str(&format!(
            r#"<rect x="{:.2}" y="0" width="{:.2}" height="{gh:.2}" fill="{kasten}" fill-opacity="{:.2}"/>"#,
            g.ruecken_x,
            g.ruecken_b,
            d.ruecken_kasten_deckkraft.clamp(0.0, 1.0)
        ));
        let mitte = g.ruecken_x + g.ruecken_b / 2.0;
        let schrift = (g.ruecken_b * 0.45).min(9.0);
        s.push_str(&format!(
            r#"<g transform="translate({mitte:.2} {:.2}) rotate(90)"><text font-size="{schrift:.2}" fill="{}" font-family="{SCHRIFT}" font-weight="bold" text-anchor="start" dominant-baseline="central">{}</text></g>"#,
            g.beschnitt + 18.0,
            farbe(&d.farbe_ruecken, &tx),
            maskieren(&format!("{}  ·  {}", d.titel, d.autor))
        ));
        s.push_str(&format!(
            r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="1" fill="{ak}"/>"#,
            g.ruecken_x + g.ruecken_b * 0.2,
            g.beschnitt + fh - 22.0,
            g.ruecken_b * 0.6
        ));
    }

    // Vorderseite: nur die Texte (Hintergrund liegt schon darunter)
    s.push_str(&format!(
        r#"<svg x="{:.2}" y="0" width="{:.2}" height="{gh:.2}" viewBox="0 0 {:.2} {gh:.2}">{}</svg>"#,
        g.vorn_x,
        g.vorn_b,
        g.vorn_b,
        vorderseite_texte(d, g.vorn_b, gh)
    ));
    s.push_str("</svg>");
    s
}

/// Eine Anforderung an das Hintergrundbild.
#[derive(Debug, Clone, PartialEq)]
pub struct BildAnforderung {
    /// Wofür das Bild gebraucht wird.
    pub ziel: String,
    /// Mindestgröße in Pixeln (Breite, Höhe).
    pub benoetigt: (u32, u32),
    /// Erläuterung (Maße in Millimetern, Auflösung).
    pub erlaeuterung: String,
}

fn px_bei(mm: f32, dpi: f32) -> u32 {
    (mm / 25.4 * dpi).round() as u32
}

/// Welche Bildgröße die Portale für den gewählten Bildmodus verlangen (300 dpi im Druck).
pub fn anforderungen(portale: &[Portal], seiten: u32, nur_vorderseite: bool) -> Vec<BildAnforderung> {
    let mut liste = Vec::new();
    for p in portale {
        let g = geometrie(p, seiten);
        if nur_vorderseite {
            liste.push(BildAnforderung {
                ziel: format!("Vorderseite {} (Druck)", p.name),
                benoetigt: (px_bei(g.vorn_b, 300.0), px_bei(g.gesamt.1, 300.0)),
                erlaeuterung: format!("{:.1} × {:.1} mm mit Beschnitt bei 300 dpi", g.vorn_b, g.gesamt.1),
            });
        } else {
            liste.push(BildAnforderung {
                ziel: format!("Gesamtumschlag {}", p.name),
                benoetigt: (px_bei(g.gesamt.0, 300.0), px_bei(g.gesamt.1, 300.0)),
                erlaeuterung: format!("{:.1} × {:.1} mm (Rückseite, Rücken {:.1} mm, Vorderseite, Beschnitt) bei 300 dpi", g.gesamt.0, g.gesamt.1, g.ruecken_b),
            });
        }
        liste.push(BildAnforderung {
            ziel: format!("E-Book-Cover {}", p.name),
            benoetigt: p.cover_ebook_px,
            erlaeuterung: format!("{} × {} px, Seitenverhältnis {:.2}:1 (Höhe:Breite)", p.cover_ebook_px.0, p.cover_ebook_px.1, p.cover_ebook_px.1 as f32 / p.cover_ebook_px.0 as f32),
        });
    }
    liste
}

/// Bewertung eines vorhandenen Bildes.
#[derive(Debug, Clone, PartialEq)]
pub struct BildBewertung {
    pub ziel: String,
    /// 0 = nicht ausreichend, 1 = hochskaliert/knapp, 2 = ausreichend.
    pub stufe: u8,
    pub text: String,
}

/// Prüft ein Bild (Breite, Höhe in Pixeln) gegen die Anforderungen. Das Bild wird zugeschnitten skaliert,
/// deshalb zählt die Seite, die beim Füllen der Fläche begrenzt.
pub fn bewerten(bild: (u32, u32), portale: &[Portal], seiten: u32, nur_vorderseite: bool) -> Vec<BildBewertung> {
    let (iw, ih) = (bild.0 as f32, bild.1 as f32);
    let mut liste = Vec::new();
    for p in portale {
        let g = geometrie(p, seiten);
        let (fl_b, fl_h, ziel) = if nur_vorderseite {
            (g.vorn_b, g.gesamt.1, format!("Vorderseite {} (Druck)", p.name))
        } else {
            (g.gesamt.0, g.gesamt.1, format!("Gesamtumschlag {}", p.name))
        };
        // Die Fläche wird gefüllt: begrenzend ist die Seite mit dem kleineren Pixel-pro-Millimeter-Wert
        let dpi = (iw / (fl_b / 25.4)).min(ih / (fl_h / 25.4));
        let (stufe, hinweis) = if dpi >= 299.0 {
            (2, "ausreichend".to_string())
        } else if dpi >= 200.0 {
            (1, "wird etwas hochskaliert, der Druck kann weicher wirken".to_string())
        } else {
            (0, "zu klein, das Bild würde sichtbar unscharf".to_string())
        };
        liste.push(BildBewertung { ziel, stufe, text: format!("{dpi:.0} dpi effektiv – {hinweis}") });

        // E-Book: Ausschnitt der Vorderseite, soll nicht vergrößert werden müssen
        let region_b = if nur_vorderseite { iw } else { iw * g.vorn_b / g.gesamt.0 };
        let (ew, eh) = (p.cover_ebook_px.0 as f32, p.cover_ebook_px.1 as f32);
        let faktor = (ew / region_b).max(eh / ih);
        let (stufe, hinweis) = if faktor <= 1.0 {
            (2, "ausreichend".to_string())
        } else if faktor <= 1.5 {
            (1, format!("wird {faktor:.1}-fach vergrößert"))
        } else {
            (0, format!("müsste {faktor:.1}-fach vergrößert werden, zu unscharf"))
        };
        liste.push(BildBewertung { ziel: format!("E-Book-Cover {}", p.name), stufe, text: hinweis });
    }
    liste
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::portale::Portalregeln;

    fn portal() -> Portal {
        Portalregeln::default().portale[1].clone()
    }

    #[test]
    fn farben_werden_geprueft() {
        assert_eq!(farbe("#ABC", "#000000"), "#ABC");
        assert_eq!(farbe("00875a", "#000000"), "#00875a");
        assert_eq!(farbe("red\" onload=\"x", "#123456"), "#123456");
    }

    #[test]
    fn umbruch() {
        assert_eq!(umbrechen("Die Rust Werkstatt", 9), ["Die Rust", "Werkstatt"]);
        assert_eq!(umbrechen("a\n\nb", 10), ["a", "", "b"]);
    }

    #[test]
    fn vorderseite_enthaelt_texte_maskiert_und_eigene_farben() {
        let d = CoverDaten {
            titel: "Rust & <Co>".into(),
            farbe_titel: "#ff0000".into(),
            farbe_autor: "#00ff00".into(),
            ..Default::default()
        };
        let svg = svg_vorderseite(&d, 1600.0, 2560.0);
        assert!(svg.contains("Rust &amp; &lt;Co&gt;"));
        assert!(svg.contains("BAND 1"));
        assert!(svg.contains(r##"fill="#ff0000" font-family="Libertinus"##), "Titelfarbe");
        assert!(svg.contains(r##"fill="#00ff00""##), "Autorfarbe");
        assert!(!svg.contains("<Co>"));
    }

    #[test]
    fn umschlag_hat_massangaben_barcode_ruecken_und_kaesten() {
        let p = portal();
        let d = CoverDaten {
            ruecken_kasten_deckkraft: 0.25,
            farbe_ruecken: "#ffcc00".into(),
            rueckseite_kasten_deckkraft: 0.8,
            ..Default::default()
        };
        let svg = svg_umschlag(&d, &p, 400, "Ein Klappentext über Rust.");
        let (b, h) = p.umschlag_mm(400);
        assert!(svg.contains(&format!(r#"width="{b:.2}mm""#)));
        assert!(svg.contains(&format!(r#"height="{h:.2}mm""#)));
        assert!(svg.contains("Ein Klappentext"));
        assert!(svg.contains(r##"width="50" height="30" fill="#ffffff""##), "Barcode-Fläche");
        assert!(svg.contains("rotate(90)"), "Rückentext");
        assert!(svg.contains(r#"fill-opacity="0.25""#), "Deckkraft des Rückenkastens");
        assert!(svg.contains(r#"fill-opacity="0.80""#), "Deckkraft des Rückseitenkastens");
        assert!(svg.contains(r##"fill="#ffcc00""##), "Farbe des Rückentextes");
        let duenn = svg_umschlag(&d, &p, 40, "");
        assert!(!duenn.contains("rotate(90)"), "zu schmaler Rücken bekommt keinen Text");
    }

    fn png(b: u32, h: u32) -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R'];
        v.extend(b.to_be_bytes());
        v.extend(h.to_be_bytes());
        v.extend([8, 2, 0, 0, 0]);
        v
    }

    #[test]
    fn bildmasse_png_und_jpeg() {
        let o = std::env::temp_dir().join("werkbank-test-cover-bild");
        let _ = fs::create_dir_all(&o);
        fs::write(o.join("a.png"), png(4830, 3260)).unwrap();
        assert_eq!(bildmasse(o.join("a.png").to_str().unwrap()), Some((4830, 3260)));
        // JPEG: SOI, APP0 (Länge 16), SOF0 mit 1200 × 800
        let mut j = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
        j.extend([0u8; 14]);
        j.extend([0xFF, 0xC0, 0x00, 0x11, 8]);
        j.extend(800u16.to_be_bytes());
        j.extend(1200u16.to_be_bytes());
        j.extend([3, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        fs::write(o.join("b.jpg"), j).unwrap();
        assert_eq!(bildmasse(o.join("b.jpg").to_str().unwrap()), Some((1200, 800)));
        assert_eq!(bildmasse("/nichts/da.png"), None);
    }

    #[test]
    fn bild_wird_ueber_umschlag_oder_vorderseite_gelegt() {
        let o = std::env::temp_dir().join("werkbank-test-cover-bild2");
        let _ = fs::create_dir_all(&o);
        let datei = o.join("x.png");
        fs::write(&datei, png(100, 100)).unwrap();
        let p = portal();
        let mut d = CoverDaten {
            bild: datei.to_string_lossy().into_owned(),
            bild_abdunkeln: 0.5,
            bild_ausrichtung: "rechts".into(),
            ..Default::default()
        };
        let (gb, _) = p.umschlag_mm(300);
        let ganz = svg_umschlag(&d, &p, 300, "");
        assert!(ganz.contains(&format!(r#"width="{gb:.2}" height="#)), "Bild über den ganzen Umschlag");
        assert!(ganz.contains(r#"preserveAspectRatio="xMaxYMid slice""#));
        assert!(ganz.contains(r#"opacity="0.50""#));
        d.bild_modus = "vorderseite".into();
        let vorn = svg_umschlag(&d, &p, 300, "");
        let g = geometrie(&p, 300);
        assert!(vorn.contains(&format!(r#"x="{:.2}" y="0.00" width="{:.2}""#, g.vorn_x, g.vorn_b)), "Bild nur vorn");
        // E-Book: Ausschnitt der Vorderseite aus dem Umschlagbild
        d.bild_modus = "umschlag".into();
        let e = svg_ebook(&d, 1600.0, 2560.0, Some((&p, 300)));
        assert!(e.contains(&format!(r#"viewBox="{:.2} 0 {:.2} {:.2}""#, g.vorn_x, g.vorn_b, g.gesamt.1)));
    }

    #[test]
    fn anforderungen_und_bewertung_nach_aufloesung() {
        let p = portal();
        let g = geometrie(&p, 400);
        let a = anforderungen(std::slice::from_ref(&p), 400, false);
        assert_eq!(a[0].benoetigt, (px_bei(g.gesamt.0, 300.0), px_bei(g.gesamt.1, 300.0)));
        assert_eq!(a[1].benoetigt, (1600, 2560));
        let genug = bewerten(a[0].benoetigt, std::slice::from_ref(&p), 400, false);
        assert_eq!(genug[0].stufe, 2, "{:?}", genug[0]);
        let klein = bewerten((a[0].benoetigt.0 / 2, a[0].benoetigt.1 / 2), std::slice::from_ref(&p), 400, false);
        assert_eq!(klein[0].stufe, 0);
        let knapp = bewerten((a[0].benoetigt.0 * 3 / 4, a[0].benoetigt.1 * 3 / 4), std::slice::from_ref(&p), 400, false);
        assert_eq!(knapp[0].stufe, 1);
    }

    #[test]
    fn bild_wird_eingebettet() {
        let o = std::env::temp_dir().join("werkbank-test-cover");
        let _ = fs::create_dir_all(&o);
        let p = o.join("x.png");
        fs::write(&p, [137, 80, 78, 71, 1, 2, 3]).unwrap();
        assert_eq!(bild_uri(p.to_str().unwrap()).unwrap(), "data:image/png;base64,iVBORwECAw==");
    }
}
