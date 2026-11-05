//! Gestaltung der Markdown-Elemente ohne Typst-Kenntnisse: je Element Schrift, Größe, Farben, Rand, Kasten.
//!
//! Die Werte stehen in `.werkbank/satzstil.json`; daraus entsteht Typst, das nach den Voreinstellungen
//! des Satzes eingefügt wird (vor `satz-stil.typ`, das also weiter das letzte Wort hat).
//! Im Schwarzweiß-Druck werden alle Farben in Grauwerte umgerechnet.

use crate::markdown::zeichenkette;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Gestaltbare Elemente: (Schlüssel, Anzeigename, hat Kasten/Rand, ist Kastenart).
pub const ELEMENTE: [(&str, &str); 17] = [
    ("text", "Fließtext"),
    ("h1", "Teil-Überschrift (H1)"),
    ("h2", "Kapitelüberschrift (H2)"),
    ("h3", "Abschnitt (H3)"),
    ("h4", "Unterabschnitt (H4)"),
    ("zitat", "Zitat (> …)"),
    ("codeblock", "Codeblock (``` …)"),
    ("code", "Code im Text (`…`)"),
    ("tabelle", "Tabelle"),
    ("info", "Kasten: info"),
    ("hinweis", "Kasten: hinweis"),
    ("tipp", "Kasten: tipp"),
    ("merke", "Kasten: merke"),
    ("wichtig", "Kasten: wichtig"),
    ("warnung", "Kasten: warnung"),
    ("beispiel", "Kasten: beispiel"),
    ("kaesten", "Alle Kästen: Form"),
];

/// Standardtitel der eingebauten Kästen.
fn kasten_titel(art: &str) -> &'static str {
    match art {
        "info" => "Info",
        "hinweis" => "Hinweis",
        "tipp" => "Tipp",
        "merke" => "Merke",
        "wichtig" => "Wichtig",
        "warnung" => "Warnung",
        _ => "Beispiel",
    }
}

/// Welche Felder ein Element hat (Schlüssel der Felder, siehe [`Format::feld`]).
pub fn felder_von(element: &str) -> &'static [&'static str] {
    match element {
        "text" => &["schrift", "groesse", "textfarbe"],
        "h1" => &["schrift", "textfarbe"],
        "h2" | "h3" | "h4" => &["schrift", "groesse", "textfarbe", "fett", "kursiv"],
        "zitat" => &["schrift", "groesse", "textfarbe", "kursiv", "fuellung", "rand", "rand_staerke", "rand_seite", "innen", "ecken"],
        "codeblock" => &["schrift", "groesse", "textfarbe", "fuellung", "rand", "rand_staerke", "innen", "ecken"],
        "code" => &["schrift", "groesse", "textfarbe", "fuellung", "ecken"],
        "tabelle" => &["groesse", "fuellung", "rand", "rand_staerke", "innen"],
        "kaesten" => &["rand_staerke", "rand_seite", "innen", "ecken"],
        _ => &["fuellung", "rand"],
    }
}

/// Anzeigename eines Feldes und ein Hinweis.
pub fn feld_name(feld: &str) -> (&'static str, &'static str) {
    match feld {
        "schrift" => ("Schrift", "z. B. DejaVu Sans, Libertinus Serif, Arial (installierte gehen auch)"),
        "groesse" => ("Größe in pt", "z. B. 10.5"),
        "textfarbe" => ("Textfarbe", "#rrggbb"),
        "fett" => ("Fett", "ja / nein"),
        "kursiv" => ("Kursiv", "ja / nein"),
        "fuellung" => ("Hintergrund", "#rrggbb"),
        "rand" => ("Randfarbe", "#rrggbb"),
        "rand_staerke" => ("Randstärke in pt", "z. B. 1.5"),
        "rand_seite" => ("Rand", "links / rundum / keine"),
        "innen" => ("Innenabstand in pt", "z. B. 6"),
        "ecken" => ("Eckenradius in pt", "z. B. 3"),
        _ => ("?", ""),
    }
}

