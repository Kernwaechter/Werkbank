//! Komplettumschlag (Rückseite U4, Rücken, Vorderseite U1, mit Beschnitt) wie
//! `build_wraparound_cover_pdf` im bisherigen `build_book.py` – als Typst statt HTML/Chrome.
//! Ist Ghostscript installiert, wird das PDF wie bisher zusätzlich nach CMYK umgewandelt (optional).

use super::pakete::{Plattform, umschlag_masse};
use super::pdf::inline_typst;
use super::profil::{Band, Profil};
use crate::markdown::zeichenkette;
use std::collections::HashMap;
use std::path::Path;

/// Text mit einfacher HTML-Auszeichnung (`<strong>`, `<code>`, `<br/>`, `&amp;`) als Typst.
fn html(text: &str) -> String {
    inline_typst(&text.replace("<br/>", "<br />").replace("<br>", "<br />"))
}

/// Titelzeile: `[Akzent]` farbig, `\n` Zeilenumbruch.
fn titel(text: &str) -> String {
    let mut aus = String::new();
    let mut puffer = String::new();
    let mut akzent = false;
    let leeren = |p: &mut String, aus: &mut String, akzent: bool| {
        if !p.is_empty() {
            let t = zeichenkette(p);
            aus.push_str(&if akzent { format!("#text(fill: hell, {t})") } else { format!("#{t}") });
            p.clear();
        }
    };
    for c in text.replace("\\n", "\n").chars() {
        match c {
            '[' => {
                leeren(&mut puffer, &mut aus, akzent);
                akzent = true;
            }
            ']' => {
                leeren(&mut puffer, &mut aus, akzent);
                akzent = false;
            }
            '\n' => {
                leeren(&mut puffer, &mut aus, akzent);
                aus.push_str("#linebreak()");
            }
            c => puffer.push(c),
        }
    }
    leeren(&mut puffer, &mut aus, akzent);
    aus
}

