//! Setzt den Umschlag eines Bandes zur Probe: PDF und PNG für Tolino und KDP.
//! Aufruf: `cargo run -p werkbank-satz --example umschlag_probe -- <Buchordner> <Bandnummer> <Zielordner>`

use werkbank_satz::buchbau::{klappentext, profil::Profil, umschlag};

fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let (buch, nummer, ziel) = (std::path::Path::new(&a[0]), a[1].parse::<u32>().map_err(|e| e.to_string())?, std::path::Path::new(&a[2]));
    let profil = Profil::laden(buch)?;
    let band = profil.band(nummer).ok_or("Band nicht gefunden")?;
    let ordner = buch.join(&band.ordner);
    let md = std::fs::read_to_string(ordner.join("klappentext.md")).map_err(|e| e.to_string())?;
    let text = klappentext::lesen(&md);
    let mut cover = werkbank_kern::cover::CoverDaten::laden(&ordner).unwrap_or_default();
    if std::env::var_os("AUTOFARBEN").is_some() {
        let v = werkbank_satz::buchbau::farben::aus_bild(&std::fs::read(ordner.join("cover.png")).map_err(|e| e.to_string())?)?;
        println!("Vorschlag: {v:?}");
        cover.hintergrund = v.hintergrund;
        cover.text = v.text;
        cover.akzent = v.akzent;
        cover.ruecken_kasten_farbe = v.ruecken;
        cover.ruecken_kasten_deckkraft = v.ruecken_deckkraft;
        cover.rueckseite_kasten_farbe = v.rueckseite_kasten;
        cover.rueckseite_kasten_deckkraft = v.rueckseite_kasten_deckkraft;
    }
    let isbn = a.get(3).cloned().or_else(|| werkbank_satz::buchbau::isbn_aus_ordner(&ordner));
    println!("ISBN: {isbn:?}");
    std::fs::create_dir_all(ziel).map_err(|e| e.to_string())?;
    for (name, p) in [("tolino", werkbank_satz::buchbau::pakete::TOLINO), ("kdp", werkbank_satz::buchbau::pakete::KDP)] {
        let e = umschlag::Eingabe {
            masse: umschlag::Masse::aus_plattform(&profil, p, 332),
            farben: umschlag::Farben::aus_cover(&cover),
            klappentext: &text,
            seiten: 332,
            autor: &profil.autor,
            ruecken_titel: if profil.reihe.ruecken_titel.is_empty() { &profil.titel } else { &profil.reihe.ruecken_titel },
            band_ruecken: &band.umschlag.band_ruecken,
            vorderseite: std::fs::read(ordner.join("cover.png")).ok(),
            hintergrund: None,
            titeltexte_svg: None,
            barcode: umschlag::Barcode::fuer(p.name, cover.barcode, isbn.as_deref()),
            schrift_pt: cover.rueckseite_schrift_pt,
            schrift_auto: cover.rueckseite_schrift_auto,
        };
        let r = umschlag::setzen(&e, true, Some(1800))?;
        println!("{name}: {:?}", r.bericht);
        std::fs::write(ziel.join(format!("{name}.pdf")), r.pdf.unwrap()).map_err(|e| e.to_string())?;
        let b = r.bild.unwrap();
        image::RgbaImage::from_raw(b.breite, b.hoehe, b.rgba).unwrap().save(ziel.join(format!("{name}.png"))).map_err(|e| e.to_string())?;
    }
    Ok(())
}
