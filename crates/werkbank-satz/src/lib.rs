//! # Nativer Buchsatz
//!
//! Setzt ein Buchprojekt ohne Skripte: Markdown wird zu Typst-Markup übersetzt und mit der
//! Typst-Bibliothek zu PDF gesetzt; EPUB 3 entsteht über `epub-builder`. Auch das Cover
//! (SVG) lässt sich über Typst als PDF ausgeben.
//!
//! Das Ergebnis ist **nicht** dasselbe wie die Ausgabe der bisherigen Python-Skripte;
//! es gibt keinen Golden-Test gegen sie.

pub mod epub;
pub mod markdown;
pub mod vorschau;
pub mod welt;

pub use epub::epub_schreiben;
pub use markdown::{SatzOptionen, Seitenzahl, Zaehlung, buch_zu_typst, buch_zu_typst_mit_ende};
pub use vorschau::{Seitenbild, VorschauErgebnis, hauptversion, pandoc_version, produktionsordner, programm_version, vorschau};
pub use welt::{PdfErgebnis, pdf_setzen, svg_als_pdf, typst_zu_pdf, typst_zu_seiten};

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