/// Eigenschaften eines Elements; leer = Voreinstellung des Satzes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Format {
    pub schrift: String,
    pub groesse: String,
    pub textfarbe: String,
    pub fett: String,
    pub kursiv: String,
    pub fuellung: String,
    pub rand: String,
    pub rand_staerke: String,
    pub rand_seite: String,
    pub innen: String,
    pub ecken: String,
}

impl Format {
    /// Wert eines Feldes.
    pub fn feld(&self, name: &str) -> &str {
        match name {
            "schrift" => &self.schrift,
            "groesse" => &self.groesse,
            "textfarbe" => &self.textfarbe,
            "fett" => &self.fett,
            "kursiv" => &self.kursiv,
            "fuellung" => &self.fuellung,
            "rand" => &self.rand,
            "rand_staerke" => &self.rand_staerke,
            "rand_seite" => &self.rand_seite,
            "innen" => &self.innen,
            "ecken" => &self.ecken,
            _ => "",
        }
    }

    /// Setzt ein Feld.
    pub fn setze(&mut self, name: &str, wert: String) {
        match name {
            "schrift" => self.schrift = wert,
            "groesse" => self.groesse = wert,
            "textfarbe" => self.textfarbe = wert,
            "fett" => self.fett = wert,
            "kursiv" => self.kursiv = wert,
            "fuellung" => self.fuellung = wert,
            "rand" => self.rand = wert,
            "rand_staerke" => self.rand_staerke = wert,
            "rand_seite" => self.rand_seite = wert,
            "innen" => self.innen = wert,
            "ecken" => self.ecken = wert,
            _ => {}
        }
    }

    fn leer(&self) -> bool {
        *self == Format::default()
    }
}

/// Alle Formate eines Bandes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Satzstil {
    pub formate: BTreeMap<String, Format>,
}

/// `#rrggbb` (auch `rrggbb`, `#rgb`) als RGB.
pub fn farbe_lesen(s: &str) -> Option<(u8, u8, u8)> {
    let h = s.trim().trim_start_matches('#');
    let h: String = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).collect(),
        6 => h.to_string(),
        _ => return None,
    };
    let v = u32::from_str_radix(&h, 16).ok()?;
    Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}

/// Typst-Farbe: farbig als `rgb`, sonst als Grauwert.
fn farbe(s: &str, farbig: bool) -> Option<String> {
    let (r, g, b) = farbe_lesen(s)?;
    Some(if farbig {
        format!("rgb(\"#{r:02x}{g:02x}{b:02x}\")")
    } else {
        let grau = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32).round() as u8;
        format!("luma({grau})")
    })
}

fn zahl(s: &str) -> Option<f32> {
    s.trim().replace(',', ".").trim_end_matches("pt").trim().parse::<f32>().ok().filter(|z| *z >= 0.0 && *z < 500.0)
}

fn schriftliste(s: &str, rueckfall: &str) -> Option<String> {
    let namen: Vec<String> = s.split(',').map(str::trim).filter(|n| !n.is_empty()).map(zeichenkette).collect();
    (!namen.is_empty()).then(|| format!("({}, {})", namen.join(", "), zeichenkette(rueckfall)))
}

impl Satzstil {
    /// Liest `.werkbank/satzstil.json` (fehlt sie oder ist sie defekt: leer).
    pub fn laden(band: &std::path::Path) -> Self {
        std::fs::read_to_string(band.join(".werkbank").join("satzstil.json"))
            .ok()
            .and_then(|t| serde_json::from_str(t.trim_start_matches('\u{feff}')).ok())
            .unwrap_or_default()
    }

    /// Schreibt `.werkbank/satzstil.json` (ohne leere Formate).
    pub fn speichern(&self, band: &std::path::Path) -> Result<(), String> {
        let mut kopie = self.clone();
        kopie.formate.retain(|_, f| !f.leer());
        let ordner = band.join(".werkbank");
        std::fs::create_dir_all(&ordner).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(&kopie).map_err(|e| e.to_string())?;
        std::fs::write(ordner.join("satzstil.json"), text).map_err(|e| e.to_string())
    }

