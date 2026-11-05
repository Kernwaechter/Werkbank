//! # Nativer Buchsatz
//!
//! Setzt ein Buchprojekt ohne Skripte: Markdown wird zu Typst-Markup übersetzt und mit der
//! Typst-Bibliothek zu PDF gesetzt; EPUB 3 entsteht über `epub-builder`. Auch das Cover
//! (SVG) lässt sich über Typst als PDF ausgeben.
//!
//! Das Ergebnis ist **nicht** dasselbe wie die Ausgabe der bisherigen Python-Skripte;
//! es gibt keinen Golden-Test gegen sie.

pub mod buchbau;
pub mod epub;
pub mod gestaltung;
pub mod markdown;
pub mod vorschau;
pub mod welt;

pub use epub::epub_schreiben;
pub use markdown::{SatzOptionen, Seitenzahl, Zaehlung, buch_zu_typst, buch_zu_typst_mit_ende};
pub use vorschau::{Seitenbild, VorschauErgebnis, hauptversion, pandoc_programm, pandoc_version, produktionsordner, programm_version, vorschau};
pub use welt::{PdfErgebnis, pdf_setzen, svg_als_pdf, typst_selbsttest, typst_zu_pdf, typst_zu_seiten};

/// Vorlage für `.werkbank/satz-stil.typ`: eigener Typst-Text, der nach den Voreinstellungen des
/// Satzes eingefügt wird und sie überschreibt. Alles ist auskommentiert (`//`); Zeilen einschalten, die gelten sollen.
/// Gilt für alle Fassungen (Druck je Portal, Lese-/Web-PDF). `kasten-farbig` sagt, ob farbig gesetzt wird.
pub const STIL_VORLAGE: &str = r##"// Eigener Stil der Werkbank (Typst). Zeilen mit // sind aus; Anfang // entfernen zum Einschalten.
// Gilt für Druck (Schwarzweiß) und Lese-/Web-PDF. Mit #if kasten-farbig lässt sich nach Fassung unterscheiden.

// --- Schrift (eingebaut: Libertinus Serif, DejaVu Serif, DejaVu Sans Mono; installierte Schriften gehen auch)
// #set text(font: ("Libertinus Serif", "DejaVu Serif"))
// #set par(leading: 0.7em, spacing: 0.9em)
// #show raw: set text(font: ("DejaVu Sans Mono",), size: 0.74em)

// --- Seitenränder (innen, außen, oben, unten)
// #set page(margin: (inside: 24mm, outside: 15mm, top: 20mm, bottom: 24mm))

// --- Überschriften: Ebene 1 = Teil, Ebene 2 = Kapitel, 3 und 4 = Abschnitte
// #show heading.where(level: 2): it => { pagebreak(weak: true); v(8mm); text(size: 20pt, weight: "bold", fill: if kasten-farbig { rgb("#00875a") } else { black }, it.body); v(5mm) }
// #show heading.where(level: 3): it => { v(4mm); text(size: 13pt, weight: "bold", it.body); v(2mm) }

// --- Codekästen (Markdown ``` ... ```)
// #show raw.where(block: true): it => block(width: 100%, fill: luma(238), inset: 7pt, radius: 3pt, breakable: true, it)

// --- Kästen (Markdown: ::: info ... :::). Eingebaut: info, hinweis, tipp, merke, wichtig, achtung, warnung, beispiel.
// Eigene Arten: (Titel, Hintergrund, Randfarbe). Jede andere Art unter ::: name erscheint mit ihrem Namen als Titel.
// #let kasten-eigene = (
//   aufgabe: ("Aufgabe", rgb("#fff0f5"), rgb("#b83280")),
//   loesung: ("Lösung", rgb("#f0fff4"), rgb("#2f855a")),
// )
// Eigene Form für alle Kästen (statt des Streifens links):
// #let kasten-form = (art, titel, fuellung, rand, body) => block(
//   width: 100%, breakable: true, inset: 9pt, radius: 3pt,
//   fill: if kasten-farbig { fuellung } else { none },
//   stroke: 0.8pt + if kasten-farbig { rand } else { black },
//   [*#titel* \ #body],
// )

// --- Tabellen
// #set table(inset: 6pt, stroke: 0.4pt + luma(100))
"##;

/// Die in Typst eingebetteten Schriften einer Familie (z. B. „Libertinus Serif“), damit die
/// Oberfläche dieselbe Schrift wie der Satz verwenden kann.
pub fn eingebettete_schriften(familie: &str) -> Vec<&'static [u8]> {
    typst_assets::fonts()
        .filter(|daten| {
            ttf_parser::Face::parse(daten, 0).is_ok_and(|f| {
                f.names()
                    .into_iter()
                    .any(|n| n.name_id == ttf_parser::name_id::FAMILY && n.to_string().as_deref() == Some(familie))
            })
        })
        .collect()
}

#[cfg(test)]
mod schriften {
    #[test]
    fn libertinus_ist_eingebettet() {
        let s = super::eingebettete_schriften("Libertinus Serif");
        assert!(s.len() >= 2, "gefunden: {}", s.len());
        assert!(super::eingebettete_schriften("Gibt Es Nicht").is_empty());
    }
}
