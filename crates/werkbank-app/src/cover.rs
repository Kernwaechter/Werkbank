//! # Cover rendern und exportieren
//!
//! Wandelt die SVG-Vorlage aus dem Kern mit `resvg` in Pixelbilder um
//! (Vorschau und PNG-Export) und schreibt SVG- und PNG-Dateien nach
//! `.werkbank/cover/`.

use resvg::{tiny_skia, usvg};
use std::path::{Path, PathBuf};
use werkbank_kern::cover::{self, CoverDaten};
use werkbank_kern::portale::Portal;
use werkbank_satz::buchbau::{klappentext, umschlag};

fn baum(svg: &str) -> Result<usvg::Tree, String> {
    let mut optionen = usvg::Options::default();
    optionen.fontdb_mut().load_system_fonts();
    usvg::Tree::from_str(svg, &optionen).map_err(|e| format!("SVG nicht lesbar: {e}"))
}

/// Rendert auf eine Zielbreite in Pixeln. Liefert (Breite, Höhe, RGBA-Pixel).
pub fn rendern(svg: &str, ziel_breite: u32) -> Result<(u32, u32, Vec<u8>), String> {
    let baum = baum(svg)?;
    let groesse = baum.size();
    let skala = ziel_breite as f32 / groesse.width();
    let (b, h) = (ziel_breite.max(1), (groesse.height() * skala).round().max(1.0) as u32);
    let mut pixel = tiny_skia::Pixmap::new(b, h).ok_or("Bild zu groß")?;
    resvg::render(&baum, tiny_skia::Transform::from_scale(skala, skala), &mut pixel.as_mut());
    Ok((b, h, pixel.take()))
}

/// Rendert in eine PNG-Datei.
pub fn png_schreiben(svg: &str, ziel_breite: u32, datei: &Path) -> Result<(), String> {
    let (b, h, daten) = rendern(svg, ziel_breite)?;
    let pixel = tiny_skia::Pixmap::from_vec(daten, tiny_skia::IntSize::from_wh(b, h).ok_or("Größe")?).ok_or("Bild")?;
    pixel.save_png(datei).map_err(|e| format!("{}: {e}", datei.display()))
}

/// Klappentext aus `klappentext.md` als Markdown (Schlagzeile, Absätze, Punkte, Fettdruck werden beim Satz ausgewertet).
pub fn klappentext_lesen(bandordner: &Path) -> String {
    std::fs::read_to_string(bandordner.join("klappentext.md"))
        .unwrap_or_default()
        .trim_start_matches('\u{feff}')
        .to_string()
}

fn kurzname(name: &str) -> String {
    werkbank_kern::speicher::kennung_aus_titel(name)
}

/// Wandelt RGBA-Pixel für die Vorschau um: `eink_stufen = None` ergibt stufenlose Graustufen,
/// `Some(16)` simuliert die 16 Grauwerte eines E-Ink-Displays.
pub fn graustufen(rgba: &[u8], eink_stufen: Option<u32>) -> Vec<u8> {
    rgba.as_chunks::<4>().0
        .iter()
        .flat_map(|p| {
            let l = 0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32;
            let g = match eink_stufen {
                Some(n) if n >= 2 => ((l / 255.0 * (n - 1) as f32).round() * 255.0 / (n - 1) as f32) as u8,
                _ => l.round() as u8,
            };
            [g, g, g, p[3]]
        })
        .collect()
}

/// Umschlagmaße eines Portals für den Umschlagsatz.
pub fn masse(p: &Portal, seiten: u32) -> umschlag::Masse {
    let (gesamt_b, gesamt_h) = p.umschlag_mm(seiten);
    umschlag::Masse { gesamt_b, gesamt_h, ruecken: p.ruecken_mm(seiten), beschnitt: p.beschnitt_mm, seite_b: p.druck_format_mm.0, seite_h: p.druck_format_mm.1 }
}