    /// Format eines Elements (leer, wenn nichts eingestellt ist).
    pub fn format(&self, element: &str) -> Format {
        self.formate.get(element).cloned().unwrap_or_default()
    }

    /// Ändert ein Feld eines Elements.
    pub fn setze(&mut self, element: &str, feld: &str, wert: String) {
        self.formate.entry(element.to_string()).or_default().setze(feld, wert);
    }

    /// Typst für die gewählte Fassung (farbig oder Graustufen). Leer, wenn nichts eingestellt ist.
    pub fn zu_typst(&self, farbig: bool) -> String {
        let mut t = String::new();
        let f = |name: &str| self.format(name);
        let txt = |fm: &Format, mit_fett: bool| -> String {
            let mut a = Vec::new();
            if let Some(s) = schriftliste(&fm.schrift, "Libertinus Serif") {
                a.push(format!("font: {s}"));
            }
            if let Some(g) = zahl(&fm.groesse) {
                a.push(format!("size: {g}pt"));
            }
            if let Some(c) = farbe(&fm.textfarbe, farbig) {
                a.push(format!("fill: {c}"));
            }
            if mit_fett {
                match fm.fett.trim().to_lowercase().as_str() {
                    "ja" => a.push("weight: \"bold\"".into()),
                    "nein" => a.push("weight: \"regular\"".into()),
                    _ => {}
                }
            }
            match fm.kursiv.trim().to_lowercase().as_str() {
                "ja" => a.push("style: \"italic\"".into()),
                "nein" => a.push("style: \"normal\"".into()),
                _ => {}
            }
            a.join(", ")
        };

        // Fließtext
        let fm = f("text");
        let a = txt(&fm, false);
        if !a.is_empty() {
            t.push_str(&format!("#set text({a})\n"));
        }

        // Teil-Überschrift: Größe und Fettdruck bleiben (feste Regeln des Satzes), Schrift und Farbe lassen sich ändern
        let fm = f("h1");
        let (s, c) = (schriftliste(&fm.schrift, "Libertinus Serif"), farbe(&fm.textfarbe, farbig));
        if s.is_some() || c.is_some() {
            let mut a = Vec::new();
            if let Some(s) = s {
                a.push(format!("font: {s}"));
            }
            if let Some(c) = c {
                a.push(format!("fill: {c}"));
            }
            t.push_str(&format!("#show heading.where(level: 1): set text({})\n", a.join(", ")));
        }

        // Überschriften: ersetzen die Voreinstellung (Abstände bleiben ähnlich)
        for (ebene, oben, unten, gr, fett) in [(2, "8mm", "5mm", 17.0, true), (3, "4mm", "2mm", 12.5, true), (4, "2mm", "1mm", 11.0, true)] {
            let fm = f(&format!("h{ebene}"));
            if fm.leer() {
                continue;
            }
            let mut a = txt(&fm, true);
            if zahl(&fm.groesse).is_none() {
                a = format!("size: {gr}pt{}{a}", if a.is_empty() { "" } else { ", " });
            }
            if fett && !a.contains("weight:") {
                a.push_str(", weight: \"bold\"");
            }
            let start = if ebene == 2 { "kapitel-start(); " } else { "" };
            t.push_str(&format!(
                "#show heading.where(level: {ebene}): it => {{ {start}v({oben}); text({a}, it.body); v({unten}) }}\n"
            ));
        }

        // Zitat
        let fm = f("zitat");
        if !fm.leer() {
            let c = farbe(&fm.rand, farbig).unwrap_or_else(|| "luma(170)".into());
            let w = zahl(&fm.rand_staerke).unwrap_or(1.5);
            let stroke = match fm.rand_seite.trim().to_lowercase().as_str() {
                "keine" => "none".to_string(),
                "rundum" => format!("{w}pt + {c}"),
                _ => format!("(left: {w}pt + {c})"),
            };
            let innen = zahl(&fm.innen).unwrap_or(8.0);
            let fuell = farbe(&fm.fuellung, farbig).map_or("none".to_string(), |c| c);
            let ecken = zahl(&fm.ecken).unwrap_or(0.0);
            let a = txt(&fm, false);
            t.push_str(&format!(
                "#let zitat-form = body => block(width: 100%, above: 1em, below: 1em, breakable: true, fill: {fuell}, stroke: {stroke}, inset: {innen}pt, radius: {ecken}pt, text({a}, body))\n"
            ));
        }

        // Code: Schrift und Größe für alle Rohtexte, Rahmen getrennt
        let (cb, ci) = (f("codeblock"), f("code"));
        for (sel, fm) in [("true", &cb), ("false", &ci)] {
            let mut a = txt(fm, false);
            a = a.replace("size: ", "size: ");
            if !a.is_empty() {
                t.push_str(&format!("#show raw.where(block: {sel}): set text({a})\n"));
            }
        }
        if !cb.leer() {
            let fuell = farbe(&cb.fuellung, farbig).unwrap_or_else(|| "luma(242)".into());
            let stroke = match farbe(&cb.rand, farbig) {
                Some(c) => format!("{}pt + {c}", zahl(&cb.rand_staerke).unwrap_or(0.8)),
                None => "none".into(),
            };
            let innen = zahl(&cb.innen).unwrap_or(6.0);
            let ecken = zahl(&cb.ecken).unwrap_or(2.0);
            t.push_str(&format!(
                "#show raw.where(block: true): it => block(width: 100%, fill: {fuell}, stroke: {stroke}, inset: {innen}pt, radius: {ecken}pt, breakable: true, {{\n  show raw.line: l => box(width: 1.8em, align(right, text(fill: luma(140), str(l.number)))) + h(0.8em) + l.body\n  it\n}})\n"
            ));
        }
        if !ci.leer() && farbe(&ci.fuellung, farbig).is_some() {
            let fuell = farbe(&ci.fuellung, farbig).unwrap_or_default();
            let ecken = zahl(&ci.ecken).unwrap_or(2.0);
            t.push_str(&format!(
                "#show raw.where(block: false): it => box(fill: {fuell}, inset: (x: 3pt), outset: (y: 3pt), radius: {ecken}pt, it)\n"
            ));
        }

        // Tabelle
        let fm = f("tabelle");
        if !fm.leer() {
            let c = farbe(&fm.rand, farbig).unwrap_or_else(|| "luma(120)".into());
            let w = zahl(&fm.rand_staerke).unwrap_or(0.4);
            let innen = zahl(&fm.innen).unwrap_or(5.0);
            let kopf = farbe(&fm.fuellung, farbig).map_or("none".to_string(), |c| c);
            t.push_str(&format!("#set table(inset: {innen}pt, stroke: {w}pt + {c}, fill: (x, y) => if y == 0 {{ {kopf} }} else {{ none }})\n"));
            if let Some(g) = zahl(&fm.groesse) {
                t.push_str(&format!("#show table: set text(size: {g}pt)\n"));
            }
        }

        // Kästen: Farben der eingebauten Arten
        let mut arten = Vec::new();
        for (art, _) in ELEMENTE.iter().filter(|(k, _)| matches!(*k, "info" | "hinweis" | "tipp" | "merke" | "wichtig" | "warnung" | "beispiel")) {
            let fm = f(art);
            let (fu, ra) = (farbe(&fm.fuellung, farbig), farbe(&fm.rand, farbig));
            if fu.is_some() || ra.is_some() {
                arten.push(format!(
                    "{art}: ({}, {}, {})",
                    zeichenkette(kasten_titel(art)),
                    fu.unwrap_or_else(|| "luma(245)".into()),
                    ra.unwrap_or_else(|| "luma(100)".into())
                ));
            }
        }
        if !arten.is_empty() {
            t.push_str(&format!("#let kasten-eigene = ({},)\n", arten.join(", ")));
        }
        // Form aller Kästen
        let fm = f("kaesten");
        if !fm.leer() {
            let w = zahl(&fm.rand_staerke).unwrap_or(2.2);
            let stroke = match fm.rand_seite.trim().to_lowercase().as_str() {
                "keine" => "none".to_string(),
                "rundum" => format!("{w}pt + r"),
                _ => format!("(left: {w}pt + r)"),
            };
            let (innen, ecken) = (zahl(&fm.innen).unwrap_or(8.0), zahl(&fm.ecken).unwrap_or(2.0));
            t.push_str(&format!(
                "#let kasten-form = (art, titel, fuellung, rand, body) => {{\n  let fl = if kasten-farbig {{ fuellung }} else {{ luma(245) }}\n  let r = if kasten-farbig {{ rand }} else {{ luma(60) }}\n  block(width: 100%, breakable: true, above: 1em, below: 1em, inset: {innen}pt, radius: {ecken}pt, fill: fl, stroke: {stroke}, {{\n    text(weight: \"bold\", size: 0.9em, fill: r, titel)\n    parbreak()\n    body\n  }})\n}}\n"
            ));
        }
        t
    }

