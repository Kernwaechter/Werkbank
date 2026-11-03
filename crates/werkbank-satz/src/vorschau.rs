//! Satzvorschau: Markdown wird wie in den Bauskripten gesetzt und als Seitenbilder angezeigt.
//!
//! Der Weg der Skripte lautet: Markdown → **Pandoc** (`-t typst`) → **Typst** → PDF (→ Ghostscript für PDF/X).
//! Die Vorschau geht denselben Weg bis zu den Seiten, wenn Pandoc installiert ist; sonst setzt der
//! eingebaute Übersetzer das Kapitel (gleiche Schriften und Seitengröße, aber nicht Pandocs Vorlage).

use crate::markdown::markdown_zu_typst;
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Eine gesetzte Seite als Bild (RGBA, 8 Bit).
#[derive(Debug, Clone)]
pub struct Seitenbild {
    pub breite: u32,
    pub hoehe: u32,
    pub rgba: Vec<u8>,
}

/// Ergebnis einer Vorschau.
#[derive(Debug, Clone)]
pub struct VorschauErgebnis {
    /// Die ersten gesetzten Seiten.
    pub seiten: Vec<Seitenbild>,
    /// Gesamtzahl der Seiten des gesetzten Kapitels.
    pub seitenzahl: usize,
    /// Womit gesetzt wurde: „Pandoc und Typst“ oder „eingebauter Satz“.
    pub quelle: &'static str,
    /// Hinweise (Warnungen des Setzers, Rückfall-Grund).
    pub hinweise: Vec<String>,
}

/// Version von Pandoc (`pandoc 3.1.3`) oder `None`, wenn es nicht läuft.
pub fn pandoc_version() -> Option<String> {
    programm_version("pandoc", &["--version"]).map(|z| z.trim_start_matches("pandoc").trim().to_string())
}

/// Erste Zeile der Versionsausgabe eines Programms.
pub fn programm_version(programm: &str, args: &[&str]) -> Option<String> {
    let aus = Command::new(programm).args(args).stdin(Stdio::null()).output().ok()?;
    if !aus.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(if aus.stdout.is_empty() { &aus.stderr } else { &aus.stdout }).to_string();
    text.lines().next().map(|z| z.trim().to_string()).filter(|z| !z.is_empty())
}

/// Hauptzahl einer Version (`3.1.3` → 3).
pub fn hauptversion(version: &str) -> Option<u32> {
    version.split(|c: char| !c.is_ascii_digit()).find(|t| !t.is_empty())?.parse().ok()
}

/// Sucht die Typst-Kopfdatei des Bandes (`pdf-header.typ`) im Band und in `werkstatt/`.
fn kopfdatei(wurzel: &Path) -> Option<PathBuf> {
    [wurzel.to_path_buf(), wurzel.join("werkstatt")]
        .into_iter()
        .map(|d| d.join("pdf-header.typ"))
        .find(|p| p.is_file())
}

/// Der Ordner, in dem die Bauskripte arbeiten (dort liegen Kopfdatei, Bilder und Schriften).
pub fn produktionsordner(wurzel: &Path) -> PathBuf {
    if wurzel.join("pdf-header.typ").is_file() || !wurzel.join("werkstatt").join("pdf-header.typ").is_file() {
        wurzel.to_path_buf()
    } else {
        wurzel.join("werkstatt")
    }
}

