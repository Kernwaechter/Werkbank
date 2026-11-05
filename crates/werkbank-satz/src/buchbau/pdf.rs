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
use crate::gestaltung::{self, Satzstil};
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

/// Werte der Gestaltung (`satzstil.json`) für eine Fassung; leere Felder ergeben die Werkstatt-Voreinstellung.
struct Wahl<'a> {
    stil: &'a Satzstil,
    fassung: Fassung,
}

impl Wahl<'_> {
    fn roh(&self, element: &str, feld: &str) -> String {
        self.stil.format(element).feld(feld).trim().to_string()
    }
    /// Eingestellte Farbe (im Graudruck als Grau) oder `standard` (schon Typst).
    fn farbe(&self, element: &str, feld: &str, standard: &str) -> String {
        gestaltung::farbe(&self.roh(element, feld), self.fassung == Fassung::Farbe).unwrap_or_else(|| standard.to_string())
    }
    fn farbe_gesetzt(&self, element: &str, feld: &str) -> Option<String> {
        gestaltung::farbe(&self.roh(element, feld), self.fassung == Fassung::Farbe)
    }
    fn zahl(&self, element: &str, feld: &str, standard: f32) -> f32 {
        gestaltung::zahl(&self.roh(element, feld)).unwrap_or(standard)
    }
    /// Schriftliste: eingestellte Schriften vor der Voreinstellung (`serif`, `sans`, `mono`).
    fn schrift(&self, element: &str, standard: &str) -> String {
        let namen: Vec<String> = self.roh(element, "schrift").split(',').map(str::trim).filter(|n| !n.is_empty()).map(zeichenkette).collect();
        if namen.is_empty() { standard.to_string() } else { format!("(({},) + {standard})", namen.join(", ")) }
    }
    fn ja(&self, element: &str, feld: &str, standard: bool) -> bool {
        match self.roh(element, feld).to_lowercase().as_str() {
            "ja" => true,
            "nein" => false,
            _ => standard,
        }
    }
    /// Strich (`Stärke + Farbe`, `none` bei 0).
    fn strich(&self, element: &str, standard_staerke: f32, farbe: &str) -> String {
        let w = self.zahl(element, "rand_staerke", standard_staerke);
        if w <= 0.0 { "none".to_string() } else { format!("{w}pt + {farbe}") }
    }
    /// Funktion für den Text einer Überschrift: Schrift, Größe, Gewicht, Stil; die Farbe lässt sich übergeben.
    fn kopf(&self, element: &str, groesse: f32, farbe: &str) -> String {
        format!(
            "(inhalt, farbe: {}) => text(font: {}, size: {}pt, fill: farbe, weight: \"{}\", style: \"{}\", inhalt)",
            self.farbe(element, "textfarbe", farbe),
            self.schrift(element, "sans"),
            self.zahl(element, "groesse", groesse),
            if self.ja(element, "fett", true) { "bold" } else { "regular" },
            if self.ja(element, "kursiv", false) { "italic" } else { "normal" },
        )
    }
}

/// Standardfarben der Kästen (`::: art`): (Art, Titel, Hintergrund, Rand).
const KAESTEN: [(&str, &str, &str, &str); 7] = [
    ("info", "Info", "#EFF6FF", "#1E3A8A"),
    ("hinweis", "Hinweis", "#F1F5F9", "#334155"),
    ("merke", "Merke", "#EFF6FF", "#1E3A8A"),
    ("tipp", "Tipp", "#F0FDFA", "#0F766E"),
    ("wichtig", "Wichtig", "#FFFBEB", "#B45309"),
    ("warnung", "Warnung", "#FEF2F2", "#B91C1C"),
    ("beispiel", "Beispiel", "#F5F3FF", "#6D28D9"),
];