    /// Typst-Dokument mit Beispielen aller gestaltbaren Elemente (für die Musterseite).
    pub fn muster(&self, farbig: bool) -> String {
        let md = "# Teil 1: Muster\n\n## Kapitel 1: Muster\n\nFließtext mit **fett**, *kursiv* und `Code im Text`. Ein zweiter Satz, damit der Absatz etwas länger wird.\n\n### Ein Abschnitt\n\n#### Ein Unterabschnitt\n\n> Ein Zitat über mehrere Wörter, damit man den Kasten und die Schrift sieht.\n\n```rust\nfn main() {\n    println!(\"Hallo\");\n}\n```\n\n| Spalte A | Spalte B |\n|---|---|\n| eins | zwei |\n| drei | vier |\n\n::: info\nEin Infokasten.\n:::\n\n::: tipp\nEin Tipp.\n:::\n\n::: warnung\nEine Warnung.\n:::\n\n::: wichtig\nWichtig!\n:::\n";
        crate::vorschau::eingebaut_zu_typst_mit_stil(md, std::path::Path::new("."), &Default::default(), farbig, &self.zu_typst(farbig))
    }
}

/// Eine fest mitgelieferte Gestaltung.
pub struct Vorlage {
    pub name: &'static str,
    pub beschreibung: &'static str,
    pub stil: Satzstil,
}

