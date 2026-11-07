//! `klappentext.md`: die einzige Quelle für den Text der Umschlag-Rückseite.
//!
//! Aufbau (alles außer der Schlagzeile ist freiwillig):
//!
//! ```text
//! Dachzeile über der Schlagzeile (erster Absatz vor der ersten Überschrift)
//!
//! # Schlagzeile (`<br/>` = Zeilenumbruch)
//! Absätze des Fließtextes mit **fett**, *kursiv* und `Code`.
//!
//! ## Titel des Merkmalskastens      ← Überschrift mit Aufzählung = Kasten
//! - **Punkt:** Erklärung
//!
//! ## Über den Autor                 ← Überschrift ohne Aufzählung = Infoblock unten
//! Text …
//!
//! ---
//! Fußzeile 1 (z. B. Webseiten)       ← Zeilen nach dem letzten `---` (höchstens vier)
//! Fußzeile 2 · {seiten} Seiten       ← `{seiten}` wird durch die Seitenzahl ersetzt
//! ```

use super::profil::{Band, Profil};
use crate::markdown::zeichenkette;

/// Kasten mit Titel und Aufzählungspunkten.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Kasten {
    pub titel: String,
    pub punkte: Vec<String>,
}

/// Infoblock (z. B. „Über den Autor“) am unteren Rand des Textes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Info {
    pub titel: String,
    pub absaetze: Vec<String>,
}

/// Der gelesene Klappentext. Texte sind Markdown (Inline: `**`, `*`, `` ` ``, `<br/>`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Klappentext {
    pub dachzeile: String,
    pub schlagzeile: String,
    pub absaetze: Vec<String>,
    pub kaesten: Vec<Kasten>,
    pub infos: Vec<Info>,
    pub fuss: Vec<String>,
}

impl Klappentext {
    /// Ist nichts zu setzen?
    pub fn leer(&self) -> bool {
        self.schlagzeile.is_empty() && self.absaetze.is_empty() && self.kaesten.is_empty() && self.infos.is_empty()
    }
}

fn ist_punkt(z: &str) -> Option<&str> {
    z.strip_prefix("- ").or_else(|| z.strip_prefix("* ")).or_else(|| z.strip_prefix("• "))
}

fn ist_trenner(z: &str) -> bool {
    z.len() >= 3 && z.chars().all(|c| c == '-') || z == "***" || z == "___"
}

/// Zerlegt Zeilen in Absätze (durch Leerzeilen getrennt, Zeilen laufen zusammen) und Aufzählungspunkte.
fn absaetze_und_punkte(zeilen: &[&str]) -> (Vec<String>, Vec<String>) {
    let (mut absaetze, mut punkte) = (Vec::new(), Vec::new());
    let mut aktuell: Vec<&str> = Vec::new();
    let fertig = |aktuell: &mut Vec<&str>, absaetze: &mut Vec<String>| {
        if !aktuell.is_empty() {
            absaetze.push(aktuell.join(" "));
            aktuell.clear();
        }
    };
    for z in zeilen {
        let t = z.trim();
        if t.is_empty() {
            fertig(&mut aktuell, &mut absaetze);
        } else if let Some(p) = ist_punkt(t) {
            fertig(&mut aktuell, &mut absaetze);
            punkte.push(p.trim().to_string());
        } else {
            aktuell.push(t);
        }
    }
    fertig(&mut aktuell, &mut absaetze);
    (absaetze, punkte)
}

/// Liest `klappentext.md`.
pub fn lesen(md: &str) -> Klappentext {
    let zeilen: Vec<&str> = md.trim_start_matches('\u{feff}').lines().collect();
    // Fußzeile: die Zeilen nach dem letzten Trenner, wenn es höchstens vier sind
    let mut ende = zeilen.len();
    let mut fuss = Vec::new();
    if let Some(i) = zeilen.iter().rposition(|z| ist_trenner(z.trim())) {
        let rest: Vec<String> = zeilen[i + 1..].iter().map(|z| z.trim()).filter(|z| !z.is_empty()).map(str::to_string).collect();
        if !rest.is_empty() && rest.len() <= 4 && !zeilen[i + 1..].iter().any(|z| z.trim().starts_with('#')) {
            fuss = rest;
            ende = i;
        }
    }
    // Abschnitte nach Überschriften
    let mut vorspann: Vec<&str> = Vec::new();
    let mut abschnitte: Vec<(String, Vec<&str>)> = Vec::new();
    for z in &zeilen[..ende] {
        let t = z.trim();
        if t.starts_with('#') {
            abschnitte.push((t.trim_start_matches('#').trim().to_string(), Vec::new()));
        } else if ist_trenner(t) {
            // Trenner im Text zählen nicht
        } else if let Some(letzter) = abschnitte.last_mut() {
            letzter.1.push(z);
        } else {
            vorspann.push(z);
        }
    }
    let mut k = Klappentext { fuss, ..Default::default() };
    k.dachzeile = absaetze_und_punkte(&vorspann).0.into_iter().next().unwrap_or_default();
    for (n, (titel, rumpf)) in abschnitte.into_iter().enumerate() {
        let (absaetze, punkte) = absaetze_und_punkte(&rumpf);
        if n == 0 {
            k.schlagzeile = titel;
            k.absaetze = absaetze;
            if !punkte.is_empty() {
                k.kaesten.push(Kasten { titel: String::new(), punkte });
            }
        } else if !punkte.is_empty() {
            k.kaesten.push(Kasten { titel, punkte });
        } else {
            k.infos.push(Info { titel, absaetze });
        }
    }
    k
}

