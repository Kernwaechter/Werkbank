//! Komplettumschlag (Rückseite U4, Rücken, Vorderseite U1, mit Beschnitt) als Typst.
//!
//! Das ist der **einzige** Umschlagsatz der Werkbank: Das Bauen (`tolino/`, `kdp/`) und der Cover-Designer
//! (Vorschau und Export) rufen [`setzen`] auf.
//!
//! * Vorderseite: das fertige `cover.png`, sonst das Bild und die Titeltexte aus dem Cover-Designer.
//! * Rückseite: der Text aus `klappentext.md` ([`super::klappentext`]), mit Schrift, die bei Bedarf kleiner wird.
//! * Farben: aus dem Cover-Designer ([`Farben`]).
//! * Barcode: bei Tolino der EAN-13 der ISBN (33,4 × 18,1 mm, 16 mm Abstand), bei KDP eine weiße Freifläche
//!   (50,8 × 30,5 mm), in die KDP den Barcode selbst druckt.
//!
//! Ist Ghostscript installiert, wandelt [`schreiben`] das PDF zusätzlich nach CMYK um (optional).

use super::barcode;
use super::klappentext::{self, Klappentext, inline_typst};
use super::pakete::{Plattform, umschlag_masse};
use super::profil::{Band, Profil};
use crate::markdown::zeichenkette;
use crate::vorschau::Seitenbild;
use std::collections::HashMap;
use std::path::Path;

/// Maße des Umschlags in Millimetern.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Masse {
    pub gesamt_b: f32,
    pub gesamt_h: f32,
    pub ruecken: f32,
    pub beschnitt: f32,
    /// Format einer Buchseite.
    pub seite_b: f32,
    pub seite_h: f32,
}

impl Masse {
    /// Maße für eine Druckplattform aus dem Buchprofil.
    pub fn aus_plattform(profil: &Profil, plattform: Plattform, seiten: usize) -> Self {
        let (ruecken, gesamt_b, gesamt_h) = umschlag_masse(profil, plattform, seiten);
        Self { gesamt_b, gesamt_h, ruecken, beschnitt: plattform.beschnitt_mm, seite_b: profil.format_mm[0], seite_h: profil.format_mm[1] }
    }
}

/// Farben des Umschlags (aus dem Cover-Designer); `#rgb` oder `#rrggbb`.
#[derive(Debug, Clone, PartialEq)]
pub struct Farben {
    pub hintergrund: String,
    pub text: String,
    pub akzent: String,
    pub rueckseite_text: String,
    pub ruecken_text: String,
    /// Kasten hinter dem Rückseitentext und auf dem Rücken: Farbe und Deckkraft (0 bis 1).
    pub rueckseite_kasten: (String, f32),
    pub ruecken_kasten: (String, f32),
}

impl Farben {
    pub fn aus_cover(d: &werkbank_kern::cover::CoverDaten) -> Self {
        use werkbank_kern::cover::farbe;
        let text = farbe(&d.text, "#f5f4f0");
        Self {
            hintergrund: farbe(&d.hintergrund, "#1b1c1f"),
            akzent: farbe(&d.akzent, "#7fa6c9"),
            rueckseite_text: farbe(&d.farbe_rueckseite, &text),
            ruecken_text: farbe(&d.farbe_ruecken, &text),
            rueckseite_kasten: (farbe(&d.rueckseite_kasten_farbe, "#000000"), d.rueckseite_kasten_deckkraft.clamp(0.0, 1.0)),
            ruecken_kasten: (farbe(&d.ruecken_kasten_farbe, "#000000"), d.ruecken_kasten_deckkraft.clamp(0.0, 1.0)),
            text,
        }
    }
}

impl Default for Farben {
    fn default() -> Self {
        Self::aus_cover(&werkbank_kern::cover::CoverDaten::default())
    }
}

/// Barcode-Bereich unten rechts auf der Rückseite.
#[derive(Debug, Clone, PartialEq)]
pub enum Barcode {
    /// Kein Bereich.
    Keiner,
    /// Weiße Freifläche (Breite, Höhe, Abstand zu Rücken und Unterkante der Seite in mm).
    Frei { b: f32, h: f32, abstand: f32 },
    /// EAN-13 aus der ISBN, unverändert gesetzt (Breite, Höhe, Abstand in mm).
    Ean13 { ziffern: [u8; 13], isbn: String, b: f32, h: f32, abstand: f32 },
}

/// KDP: Freifläche 2 × 1,2 Zoll; die Vorgabe verlangt mindestens 0,25 Zoll (6,35 mm) Abstand, wir nehmen 8 mm.
pub const KDP_FREIFLAECHE: (f32, f32, f32) = (50.8, 30.5, 8.0);
/// Tolino Media: Barcode 3,34 × 1,81 cm, nicht verkleinern oder verzerren, mindestens 15 mm Abstand zu Rücken
/// und Rändern (wir nehmen 16 mm vom Rücken und von der Seitenkante, also noch mehr als verlangt).
pub const TOLINO_BARCODE: (f32, f32, f32) = (33.4, 18.1, 16.0);