/// Setzt ein Markdown-Stück mit Pandoc nach Typst (wie `build.py` für das Lese-PDF).
fn pandoc_zu_typst(md: &str, wurzel: &Path) -> Result<String, String> {
    let temp = std::env::temp_dir().join(format!("werkbank-vorschau-{}", std::process::id()));
    std::fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
    let kopf = temp.join("kopf.typ");
    let mut inhalt = kopfdatei(wurzel).and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    inhalt.push_str(CODE_ZEILENNUMMERN);
    std::fs::write(&kopf, inhalt).map_err(|e| e.to_string())?;

    let mut kind = Command::new("pandoc")
        .args(["-f", "markdown-citations", "-s", "-t", "typst", "--wrap=none"])
        .arg(format!("--include-in-header={}", kopf.display()))
        .args([
            "-V", "papersize=iso-b5", "-V", "mainfont=Libertinus Serif", "-V", "fontsize=10.5pt",
            "-V", "codefont=DejaVu Sans Mono", "--highlight-style=pygments",
        ])
        .current_dir(wurzel)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Pandoc nicht startbar: {e}"))?;
    kind.stdin.take().ok_or("keine Eingabe")?.write_all(md.as_bytes()).map_err(|e| e.to_string())?;
    let aus = kind.wait_with_output().map_err(|e| e.to_string())?;
    if !aus.status.success() {
        return Err(format!("Pandoc-Fehler: {}", String::from_utf8_lossy(&aus.stderr).lines().take(4).collect::<Vec<_>>().join(" ")));
    }
    Ok(typst_013_tauglich(&String::from_utf8_lossy(&aus.stdout)))
}

/// Neuere Pandoc-Vorlagen (ab 3.8) prüfen `"divider" in std`; das kann erst Typst 0.15.
/// Die eingebaute Typst-Version bekommt stattdessen direkt die Linie.
fn typst_013_tauglich(typ: &str) -> String {
    typ.replace(r#"if "divider" in std { divider } else { horizontalRule }"#, "horizontalRule")
}

/// Zeilennummern in Codeblöcken (je Block ab 1), auch für den Pandoc-Weg.
const CODE_ZEILENNUMMERN: &str = "
#show raw.where(block: true): it => {
  show raw.line: l => box(width: 1.8em, align(right, text(fill: luma(140), str(l.number)))) + h(0.8em) + l.body
  it
}
";

/// Eingebauter Satz eines Kapitels (Rückfall ohne Pandoc): B5, Libertinus, wie das Lese-PDF.
pub fn eingebaut_zu_typst(md: &str, wurzel: &Path, nummern: &HashMap<String, usize>) -> String {
    let mut t = String::from(
        r##"#set page(width: 176mm, height: 250mm, margin: (inside: 22mm, outside: 16mm, top: 20mm, bottom: 22mm), numbering: "1")
#set text(font: ("Libertinus Serif", "DejaVu Serif"), size: 10.5pt, lang: "de")
#set par(justify: true, leading: 0.62em, spacing: 0.8em)
#show raw: set text(font: ("DejaVu Sans Mono",), size: 0.78em)
// Codeblöcke: jede Zeile mit Nummer (je Block ab 1, wie der Buchtext sie zitiert)
#show raw.where(block: true): it => block(width: 100%, fill: luma(242), inset: 6pt, radius: 2pt, breakable: true, {
  show raw.line: l => box(width: 1.8em, align(right, text(fill: luma(140), str(l.number)))) + h(0.8em) + l.body
  it
})
#show link: set text(fill: rgb("#2c5c82"))
#show heading.where(level: 1): it => { v(6mm); text(size: 22pt, weight: "bold", it.body); v(4mm) }
#show heading.where(level: 2): it => { v(5mm); text(size: 16pt, weight: "bold", it.body); v(3mm) }
#show heading.where(level: 3): it => { v(3mm); text(size: 12.5pt, weight: "bold", it.body); v(2mm) }
#show heading.where(level: 4): it => { v(2mm); text(size: 11pt, weight: "bold", style: "italic", it.body); v(1mm) }
"##,
    );
    t.push_str(&markdown_zu_typst(md, Some(wurzel), true, nummern));
    t
}