/// HTML-Auszeichnung der alten `buchbau.yaml` → Markdown (`<strong>`, `<em>`, `<code>`, Entitäten; `<br/>` bleibt).
fn html_zu_markdown(h: &str) -> String {
    h.replace("<strong>", "**")
        .replace("</strong>", "**")
        .replace("<em>", "*")
        .replace("</em>", "*")
        .replace("<code>", "`")
        .replace("</code>", "`")
        .replace("<br>", "<br/>")
        .replace("<br />", "<br/>")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

/// Baut aus den Umschlagtexten der alten `buchbau.yaml` einen `klappentext.md` (Übernahme in die neue Quelle).
pub fn aus_umschlag(profil: &Profil, band: &Band) -> String {
    let (u, r) = (&band.umschlag, &profil.reihe);
    let mut s = String::new();
    if !u.rueckseite_dachzeile.trim().is_empty() {
        s.push_str(&format!("{}\n\n", html_zu_markdown(u.rueckseite_dachzeile.trim())));
    }
    s.push_str(&format!("# {}\n\n", html_zu_markdown(u.rueckseite_titel.trim())));
    if !u.rueckseite_text.trim().is_empty() {
        s.push_str(&format!("{}\n\n", html_zu_markdown(u.rueckseite_text.trim())));
    }
    if !u.merkmale.is_empty() {
        s.push_str(&format!("## {}\n\n", html_zu_markdown(u.merkmale_titel.trim())));
        for m in &u.merkmale {
            s.push_str(&format!("- {}\n", html_zu_markdown(m.trim())));
        }
        s.push('\n');
    }
    if !r.autor_bio.trim().is_empty() {
        s.push_str(&format!("## {}\n\n{}\n\n", html_zu_markdown(r.autor_bio_titel.trim()), html_zu_markdown(r.autor_bio.trim())));
    }
    if !r.webseiten_zeile.trim().is_empty() || !r.download_zeile.trim().is_empty() {
        s.push_str("---\n");
        if !r.webseiten_zeile.trim().is_empty() {
            s.push_str(&format!("{}\n", r.webseiten_zeile.trim()));
        }
        if !r.download_zeile.trim().is_empty() {
            s.push_str(&format!("{} · {{seiten}} Seiten\n", r.download_zeile.trim()));
        }
    }
    s
}

fn entitaeten(s: &str) -> String {
    s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&amp;", "&")
}

/// Inline-Markdown (`**fett**`, `*kursiv*`, `` `Code` ``, `<br/>`) → Typst-Markup.
pub fn inline_typst(md: &str) -> String {
    fn lauf(zeichen: &[char], pos: &mut usize, bis: Option<&str>) -> String {
        let mut aus = String::new();
        let mut text = String::new();
        let leeren = |text: &mut String, aus: &mut String| {
            if !text.is_empty() {
                aus.push('#');
                aus.push_str(&zeichenkette(&entitaeten(text)));
                text.clear();
            }
        };
        let rest = |p: usize| -> String { zeichen[p..].iter().take(8).collect() };
        while *pos < zeichen.len() {
            let r = rest(*pos);
            if let Some(b) = bis
                && r.starts_with(b)
            {
                *pos += b.chars().count();
                leeren(&mut text, &mut aus);
                return aus;
            }
            if r.starts_with("<br/>") || r.starts_with("<br>") {
                leeren(&mut text, &mut aus);
                aus.push_str("#linebreak()");
                *pos += if r.starts_with("<br/>") { 5 } else { 4 };
            } else if r.starts_with("**") {
                leeren(&mut text, &mut aus);
                *pos += 2;
                let innen = lauf(zeichen, pos, Some("**"));
                aus.push_str(&format!("#strong[{innen}]"));
            } else if r.starts_with('*') || (r.starts_with('_') && (*pos == 0 || !zeichen[*pos - 1].is_alphanumeric())) {
                let marke = if r.starts_with('*') { "*" } else { "_" };
                leeren(&mut text, &mut aus);
                *pos += 1;
                let innen = lauf(zeichen, pos, Some(marke));
                aus.push_str(&format!("#emph[{innen}]"));
            } else if r.starts_with('`') {
                leeren(&mut text, &mut aus);
                *pos += 1;
                let mut code = String::new();
                while *pos < zeichen.len() && zeichen[*pos] != '`' {
                    code.push(zeichen[*pos]);
                    *pos += 1;
                }
                *pos += 1;
                aus.push_str(&format!("#ic({})", zeichenkette(&entitaeten(&code))));
            } else if zeichen[*pos] == '\\' && *pos + 1 < zeichen.len() {
                text.push(zeichen[*pos + 1]);
                *pos += 2;
            } else {
                text.push(zeichen[*pos]);
                *pos += 1;
            }
        }
        leeren(&mut text, &mut aus);
        aus
    }
    let zeichen: Vec<char> = md.chars().collect();
    let mut pos = 0;
    lauf(&zeichen, &mut pos, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEISPIEL: &str = "Programmieren lernen\n\n# Keine Theorie.<br/>Sofort loslegen.\n\nDu möchtest **loslegen**?\nHier geht es los.\n\n\
Zweiter Absatz.\n\n## Deine 2 Projekte:\n\n- **Eins:** `State`\n- Zwei\n\n## Über den Autor\n\nJahrgang 1971.\n\n---\nfoo.de · bar.de\nDownload · {seiten} Seiten\n";

    #[test]
    fn struktur_wird_erkannt() {
        let k = lesen(BEISPIEL);
        assert_eq!(k.dachzeile, "Programmieren lernen");
        assert_eq!(k.schlagzeile, "Keine Theorie.<br/>Sofort loslegen.");
        assert_eq!(k.absaetze, vec!["Du möchtest **loslegen**? Hier geht es los.", "Zweiter Absatz."]);
        assert_eq!(k.kaesten.len(), 1);
        assert_eq!(k.kaesten[0].titel, "Deine 2 Projekte:");
        assert_eq!(k.kaesten[0].punkte, vec!["**Eins:** `State`", "Zwei"]);
        assert_eq!(k.infos, vec![Info { titel: "Über den Autor".into(), absaetze: vec!["Jahrgang 1971.".into()] }]);
        assert_eq!(k.fuss, vec!["foo.de · bar.de", "Download · {seiten} Seiten"]);
    }

    #[test]
    fn alter_text_ohne_dachzeile_und_fuss_geht_auch() {
        let k = lesen("# Schlagzeile\n\nText.\n\n- a\n- b\n");
        assert_eq!(k.dachzeile, "");
        assert_eq!(k.schlagzeile, "Schlagzeile");
        assert_eq!(k.kaesten.len(), 1);
        assert!(k.fuss.is_empty());
        assert!(lesen("").leer());
    }

    #[test]
    fn trenner_mitten_im_text_ist_keine_fusszeile() {
        let k = lesen("# A\n\nText\n\n---\n\n## B\n\nMehr Text\n");
        assert!(k.fuss.is_empty());
        assert_eq!(k.infos.len(), 1);
    }

    #[test]
    fn inline_wird_typst() {
        let t = inline_typst("A **b** & *c* `d` x<br/>y");
        assert!(t.contains("#strong[#\"b\"]"), "{t}");
        assert!(t.contains("#emph[#\"c\"]"), "{t}");
        assert!(t.contains("#ic(\"d\")"), "{t}");
        assert!(t.contains("#linebreak()"), "{t}");
        assert!(t.contains("#\" & \""), "{t}");
    }

    #[test]
    fn umschlagtexte_der_alten_datei_werden_uebernommen() {
        let mut p: Profil = serde_yaml::from_str(
            "titel: B\nautor: A\nverlag: V\nformat_mm: [170, 240]\nraender_mm: {innen: 22, aussen: 18, oben: 20, unten: 22}\nquellen: [a.md]\nrubriken: []\nbaende:\n- {nummer: 1, ordner: B1, untertitel: U, uuid: x, kurzname: k}\n",
        )
        .unwrap();
        p.reihe.autor_bio_titel = "Über den Autor".into();
        p.reihe.autor_bio = "Er <strong>kann</strong> es.".into();
        p.reihe.webseiten_zeile = "a.de · b.de".into();
        p.reihe.download_zeile = "Download".into();
        let mut b = p.baende[0].clone();
        b.umschlag.rueckseite_dachzeile = "Dach".into();
        b.umschlag.rueckseite_titel = "Eins.<br/>Zwei &amp; drei".into();
        b.umschlag.rueckseite_text = "Text mit <code>x</code>.".into();
        b.umschlag.merkmale_titel = "Projekte:".into();
        b.umschlag.merkmale = vec!["<strong>P:</strong> a &amp; b".into()];
        let md = aus_umschlag(&p, &b);
        let k = lesen(&md);
        assert_eq!(k.dachzeile, "Dach");
        assert_eq!(k.schlagzeile, "Eins.<br/>Zwei & drei");
        assert_eq!(k.absaetze, vec!["Text mit `x`."]);
        assert_eq!(k.kaesten[0].punkte, vec!["**P:** a & b"]);
        assert_eq!(k.infos[0].absaetze, vec!["Er **kann** es."]);
        assert_eq!(k.fuss, vec!["a.de · b.de", "Download · {seiten} Seiten"]);
    }
}