impl Barcode {
    /// Barcode-Bereich für ein Portal (`name` enthält „Tolino“ oder „KDP“/„Amazon“). Ohne `reservieren` kein Bereich.
    /// Tolino bekommt den EAN-13, wenn die ISBN gültig ist, sonst die Freifläche in Barcode-Größe.
    pub fn fuer(name: &str, reservieren: bool, isbn: Option<&str>) -> Self {
        if !reservieren {
            return Self::Keiner;
        }
        if name.to_lowercase().contains("tolino") {
            let (b, h, abstand) = TOLINO_BARCODE;
            match isbn.and_then(|i| barcode::isbn_ziffern(i).map(|z| (z, i))) {
                Some((ziffern, i)) => Self::Ean13 { ziffern, isbn: i.trim().to_string(), b, h, abstand },
                None => Self::Frei { b, h, abstand },
            }
        } else {
            let (b, h, abstand) = KDP_FREIFLAECHE;
            Self::Frei { b, h, abstand }
        }
    }

    /// Wie weit der Bereich von der Unterkante der Seite nach oben reicht (mm).
    fn hoehe_ueber_unterkante(&self) -> f32 {
        match self {
            Self::Keiner => 0.0,
            Self::Frei { h, abstand, .. } | Self::Ean13 { h, abstand, .. } => abstand + h,
        }
    }

    /// Wie weit der Bereich von der Rückenkante nach links reicht (mm).
    fn breite_vom_ruecken(&self) -> f32 {
        match self {
            Self::Keiner => 0.0,
            Self::Frei { b, abstand, .. } | Self::Ean13 { b, abstand, .. } => abstand + b,
        }
    }
}

/// Hintergrundbild aus dem Cover-Designer.
#[derive(Debug, Clone)]
pub struct Hintergrund {
    pub daten: Vec<u8>,
    /// Nur auf der Vorderseite (sonst über den ganzen Umschlag).
    pub nur_vorn: bool,
    /// Abdunkelung 0 bis 1 in der Hintergrundfarbe.
    pub abdunkeln: f32,
    /// Waagerechter Ausschnitt: 0 = links, 0,5 = Mitte, 1 = rechts.
    pub ausrichtung: f32,
}

/// Alles, was der Umschlag braucht.
pub struct Eingabe<'a> {
    pub masse: Masse,
    pub farben: Farben,
    pub klappentext: &'a Klappentext,
    /// Seitenzahl (für `{seiten}` in der Fußzeile).
    pub seiten: usize,
    pub autor: &'a str,
    /// Titel und Bandbezeichnung auf dem Rücken.
    pub ruecken_titel: &'a str,
    pub band_ruecken: &'a str,
    /// Fertiges Frontcover (`cover.png`): füllt die Vorderseite allein.
    pub vorderseite: Option<Vec<u8>>,
    /// Sonst: Bild des Cover-Designers …
    pub hintergrund: Option<Hintergrund>,
    /// … und dessen Titeltexte als SVG der Vorderseite (durchsichtig, Seitenverhältnis Vorderseite samt Beschnitt).
    pub titeltexte_svg: Option<String>,
    pub barcode: Barcode,
    /// Gewünschte Schriftgröße des Klappentextes in pt und ob sie bis 7,5 pt schrumpfen darf.
    pub schrift_pt: f32,
    pub schrift_auto: bool,
}

/// Wie der Klappentext auf die Rückseite gepasst hat.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Bericht {
    /// Verwendete und gewünschte Schriftgröße (pt).
    pub groesse_pt: f32,
    pub gewuenscht_pt: f32,
    /// Passt der Text auf die Rückseite?
    pub passt: bool,
    /// Um wie viele mm der Text zu hoch ist (0 = passt), und wie hoch er ist / wie viel frei ist.
    pub ueberhang_mm: f32,
    pub hoehe_mm: f32,
    pub verfuegbar_mm: f32,
    /// Warnungen des Setzers (z. B. fehlende Schriften).
    pub warnungen: Vec<String>,
    /// Hinweise für den Autor (z. B. Barcode ohne gültige ISBN).
    pub hinweise: Vec<String>,
}

/// Ergebnis von [`setzen`].
pub struct Ergebnis {
    pub pdf: Option<Vec<u8>>,
    pub bild: Option<Seitenbild>,
    pub bericht: Bericht,
}

const MIN_PT: f32 = 7.5;
const RAND_SEITE: f32 = 14.0;
const RAND_OBEN: f32 = 14.0;
const RAND_UNTEN: f32 = 12.0;