fn stil_aus(zeilen: &[(&str, &str, &str)]) -> Satzstil {
    let mut s = Satzstil::default();
    for (e, f, w) in zeilen {
        s.setze(e, f, (*w).to_string());
    }
    s
}

/// Die mitgelieferten Vorlagen (nur mit eingebauten Schriften, damit der Satz überall gleich aussieht).
pub fn vorlagen() -> Vec<Vorlage> {
    let klassisch = stil_aus(&[
        ("text", "textfarbe", "#1a1a1a"),
        ("h2", "schrift", "Libertinus Serif"), ("h2", "groesse", "20"), ("h2", "fett", "ja"), ("h2", "textfarbe", "#000000"),
        ("h3", "groesse", "13"), ("h3", "fett", "ja"), ("h3", "textfarbe", "#1a1a1a"),
        ("h4", "groesse", "11"), ("h4", "fett", "ja"), ("h4", "kursiv", "ja"),
        ("zitat", "kursiv", "ja"), ("zitat", "rand", "#8a8a8a"), ("zitat", "rand_staerke", "1"), ("zitat", "rand_seite", "links"), ("zitat", "innen", "10"),
        ("codeblock", "groesse", "7.8"), ("codeblock", "fuellung", "#f4f1ea"), ("codeblock", "rand", "#d9d2c0"), ("codeblock", "rand_staerke", "0.6"), ("codeblock", "innen", "7"), ("codeblock", "ecken", "2"),
        ("code", "fuellung", "#f1ede4"), ("code", "textfarbe", "#3b3b3b"), ("code", "ecken", "2"),
        ("tabelle", "fuellung", "#ebe6da"), ("tabelle", "rand", "#9a9484"), ("tabelle", "rand_staerke", "0.5"), ("tabelle", "innen", "5"),
        ("info", "fuellung", "#eef2f7"), ("info", "rand", "#4a6a8a"), ("hinweis", "fuellung", "#eef2f7"), ("hinweis", "rand", "#4a6a8a"),
        ("merke", "fuellung", "#eef2f7"), ("merke", "rand", "#4a6a8a"),
        ("tipp", "fuellung", "#eef5ee"), ("tipp", "rand", "#4f7a4f"),
        ("wichtig", "fuellung", "#f8f1e3"), ("wichtig", "rand", "#a67c2a"),
        ("warnung", "fuellung", "#f8ecec"), ("warnung", "rand", "#9b3b3b"),
        ("beispiel", "fuellung", "#f1eef6"), ("beispiel", "rand", "#6d5a8f"),
        ("kaesten", "rand_seite", "links"), ("kaesten", "rand_staerke", "2.2"), ("kaesten", "innen", "8"), ("kaesten", "ecken", "0"),
    ]);
    let modern = stil_aus(&[
        ("h1", "schrift", "Fira Sans"), ("h1", "textfarbe", "#0f766e"),
        ("h2", "schrift", "Fira Sans"), ("h2", "groesse", "24"), ("h2", "fett", "ja"), ("h2", "textfarbe", "#0f766e"),
        ("h3", "schrift", "Fira Sans"), ("h3", "groesse", "14"), ("h3", "fett", "ja"), ("h3", "textfarbe", "#134e4a"),
        ("h4", "schrift", "Fira Sans"), ("h4", "groesse", "11.5"), ("h4", "fett", "ja"), ("h4", "kursiv", "nein"), ("h4", "textfarbe", "#0f766e"),
        ("zitat", "kursiv", "nein"), ("zitat", "fuellung", "#f0fdfa"), ("zitat", "rand", "#14b8a6"), ("zitat", "rand_staerke", "3"), ("zitat", "rand_seite", "links"), ("zitat", "innen", "10"), ("zitat", "ecken", "4"),
        ("codeblock", "groesse", "7.6"), ("codeblock", "fuellung", "#f1f5f9"), ("codeblock", "rand", "#cbd5e1"), ("codeblock", "rand_staerke", "0.8"), ("codeblock", "innen", "8"), ("codeblock", "ecken", "5"),
        ("code", "fuellung", "#e8eef5"), ("code", "textfarbe", "#0f4c5c"), ("code", "ecken", "3"),
        ("tabelle", "fuellung", "#ccfbf1"), ("tabelle", "rand", "#94a3b8"), ("tabelle", "rand_staerke", "0.5"), ("tabelle", "innen", "6"),
        ("info", "fuellung", "#e0f2fe"), ("info", "rand", "#0284c7"), ("hinweis", "fuellung", "#e0f2fe"), ("hinweis", "rand", "#0284c7"),
        ("merke", "fuellung", "#e0f2fe"), ("merke", "rand", "#0284c7"),
        ("tipp", "fuellung", "#dcfce7"), ("tipp", "rand", "#16a34a"),
        ("wichtig", "fuellung", "#fef3c7"), ("wichtig", "rand", "#d97706"),
        ("warnung", "fuellung", "#fee2e2"), ("warnung", "rand", "#dc2626"),
        ("beispiel", "fuellung", "#ede9fe"), ("beispiel", "rand", "#7c3aed"),
        ("kaesten", "rand_seite", "rundum"), ("kaesten", "rand_staerke", "1"), ("kaesten", "innen", "9"), ("kaesten", "ecken", "6"),
    ]);
    vec![
        Vorlage { name: "Klassisch – Buchdruck", beschreibung: "Ruhig und zeitlos: Serifenschrift, schwarze Überschriften, helle Papiertöne für Code und Tabellen, Kästen mit schmalem Streifen links. Gut für Schwarzweißdruck.", stil: klassisch },
        Vorlage { name: "Modern – Technikbuch", beschreibung: "Frisch und sachlich: serifenlose Überschriften (Fira Sans) mit Akzentfarbe Petrol, abgerundete Kästen mit Rahmen, kühle Grautöne für Code. Wirkt am besten im Web-PDF; im Druck werden die Farben zu Grau.", stil: modern },
    ]
}
#[cfg(test)]
mod tests {
    use super::*;

