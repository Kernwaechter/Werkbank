//! Die Druck-PDFs des Buchbaus: Typst-Vorlage im Layout des bisherigen `build_book.py`
//! (CSS-Werte aus [`super::css`]) und Übersetzung der Knoten aus [`super::md`].
//!
//! Text wird nie als Typst-Auszeichnung gelesen, sondern als Zeichenkette (`#"…"`) gesetzt:
//! So bleiben gerade Anführungszeichen gerade und „1. Schritt“ am Absatzanfang wird keine Liste –
//! genau wie im bisherigen Chrome-Druck.

use super::css::{FARBE, GRAU, Palette};
use super::md::{Knoten, format_inline, reiner_titel};
use super::profil::Profil;
use super::Abschnittsdaten;
use crate::markdown::zeichenkette;

/// Fassung eines Druck-PDFs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fassung {
    /// Farbiges PDF (mit Cover als erster Seite, wenn vorhanden).
    Farbe,
    /// Graustufen für den Druck; am Ende Notizseiten bis die Seitenzahl durch 4 teilbar ist.
    Grau,
}

/// Grauwert (0–255) einer `#RRGGBB`-Farbe nach der Helligkeit (wie die Grau-Umwandlung im Druck).
fn grau_von(hex: &str) -> u8 {
    let h = hex.trim_start_matches('#');
    let k = |i: usize| u8::from_str_radix(h.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0) as f32;
    (0.299 * k(0) + 0.587 * k(2) + 0.114 * k(4)).round() as u8
}

/// Typst-Farbe: farbig als `rgb`, im Graudruck als reines Grau (`luma`).
fn farbe(hex: &str, fassung: Fassung) -> String {
    match fassung {
        Fassung::Farbe => format!("rgb(\"{hex}\")"),
        Fassung::Grau => format!("luma({})", grau_von(hex)),
    }
}

/// Die Vorlage: Seite, Schriften, Gestaltung aller Elemente.
pub fn vorlage(profil: &Profil, fassung: Fassung, untertitel: &str) -> String {
    let p: &Palette = if fassung == Fassung::Farbe { &FARBE } else { &GRAU };
    let f = |hex: &str| farbe(hex, fassung);
    let (rubrik_regeln, link_farbe) = if fassung == Fassung::Farbe {
        (
            r##"(
  werkbank: (rgb("#EFF6FF"), rgb("#1E3A8A"), rgb("#1E3A8A")),
  hinschauen: (rgb("#EFF6FF"), rgb("#1E3A8A"), rgb("#1E3A8A")),
  bughunter: (rgb("#FEF2F2"), rgb("#B91C1C"), rgb("#991B1B")),
  aufgabe: (rgb("#F0FDFA"), rgb("#0F766E"), rgb("#115E59")),
  klartext: (rgb("#FFFBEB"), rgb("#B45309"), rgb("#92400E")),
  toolbox: (rgb("#F1F5F9"), rgb("#334155"), rgb("#0F172A")),
)"##
            .to_string(),
            "rgb(\"#0000EE\")".to_string(),
        )
        } else {
            ("(:)".to_string(), "luma(30)".to_string())
        };
    let zeilennummern = if profil.zeilennummern {
        "#show raw.where(block: true): it => { show raw.line: l => box(width: 1.8em, align(right, text(fill: luma(140), str(l.number)))) + h(0.8em) + l.body; it }"
    } else {
        ""
    };
    let r = &profil.raender_mm;
    let ersetzungen = [
        ("@@BREITE@@", format!("{}mm", profil.format_mm[0])),
        ("@@HOEHE@@", format!("{}mm", profil.format_mm[1])),
        ("@@INNEN@@", format!("{}mm", r.innen)),
        ("@@AUSSEN@@", format!("{}mm", r.aussen)),
        ("@@OBEN@@", format!("{}mm", r.oben)),
        ("@@UNTEN@@", format!("{}mm", r.unten)),
        ("@@TITEL@@", zeichenkette(&format!("{} – {}", profil.titel, untertitel))),
        ("@@AUTOR@@", zeichenkette(&profil.autor)),
        ("@@SPRACHE@@", zeichenkette(&profil.sprache)),
        ("@@AKZENT1@@", f(p.akzent_primaer)),
        ("@@AKZENT2@@", f(p.akzent_sekundaer)),
        ("@@CODEGRUND@@", f(p.code_grund)),
        ("@@CODERAND@@", f(p.code_rand)),
        ("@@KOPFGRUND@@", f(p.code_kopf_grund)),
        ("@@KOPFSCHRIFT@@", f(p.code_kopf_schrift)),
        ("@@TEXT@@", f("#111111")),
        ("@@FUSS@@", f("#333333")),
        ("@@UNTERSCHRIFT@@", f("#444444")),
        ("@@NOTIZ@@", f("#777777")),
        ("@@NOTIZRAND@@", f("#CCCCCC")),
        ("@@LINK@@", link_farbe),
        ("@@GRAU@@", if fassung == Fassung::Grau { "true" } else { "false" }.to_string()),
        ("@@RUBRIKEN@@", rubrik_regeln),
        ("@@ZEILENNUMMERN@@", zeilennummern.to_string()),
    ];
    let mut t = VORLAGE.to_string();
    for (name, wert) in ersetzungen {
        t = t.replace(name, &wert);
    }
    t
}