/// Setzt ein Kapitel und liefert die ersten `max_seiten` Seiten als Bilder.
///
/// Mit `pandoc = true` wird der Weg der Bauskripte gegangen; schlägt er fehl, setzt der eingebaute
/// Übersetzer und der Grund steht in den Hinweisen.
pub fn vorschau(
    md: &str,
    wurzel: &Path,
    nummern: &HashMap<String, usize>,
    teile: &HashMap<String, usize>,
    pandoc: bool,
    max_seiten: usize,
    pixel_pro_pt: f32,
) -> Result<VorschauErgebnis, String> {
    let ordner = produktionsordner(wurzel);
    // Den Pandoc-Weg gehen nur Bände, die auch mit Pandoc gebaut werden (Kopfdatei `pdf-header.typ`).
    // Andere Bücher (z. B. mit eigenem HTML-Bauskript) setzt der eingebaute Satz – näher an ihrem Druckbild.
    let pandoc = pandoc && kopfdatei(&ordner).is_some();
    let mut hinweise = Vec::new();
    // Verweise wie im Buchbau auflösen („Kapitel 12“)
    let md = crate::markdown::produktionsersetzung(md, nummern, teile);

    let versuche: Vec<(&'static str, Result<String, String>)> = if pandoc {
        vec![
            ("Pandoc und Typst", pandoc_zu_typst(&md, &ordner)),
            ("eingebauter Satz", Ok(eingebaut_zu_typst(&md, &ordner, nummern))),
        ]
    } else {
        vec![("eingebauter Satz", Ok(eingebaut_zu_typst(&md, &ordner, nummern)))]
    };

    let mut letzter_fehler = String::new();
    for (quelle, typ) in versuche {
        let typ = match typ {
            Ok(t) => t,
            Err(f) => {
                hinweise.push(format!("{quelle}: {f}"));
                continue;
            }
        };
        match crate::welt::typst_zu_seiten(typ, Some(ordner.clone()), max_seiten, pixel_pro_pt) {
            Ok((seiten, anzahl, warnungen)) => {
                hinweise.extend(warnungen);
                return Ok(VorschauErgebnis { seiten, seitenzahl: anzahl, quelle, hinweise });
            }
            Err(f) => {
                hinweise.push(format!("{quelle}: {f}"));
                letzter_fehler = f;
            }
        }
    }
    Err(letzter_fehler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versionen_lesen() {
        assert_eq!(hauptversion("3.1.3"), Some(3));
        assert_eq!(hauptversion("pandoc 2.9.2.1"), Some(2));
        assert_eq!(hauptversion("ohne Zahl"), None);
    }

    #[test]
    fn eingebaute_vorschau_setzt_seiten() {
        let o = std::env::temp_dir().join("werkbank-satz-vorschau");
        let _ = std::fs::create_dir_all(&o);
        let md = format!("## Titel\n\nText mit `Code` und @kap:x.\n\n```rust\nfn main() {{}}\n```\n\n{}", "Ein Absatz. ".repeat(200));
        let nummern = HashMap::from([("x".to_string(), 4usize)]);
        let e = vorschau(&md, &o, &nummern, &HashMap::new(), false, 3, 1.0).unwrap();
        assert_eq!(e.quelle, "eingebauter Satz");
        assert!(!e.seiten.is_empty());
        let s = &e.seiten[0];
        assert_eq!(s.rgba.len(), (s.breite * s.hoehe * 4) as usize);
        assert!(s.rgba.chunks(4).any(|p| p[0] < 80), "es muss dunkler Text auf der Seite stehen");
    }

    #[test]
    fn pandoc_vorlage_wird_typst_013_tauglich() {
        let t = typst_013_tauglich(r#"#let divider = if "divider" in std { divider } else { horizontalRule }"#);
        assert_eq!(t, "#let divider = horizontalRule");
    }

    #[test]
    fn pandoc_weg_wenn_vorhanden() {
        if pandoc_version().is_none() || hauptversion(&pandoc_version().unwrap()).unwrap_or(0) < 3 {
            return;
        }
        let o = std::env::temp_dir().join("werkbank-satz-vorschau-pandoc");
        let _ = std::fs::create_dir_all(&o);
        // Mit Kopfdatei gilt der Band als Pandoc-Buch
        std::fs::write(o.join("pdf-header.typ"), "").unwrap();
        let md = "## Kapitel [x]: Titel\n\nText **fett** und *kursiv*, `code`.\n\n- eins\n- zwei\n\n```rust\nfn main() {}\n```\n";
        let e = vorschau(md, &o, &HashMap::new(), &HashMap::new(), true, 2, 1.0).unwrap();
        assert_eq!(e.quelle, "Pandoc und Typst", "Hinweise: {:?}", e.hinweise);
        assert!(!e.seiten.is_empty());
    }
}