    fn voll() -> Satzstil {
        let mut s = Satzstil::default();
        for (e, _) in ELEMENTE {
            for feld in felder_von(e) {
                let wert = match *feld {
                    "schrift" => "DejaVu Sans Mono",
                    "groesse" => "11",
                    "textfarbe" | "fuellung" | "rand" => "#336699",
                    "fett" | "kursiv" => "ja",
                    "rand_seite" => "rundum",
                    _ => "4",
                };
                s.setze(e, feld, wert.to_string());
            }
        }
        s
    }

    #[test]
    fn farben_und_grau() {
        assert_eq!(farbe_lesen("#ff8000"), Some((255, 128, 0)));
        assert_eq!(farbe_lesen("fff"), Some((255, 255, 255)));
        assert_eq!(farbe_lesen("blau"), None);
        assert_eq!(farbe("#000000", false).as_deref(), Some("luma(0)"));
        assert_eq!(farbe("#ffffff", true).as_deref(), Some("rgb(\"#ffffff\")"));
    }

    #[test]
    fn vorlagen_sind_gueltig_und_verschieden() {
        let v = vorlagen();
        assert_eq!(v.len(), 2);
        assert_ne!(v[0].stil, v[1].stil);
        for vl in &v {
            for farbig in [true, false] {
                let q = vl.stil.muster(farbig);
                crate::welt::typst_zu_pdf(q.clone(), None, Default::default()).unwrap_or_else(|f| panic!("{} (farbig={farbig}): {f}\n{q}", vl.name));
            }
        }
    }