fn hex8(farbe: &str, deckkraft: f32) -> String {
    let h = farbe.trim_start_matches('#');
    let h = if h.len() == 3 { h.chars().flat_map(|c| [c, c]).collect::<String>() } else { h.to_string() };
    format!("#{h}{:02x}", (deckkraft.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn zahl(x: f32) -> String {
    format!("{x:.3}")
}

fn bildmasse(daten: &[u8]) -> Option<(u32, u32)> {
    image::ImageReader::new(std::io::Cursor::new(daten)).with_guessed_format().ok()?.into_dimensions().ok()
}

fn endung(daten: &[u8]) -> &'static str {
    match image::guess_format(daten) {
        Ok(image::ImageFormat::Jpeg) => "jpg",
        _ => "png",
    }
}

/// Bild so legen, dass es die Fläche `b` × `h` (mm) ausfüllt (zugeschnitten, nicht verzerrt).
fn fuellen(datei: &str, masse: (u32, u32), b: f32, h: f32, ausrichtung: f32) -> String {
    let (iw, ih) = (masse.0.max(1) as f32, masse.1.max(1) as f32);
    let skala = (b / iw).max(h / ih);
    let (w, hh) = (iw * skala, ih * skala);
    format!(
        "box(width: {b}mm, height: {h}mm, clip: true, place(top + left, dx: {dx}mm, dy: {dy}mm, image(\"{datei}\", width: {w}mm, height: {hh}mm)))",
        b = zahl(b),
        h = zahl(h),
        dx = zahl((b - w) * ausrichtung.clamp(0.0, 1.0)),
        dy = zahl((h - hh) / 2.0),
        w = zahl(w),
        hh = zahl(hh),
    )
}

fn markup(md: &str) -> String {
    format!("[{}]", inline_typst(md))
}

/// Typst-Quelltext des Umschlags und die Dateien, die er braucht.
pub fn typst(e: &Eingabe) -> (String, HashMap<String, Vec<u8>>) {
    let m = &e.masse;
    let k = e.klappentext;
    let f = &e.farben;
    let s = m.beschnitt;
    let (tw, th) = (m.seite_b, m.seite_h);
    let (rx, fx) = (s + tw, s + tw + m.ruecken);
    let fb = tw + s;
    let mut dateien: HashMap<String, Vec<u8>> = HashMap::new();

    let schriften = crate::welt::vorhandene_familien(&["Noto Sans", "Liberation Sans", "DejaVu Sans", "Fira Sans"]);
    let sans = schriften.iter().map(|n| zeichenkette(n)).collect::<Vec<_>>().join(", ");

    // --- Hintergrundbild des Cover-Designers (nur ohne fertiges cover.png)
    let mut unten_schicht = String::new();
    let mut vorn = String::new();
    let hintergrund = e.hintergrund.as_ref().filter(|_| e.vorderseite.is_none()).and_then(|hg| bildmasse(&hg.daten).map(|ma| (hg, ma)));
    if let Some((hg, masse)) = hintergrund {
        let datei = format!("hintergrund.{}", endung(&hg.daten));
        dateien.insert(datei.clone(), hg.daten.clone());
        let abdunkeln = hex8(&f.hintergrund, hg.abdunkeln);
        let (x, breite) = if hg.nur_vorn { (fx, fb) } else { (0.0, m.gesamt_b) };
        let schicht = format!(
            "#place(top + left, dx: {x}mm, {})\n#place(top + left, dx: {x}mm, rect(width: {b}mm, height: {h}mm, fill: rgb(\"{abdunkeln}\")))\n",
            fuellen(&datei, masse, breite, m.gesamt_h, hg.ausrichtung),
            x = zahl(x),
            b = zahl(breite),
            h = zahl(m.gesamt_h),
        );
        if hg.nur_vorn {
            vorn.push_str(&schicht);
        } else {
            unten_schicht.push_str(&schicht);
        }
    }

    // --- Vorderseite: fertiges Cover, sonst (Bild und) Titeltexte des Cover-Designers
    if let Some(png) = &e.vorderseite
        && let Some(masse) = bildmasse(png)
    {
        let datei = format!("vorn.{}", endung(png));
        dateien.insert(datei.clone(), png.clone());
        vorn.push_str(&format!("#place(top + left, dx: {}mm, {})\n", zahl(fx), fuellen(&datei, masse, fb, m.gesamt_h, 0.5)));
    } else if let Some(svg) = &e.titeltexte_svg {
        dateien.insert("titel.svg".into(), svg.clone().into_bytes());
        vorn.push_str(&format!(
            "#place(top + left, dx: {}mm, image(\"titel.svg\", width: {}mm, height: {}mm))\n",
            zahl(fx),
            zahl(fb),
            zahl(m.gesamt_h)
        ));
    }

    // --- Rückseite: Text mit passender Größe
    let mut oben = String::new();
    if !k.dachzeile.is_empty() {
        oben.push_str(&format!(
            "  block(below: 1.1em, text(fill: akzent, size: 0.82em, weight: 800, tracking: 0.12em, upper({})))\n",
            markup(&k.dachzeile)
        ));
    }
    if !k.schlagzeile.is_empty() {
        oben.push_str(&format!(
            "  block(below: 1.2em, par(leading: 0.34em, justify: false, text(size: 1.7em, weight: 900, hyphenate: false, {})))\n",
            markup(&k.schlagzeile)
        ));
    }
    for a in &k.absaetze {
        oben.push_str(&format!("  block(below: 1.0em, par(leading: 0.55em, justify: true, {}))\n", markup(a)));
    }
    for ka in &k.kaesten {
        let punkte = ka.punkte.iter().map(|p| markup(p)).collect::<Vec<_>>().join(", ");
        let titel = if ka.titel.is_empty() {
            String::new()
        } else {
            format!("    block(below: 0.55em, text(fill: akzent, size: 1.0em, weight: \"bold\", tracking: 0.04em, {}))\n", markup(&ka.titel))
        };
        oben.push_str(&format!(
            "  block(above: 0.4em, below: 0.9em, width: 100%, fill: rgb(\"{kasten}\"), stroke: (left: 1.2mm + akzent), radius: 1.5mm, inset: (x: 4.5mm, y: 3.6mm), {{\n    set par(leading: 0.5em, justify: false)\n{titel}    for p in ({punkte},) {{\n      block(above: 0.4em, below: 0.4em, text(size: 0.95em, [#text(fill: akzent, weight: \"bold\")[▸ ]#p]))\n    }}\n  }})\n",
            kasten = hex8(&f.rueckseite_kasten.0, f.rueckseite_kasten.1.max(0.0)),
        ));
    }
    let mut unten = String::new();
    for info in &k.infos {
        let absaetze = info.absaetze.iter().map(|a| markup(a)).collect::<Vec<_>>().join(", ");
        unten.push_str(&format!(
            "  block(width: 100%, above: 0.4em, below: 0em, stroke: (top: 0.3mm + akzent.transparentize(60%)), inset: (top: 3.2mm), {{\n    set par(leading: 0.45em, justify: false)\n    text(size: 1.0em, weight: \"bold\", {})\n    v(0.3em)\n    for a in ({absaetze},) {{ block(above: 0em, below: 0.4em, text(size: 0.9em, a)) }}\n  }})\n",
            markup(&info.titel)
        ));
    }

    // Platz für Text: unten bleibt der Barcode-Bereich und die Fußzeile frei
    let fuss_h = k.fuss.len() as f32 * 5.0;
    let frei_unten = m_max(e.barcode.hoehe_ueber_unterkante(), RAND_UNTEN + fuss_h) + 6.0;
    let text_b = tw - 2.0 * RAND_SEITE;
    let text_h = (th - RAND_OBEN - frei_unten).max(20.0);
    let (x0, y0) = (s + RAND_SEITE, s + RAND_OBEN);
    let mit_text = !k.leer();
    let gewuenscht = if e.schrift_pt >= 6.0 { e.schrift_pt.min(20.0) } else { 11.0 };
    let text_teil = if mit_text {
        let kasten_rueck = if f.rueckseite_kasten.1 > 0.0 {
            format!(
                "#place(top + left, dx: {}mm, dy: {}mm, rect(width: {}mm, height: {}mm, radius: 2mm, fill: rgb(\"{}\")))\n",
                zahl(s + 6.0),
                zahl(y0 - 6.0),
                zahl(tw - 12.0),
                zahl(text_h + 8.0),
                hex8(&f.rueckseite_kasten.0, f.rueckseite_kasten.1)
            )
        } else {
            String::new()
        };
        format!(
            "{kasten_rueck}#let oben(pt) = {{\n  set text(size: pt)\n  block(width: 100%, {{\n{oben}  }})\n}}\n#let unten(pt) = {{\n  set text(size: pt)\n  block(width: 100%, {{\n{unten}  }})\n}}\n#context {{\n  let w = {w}mm\n  let hoch = {h}mm\n  let start = {start}pt\n  let kleinste = {min}pt\n  let pt = start\n  if {auto} {{\n    while pt > kleinste and measure(width: w, stack(oben(pt), unten(pt))).height > hoch {{ pt = pt - 0.25pt }}\n  }}\n  let natuerlich = measure(width: w, stack(oben(pt), unten(pt))).height\n  [#metadata((pt: pt.pt(), start: start.pt(), natuerlich: natuerlich.mm(), hoch: hoch.mm())) <fit>]\n  place(top + left, dx: {x}mm, dy: {y}mm, box(width: w, height: hoch, grid(rows: (1fr, auto), oben(pt), unten(pt))))\n}}\n",
            w = zahl(text_b),
            h = zahl(text_h),
            start = zahl(gewuenscht),
            min = zahl(MIN_PT.min(gewuenscht)),
            auto = e.schrift_auto,
            x = zahl(x0),
            y = zahl(y0),
        )
    } else {
        String::new()
    };

    // Fußzeile links unten (Platz neben dem Barcode)
    let fuss_b = (tw - RAND_SEITE - m_max(e.barcode.breite_vom_ruecken() + 8.0, RAND_SEITE)).max(40.0);
    let fusszeile = if k.fuss.is_empty() {
        String::new()
    } else {
        let zeilen = k
            .fuss
            .iter()
            .enumerate()
            .map(|(i, z)| {
                let z = z.replace("{seiten}", &e.seiten.to_string());
                if i == 0 {
                    format!("text(font: mono, size: 9pt, weight: \"bold\", fill: akzent, {})", markup(&z))
                } else {
                    format!("text(size: 7.5pt, fill: rtext.transparentize(25%), {})", markup(&z))
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "#place(bottom + left, dx: {}mm, dy: -{}mm, box(width: {}mm, stack(spacing: 1.2mm, {zeilen})))\n",
            zahl(x0),
            zahl(RAND_UNTEN),
            zahl(fuss_b)
        )
    };

    // --- Barcode
    let barcode = match &e.barcode {
        Barcode::Keiner => String::new(),
        Barcode::Frei { b, h, abstand } => format!(
            "#place(top + left, dx: {}mm, dy: {}mm, rect(width: {}mm, height: {}mm, fill: white))\n",
            zahl(s + tw - abstand - b),
            zahl(s + th - abstand - h),
            zahl(*b),
            zahl(*h)
        ),
        Barcode::Ean13 { ziffern, isbn, b, h, abstand } => {
            // 11 Module Ruhezone links, 95 Module Code, 7 rechts: zusammen 113 Module in der Breite des Feldes
            let mod_b = b / 113.0;
            let (balken_y, balken_h) = (3.6f32, h - 3.6 - 3.2);
            let bits = barcode::module(ziffern);
            let mut balken = String::new();
            let mut i = 0;
            while i < bits.len() {
                if bits[i] {
                    let start = i;
                    while i < bits.len() && bits[i] {
                        i += 1;
                    }
                    balken.push_str(&format!(
                        "  place(top + left, dx: {}mm, dy: {}mm, rect(width: {}mm, height: {}mm, fill: black, stroke: none))\n",
                        zahl((11 + start) as f32 * mod_b),
                        zahl(balken_y),
                        zahl((i - start) as f32 * mod_b),
                        zahl(balken_h)
                    ));
                } else {
                    i += 1;
                }
            }
            let z = |a: usize, b: usize| ziffern[a..b].iter().map(|d| d.to_string()).collect::<String>();
            let ziffern_y = balken_y + balken_h + 0.2;
            format!(
                "#place(top + left, dx: {x}mm, dy: {y}mm, box(width: {b}mm, height: {h}mm, fill: white, {{\n  set text(font: mono, size: 6pt, fill: black, top-edge: \"ascender\", bottom-edge: \"descender\")\n  place(top + center, dy: 0.7mm, [ISBN {isbn}])\n{balken}  place(top + left, dx: {d0}mm, dy: {dy}mm, [{z0}])\n  place(top + left, dx: {d1}mm, dy: {dy}mm, box(width: {bw}mm, align(center, [{z1}])))\n  place(top + left, dx: {d2}mm, dy: {dy}mm, box(width: {bw}mm, align(center, [{z2}])))\n}}))\n",
                x = zahl(s + tw - abstand - b),
                y = zahl(s + th - abstand - h),
                b = zahl(*b),
                h = zahl(*h),
                isbn = isbn.replace(['[', ']', '#'], ""),
                d0 = zahl(2.0 * mod_b),
                d1 = zahl((11 + 3) as f32 * mod_b),
                d2 = zahl((11 + 50) as f32 * mod_b),
                bw = zahl(42.0 * mod_b),
                dy = zahl(ziffern_y),
                z0 = z(0, 1),
                z1 = z(1, 7),
                z2 = z(7, 13),
            )
        }
    };

    // --- Rücken
    let ruecken = if m.ruecken >= 6.0 {
        let pt = (m.ruecken * 2.835 * 0.5).clamp(6.0, 10.0);
        let band = if e.band_ruecken.trim().is_empty() {
            String::new()
        } else {
            format!(
                ", align(horizon, box(fill: akzent, inset: (x: 1.5mm, y: 1mm), radius: 1mm, text(size: {:.1}pt, weight: 900, fill: grund, {})))",
                (pt * 0.85).max(5.5),
                markup(&e.band_ruecken.replace(['*', '`'], ""))
            )
        };
        let kasten = if f.ruecken_kasten.1 > 0.0 {
            format!(
                "#place(top + left, dx: {}mm, rect(width: {}mm, height: {}mm, fill: rgb(\"{}\")))\n",
                zahl(rx),
                zahl(m.ruecken),
                zahl(m.gesamt_h),
                hex8(&f.ruecken_kasten.0, f.ruecken_kasten.1)
            )
        } else {
            String::new()
        };
        format!(
            "{kasten}#place(top + left, dx: {rx}mm, box(width: {r}mm, height: {gh}mm, align(center + horizon, rotate(-90deg, reflow: true, box(stack(dir: ltr, spacing: 6mm, align(horizon, text(size: {pt:.1}pt, weight: 600, fill: stext.transparentize(15%), {autor})), align(horizon, text(size: {pt:.1}pt, weight: 800, fill: akzent, {titel})){band}))))))\n",
            rx = zahl(rx),
            r = zahl(m.ruecken),
            gh = zahl(m.gesamt_h),
            autor = markup(e.autor),
            titel = markup(e.ruecken_titel),
        )
    } else {
        String::new()
    };

    let quelle = format!(
        "#let sans = ({sans},)\n#let mono = (\"DejaVu Sans Mono\", \"Liberation Mono\")\n#let grund = rgb(\"{bg}\")\n#let akzent = rgb(\"{akzent}\")\n#let rtext = rgb(\"{rtext}\")\n#let stext = rgb(\"{stext}\")\n#let ic(code) = text(font: mono, code)\n#set page(width: {gb}mm, height: {gh}mm, margin: 0pt, fill: grund)\n#set text(font: sans, fill: rtext, lang: \"de\", hyphenate: true, top-edge: 0.78em, bottom-edge: -0.22em)\n#set par(justify: false)\n{unten_schicht}{text_teil}{fusszeile}{barcode}{ruecken}{vorn}",
        bg = f.hintergrund,
        akzent = f.akzent,
        rtext = f.rueckseite_text,
        stext = f.ruecken_text,
        gb = zahl(m.gesamt_b),
        gh = zahl(m.gesamt_h),
    );
    (quelle, dateien)
}

fn m_max(a: f32, b: f32) -> f32 {
    if a > b { a } else { b }
}

/// Baut die Eingabe so, wie der Cover-Designer das Cover festlegt (gemeinsam für Bauen und Designer):
/// Ohne Hintergrundbild gilt das fertige `cover.png` des Bandes als Vorderseite, sonst Bild und Titeltexte
/// des Designers. Farben, Schriftgröße und Barcode-Wunsch kommen ebenfalls aus den Cover-Daten.
#[allow(clippy::too_many_arguments)]
pub fn eingabe_aus_cover<'a>(
    cover: &werkbank_kern::cover::CoverDaten,
    bandordner: &Path,
    masse: Masse,
    klappentext: &'a Klappentext,
    seiten: usize,
    autor: &'a str,
    ruecken_titel: &'a str,
    band_ruecken: &'a str,
    portal_name: &str,
    isbn: Option<&str>,
) -> Eingabe<'a> {
    let fertig = if cover.bild.is_empty() { std::fs::read(bandordner.join("cover.png")).ok() } else { None };
    let (hintergrund, titeltexte_svg) = if fertig.is_some() {
        (None, None)
    } else {
        let hg = (!cover.bild.is_empty())
            .then(|| std::fs::read(werkbank_kern::cover::bild_aufloesen(&cover.bild, bandordner)).ok())
            .flatten()
            .map(|daten| Hintergrund {
                daten,
                nur_vorn: cover.bild_nur_vorn(),
                abdunkeln: cover.bild_abdunkeln.clamp(0.0, 1.0),
                ausrichtung: match cover.bild_ausrichtung.as_str() {
                    "links" => 0.0,
                    "rechts" => 1.0,
                    _ => 0.5,
                },
            });
        let svg = werkbank_kern::cover::svg_vorderseite_texte(cover, masse.seite_b + masse.beschnitt, masse.gesamt_h);
        (hg, Some(svg))
    };
    Eingabe {
        masse,
        farben: Farben::aus_cover(cover),
        klappentext,
        seiten,
        autor,
        ruecken_titel,
        band_ruecken,
        vorderseite: fertig,
        hintergrund,
        titeltexte_svg,
        barcode: Barcode::fuer(portal_name, cover.barcode, isbn),
        schrift_pt: cover.rueckseite_schrift_pt,
        schrift_auto: cover.rueckseite_schrift_auto,
    }
}

/// Setzt den Umschlag. `bild_breite_px`: Vorschaubild dieser Breite mitliefern; `pdf`: PDF mitliefern.
pub fn setzen(e: &Eingabe, pdf: bool, bild_breite_px: Option<u32>) -> Result<Ergebnis, String> {
    let (quelle, dateien) = typst(e);
    let pixel_pro_pt = bild_breite_px.map(|b| b as f32 / (e.masse.gesamt_b * 72.0 / 25.4));
    let r = crate::welt::typst_mit_werten(quelle, dateien, "fit", pdf, pixel_pro_pt)?;
    let w = |n: &str| r.werte.get(n).copied().unwrap_or(0.0) as f32;
    let gewuenscht = if e.schrift_pt >= 6.0 { e.schrift_pt.min(20.0) } else { 11.0 };
    let bericht = if r.werte.is_empty() {
        Bericht { gewuenscht_pt: gewuenscht, groesse_pt: gewuenscht, passt: true, warnungen: r.warnungen.clone(), ..Default::default() }
    } else {
        let (hoch, natuerlich) = (w("hoch"), w("natuerlich"));
        Bericht {
            groesse_pt: w("pt"),
            gewuenscht_pt: w("start"),
            passt: natuerlich <= hoch + 0.05,
            ueberhang_mm: (natuerlich - hoch).max(0.0),
            hoehe_mm: natuerlich,
            verfuegbar_mm: hoch,
            warnungen: r.warnungen.clone(),
            hinweise: Vec::new(),
        }
    };
    Ok(Ergebnis { pdf: r.pdf, bild: r.bild, bericht })
}

/// Setzt den Komplettumschlag eines Bandes als PDF (für `tolino/` und `kdp/`); wandelt ihn nach CMYK,
/// wenn Ghostscript vorhanden ist. Quellen: `klappentext.md` und `.werkbank/cover.json` des Bandes
/// (Farben, Bild, Barcode-Wunsch), `cover.png` als Vorderseite, `isbn` für den Barcode.
pub fn schreiben(
    profil: &Profil,
    band: &Band,
    bandordner: &Path,
    plattform: Plattform,
    seiten: usize,
    ziel: &Path,
    isbn: Option<&str>,
) -> Result<Bericht, String> {
    let md = std::fs::read_to_string(bandordner.join("klappentext.md"))
        .map_err(|_| format!("klappentext.md fehlt in {} – er liefert den Text der Umschlag-Rückseite", bandordner.display()))?;
    let text = klappentext::lesen(&md);
    if text.leer() {
        return Err(format!("{}/klappentext.md ist leer – er liefert den Text der Umschlag-Rückseite", bandordner.display()));
    }
    let cover = werkbank_kern::cover::CoverDaten::laden(bandordner).unwrap_or_default();
    let ruecken_titel = if profil.reihe.ruecken_titel.is_empty() { &profil.titel } else { &profil.reihe.ruecken_titel };
    let masse = Masse::aus_plattform(profil, plattform, seiten);
    let eingabe = eingabe_aus_cover(&cover, bandordner, masse, &text, seiten, &profil.autor, ruecken_titel, &band.umschlag.band_ruecken, plattform.name, isbn);
    let ergebnis = setzen(&eingabe, true, None)?;
    let mut bericht = ergebnis.bericht;
    if plattform.name.eq_ignore_ascii_case("tolino") && cover.barcode {
        match isbn.map(str::trim).filter(|i| !i.is_empty()) {
            None => bericht.hinweise.push("Keine ISBN in den Buchdaten: Es bleibt ein weißes Feld für den Barcode aus dem Tolino-Dashboard.".to_string()),
            Some(i) if barcode::isbn_ziffern(i).is_none() => {
                bericht.hinweise.push(format!("„{i}“ ist keine gültige ISBN-13 (Prüfziffer?): Es bleibt ein weißes Feld für den Barcode."))
            }
            _ => {}
        }
    }
    let bytes = ergebnis.pdf.ok_or("kein PDF erzeugt")?;
    let gs = super::programm_finden(&["gs", "gswin64c"]);
    match gs {
        Some(gs) => {
            let roh = ziel.with_extension("roh.pdf");
            std::fs::write(&roh, &bytes).map_err(|e| e.to_string())?;
            let ok = std::process::Command::new(gs)
                .args(["-sDEVICE=pdfwrite", "-dCompatibilityLevel=1.4", "-sColorConversionStrategy=CMYK", "-dProcessColorModel=/DeviceCMYK", "-dNOPAUSE", "-dQUIET", "-dBATCH"])
                .arg(format!("-sOutputFile={}", ziel.display()))
                .arg(&roh)
                .status()
                .is_ok_and(|s| s.success());
            let _ = std::fs::remove_file(&roh);
            if !ok {
                std::fs::write(ziel, &bytes).map_err(|e| e.to_string())?;
            }
        }
        None => std::fs::write(ziel, &bytes).map_err(|e| format!("{}: {e}", ziel.display()))?,
    }
    Ok(bericht)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "Dachzeile\n\n# Schlagzeile<br/>zweite Zeile\n\nEin **langer** Absatz mit `Code` und genug Text, damit die Rückseite etwas zu setzen hat. Noch ein Satz dazu.\n\n## Deine Projekte:\n\n- **Eins:** Text\n- **Zwei:** Text\n\n## Über den Autor\n\nJahrgang 1971, IT-Sicherheit.\n\n---\nflutter-werkstatt.de\nDownload · {seiten} Seiten\n";

    fn eingabe<'a>(k: &'a Klappentext, barcode: Barcode) -> Eingabe<'a> {
        Eingabe {
            masse: Masse { gesamt_b: 365.0, gesamt_h: 246.0, ruecken: 18.0, beschnitt: 3.0, seite_b: 170.0, seite_h: 240.0 },
            farben: Farben::default(),
            klappentext: k,
            seiten: 332,
            autor: "Volker Dähn",
            ruecken_titel: "Die Flutter- & Dart-Werkstatt",
            band_ruecken: "BAND 1",
            vorderseite: None,
            hintergrund: None,
            titeltexte_svg: None,
            barcode,
            schrift_pt: 10.0,
            schrift_auto: true,
        }
    }

    #[test]
    fn umschlag_wird_gesetzt_und_text_passt() {
        let k = klappentext::lesen(TEXT);
        let e = setzen(&eingabe(&k, Barcode::fuer("Amazon KDP", true, None)), true, Some(600)).unwrap();
        assert!(e.pdf.as_ref().unwrap().starts_with(b"%PDF"));
        let b = &e.bericht;
        assert!(b.passt, "{b:?}");
        assert!(b.groesse_pt <= 10.0 && b.groesse_pt >= 7.5, "{b:?}");
        assert!(b.verfuegbar_mm > 100.0 && b.hoehe_mm > 20.0 && b.hoehe_mm <= b.verfuegbar_mm, "{b:?}");
        let bild = e.bild.unwrap();
        assert_eq!(bild.breite, 600);
        assert_eq!(bild.hoehe, (600.0f32 * 246.0 / 365.0).round() as u32);
    }

    #[test]
    fn zu_langer_text_schrumpft_dann_wird_er_als_zu_lang_gemeldet() {
        let lang = format!("# Titel\n\n{}\n", "Dies ist ein sehr langer Absatz, der die Rückseite füllt. ".repeat(300));
        let k = klappentext::lesen(&lang);
        let mut e = eingabe(&k, Barcode::Keiner);
        let r = setzen(&e, false, None).unwrap().bericht;
        assert!(!r.passt && r.ueberhang_mm > 0.0, "{r:?}");
        assert!((r.groesse_pt - MIN_PT).abs() < 0.3, "bis zur kleinsten Größe verkleinert: {r:?}");
        e.schrift_auto = false;
        let fest = setzen(&e, false, None).unwrap().bericht;
        assert!((fest.groesse_pt - 10.0).abs() < 0.01, "ohne Automatik bleibt die Größe: {fest:?}");
        assert!(fest.ueberhang_mm > r.ueberhang_mm);
    }

    #[test]
    fn barcode_regeln_je_portal() {
        // KDP: Freifläche 50,8 × 30,5 mm, mindestens 6,35 mm Abstand
        match Barcode::fuer("Amazon KDP", true, Some("978-3-9825412-0-4")) {
            Barcode::Frei { b, h, abstand } => assert!((b, h) == (50.8, 30.5) && abstand >= 6.35),
            a => panic!("{a:?}"),
        }
        // Tolino mit gültiger ISBN: EAN-13 in 33,4 × 18,1 mm, mindestens 15 mm Abstand
        match Barcode::fuer("Tolino Media", true, Some("978-3-9825412-0-4")) {
            Barcode::Ean13 { b, h, abstand, .. } => assert!((b, h) == (33.4, 18.1) && abstand >= 15.0),
            a => panic!("{a:?}"),
        }
        // Tolino ohne oder mit falscher ISBN: Freifläche in Barcode-Größe
        assert!(matches!(Barcode::fuer("Tolino Media", true, None), Barcode::Frei { b, .. } if b == 33.4));
        assert!(matches!(Barcode::fuer("Tolino Media", true, Some("978-3-9825412-0-5")), Barcode::Frei { .. }));
        assert_eq!(Barcode::fuer("Tolino Media", false, Some("978-3-9825412-0-4")), Barcode::Keiner);
    }

    #[test]
    fn tolino_barcode_wird_gesetzt() {
        let k = klappentext::lesen(TEXT);
        let b = Barcode::fuer("Tolino Media", true, Some("978-3-9825412-0-4"));
        let (quelle, _) = typst(&eingabe(&k, b.clone()));
        assert!(quelle.contains("ISBN 978-3-9825412-0-4"), "ISBN als Text über dem Code");
        assert!(quelle.contains("fill: black"), "Balken");
        let r = setzen(&eingabe(&k, b), true, Some(500)).unwrap();
        assert!(r.pdf.unwrap().starts_with(b"%PDF"));
    }

    #[test]
    fn fertiges_cover_fuellt_die_vorderseite() {
        let mut png = image::RgbImage::new(400, 600);
        png.pixels_mut().for_each(|p| *p = image::Rgb([200, 20, 20]));
        let mut daten = Vec::new();
        image::DynamicImage::ImageRgb8(png).write_to(&mut std::io::Cursor::new(&mut daten), image::ImageFormat::Png).unwrap();
        let k = klappentext::lesen(TEXT);
        let mut e = eingabe(&k, Barcode::Keiner);
        e.vorderseite = Some(daten);
        let r = setzen(&e, false, Some(730)).unwrap();
        let bild = r.bild.unwrap();
        // Mitte der Vorderseite: rechts vom Rücken, Seitenmitte
        let x = ((3.0 + 170.0 + 18.0 + (170.0 + 3.0) / 2.0) / 365.0 * bild.breite as f32) as usize;
        let y = (bild.hoehe / 2) as usize;
        let i = (y * bild.breite as usize + x) * 4;
        let p = &bild.rgba[i..i + 3];
        assert!(p[0] > 150 && p[1] < 80 && p[2] < 80, "Vorderseite soll das rote cover.png sein: {p:?}");
        // Rückseite links bleibt in der Hintergrundfarbe oder Textfarbe, nicht rot
        let i = ((bild.hoehe / 2) as usize * bild.breite as usize + 8) * 4;
        assert!(!(bild.rgba[i] > 150 && bild.rgba[i + 1] < 80), "Rückseite nicht rot");
    }

    #[test]
    fn farben_kommen_aus_dem_designer() {
        let d = werkbank_kern::cover::CoverDaten { hintergrund: "#102030".into(), akzent: "#ff8800".into(), ..Default::default() };
        let f = Farben::aus_cover(&d);
        let k = klappentext::lesen(TEXT);
        let mut e = eingabe(&k, Barcode::Keiner);
        e.farben = f;
        let (quelle, _) = typst(&e);
        assert!(quelle.contains("rgb(\"#102030\")") && quelle.contains("rgb(\"#ff8800\")"), "{}", &quelle[..300]);
        assert!(!quelle.contains("#0B1120") && !quelle.contains("#38BDF8"), "keine festen Farben mehr");
    }
}