/// Bandbezeichnung für den Rücken („BAND 1“) aus `buchbau.yaml`, falls der Band dort steht.
pub fn band_ruecken(bandordner: &Path) -> String {
    use werkbank_satz::buchbau::profil::Profil;
    Profil::suchen(bandordner)
        .and_then(|(buch, profil)| profil.band_zu_ordner(&buch, bandordner).map(|b| b.umschlag.band_ruecken.clone()))
        .unwrap_or_default()
}

/// Setzt den Umschlag für ein Portal: Vorschaubild (`bild_px` breit) und/oder PDF. Dieselbe Funktion wie beim Bauen.
pub fn umschlag_setzen(
    daten: &CoverDaten,
    portal: &Portal,
    seiten: u32,
    bandordner: Option<&Path>,
    isbn: Option<&str>,
    mit_pdf: bool,
    bild_px: Option<u32>,
) -> Result<umschlag::Ergebnis, String> {
    let ordner = bandordner.unwrap_or(Path::new(""));
    let text = klappentext::lesen(&bandordner.map(klappentext_lesen).unwrap_or_default());
    let ruecken = bandordner.map(band_ruecken).unwrap_or_default();
    let e = umschlag::eingabe_aus_cover(daten, ordner, masse(portal, seiten), &text, seiten as usize, &daten.autor, &daten.titel, &ruecken, &portal.name, isbn);
    umschlag::setzen(&e, mit_pdf, bild_px)
}

/// Das fertige Frontcover des Bandes (`cover.png`), wenn im Cover-Designer kein Hintergrundbild gewählt ist.
/// Die Vorschau zeigt in diesem Fall genau dieses Bild, also muss auch der Export es nehmen (statt eines
/// Standardentwurfs aus den Cover-Daten). Liefert das SVG, das das Bild in Originalproportion auf `breite` skaliert.
fn fertiges_cover(daten: &CoverDaten, bandordner: &Path, breite: u32) -> Option<(String, u32, u32)> {
    if !daten.bild.is_empty() {
        return None;
    }
    let pfad = bandordner.join("cover.png");
    let voll = tiny_skia::Pixmap::decode_png(&std::fs::read(&pfad).ok()?).ok()?;
    let hoehe = (voll.height() as f32 * breite as f32 / voll.width() as f32).round().max(1.0) as u32;
    let uri = cover::bild_uri(pfad.to_str()?)?;
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{breite}" height="{hoehe}" viewBox="0 0 {breite} {hoehe}"><image href="{uri}" x="0" y="0" width="{breite}" height="{hoehe}"/></svg>"#
    );
    Some((svg, breite, hoehe))
}

/// Schreibt alle Formate in `.werkbank/cover/` und liefert die erzeugten Dateien.
///
/// Die E-Book-Formate (PNG, PDF) kommen aus `cover.png` des Bandes, solange im Cover-Designer kein Hintergrundbild
/// gewählt ist; die Komplettumschläge werden immer aus den Cover-Daten gesetzt.
pub fn exportieren(daten: &CoverDaten, portale: &[Portal], seiten: u32, bandordner: &Path, isbn: Option<&str>) -> Result<Vec<PathBuf>, String> {
    let ziel = bandordner.join(".werkbank").join("cover");
    std::fs::create_dir_all(&ziel).map_err(|e| e.to_string())?;
    let mut dateien = Vec::new();

    for p in portale {
        let k = kurzname(&p.name);
        let (b, h) = p.cover_ebook_px;
        let (svg, b, h) = fertiges_cover(daten, bandordner, b)
            .unwrap_or_else(|| (cover::svg_ebook(daten, b as f32, h as f32, Some((p, seiten))), b, h));
        let datei = ziel.join(format!("cover-ebook-{k}-{b}x{h}.png"));
        png_schreiben(&svg, b, &datei)?;
        dateien.push(datei);
        // PDF der Vorderseite, 96 Pixel je Zoll
        let pdf = ziel.join(format!("cover-ebook-{k}-{b}x{h}.pdf"));
        std::fs::write(&pdf, werkbank_satz::svg_als_pdf(&svg, b as f32 / 96.0 * 25.4, h as f32 / 96.0 * 25.4)?).map_err(|e| e.to_string())?;
        dateien.push(pdf);

        // Komplettumschlag: derselbe Satz wie beim Bauen (Typst), Text aus klappentext.md
        let (ub, _) = p.umschlag_mm(seiten);
        let r = umschlag_setzen(daten, p, seiten, Some(bandordner), isbn, true, Some((ub / 25.4 * 300.0).round() as u32))?; // 300 dpi
        let png = ziel.join(format!("umschlag-{k}-{seiten}-seiten.png"));
        let bild = r.bild.ok_or("kein Umschlagbild")?;
        let pixel = tiny_skia::Pixmap::from_vec(bild.rgba, tiny_skia::IntSize::from_wh(bild.breite, bild.hoehe).ok_or("Größe")?).ok_or("Bild")?;
        pixel.save_png(&png).map_err(|e| format!("{}: {e}", png.display()))?;
        dateien.push(png);
        let pdf = ziel.join(format!("umschlag-{k}-{seiten}-seiten.pdf"));
        std::fs::write(&pdf, r.pdf.ok_or("kein Umschlag-PDF")?).map_err(|e| e.to_string())?;
        dateien.push(pdf);
    }
    Ok(dateien)
}