/// Die Vorlage: Seite, Schriften, Gestaltung aller Elemente. Ohne eigene Gestaltung entspricht sie
/// genau dem Layout des bisherigen `build_book.py`.
pub fn vorlage(profil: &Profil, fassung: Fassung, untertitel: &str, stil: &Satzstil) -> String {
    let p: &Palette = if fassung == Fassung::Farbe { &FARBE } else { &GRAU };
    let f = |hex: &str| farbe(hex, fassung);
    let w = Wahl { stil, fassung };
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
    let akzent1 = f(p.akzent_primaer);
    let akzent2 = f(p.akzent_sekundaer);
    let coderand = f(p.code_rand);
    let zeilennummern = if w.ja("codeblock", "zeilennummern", profil.zeilennummern) {
        "#show raw.where(block: true): it => { show raw.line: l => box(width: 1.8em, align(right, text(fill: luma(140), str(l.number)))) + h(0.8em) + l.body; it }"
    } else {
        ""
    };
    let auto = |o: Option<String>| o.unwrap_or_else(|| "auto".to_string());

    // Überschrift 1: Linie darunter
    let h1_linie = {
        let farbe = w.farbe("h1", "rand", &akzent1);
        match w.strich("h1", 1.5, &farbe).as_str() {
            "none" => "none".to_string(),
            s => format!("(bottom: {s})"),
        }
    };
    // Zitat
    let z_rand = {
        let farbe = w.farbe("zitat", "rand", &akzent1);
        let s = w.strich("zitat", 3.0, &farbe);
        match w.roh("zitat", "rand_seite").to_lowercase().as_str() {
            "keine" => "none".to_string(),
            "rundum" => s,
            _ if s == "none" => s,
            _ => format!("(left: {s})"),
        }
    };
    let z_text = {
        let mut a = vec![format!("font: {}", w.schrift("zitat", "serif"))];
        if let Some(g) = gestaltung::zahl(&w.roh("zitat", "groesse")) {
            a.push(format!("size: {g}pt"));
        }
        if let Some(c) = w.farbe_gesetzt("zitat", "textfarbe") {
            a.push(format!("fill: {c}"));
        }
        a.push(format!("style: \"{}\"", if w.ja("zitat", "kursiv", false) { "italic" } else { "normal" }));
        a.join(", ")
    };
    // Code im Text
    let c_text = match w.farbe_gesetzt("code", "textfarbe") {
        Some(c) => format!("text(fill: {c}, raw(code))"),
        None => "raw(code)".to_string(),
    };
    // Kästen
    let k_form = {
        let staerke = w.zahl("kaesten", "rand_staerke", 3.0);
        let s = if staerke <= 0.0 { "none".to_string() } else { format!("{staerke}pt + rand") };
        match w.roh("kaesten", "rand_seite").to_lowercase().as_str() {
            "keine" => "none".to_string(),
            "rundum" => s,
            _ if s == "none" => s,
            _ => format!("(left: {s})"),
        }
    };
    let kaesten = KAESTEN
        .iter()
        .map(|(art, titel, grund, rand)| {
            format!(
                "  {art}: ({}, {}, {}),",
                zeichenkette(titel),
                w.farbe(art, "fuellung", &f(grund)),
                w.farbe(art, "rand", &f(rand))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

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
        ("@@AKZENT1@@", akzent1.clone()),
        ("@@AKZENT2@@", akzent2.clone()),
        ("@@CODERAND@@", coderand.clone()),
        ("@@KOPFGRUND@@", f(p.code_kopf_grund)),
        // Fließtext
        ("@@T_SCHRIFT@@", w.schrift("text", "serif")),
        ("@@T_GROESSE@@", format!("{}pt", w.zahl("text", "groesse", 10.5))),
        ("@@TEXT@@", w.farbe("text", "textfarbe", &f("#111111"))),
        // Überschriften
        ("@@H1@@", w.kopf("h1", 20.0, &akzent1)),
        ("@@H1_LINIE@@", h1_linie),
        ("@@H2@@", w.kopf("h2", 15.0, &akzent2)),
        ("@@H3@@", w.kopf("h3", 12.0, &akzent1)),
        ("@@H4@@", w.kopf("h4", 10.5, &akzent1)),
        // Rubriken (auto = Farbe der Rubrik)
        ("@@R_GRUND@@", auto(w.farbe_gesetzt("rubrik", "fuellung"))),
        ("@@R_RAND@@", auto(w.farbe_gesetzt("rubrik", "rand"))),
        ("@@R_SCHRIFT@@", auto(w.farbe_gesetzt("rubrik", "textfarbe"))),
        ("@@R_STAERKE@@", format!("{}pt", w.zahl("rubrik", "rand_staerke", 3.0))),
        ("@@R_ECKEN@@", format!("{}pt", w.zahl("rubrik", "ecken", 3.0))),
        // Codeblöcke
        ("@@CB_SCHRIFT@@", w.schrift("codeblock", "mono")),
        ("@@CB_GROESSE@@", format!("{}pt", w.zahl("codeblock", "groesse", 8.8))),
        ("@@CB_GRUND@@", w.farbe("codeblock", "fuellung", &f(p.code_grund))),
        ("@@CB_RAND@@", w.strich("codeblock", 0.75, &w.farbe("codeblock", "rand", &coderand))),
        ("@@CB_RANDFARBE@@", w.farbe("codeblock", "rand", &coderand)),
        ("@@CB_ECKEN@@", format!("{}pt", w.zahl("codeblock", "ecken", 3.0))),
        ("@@CB_INNEN@@", format!("{}pt", w.zahl("codeblock", "innen", 7.5))),
        ("@@CB_KOPF@@", w.ja("codeblock", "kopf", true).to_string()),
        ("@@CB_KOPFGRUND@@", w.farbe("codeblock", "kopf_fuellung", &f(p.code_kopf_grund))),
        ("@@CB_KOPFSCHRIFT@@", w.farbe("codeblock", "kopf_farbe", &f(p.code_kopf_schrift))),
        // Code im Text
        ("@@C_SCHRIFT@@", w.schrift("code", "mono")),
        ("@@C_GROESSE@@", format!("{}pt", w.zahl("code", "groesse", 9.0))),
        ("@@C_GRUND@@", w.farbe("code", "fuellung", &f(p.code_grund))),
        ("@@C_RAND@@", {
            let farbe = w.farbe("code", "rand", &coderand);
            if w.roh("code", "rand").is_empty() && !w.roh("code", "fuellung").is_empty() { "none".to_string() } else { format!("0.75pt + {farbe}") }
        }),
        ("@@C_ECKEN@@", format!("{}pt", w.zahl("code", "ecken", 2.25))),
        ("@@C_TEXT@@", c_text),
        // Tabellen
        ("@@TB_GROESSE@@", format!("{}pt", w.zahl("tabelle", "groesse", 9.5))),
        ("@@TB_KOPF@@", w.farbe("tabelle", "fuellung", &f(p.code_kopf_grund))),
        ("@@TB_RAND@@", w.strich("tabelle", 0.75, &w.farbe("tabelle", "rand", &coderand))),
        ("@@TB_INNEN@@", format!("{}pt", w.zahl("tabelle", "innen", 6.0))),
        // Abbildungen
        ("@@A_BREITE@@", format!("{}", (w.zahl("abbildung", "breite", 92.0) / 100.0).clamp(0.1, 1.0))),
        ("@@A_RAND@@", w.strich("abbildung", 0.75, &w.farbe("abbildung", "rand", &coderand))),
        ("@@A_ECKEN@@", format!("{}pt", w.zahl("abbildung", "ecken", 4.5))),
        ("@@UNTERSCHRIFT@@", w.farbe("abbildung", "textfarbe", &f("#444444"))),
        // Zitat
        ("@@Z_GRUND@@", w.farbe("zitat", "fuellung", &f("#F8FAFC"))),
        ("@@Z_RAND@@", z_rand),
        ("@@Z_INNEN@@", format!("{}pt", w.zahl("zitat", "innen", 9.0))),
        ("@@Z_ECKEN@@", format!("{}pt", w.zahl("zitat", "ecken", 0.0))),
        ("@@Z_TEXT@@", z_text),
        // Kästen
        ("@@KAESTEN@@", kaesten),
        ("@@K_RAND@@", k_form),
        ("@@K_INNEN@@", format!("{}pt", w.zahl("kaesten", "innen", 9.0))),
        ("@@K_ECKEN@@", format!("{}pt", w.zahl("kaesten", "ecken", 3.0))),
        // Sonstiges
        ("@@FUSS@@", f("#333333")),
        ("@@NOTIZ@@", f("#777777")),
        ("@@NOTIZRAND@@", f("#CCCCCC")),
        ("@@LINK@@", link_farbe),
        ("@@GRAU@@", if fassung == Fassung::Grau { "true" } else { "false" }.to_string()),
        ("@@RUBRIKEN@@", rubrik_regeln),
        ("@@ZEILENNUMMERN@@", zeilennummern.to_string()),
    ];
    let liste = |namen: &[&str]| format!("({},)", crate::welt::vorhandene_familien(namen).iter().map(|n| format!("\"{n}\"")).collect::<Vec<_>>().join(", "));
    let mut t = VORLAGE.to_string();
    t = t.replace("@@SERIFE@@", &liste(&["Noto Serif", "Liberation Serif", "DejaVu Serif", "Libertinus Serif"]));
    t = t.replace("@@SANSE@@", &liste(&["Noto Sans", "Liberation Sans", "DejaVu Sans", "Fira Sans"]));
    t = t.replace("@@MONOE@@", &liste(&["DejaVu Sans Mono", "Liberation Mono"]));
    for (name, wert) in ersetzungen {
        t = t.replace(name, &wert);
    }
    t
}

/// Typst-Vorlage. Werte aus dem Stylesheet: 1 px = 0,75 pt; Zeilenhöhe als 1-em-Zeilenkasten plus Abstand.
/// Alle `@@…@@` kommen aus der Gestaltung (leer = Werkstatt-Layout).
const VORLAGE: &str = r##"// Buchsatz der Werkbank – Werkstatt-Layout (wie das bisherige build_book.py) mit eigener Gestaltung
#let serif = @@SERIFE@@
#let sans = @@SANSE@@
#let mono = @@MONOE@@
#let akzent1 = @@AKZENT1@@
#let akzent2 = @@AKZENT2@@
#let coderand = @@CODERAND@@
#let kopfgrund = @@KOPFGRUND@@
#let grau = @@GRAU@@
#let wahl(x, standard) = if x == auto { standard } else { x }

#set document(title: @@TITEL@@, author: @@AUTOR@@)
#set page(
  width: @@BREITE@@, height: @@HOEHE@@,
  margin: (inside: @@INNEN@@, outside: @@AUSSEN@@, top: @@OBEN@@, bottom: @@UNTEN@@),
  footer: context align(center, text(font: sans, size: 9pt, fill: @@FUSS@@, counter(page).display())),
)
// Zeilenkasten von 1 em; der Abstand ergibt die Zeilenhöhe des Stylesheets (1,55)
#set text(font: @@T_SCHRIFT@@, size: @@T_GROESSE@@, lang: @@SPRACHE@@, fill: @@TEXT@@, hyphenate: true, top-edge: 0.78em, bottom-edge: -0.22em)
#set par(justify: true, leading: 0.55em, spacing: 8pt)
#set list(indent: 6pt, body-indent: 7pt, spacing: 0.55em + 4pt)
#set enum(indent: 4pt, body-indent: 6pt, spacing: 0.55em + 4pt)
#set raw(theme: "werkstatt.tmTheme")
// Typst verkleinert Code sonst auf 0,8 em; hier gelten die Größen des Stylesheets
#show raw: set text(font: mono)
#show raw.where(block: true): set text(font: @@CB_SCHRIFT@@, size: @@CB_GROESSE@@)
#show raw.where(block: false): set text(font: @@C_SCHRIFT@@, size: @@C_GROESSE@@)
@@ZEILENNUMMERN@@

// Überschriften
#let h1text = @@H1@@
#let h2text = @@H2@@
#let h3text = @@H3@@
#let h4text = @@H4@@
#let h1kopf(inhalt) = block(sticky: true, width: 100%, stroke: @@H1_LINIE@@, inset: (bottom: 4.5pt), below: 13.4pt, h1text(inhalt))
// Ebene 1 und 2: neue Seite, nicht vom Folgetext getrennt
#show heading.where(level: 1): it => {
  pagebreak(weak: true)
  v(28pt)
  h1kopf(it.body)
}
#show heading.where(level: 2): it => {
  pagebreak(weak: true)
  v(22pt)
  block(sticky: true, width: 100%, below: 12.45pt, h2text(it.body))
}
#let h3(inhalt) = block(sticky: true, above: 16pt, below: 12pt, h3text(inhalt))
#let h4(inhalt) = block(sticky: true, above: 12pt, below: 14pt, h4text(inhalt))
#let h5(inhalt) = block(sticky: true, above: 12pt, below: 17.5pt, h4text(inhalt))
#let rubriken = @@RUBRIKEN@@
#let rubrik(art, inhalt) = {
  let (grund, rand, schrift) = rubriken.at(art, default: (rgb("#EFF6FF"), akzent1, akzent1))
  let stil = if grau {
    (fill: wahl(@@R_GRUND@@, luma(242)), stroke: (left: @@R_STAERKE@@ + wahl(@@R_RAND@@, black), top: 0.75pt + luma(187), bottom: 0.75pt + luma(187)), radius: 0pt, schrift: wahl(@@R_SCHRIFT@@, black))
  } else {
    (fill: wahl(@@R_GRUND@@, grund), stroke: (left: @@R_STAERKE@@ + wahl(@@R_RAND@@, rand)), radius: (top-right: @@R_ECKEN@@, bottom-right: @@R_ECKEN@@), schrift: wahl(@@R_SCHRIFT@@, schrift))
  }
  block(sticky: true, width: 100%, above: 16pt, below: 12pt, fill: stil.fill, stroke: stil.stroke, radius: stil.radius,
    inset: (x: 9pt, y: 4.5pt), h3text(inhalt, farbe: stil.schrift))
}

