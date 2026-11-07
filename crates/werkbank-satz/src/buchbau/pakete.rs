//! Verlagsordner `webpublishing/`, `kdp/` und `tolino/` wie `build_publishing_packages`
//! im bisherigen `build_book.py` (Texte der Hinweisdateien wörtlich übernommen).

use super::profil::{Band, Profil};
use std::path::{Path, PathBuf};

/// Papierstärke und Beschnitt einer Druckplattform.
#[derive(Debug, Clone, Copy)]
pub struct Plattform {
    pub name: &'static str,
    /// Rückenstärke je Seite in mm.
    pub mm_je_seite: f32,
    /// Beschnitt rundum in mm.
    pub beschnitt_mm: f32,
}

/// Amazon KDP: weißes Papier, 0,0572 mm je Seite, 3,2 mm Beschnitt.
pub const KDP: Plattform = Plattform { name: "KDP", mm_je_seite: 0.0572, beschnitt_mm: 3.2 };
/// Tolino Media: 90 g/m² weiß, ca. 0,055 mm je Seite, 3,0 mm Beschnitt.
pub const TOLINO: Plattform = Plattform { name: "Tolino", mm_je_seite: 0.055, beschnitt_mm: 3.0 };

/// Auf zwei Nachkommastellen runden (wie Pythons `round(x, 2)`).
fn r2(x: f32) -> f32 {
    (x * 100.0).round() / 100.0
}

/// Maße eines Komplettumschlags: (Rücken, Gesamtbreite, Gesamthöhe) in mm.
pub fn umschlag_masse(profil: &Profil, plattform: Plattform, seiten: usize) -> (f32, f32, f32) {
    let ruecken = r2(seiten as f32 * plattform.mm_je_seite);
    let [b, h] = profil.format_mm;
    let s = plattform.beschnitt_mm;
    (ruecken, r2(s + b + ruecken + b + s), r2(s + h + s))
}

/// Zahl im deutschen Format mit `stellen` Nachkommastellen (Komma als Trennzeichen).
fn de(x: f32, stellen: usize) -> String {
    format!("{x:.stellen$}").replace('.', ",")
}

/// E-Book-Cover 1600 × 2560 px (1 : 1,6) als JPEG aus `cover.png`: auf 1600 px Breite skaliert,
/// mittig auf dunklen Grund (#0B1120) gesetzt, der linke Akzentstreifen (21 px) läuft über die volle Höhe.
pub fn ebook_cover_jpg(cover_png: &Path, ziel: &Path) -> Result<(), String> {
    let bild = image::open(cover_png).map_err(|e| format!("{}: {e}", cover_png.display()))?.to_rgb8();
    let (breite, hoehe) = (1600u32, 2560u32);
    let skaliert_h = (bild.height() as f32 * (breite as f32 / bild.width() as f32)).round() as u32;
    let skaliert = image::imageops::resize(&bild, breite, skaliert_h, image::imageops::FilterType::Lanczos3);
    let mut leinwand = image::RgbImage::from_pixel(breite, hoehe, image::Rgb([11, 17, 32]));
    let y_off = (hoehe.saturating_sub(skaliert_h)) / 2;
    image::imageops::replace(&mut leinwand, &skaliert, 0, y_off as i64);
    for x in 0..21.min(breite) {
        let oben = *skaliert.get_pixel(x, 5.min(skaliert_h - 1));
        let unten = *skaliert.get_pixel(x, skaliert_h.saturating_sub(5));
        for y in 0..y_off {
            leinwand.put_pixel(x, y, oben);
        }
        for y in (y_off + skaliert_h)..hoehe {
            leinwand.put_pixel(x, y, unten);
        }
    }
    let datei = std::fs::File::create(ziel).map_err(|e| format!("{}: {e}", ziel.display()))?;
    let mut kodierer = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(datei), 95);
    kodierer.encode_image(&leinwand).map_err(|e| e.to_string())
}