#[cfg(test)]
mod tests {
    use super::*;
    use werkbank_kern::portale::Portalregeln;

    #[test]
    fn export_nimmt_fertiges_cover_png_ohne_hintergrundbild() {
        let o = std::env::temp_dir().join(format!("werkbank-test-cover-fertig-{}", std::process::id()));
        std::fs::create_dir_all(&o).unwrap();
        // Rotes Titelbild im Format 2 : 3
        let mut bild = tiny_skia::Pixmap::new(400, 600).unwrap();
        bild.fill(tiny_skia::Color::from_rgba8(200, 20, 20, 255));
        bild.save_png(o.join("cover.png")).unwrap();
        let regeln = Portalregeln::default();

        let dateien = exportieren(&CoverDaten::default(), &regeln.portale[..1], 200, &o, None).unwrap();
        let png = dateien.iter().find(|d| d.to_string_lossy().contains("cover-ebook") && d.extension().is_some_and(|e| e == "png")).unwrap();
        let aus = tiny_skia::Pixmap::load_png(png).unwrap();
        assert_eq!(aus.height() * 2, aus.width() * 3, "Proportion des cover.png bleibt");
        let mitte = aus.pixel(aus.width() / 2, aus.height() / 2).unwrap();
        assert!(mitte.red() > 150 && mitte.green() < 80, "Mitte muss das rote cover.png sein: {mitte:?}");

        // Mit gewähltem Hintergrundbild gilt der Entwurf aus den Cover-Daten
        let d = CoverDaten { bild: o.join("cover.png").to_string_lossy().into_owned(), ..Default::default() };
        assert!(fertiges_cover(&d, &o, 400).is_none());
        let _ = std::fs::remove_dir_all(&o);
    }