/// Typst-Quelltext des Komplettumschlags.
pub fn typst(profil: &Profil, band: &Band, plattform: Plattform, seiten: usize, mit_grafik: bool) -> String {
    let (ruecken, gesamt_b, gesamt_h) = umschlag_masse(profil, plattform, seiten);
    let [tw, th] = profil.format_mm;
    let s = plattform.beschnitt_mm;
    let u = &band.umschlag;
    let r = &profil.reihe;
    let barcode = if plattform.name == "KDP" {
        r##"#"Platzhalter für KDP-Barcode"#linebreak()#"(wird von Amazon KDP automatisch eingesetzt)""##
    } else {
        r##"#"Platzhalter für Tolino-Barcode"#linebreak()#"(min. 15 mm Abstand zu Rand & Rücken)""##
    };
    let merkmale = u.merkmale.iter().map(|m| format!("[{}]", html(m))).collect::<Vec<_>>().join(", ");
    let grafik = if mit_grafik { "image(\"grafik.svg\", width: 100%, height: 100%, fit: \"contain\")" } else { "none" };
    let bandfarbe = format!("rgb({})", zeichenkette(if u.abzeichen_grund.is_empty() { "#F59E0B" } else { &u.abzeichen_grund }));
    let bandschrift = format!("rgb({})", zeichenkette(if u.abzeichen_schrift.is_empty() { "#0F172A" } else { &u.abzeichen_schrift }));
    let headline = format!("[{}]", html(&u.rueckseite_titel));
    let rtext = format!("[{}]", html(u.rueckseite_text.trim()));
    let bio = format!("[{}]", html(&r.autor_bio));
    let untertitel_farbe = format!("rgb({})", zeichenkette(if u.untertitel_farbe.is_empty() { "#E2E8F0" } else { &u.untertitel_farbe }));

    format!(
        r##"#let sans = ("Noto Sans", "Liberation Sans", "DejaVu Sans", "Fira Sans")
#let mono = ("DejaVu Sans Mono", "Liberation Mono")
#let grund = rgb("#0B1120")
#let hell = rgb("#38BDF8")
#let grau = rgb("#94A3B8")
#let bandfarbe = {bandfarbe}
#let ic(code) = text(font: mono, code)
#let lk(ziel, inhalt) = inhalt
#set page(width: {gesamt_b}mm, height: {gesamt_h}mm, margin: 0pt, fill: grund)
#set text(font: sans, fill: rgb("#F8FAFC"), lang: "de", top-edge: 0.78em, bottom-edge: -0.22em)
#set par(justify: false)

// Raster im Hintergrund (16 mm)
#for x in range(0, int({gesamt_b} / 16) + 1) {{ place(dx: x * 16mm, line(angle: 90deg, length: {gesamt_h}mm, stroke: 0.25mm + rgb("#38BDF80D"))) }}
#for y in range(0, int({gesamt_h} / 16) + 1) {{ place(dy: y * 16mm, line(length: {gesamt_b}mm, stroke: 0.25mm + rgb("#38BDF80D"))) }}

// U4: Rückseite
#place(dx: {s}mm, dy: {s}mm, box(width: {tw}mm, height: {th}mm, inset: (top: 16mm, x: 16mm, bottom: 15mm),
  grid(rows: (1fr, auto), {{
    block(below: 3mm, text(fill: hell, size: 9.5pt, weight: 800, tracking: 1.5pt, upper({kicker})))
    block(below: 5mm, par(leading: 0.2em, text(size: 18pt, weight: 900, fill: white, {headline})))
    block(below: 4mm, par(leading: 0.5em, text(size: 10pt, fill: rgb("#CBD5E1"), {rtext})))
    block(above: 2mm, below: 5mm, width: 100%, fill: rgb("#1E293BD9"), stroke: (left: 1.2mm + bandfarbe), radius: 2mm,
      inset: (x: 5.5mm, y: 4.5mm), {{
        block(below: 2.5mm, text(fill: rgb("#FDE047"), size: 10pt, weight: "bold", tracking: 0.8pt, upper({mtitel})))
        set par(leading: 0.55em)
        for m in ({merkmale},) {{
          block(above: 0.55em, below: 0.55em, text(size: 9.3pt, fill: rgb("#F1F5F9"), [#text(fill: hell, weight: "bold", "▸ ")#m]))
        }}
      }})
    block(width: 100%, stroke: (top: 0.3mm + rgb("#94A3B840")), inset: (top: 4mm), {{
      set par(leading: 0.45em)
      text(size: 10pt, weight: "bold", fill: rgb("#F8FAFC"), {bio_titel})
      v(1.5mm)
      text(size: 9pt, fill: grau, {bio})
    }})
  }}, grid(columns: (1fr, auto), align: (left + bottom, right + bottom), {{
    text(font: mono, size: 9.5pt, weight: "bold", fill: hell, {web})
    v(1mm)
    text(size: 7.5pt, fill: grau, {download} + " · {seiten} Seiten")
  }}, box(width: 50.8mm, height: 30.5mm, fill: white, radius: 1.5mm, inset: 2mm,
    align(center + horizon, text(size: 7pt, fill: rgb("#334155"), [{barcode}])))))))

// Buchrücken
#place(dx: {rx}mm, box(width: {ruecken}mm, height: {gesamt_h}mm,
  fill: gradient.linear(rgb("#0F172A"), rgb("#1E293B"), rgb("#0F172A"), angle: 90deg),
  stroke: (left: 0.25mm + rgb("#38BDF838"), right: 0.25mm + rgb("#38BDF838")),
  align(center + horizon, rotate(-90deg, reflow: true, box(stack(dir: ltr, spacing: 6mm,
    align(horizon, text(size: 10pt, weight: 600, fill: grau, {autor})),
    align(horizon, text(size: 10pt, weight: 800, fill: hell, {ruecken_titel})),
    align(horizon, box(fill: bandfarbe, inset: (x: 1.5mm, y: 1mm), radius: 1mm, text(size: 8.5pt, weight: 900, fill: {bandschrift}, {band_ruecken}))),
  ))))))

// U1: Vorderseite mit Akzentstreifen
#place(dx: {fx}mm, box(width: 2.2mm, height: {gesamt_h}mm, fill: gradient.linear(hell, rgb("#2563EB"), bandfarbe, angle: 90deg)))
#place(dx: {fx}mm, dy: {s}mm, box(width: {tw}mm, height: {th}mm, inset: (top: 11mm, x: 12mm, bottom: 10mm),
  grid(rows: (auto, 1fr, auto), {{
    block(below: 4mm, stack(dir: ltr, spacing: 2mm,
      box(fill: bandfarbe, inset: (x: 2.8mm, y: 1.2mm), radius: 1mm, text(size: 7.5pt, weight: 900, tracking: 0.8pt, fill: {bandschrift}, upper({band_etikett}))),
      box(fill: rgb("#0F172ABF"), stroke: 0.3mm + rgb("#38BDF873"), inset: (x: 2.4mm, y: 1.1mm), radius: 1mm,
        text(size: 6.5pt, weight: 700, tracking: 0.5pt, fill: rgb("#7DD3FC"), upper([{abzeichen}]))),
    ))
    block(below: 1.5mm, text(size: 10.5pt, weight: 700, tracking: 1pt, fill: hell, upper({dachzeile})))
    block(below: 2.8mm, par(leading: 0.06em, text(size: 31pt, weight: 900, fill: white, [{titel}])))
    block(below: 2mm, par(leading: 0.25em, text(size: 13pt, weight: 700, fill: {untertitel_farbe}, {untertitel})))
    par(leading: 0.42em, text(size: 8.8pt, fill: grau, {lead}))
  }}, align(center + horizon, box(width: 100%, height: 105mm, {grafik})),
  block(width: 100%, stroke: (top: 0.3mm + rgb("#94A3B838")), inset: (top: 3.5mm),
    grid(columns: (1fr, auto), align: (left + bottom, right + bottom), {{
      text(size: 6.5pt, tracking: 0.8pt, fill: grau, "AUTOR")
      linebreak()
      text(size: 16pt, weight: 800, fill: rgb("#F8FAFC"), {autor})
    }}, {{
      box(fill: rgb("#1E293BE6"), stroke: 0.3mm + rgb("#38BDF866"), inset: (x: 2.6mm, y: 1.2mm), radius: 1.2mm,
        text(font: mono, size: 8pt, weight: "bold", fill: hell, {domain}))
      v(1mm)
      text(size: 6.3pt, fill: grau, {system})
    }})))))