/// Typst-Vorlage. Werte aus dem Stylesheet: 1 px = 0,75 pt; Zeilenhöhe als 1-em-Zeilenkasten plus Abstand.
const VORLAGE: &str = r##"// Buchbau der Werkbank – Layout wie das bisherige build_book.py (Chrome-Druck)
#let serif = ("Noto Serif", "Liberation Serif", "DejaVu Serif", "Libertinus Serif")
#let sans = ("Noto Sans", "Liberation Sans", "DejaVu Sans", "Fira Sans")
#let mono = ("DejaVu Sans Mono", "Liberation Mono")
#let akzent1 = @@AKZENT1@@
#let akzent2 = @@AKZENT2@@
#let codegrund = @@CODEGRUND@@
#let coderand = @@CODERAND@@
#let kopfgrund = @@KOPFGRUND@@
#let kopfschrift = @@KOPFSCHRIFT@@
#let grau = @@GRAU@@

#set document(title: @@TITEL@@, author: @@AUTOR@@)
#set page(
  width: @@BREITE@@, height: @@HOEHE@@,
  margin: (inside: @@INNEN@@, outside: @@AUSSEN@@, top: @@OBEN@@, bottom: @@UNTEN@@),
  footer: context align(center, text(font: sans, size: 9pt, fill: @@FUSS@@, counter(page).display())),
)
// Zeilenkasten von 1 em; der Abstand ergibt die Zeilenhöhe des Stylesheets (1,55)
#set text(font: serif, size: 10.5pt, lang: @@SPRACHE@@, fill: @@TEXT@@, hyphenate: true, top-edge: 0.78em, bottom-edge: -0.22em)
#set par(justify: true, leading: 0.55em, spacing: 8pt)
#set list(indent: 6pt, body-indent: 7pt, spacing: 0.55em + 4pt)
#set enum(indent: 4pt, body-indent: 6pt, spacing: 0.55em + 4pt)
#set raw(theme: "werkstatt.tmTheme")
// Typst verkleinert Code sonst auf 0,8 em; hier gelten die Größen des Stylesheets
#show raw: set text(font: mono)
#show raw.where(block: true): set text(size: 8.8pt)
#show raw.where(block: false): set text(size: 9pt)
@@ZEILENNUMMERN@@