    #[test]
    fn bild_umschlag_mit_ruecken_kasten_und_graustufen() {
        // Helles, buntes Testbild
        let o = std::env::temp_dir().join("werkbank-test-cover-bildumschlag");
        std::fs::create_dir_all(&o).unwrap();
        let mut bild = tiny_skia::Pixmap::new(600, 400).unwrap();
        for (i, p) in bild.pixels_mut().iter_mut().enumerate() {
            let (x, y) = ((i % 600) as u8, (i / 600) as u8);
            *p = tiny_skia::ColorU8::from_rgba(200u8.saturating_add(x / 12), 170, 90u8.saturating_add(y / 8), 255).premultiply();
        }
        let bild_pfad = o.join("bild.png");
        bild.save_png(&bild_pfad).unwrap();

        let regeln = Portalregeln::default();
        let p = &regeln.portale[1];
        let d = CoverDaten {
            bild: bild_pfad.to_string_lossy().into_owned(),
            bild_abdunkeln: 0.0,
            ruecken_kasten_farbe: "#000000".into(),
            ruecken_kasten_deckkraft: 0.8,
            rueckseite_kasten_deckkraft: 0.0,
            ..Default::default()
        };
        let svg = cover::svg_umschlag(&d, p, 300, "Klappentext");
        let (b, h, px) = rendern(&svg, 900).unwrap();
        let g = cover::geometrie(p, 300);
        let spalte = |mm: f32| ((mm / g.gesamt.0 * b as f32) as u32).min(b - 1);
        let punkt = |x: u32, y: u32| {
            let i = ((y * b + x) * 4) as usize;
            [px[i], px[i + 1], px[i + 2]]
        };
        let y = h / 2;
        let hell = punkt(spalte(g.ruecken_x - 20.0), y);
        let ruecken = punkt(spalte(g.ruecken_x + g.ruecken_b * 0.15), y);
        let vorn = punkt(spalte(g.vorn_x + 40.0), y);
        assert!(hell[0] > 150, "Bild muss auf der Rückseite sichtbar sein: {hell:?}");
        assert!(vorn[0] > 150, "Bild läuft über die Vorderseite: {vorn:?}");
        assert!(ruecken[0] < hell[0] / 2, "Rückenkasten dunkelt ab: {ruecken:?} gegen {hell:?}");
        // Graustufen: alle Kanäle gleich
        let grau = graustufen(&px, None);
        assert!(grau.chunks(4).all(|p| p[0] == p[1] && p[1] == p[2]));
        // Ausgabe zum Ansehen
        let _ = std::fs::write(o.join("umschlag-farbe.rgba"), &px);
        let _ = std::fs::write(o.join("umschlag-grau.rgba"), &grau);
        let _ = std::fs::write(o.join("maße.txt"), format!("{b} {h}"));
    }

    #[test]
    fn graustufen_und_eink() {
        let px = [255u8, 0, 0, 255, 0, 255, 0, 255, 10, 10, 10, 255];
        let g = graustufen(&px, None);
        assert_eq!(&g[..4], &[54, 54, 54, 255], "Rot als Helligkeit 0,2126");
        assert_eq!(&g[4..8], &[182, 182, 182, 255]);
        let e = graustufen(&px, Some(16));
        // 16 Stufen: nur Vielfache von 17 kommen vor
        assert!(e.chunks(4).all(|p| p[0] % 17 == 0 && p[0] == p[1] && p[1] == p[2]), "{e:?}");
    }

    #[test]
    fn rendert_vorderseite_mit_text_und_exportiert() {
        let d = CoverDaten::default();
        let svg = cover::svg_vorderseite(&d, 800.0, 1280.0);
        let (b, h, pixel) = rendern(&svg, 200).unwrap();
        assert_eq!((b, h), (200, 320));
        assert_eq!(pixel.len(), 200 * 320 * 4);
        // Hintergrund #1b1c1f oben links, irgendwo im Bild muss heller Text (#f5f4f0) liegen
        assert_eq!(&pixel[..3], &[27, 28, 31]);
        assert!(pixel.chunks(4).any(|p| p[0] > 200 && p[1] > 200), "Text nicht gerendert (keine Systemschriften?)");

        let o = std::env::temp_dir().join("werkbank-test-cover-export");
        let _ = std::fs::remove_dir_all(&o);
        let mut regeln = Portalregeln::default();
        regeln.portale.truncate(1);
        regeln.portale[0].cover_ebook_px = (400, 640);
        let dateien = exportieren(&d, &regeln.portale, 120, &o, None).unwrap();
        // E-Book-Cover (PNG, PDF) und Komplettumschlag (PNG, PDF); der Umschlag ist nur noch Typst, kein SVG
        assert_eq!(dateien.len(), 4);
        assert!(dateien.iter().filter(|f| f.extension().is_some_and(|e| e == "pdf")).count() == 2);
        assert!(dateien.iter().all(|f| f.is_file() && std::fs::metadata(f).unwrap().len() > 1000), "{dateien:?}");
    }
}