    #[test]
    fn leer_ergibt_nichts() {
        assert_eq!(Satzstil::default().zu_typst(true), "");
    }

    #[test]
    fn alles_gesetzt_wird_ein_pdf() {
        for farbig in [true, false] {
            let q = voll().muster(farbig);
            let e = crate::welt::typst_zu_pdf(q.clone(), None, Default::default()).unwrap_or_else(|f| panic!("{f}\n{q}"));
            assert!(e.bytes.starts_with(b"%PDF"));
        }
    }

    #[test]
    fn einzeln_jedes_element_gueltig() {
        for (e, _) in ELEMENTE {
            let mut s = Satzstil::default();
            for feld in felder_von(e) {
                s.setze(e, feld, if feld.contains("farbe") || *feld == "fuellung" || *feld == "rand" { "#aa5500".into() } else { "5".into() });
            }
            let q = s.muster(true);
            crate::welt::typst_zu_pdf(q.clone(), None, Default::default()).unwrap_or_else(|f| panic!("{e}: {f}\n{q}"));
        }
    }

    #[test]
    fn speichern_und_laden() {
        let o = std::env::temp_dir().join("werkbank-test-satzstil");
        let _ = std::fs::remove_dir_all(&o);
        let mut s = Satzstil::default();
        s.setze("zitat", "fuellung", "#eeeeee".into());
        s.setze("code", "schrift", String::new());
        s.speichern(&o).unwrap();
        assert_eq!(Satzstil::laden(&o).format("zitat").fuellung, "#eeeeee");
        assert!(!Satzstil::laden(&o).formate.contains_key("code"));
    }
}