// Überschriften Ebene 1 und 2: neue Seite, nicht vom Folgetext getrennt
#show heading: set text(font: sans, weight: "bold")
#show heading.where(level: 1): it => {
  pagebreak(weak: true)
  v(28pt)
  block(sticky: true, width: 100%, stroke: (bottom: 1.5pt + akzent1), inset: (bottom: 4.5pt), below: 13.4pt,
    text(size: 20pt, fill: akzent1, it.body))
}
#show heading.where(level: 2): it => {
  pagebreak(weak: true)
  v(22pt)
  block(sticky: true, width: 100%, below: 12.45pt, text(size: 15pt, fill: akzent2, it.body))
}
#let h3(inhalt) = block(sticky: true, above: 16pt, below: 12pt, text(font: sans, weight: "bold", size: 12pt, fill: akzent1, inhalt))
#let h4(inhalt) = block(sticky: true, above: 12pt, below: 14pt, text(font: sans, weight: "bold", size: 10.5pt, fill: akzent1, inhalt))
#let h5(inhalt) = block(sticky: true, above: 12pt, below: 17.5pt, text(font: sans, weight: "bold", size: 10.5pt, fill: akzent1, inhalt))
#let rubriken = @@RUBRIKEN@@
#let rubrik(art, inhalt) = {
  let (grund, rand, schrift) = rubriken.at(art, default: (rgb("#EFF6FF"), akzent1, akzent1))
  let stil = if grau {
    (fill: luma(242), stroke: (left: 3pt + black, top: 0.75pt + luma(187), bottom: 0.75pt + luma(187)), radius: 0pt, schrift: black)
  } else {
    (fill: grund, stroke: (left: 3pt + rand), radius: (top-right: 3pt, bottom-right: 3pt), schrift: schrift)
  }
  block(sticky: true, width: 100%, above: 16pt, below: 12pt, fill: stil.fill, stroke: stil.stroke, radius: stil.radius,
    inset: (x: 9pt, y: 4.5pt), text(font: sans, weight: "bold", size: 12pt, fill: stil.schrift, inhalt))
}

// Code im Text und Codeblöcke mit Kopfzeile
#let ic(code) = box(fill: codegrund, stroke: 0.75pt + coderand, radius: 2.25pt, inset: (x: 3pt, y: 0pt), outset: (y: 1.5pt),
  raw(code))
#let codeblock(etikett, inhalt, teilbar: false) = block(width: 100%, above: 10pt, below: 12pt, breakable: teilbar,
  fill: codegrund, stroke: 0.75pt + coderand, radius: 3pt, clip: true, {
    block(width: 100%, below: 0pt, fill: kopfgrund, stroke: (bottom: 0.75pt + coderand), inset: (x: 7.5pt, y: 2.25pt),
      text(font: sans, size: 7.5pt, weight: "bold", tracking: 0.05em, fill: kopfschrift, top-edge: "cap-height", bottom-edge: "baseline", etikett))
    block(width: 100%, above: 0pt, inset: (x: 7.5pt, y: 6pt), {
      set par(leading: 0.42em, justify: false)
      inhalt
    })
  })

// Verweise wie im Browser (unterstrichen)
#let lk(ziel, inhalt) = link(ziel, underline(text(fill: @@LINK@@, inhalt)))

// Tabellen
#let tabelle(spalten, ..zellen) = block(width: 100%, above: 12pt, below: 12pt, breakable: false, {
  set text(size: 9.5pt)
  set par(justify: false)
  table(columns: spalten, stroke: 0.75pt + coderand, inset: (x: 6pt, y: 4.5pt),
    fill: (x, y) => if y == 0 { kopfgrund }, align: left + top, ..zellen)
})
#let th(inhalt) = text(font: sans, weight: "bold", inhalt)