/// Eingaben für die Verlagsordner.
pub struct Eingaben<'a> {
    pub buchordner: &'a Path,
    pub bandordner: &'a Path,
    pub profil: &'a Profil,
    pub band: &'a Band,
    pub farb_pdf: &'a Path,
    pub grau_pdf: &'a Path,
    pub epub: &'a Path,
    pub seiten: usize,
    /// Frontcover als Bild (sonst `cover.png` im Bandordner).
    pub cover: Option<&'a Path>,
    /// Komplettumschlag je Plattform erzeugen: (Plattform, Zielpfad) → Ergebnis.
    pub umschlag: &'a dyn Fn(Plattform, &Path) -> Result<(), String>,
}

/// Befüllt die drei Veröffentlichungsordner. Liefert die geschriebenen Dateien.
pub fn schreiben(e: &Eingaben, hinweise: &mut Vec<String>) -> Result<Vec<PathBuf>, String> {
    let titel = format!("{} – {}", e.profil.titel, e.band.untertitel);
    let slug = &e.band.kurzname;
    let (tolino, kdp, web) = (e.buchordner.join("tolino"), e.buchordner.join("kdp"), e.buchordner.join("webpublishing"));
    let mut dateien = Vec::new();
    for d in [&tolino, &kdp, &web] {
        std::fs::create_dir_all(d).map_err(|err| format!("{}: {err}", d.display()))?;
    }
    let kopieren = |von: &Path, nach: PathBuf, dateien: &mut Vec<PathBuf>| -> Result<(), String> {
        if von.is_file() {
            std::fs::copy(von, &nach).map_err(|err| format!("{}: {err}", nach.display()))?;
            dateien.push(nach);
        }
        Ok(())
    };
    let schreib = |pfad: PathBuf, text: &str, dateien: &mut Vec<PathBuf>| -> Result<(), String> {
        std::fs::write(&pfad, text).map_err(|err| format!("{}: {err}", pfad.display()))?;
        dateien.push(pfad);
        Ok(())
    };

    // Klappentext in alle drei Ordner
    // Shop-Text: `klappentext-shop.md` (der lange Text für Buchbeschreibungen), sonst `klappentext.md`
    let shop = [e.bandordner.join("klappentext-shop.md"), e.bandordner.join("klappentext.md")].into_iter().find(|p| p.is_file());
    match shop.map(std::fs::read_to_string).unwrap_or_else(|| Err(std::io::ErrorKind::NotFound.into())) {
        Ok(k) => {
            for d in [&tolino, &kdp, &web] {
                schreib(d.join("KLAPPENTEXT.md"), &k, &mut dateien)?;
            }
        }
        Err(_) => hinweise.push(format!("{}/klappentext.md fehlt – KLAPPENTEXT.md nicht geschrieben.", e.band.ordner)),
    }

    let cover_png = e.cover.map(Path::to_path_buf).unwrap_or_else(|| e.bandordner.join("cover.png"));
    let webseiten = match e.profil.webseiten.as_slice() {
        [] => "eure Webseite".to_string(),
        [eine] => format!("**`{eine}`**"),
        mehrere => {
            let (letzte, rest) = mehrere.split_last().unwrap();
            format!("{} und **`{letzte}`**", rest.iter().map(|w| format!("**`{w}`**")).collect::<Vec<_>>().join(", "))
        }
    };
    let [breite, hoehe] = e.profil.format_mm;
    let r = &e.profil.raender_mm;

    // 1. webpublishing
    kopieren(e.farb_pdf, web.join(format!("{slug}-farbausgabe.pdf")), &mut dateien)?;
    kopieren(e.epub, web.join(format!("{slug}-ebook.epub")), &mut dateien)?;
    kopieren(&cover_png, web.join(format!("{slug}-cover.png")), &mut dateien)?;
    schreib(
        web.join("LIESMICH-WEBPUBLISHING.md"),
        &format!(
            "# Webpublishing-Paket ({titel})\n\n\
Dieses Verzeichnis enthält die fertigen Dateien für den Direktverkauf und Download über {webseiten}:\n\n\
1. **`{slug}-farbausgabe.pdf`**\n   - Vollfarbiges Buch-PDF im Format **{b} × {h} cm** inklusive **randlosem Frontcover auf Seite 1**.\n   - Optimiert für Tablets, große Monitore und den privaten Farbdruck.\n\
2. **`{slug}-ebook.epub`**\n   - Validiertes **EPUB 3** (mit EPUB-2-NCX-Fallback) und eingebettetem Cover sowie allen farbigen Abbildungen.\n\
3. **`{slug}-cover.png`**\n   - Hochauflösendes Frontcover (`1700 × 2400 px`, 300 dpi) für Shop-Vorschauen.\n",
            b = de(breite / 10.0, 1),
            h = de(hoehe / 10.0, 1),
        ),
        &mut dateien,
    )?;

    // 2. KDP
    let (kdp_ruecken, kdp_b, kdp_h) = umschlag_masse(e.profil, KDP, e.seiten);
    kopieren(e.grau_pdf, kdp.join(format!("{slug}-kdp-buchblock-sw.pdf")), &mut dateien)?;
    kopieren(e.epub, kdp.join(format!("{slug}-kdp-kindle-ebook.epub")), &mut dateien)?;
    if cover_png.is_file() {
        let ziel = kdp.join(format!("{slug}-kdp-kindle-cover-1600x2560.jpg"));
        ebook_cover_jpg(&cover_png, &ziel)?;
        dateien.push(ziel);
    }
    let ziel = kdp.join(format!("{slug}-kdp-taschenbuch-umschlag.pdf"));
    match (e.umschlag)(KDP, &ziel) {
        Ok(()) => dateien.push(ziel),
        Err(f) => hinweise.push(format!("KDP-Umschlag nicht erzeugt: {f}")),
    }
    let zoll = |mm: f32| de(mm / 25.4, 2);
    schreib(
        kdp.join("LIESMICH-KDP.md"),
        &format!(
            "# Amazon KDP Veröffentlichungspaket ({titel})\n\n\
Alle Dateien in diesem Ordner sind exakt nach den aktuellen Vorgaben von **Amazon Kindle Direct Publishing (KDP)** (`https://kdp.amazon.com/de_DE/`) vorbereitet.\n\n---\n\n\
## 1. Taschenbuch (Print-on-Demand)\n\n### Hochzuladende Dateien im KDP-Dashboard:\n\
* **Manuskript (Buchblock):** `{slug}-kdp-buchblock-sw.pdf`\n* **Buchcover (Komplett-Umschlag):** `{slug}-kdp-taschenbuch-umschlag.pdf`\n\n\
### Einzustellende Druckparameter bei KDP:\n\
* **Druckoption:** Schwarz-Weiß-Druck mit **weißem Papier** (*Black & white interior with white paper*)\n\
* **Beschnitt (Bleed) für den Buchblock:** **Ohne Beschnitt** (*No Bleed*)\n\
* **Buchformat (Trim Size):** Benutzerdefiniertes Format **{bcm} × {hcm} cm** (`{bz} × {hz} Zoll`)\n\
* **Cover-Oberfläche:** Matt (empfohlen für dunkle Cover) oder Glänzend\n\n\
### Technische Maßkontrolle dieser Ausgabe:\n\
* **Umfang:** Exakt **{s} Seiten** (durch 4 teilbar)\n\
* **Farbraum Buchblock:** Echtes `DeviceGray` (reine Graustufen, alle Schriften eingebettet)\n\
* **Seitenränder (Gegengleich / Gespiegelt):**\n  * Bundsteg (Innen): `{innen} mm` (KDP-Minimum bei {s} Seiten: `15,9 mm`)\n  * Außensteg: `{aussen} mm` (KDP-Minimum: `6,4 mm`)\n  * Kopf- / Fußsteg: `{oben} mm` / `{unten} mm`\n\
* **Berechnete KDP-Umschlagmaße (Weißes Papier = `0,0572 mm` pro Seite):**\n\
  * **Buchrücken-Breite (Spine):** `{s} × 0,0572 mm` = **`{ruecken} mm`**\n\
  * **Beschnittzugabe Umschlag (Bleed):** umlaufend **`{beschnitt} mm`** (`0,125 Zoll`)\n\
  * **Gesamtformat Umschlag-PDF:** **`{gb} mm × {gh} mm`**\n\
  * **Barcode:** Unten rechts auf der Rückseite (U4) ist das weiße KDP-Barcode-Feld (`50,8 × 30,5 mm`, mit `8 mm` Abstand zu Rücken und Unterkante; KDP verlangt mindestens `0,25 Zoll`) freigehalten. KDP druckt den Barcode dort selbst, auch bei eigener ISBN. Das Feld nicht beschriften; bei eigenem Barcode im Cover das Häkchen „Cover enthält bereits einen Barcode“ setzen.\n\n---\n\n\
## 2. Kindle E-Book\n\n### Hochzuladende Dateien im KDP-Dashboard:\n\
* **Kindle-Manuskript:** `{slug}-kdp-kindle-ebook.epub` (EPUB 3)\n\
* **Kindle-Produktbild (Frontcover):** `{slug}-kdp-kindle-cover-1600x2560.jpg` (`1600 × 2560 Pixel`, Seitenverhältnis `1 : 1,6`, sRGB-JPEG)\n",
            bcm = de(breite / 10.0, 1),
            hcm = de(hoehe / 10.0, 1),
            bz = zoll(breite),
            hz = zoll(hoehe),
            s = e.seiten,
            innen = de(r.innen, 1),
            aussen = de(r.aussen, 1),
            oben = de(r.oben, 1),
            unten = de(r.unten, 1),
            ruecken = de(kdp_ruecken, 2),
            beschnitt = de(KDP.beschnitt_mm, 1),
            gb = de(kdp_b, 2),
            gh = de(kdp_h, 2),
        ),
        &mut dateien,
    )?;

    // 3. Tolino
    let (to_ruecken, to_b, to_h) = umschlag_masse(e.profil, TOLINO, e.seiten);
    kopieren(e.grau_pdf, tolino.join(format!("{slug}-tolino-buchblock-sw.pdf")), &mut dateien)?;
    kopieren(e.epub, tolino.join(format!("{slug}-tolino-ebook.epub")), &mut dateien)?;
    if cover_png.is_file() {
        let ziel = tolino.join(format!("{slug}-tolino-ebook-cover-1600x2560.jpg"));
        ebook_cover_jpg(&cover_png, &ziel)?;
        dateien.push(ziel);
    }
    let ziel = tolino.join(format!("{slug}-tolino-taschenbuch-umschlag.pdf"));
    match (e.umschlag)(TOLINO, &ziel) {
        Ok(()) => dateien.push(ziel),
        Err(f) => hinweise.push(format!("Tolino-Umschlag nicht erzeugt: {f}")),
    }
    schreib(
        tolino.join("LIESMICH-TOLINO.md"),
        &format!(
            "# Tolino Media Veröffentlichungspaket ({titel})\n\n\
Alle Dateien in diesem Ordner sind nach dem offiziellen Druckdatenleitfaden von **Tolino Media** (`https://www.tolino-media.de/selfpublishing/ueberblick/`) aufbereitet.\n\n---\n\n\
## 1. E-Book (Tolino-Allianz: Thalia, Hugendubel, eBook.de, Osiander, Weltbild)\n\n### Hochzuladende Dateien:\n\
* **E-Book-Datei:** `{slug}-tolino-ebook.epub` (Standardkonformes EPUB 3.0 mit NCX- und NAV-Inhaltsverzeichnis)\n\
* **E-Book-Cover:** `{slug}-tolino-ebook-cover-1600x2560.jpg` (`1600 × 2560 Pixel`, Hochformat `1 : 1,6`, RGB-JPEG, mind. 1600 px auf der kurzen Seite erfüllt)\n\n---\n\n\
## 2. Print-on-Demand (Taschenbuch / Softcover)\n\n### Hochzuladende Dateien:\n\
* **Buchblock (Innenteil):** `{slug}-tolino-buchblock-sw.pdf`\n* **Umschlag (Cover-Spread):** `{slug}-tolino-taschenbuch-umschlag.pdf`\n\n\
### Erfüllte Tolino-Media-Pflichtvorgaben:\n\
* **Seitenzahl durch 4 teilbar:** Der Buchblock hat exakt **{s} Seiten** (`{s} ÷ 4 = {boegen}` Druckbögen ohne Rest). Am Ende des Buchblocks wurden dafür bei Bedarf automatisch passende Werkbank-Notizseiten ergänzt.\n\
* **Farbraum Buchblock:** Reines `DeviceGray` (Graustufen ohne RGB-Altlasten, alle Schriften vollständig eingebettet).\n\
* **Gegengleiche Seitenränder (Bundsteg):** Innensteg `{innen} mm`, Außensteg `{aussen} mm`, Kopfsteg `{oben} mm`, Fußsteg `{unten} mm`.\n\
* **Papierempfehlung bei Tolino:** **90 g/m² Weiß** (ideal für Quellcode und Screenshots).\n\
* **Umschlag-Maße (`{gb} mm × {gh} mm`):**\n\
  * Umlaufender Beschnitt (Bleed): **`{beschnitt} mm`** (Tolino-Vorgabe für Softcover)\n\
  * Buchrücken: **`{ruecken} mm`**\n\
  * **Barcode** unten rechts auf der Rückseite (U4), `33,4 × 18,1 mm` (Tolino: nicht verkleinern, verzerren oder einfärben), mit **`16 mm` Abstand** zum Buchrücken und zur Seitenkante (Tolino fordert mindestens `15 mm`). Ist in den Buchdaten eine gültige ISBN-13 eingetragen, ist der EAN-13-Barcode daraus schon gesetzt; sonst bleibt ein weißes Feld in dieser Größe, in das du den im Tolino-Dashboard (Buchdaten) erzeugten Barcode setzt. Der Barcode gehört nur auf den Umschlag, nicht in den Buchblock.\n",
            s = e.seiten,
            boegen = e.seiten / 4,
            innen = de(r.innen, 0),
            aussen = de(r.aussen, 0),
            oben = de(r.oben, 0),
            unten = de(r.unten, 0),
            gb = de(to_b, 2),
            gh = de(to_h, 2),
            beschnitt = de(TOLINO.beschnitt_mm, 1),
            ruecken = de(to_ruecken, 2),
        ),
        &mut dateien,
    )?;
    Ok(dateien)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umschlagmasse_wie_bisher() {
        let p: Profil = serde_yaml::from_str(
            "titel: B\nautor: A\nverlag: V\nformat_mm: [170, 240]\nraender_mm: {innen: 22, aussen: 18, oben: 20, unten: 22}\nquellen: []\nrubriken: []\nbaende: []\n",
        )
        .unwrap();
        // 332 Seiten: KDP 18,99 mm Rücken, 365,39 × 246,4 mm; Tolino 18,26 mm, 364,26 × 246 mm
        assert_eq!(umschlag_masse(&p, KDP, 332), (18.99, 365.39, 246.4));
        assert_eq!(umschlag_masse(&p, TOLINO, 332), (18.26, 364.26, 246.0));
        assert_eq!(de(18.99, 2), "18,99");
    }
}