"##,
        kicker = zeichenkette(&u.rueckseite_dachzeile),
        headline = headline,
        rtext = rtext,
        mtitel = zeichenkette(&u.merkmale_titel),
        merkmale = if merkmale.is_empty() { "[]".to_string() } else { merkmale },
        bio_titel = zeichenkette(&r.autor_bio_titel),
        bio = bio,
        web = zeichenkette(&r.webseiten_zeile),
        download = zeichenkette(&r.download_zeile),
        rx = s + tw,
        fx = s + tw + ruecken,
        autor = zeichenkette(&profil.autor),
        ruecken_titel = zeichenkette(if r.ruecken_titel.is_empty() { &profil.titel } else { &r.ruecken_titel }),
        band_ruecken = zeichenkette(&u.band_ruecken),
        band_etikett = zeichenkette(&u.band_etikett),
        abzeichen = html(&u.abzeichen_text),
        dachzeile = zeichenkette(&r.dachzeile),
        titel = titel(if r.titel.is_empty() { &profil.titel } else { &r.titel }),
        untertitel = zeichenkette(&u.untertitel),
        lead = zeichenkette(&u.lead),
        domain = zeichenkette(&r.domain),
        system = zeichenkette(&r.system_hinweis),
    )
}

/// Setzt den Komplettumschlag als PDF; wandelt ihn nach CMYK, wenn Ghostscript vorhanden ist.
pub fn schreiben(profil: &Profil, band: &Band, bandordner: &Path, plattform: Plattform, seiten: usize, ziel: &Path) -> Result<(), String> {
    let svg = bandordner.join("umschlag-grafik.svg");
    let mut dateien = HashMap::new();
    if let Ok(daten) = std::fs::read(&svg) {
        dateien.insert("grafik.svg".to_string(), daten);
    }
    let quelle = typst(profil, band, plattform, seiten, dateien.contains_key("grafik.svg"));
    let ergebnis = crate::welt::typst_zu_pdf(quelle, None, dateien)?;
    let gs = super::programm_finden(&["gs", "gswin64c"]);
    match gs {
        Some(gs) => {
            let roh = ziel.with_extension("roh.pdf");
            std::fs::write(&roh, &ergebnis.bytes).map_err(|e| e.to_string())?;
            let ok = std::process::Command::new(gs)
                .args(["-sDEVICE=pdfwrite", "-dCompatibilityLevel=1.4", "-sColorConversionStrategy=CMYK", "-dProcessColorModel=/DeviceCMYK", "-dNOPAUSE", "-dQUIET", "-dBATCH"])
                .arg(format!("-sOutputFile={}", ziel.display()))
                .arg(&roh)
                .status()
                .is_ok_and(|s| s.success());
            let _ = std::fs::remove_file(&roh);
            if !ok {
                std::fs::write(ziel, &ergebnis.bytes).map_err(|e| e.to_string())?;
            }
        }
        None => std::fs::write(ziel, &ergebnis.bytes).map_err(|e| format!("{}: {e}", ziel.display()))?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn umschlag_wird_gesetzt() {
        let p: super::Profil = serde_yaml::from_str(
            "titel: Buch & Co\nautor: A\nverlag: V\nformat_mm: [170, 240]\nraender_mm: {innen: 22, aussen: 18, oben: 20, unten: 22}\nquellen: []\nrubriken: []\nbaende:\n- {nummer: 1, ordner: B1, untertitel: U, uuid: x, kurzname: k, umschlag: {band_etikett: Band 1, merkmale: ['<strong>A:</strong> <code>x</code> &amp; y']}}\nreihe: {titel: 'Die [A] &\\n[B]C', autor_bio: '<strong>F</strong>'}\n",
        )
        .unwrap();
        for plattform in [super::super::pakete::KDP, super::super::pakete::TOLINO] {
            let q = super::typst(&p, &p.baende[0], plattform, 300, false);
            let e = crate::welt::typst_zu_pdf(q, None, Default::default());
            assert!(e.is_ok(), "{}: {:?}", plattform.name, e.err());
        }
    }

    #[test]
    fn titelzeile_mit_akzent() {
        let t = super::titel("Die [Flutter-] &\\n[Dart-]Werkstatt");
        assert_eq!(t, r##"#"Die "#text(fill: hell, "Flutter-")#" &"#linebreak()#text(fill: hell, "Dart-")#"Werkstatt""##);
    }
}