// Abbildungen: höchstens 92 % breit und 115 mm hoch, mit Rahmen und kursiver Unterschrift
#let figur(pfad, unterschrift) = block(width: 100%, above: 14pt, below: 14pt, breakable: false, layout(groesse => {
  let natur = measure(image(pfad))
  let breite = calc.min(groesse.width * 0.92, 115mm * (natur.width / natur.height))
  align(center, {
    box(stroke: 0.75pt + coderand, radius: 4.5pt, clip: true, image(pfad, width: breite))
    if unterschrift != none {
      v(5pt, weak: true)
      text(font: sans, size: 8.5pt, style: "italic", fill: @@UNTERSCHRIFT@@, unterschrift)
    }
  })
}))

#let linie() = block(above: 18pt, below: 18pt, line(length: 100%, stroke: 0.75pt + coderand))

// Titelei: Schmutztitel, Titelseite, Impressum
#let titelkopf(ebene, inhalt) = {
  let groesse = (20pt, 15pt, 12pt, 10.5pt, 10.5pt).at(ebene - 1)
  let farbe = if ebene == 2 { akzent2 } else { akzent1 }
  block(above: if ebene == 1 { 13.4pt } else { 12pt }, below: (13.4pt, 12.45pt, 12pt, 14pt, 17.5pt).at(ebene - 1),
    align(center, text(font: sans, weight: "bold", size: groesse, fill: farbe, inhalt)))
}
#let schmutztitel(inhalt) = { v(60mm); align(center, inhalt) }
#let titelseite(inhalt) = { v(45mm); align(center, inhalt) }
#let impressum(inhalt) = { v(20mm); set text(size: 9pt); set par(leading: 0.45em); inhalt }

// Inhaltsverzeichnis mit Seitenzahlen
#let verzeichnis(eintraege) = {
  pagebreak(weak: true)
  v(28pt)
  block(width: 100%, stroke: (bottom: 1.5pt + akzent1), inset: (bottom: 4.5pt), below: 13.4pt,
    text(font: sans, weight: "bold", size: 20pt, fill: akzent1, "Inhaltsverzeichnis"))
  for (ebene, titel, ziel) in eintraege {
    let zeile = link(ziel, box(width: 100%, {
      titel
      box(width: 1fr, repeat(gap: 2.5pt)[.])
      // Fester Platz für die Zahl: so ändert sie nie den Umbruch (sonst würde der Satz nicht stabil)
      box(width: 2.2em, align(right, context counter(page).at(ziel).first()))
    }))
    if ebene == 1 {
      block(above: 8pt + 0.55em, below: 0pt, text(font: sans, weight: "bold", zeile))
    } else {
      block(above: 3pt + 0.55em, below: 0pt, pad(left: 16pt, zeile))
    }
  }
  pagebreak(weak: true)
}

// Notizseite (Graudruck: auffüllen, bis die Seitenzahl durch 4 teilbar ist)
#let notizseite() = {
  pagebreak()
  v(1fr)
  align(center, text(font: sans, size: 8.5pt, fill: @@NOTIZ@@, "Notizen / Eigene Werkbank-Skizzen"))
  v(8pt)
  rect(width: 100%, height: 150mm, radius: 3pt, stroke: (paint: @@NOTIZRAND@@, thickness: 0.75pt, dash: "dashed"))
  v(18mm)
}
"##;

/// Text als Typst-Zeichenkette in Auszeichnung (`#"…"`), leer bleibt leer.
fn txt(s: &str) -> String {
    if s.is_empty() { String::new() } else { format!("#{}", zeichenkette(s)) }
}

/// HTML-Maskierungen auflösen (`&amp;` … wie von `format_inline` erzeugt).
fn entities(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&#x27;", "'").replace("&#39;", "'").replace("&amp;", "&")
}