// Code im Text und Codeblöcke mit Kopfzeile
#let ic(code) = box(fill: @@C_GRUND@@, stroke: @@C_RAND@@, radius: @@C_ECKEN@@, inset: (x: 3pt, y: 0pt), outset: (y: 1.5pt),
  @@C_TEXT@@)
#let codeblock(etikett, inhalt, teilbar: false) = block(width: 100%, above: 10pt, below: 12pt, breakable: teilbar,
  fill: @@CB_GRUND@@, stroke: @@CB_RAND@@, radius: @@CB_ECKEN@@, clip: true, {
    if @@CB_KOPF@@ {
      block(width: 100%, below: 0pt, fill: @@CB_KOPFGRUND@@, stroke: (bottom: 0.75pt + @@CB_RANDFARBE@@), inset: (x: @@CB_INNEN@@, y: 2.25pt),
        text(font: sans, size: 7.5pt, weight: "bold", tracking: 0.05em, fill: @@CB_KOPFSCHRIFT@@, top-edge: "cap-height", bottom-edge: "baseline", etikett))
    }
    block(width: 100%, above: 0pt, inset: (x: @@CB_INNEN@@, y: @@CB_INNEN@@ * 0.8), {
      set par(leading: 0.42em, justify: false)
      inhalt
    })
  })