/// Übersetzt das Inline-HTML von `format_inline` (code, a, strong, em, br) nach Typst.
pub fn inline_typst(html: &str) -> String {
    fn lesen(z: &str, pos: &mut usize, ende_tag: Option<&str>) -> String {
        let mut aus = String::new();
        let mut text = String::new();
        let leeren = |text: &mut String, aus: &mut String| {
            if !text.is_empty() {
                aus.push_str(&txt(&entities(text)));
                text.clear();
            }
        };
        while *pos < z.len() {
            let rest = &z[*pos..];
            if let Some(t) = ende_tag
                && rest.starts_with(t)
            {
                *pos += t.len();
                leeren(&mut text, &mut aus);
                return aus;
            }
            if rest.starts_with("<br />") {
                leeren(&mut text, &mut aus);
                aus.push_str("#linebreak()");
                *pos += 6;
                // Wie im Browser: Leerraum am Anfang der neuen Zeile fällt weg
                while z[*pos..].starts_with(' ') {
                    *pos += 1;
                }
            } else if let Some(r) = rest.strip_prefix("<code>") {
                leeren(&mut text, &mut aus);
                let e = r.find("</code>").unwrap_or(r.len());
                // Wie im Browser: Leerraum in Code im Fließtext schrumpft auf ein Leerzeichen
                let code = entities(&r[..e]).split_whitespace().collect::<Vec<_>>().join(" ");
                let code = if r[..e].starts_with(char::is_whitespace) && !code.is_empty() { format!(" {code}") } else { code };
                aus.push_str(&format!("#ic({})", zeichenkette(&code)));
                *pos += 6 + e + 7;
            } else if rest.starts_with("<strong>") {
                leeren(&mut text, &mut aus);
                *pos += 8;
                let innen = lesen(z, pos, Some("</strong>"));
                aus.push_str(&format!("#strong[{innen}]"));
            } else if rest.starts_with("<em>") {
                leeren(&mut text, &mut aus);
                *pos += 4;
                let innen = lesen(z, pos, Some("</em>"));
                aus.push_str(&format!("#emph[{innen}]"));
            } else if let Some(r) = rest.strip_prefix("<a href=\"") {
                leeren(&mut text, &mut aus);
                let e = r.find("\">").unwrap_or(0);
                let ziel = entities(&r[..e]);
                *pos += 9 + e + 2;
                let innen = lesen(z, pos, Some("</a>"));
                aus.push_str(&format!("#lk({}, [{innen}])", zeichenkette(&ziel)));
            } else {
                let c = rest.chars().next().unwrap_or(' ');
                text.push(c);
                *pos += c.len_utf8();
            }
        }
        leeren(&mut text, &mut aus);
        aus
    }
    let mut pos = 0;
    lesen(html, &mut pos, None)
}

/// Inline-Markdown direkt nach Typst (über dasselbe HTML wie im EPUB, also dieselben Regeln).
fn inline(md: &str) -> String {
    inline_typst(&format_inline(md).replace("&lt;br /&gt;", "<br />"))
}

/// Spaltengewichte einer Tabelle nach dem längsten Inhalt je Spalte (wie das automatische Tabellenlayout).
fn spalten(zeilen: &[Vec<String>]) -> String {
    let n = zeilen.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let gewichte: Vec<usize> = (0..n)
        .map(|k| zeilen.iter().map(|z| z.get(k).map(|c| reiner_titel(c.trim()).chars().count()).unwrap_or(0)).max().unwrap_or(1).clamp(3, 60))
        .collect();
    format!("({},)", gewichte.iter().map(|g| format!("{g}fr")).collect::<Vec<_>>().join(", "))
}