// Verweise wie im Browser (unterstrichen)
#let lk(ziel, inhalt) = link(ziel, underline(text(fill: @@LINK@@, inhalt)))

// Tabellen
#let tabelle(spalten, ..zellen) = block(width: 100%, above: 12pt, below: 12pt, breakable: false, {
  set text(size: @@TB_GROESSE@@)
  set par(justify: false)
  table(columns: spalten, stroke: @@TB_RAND@@, inset: (x: @@TB_INNEN@@, y: @@TB_INNEN@@ * 0.75),
    fill: (x, y) => if y == 0 { @@TB_KOPF@@ }, align: left + top, ..zellen)
})
#let th(inhalt) = text(font: sans, weight: "bold", inhalt)

// Abbildungen: höchstens so breit wie eingestellt und 115 mm hoch, mit Rahmen und kursiver Unterschrift
#let figur(pfad, unterschrift) = block(width: 100%, above: 14pt, below: 14pt, breakable: false, layout(groesse => {
  let natur = measure(image(pfad))
  let breite = calc.min(groesse.width * @@A_BREITE@@, 115mm * (natur.width / natur.height))
  align(center, {
    box(stroke: @@A_RAND@@, radius: @@A_ECKEN@@, clip: true, image(pfad, width: breite))
    if unterschrift != none {
      v(5pt, weak: true)
      text(font: sans, size: 8.5pt, style: "italic", fill: @@UNTERSCHRIFT@@, unterschrift)
    }
  })
}))

#let linie() = block(above: 18pt, below: 18pt, line(length: 100%, stroke: 0.75pt + coderand))

// Zitate (> …)
#let zitat(inhalt) = block(width: 100%, above: 10pt, below: 12pt, breakable: true, fill: @@Z_GRUND@@, stroke: @@Z_RAND@@,
  inset: (x: @@Z_INNEN@@, y: @@Z_INNEN@@ * 0.7), radius: @@Z_ECKEN@@, text(@@Z_TEXT@@, inhalt))

// Kästen (::: art Titel … :::)
#let kasten-arten = (
@@KAESTEN@@
)
#let kasten(art, titel, inhalt) = {
  let (name, grund, rand) = kasten-arten.at(art, default: (upper(art.first()) + art.slice(1), luma(245), luma(90)))
  block(width: 100%, breakable: true, above: 12pt, below: 12pt, inset: @@K_INNEN@@, radius: @@K_ECKEN@@, fill: grund, stroke: @@K_RAND@@, {
    text(font: sans, weight: "bold", size: 0.9em, fill: rand, if titel == none { name } else { titel })
    parbreak()
    inhalt
  })
}

// Titelei: Schmutztitel, Titelseite, Impressum
#let titelkopf(ebene, inhalt) = {
  let kopf = (h1text, h2text, h3text, h4text, h4text).at(ebene - 1)
  block(above: if ebene == 1 { 13.4pt } else { 12pt }, below: (13.4pt, 12.45pt, 12pt, 14pt, 17.5pt).at(ebene - 1),
    align(center, kopf(inhalt)))
}
#let schmutztitel(inhalt) = { v(60mm); align(center, inhalt) }
#let titelseite(inhalt) = { v(45mm); align(center, inhalt) }
#let impressum(inhalt) = { v(20mm); set text(size: 9pt); set par(leading: 0.45em); inhalt }

// Inhaltsverzeichnis mit Seitenzahlen
#let verzeichnis(eintraege) = {
  pagebreak(weak: true)
  v(28pt)
  h1kopf("Inhaltsverzeichnis")
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
            Knoten::Zitat(t) => aus.push_str(&format!("#zitat[{}]\n\n", inline(t))),
            Knoten::KastenAuf { art, titel } => {
                let t = if titel.is_empty() { "none".to_string() } else { format!("[{}]", inline(titel)) };
                aus.push_str(&format!("#kasten({}, {t})[\n", zeichenkette(art)));
            }
            Knoten::KastenZu => aus.push_str("]\n\n"),
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
    stil: &Satzstil,
) -> String {
    let mut t = vorlage(profil, fassung, untertitel, stil);
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