/// Übersetzt die Knoten eines Abschnitts. `titelei`: Überschriften in Schmutztitel/Titelseite stehen zentriert ohne Seitenwechsel.
pub fn knoten_typst(knoten: &[Knoten], zeilen_teilbar_ab: usize) -> String {
    let mut aus = String::new();
    // Offene Hülle der Titelei (half-title, title-page, imprint)
    let mut huelle: Option<&'static str> = None;
    // Offene Liste: (geordnet, nächste Nummer)
    let mut liste: Option<(bool, usize)> = None;
    let mut punkte: Vec<String> = Vec::new();

    fn liste_ausgeben(aus: &mut String, liste: &mut Option<(bool, usize)>, punkte: &mut Vec<String>) {
        if punkte.is_empty() {
            return;
        }
        let (geordnet, start) = liste.unwrap_or((false, 1));
        let elemente = punkte.iter().map(|p| format!("[{p}]")).collect::<Vec<_>>().join(", ");
        if geordnet {
            aus.push_str(&format!("#enum(start: {start}, {elemente})\n\n"));
            if let Some(l) = liste.as_mut() {
                l.1 += punkte.len();
            }
        } else {
            aus.push_str(&format!("#list({elemente})\n\n"));
        }
        punkte.clear();
    }

    for k in knoten {
        if !matches!(k, Knoten::Punkt(_)) {
            liste_ausgeben(&mut aus, &mut liste, &mut punkte);
        }
        match k {
            Knoten::Html(h) => match h.as_str() {
                "<div class=\"half-title\">" => {
                    huelle = Some("schmutztitel");
                    aus.push_str("#schmutztitel[\n");
                }
                "<div class=\"title-page\">" => {
                    huelle = Some("titelseite");
                    aus.push_str("#titelseite[\n");
                }
                "<div class=\"imprint\">" => {
                    huelle = Some("impressum");
                    aus.push_str("#impressum[\n");
                }
                "</div>" => {
                    if huelle.take().is_some() {
                        aus.push_str("]\n\n");
                    }
                }
                _ => aus.push_str("#pagebreak(weak: true)\n\n"),
            },
            Knoten::Absatz(t) => aus.push_str(&format!("{}\n\n", inline(t))),
            Knoten::Linie => aus.push_str("#linie()\n\n"),
            Knoten::Bild { unterschrift, pfad } => {
                let u = if unterschrift.is_empty() { "none".to_string() } else { format!("[{}]", inline(unterschrift)) };
                aus.push_str(&format!("#figur({}, {u})\n\n", zeichenkette(pfad)));
            }
            Knoten::Tabelle(zeilen) => {
                let mut zellen = Vec::new();
                for (nr, z) in zeilen.iter().enumerate() {
                    if nr > 0 && z.iter().all(|c| { let c = c.trim(); !c.is_empty() && c.chars().all(|x| x == '-' || x == ':') && c.matches('-').count() >= 2 }) {
                        continue;
                    }
                    for zelle in z {
                        let inhalt = inline(zelle.trim());
                        zellen.push(if nr == 0 { format!("th[{inhalt}]") } else { format!("[{inhalt}]") });
                    }
                }
                aus.push_str(&format!("#tabelle({}, {})\n\n", spalten(zeilen), zellen.join(", ")));
            }
            Knoten::Ueberschrift { ebene, roh, id, rubrik } => {
                let inhalt = inline(roh);
                if huelle.is_some() {
                    aus.push_str(&format!("#titelkopf({ebene}, [{inhalt}])\n\n"));
                } else if *ebene <= 2 {
                    aus.push_str(&format!("#heading(level: {ebene}, [{inhalt}]) <{id}>\n\n"));
                } else if let Some(art) = rubrik {
                    aus.push_str(&format!("#rubrik({}, [{inhalt}])\n\n", zeichenkette(art)));
                } else {
                    aus.push_str(&format!("#h{ebene}([{inhalt}])\n\n"));
                }
            }
            Knoten::ListeAuf(geordnet) => liste = Some((*geordnet, 1)),
            Knoten::ListeZu(_) => liste = None,
            Knoten::Punkt(t) => punkte.push(inline(t)),
            Knoten::Code { sprache, code } => {
                let rein = sprache.trim().to_lowercase();
                let etikett = if rein.is_empty() || rein == "text" { "CODE".to_string() } else { rein.to_uppercase() };
                let teilbar = code.lines().count() >= zeilen_teilbar_ab;
                let lang = if rein.is_empty() || rein == "text" { "none".to_string() } else { zeichenkette(&rein) };
                aus.push_str(&format!(
                    "#codeblock({}, raw(block: true, lang: {lang}, {}), teilbar: {teilbar})\n\n",
                    zeichenkette(&etikett),
                    zeichenkette(code)
                ));
            }
        }
    }
    liste_ausgeben(&mut aus, &mut liste, &mut punkte);
    if huelle.is_some() {
        aus.push_str("]\n\n");
    }
    aus
}

/// Der ganze Typst-Quelltext eines Druck-PDFs.
/// `cover`: virtueller Pfad des Coverbildes für die erste Seite (nur farbig).
/// `notizseiten`: so viele Notizseiten am Ende (Graudruck, bis die Seitenzahl durch 4 teilbar ist).
pub fn dokument(
    profil: &Profil,
    untertitel: &str,
    abschnitte: &[Abschnittsdaten],
    fassung: Fassung,
    cover: Option<&str>,
    notizseiten: usize,
) -> String {
    let mut t = vorlage(profil, fassung, untertitel);
    t.push('\n');
    if let (Some(c), Fassung::Farbe) = (cover, fassung) {
        t.push_str(&format!(
            "#page(margin: 0pt, footer: none, image({}, width: 100%, height: 100%, fit: \"stretch\"))\n\n",
            zeichenkette(c)
        ));
    }
    // Codeblöcke ab dieser Zeilenzahl dürfen umbrechen (sonst ragten sie über die Seite)
    const TEILBAR_AB: usize = 38;
    for (nr, a) in abschnitte.iter().enumerate() {
        t.push_str(&knoten_typst(&a.abschnitt.knoten, TEILBAR_AB));
        // Nach der Titelei das Inhaltsverzeichnis (Ebene 1 und 2, ohne Titelei)
        if nr == 0 {
            let eintraege: Vec<String> = abschnitte
                .iter()
                .filter(|x| !x.ist_titelei)
                .flat_map(|x| x.abschnitt.toc.iter())
                .map(|(ebene, titel, anker)| format!("({ebene}, {}, <{anker}>)", zeichenkette(titel)))
                .collect();
            t.push_str(&format!("#verzeichnis(({},))\n\n", eintraege.join(", ")));
        }
    }
    for _ in 0..notizseiten {
        t.push_str("#notizseite()\n");
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_wird_typst() {
        assert_eq!(inline("Das **ist** `a<b>` und *so*"), r##"#"Das "#strong[#"ist"]#" "#ic("a<b>")#" und "#emph[#"so"]"##);
        assert_eq!(inline("[x](https://a.de?b=1&c=2)"), r##"#lk("https://a.de?b=1&amp;c=2", [#"x"])"##);
        assert_eq!(inline("1. Schritt \"zitiert\""), r##"#"1. Schritt \"zitiert\"""##);
        assert_eq!(inline("Zeile `    );`"), r##"#"Zeile "#ic(" );")"##);
    }

    #[test]
    fn grauwerte() {
        assert_eq!(grau_von("#000000"), 0);
        assert_eq!(grau_von("#FFFFFF"), 255);
        assert_eq!(farbe("#333333", Fassung::Grau), "luma(51)");
    }

    #[test]
    fn titelei_und_listen() {
        let a = super::super::md::zerlegen(
            "<div class=\"half-title\">\n\n# Buch\n\n</div>\n\n- a\n- b\n\n1. x\n2. y\n\n## Kap\n",
            "s1_",
            &[],
        );
        let t = knoten_typst(&a.knoten, 38);
        assert!(t.contains("#schmutztitel[\n#titelkopf(1, [#\"Buch\"])"), "{t}");
        assert!(t.contains("#list([#\"a\"], [#\"b\"])"), "{t}");
        assert!(t.contains("#enum(start: 1, [#\"x\"], [#\"y\"])"), "{t}");
        assert!(t.contains("#heading(level: 2, [#\"Kap\"]) <s1_kap>"), "{t}");
    }
}
