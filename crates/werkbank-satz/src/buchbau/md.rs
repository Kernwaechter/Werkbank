//! Markdown-Übersetzer des Buchbaus – genaue Entsprechung zu `convert_markdown_to_html` und
//! `format_inline` aus dem bisherigen `build_book.py`.
//!
//! Die Zerlegung liefert Knoten in genau der Reihenfolge, in der das Python-Skript HTML-Zeilen
//! ausgab. Daraus entsteht wieder dasselbe HTML (für das EPUB) und Typst (für die PDFs).
//! Eigenheiten des Originals (z. B. Absätze, die in einer offenen Liste landen) bleiben bewusst
//! erhalten, damit sich das Ergebnis nicht still verändert.

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Ein Eintrag im Inhaltsverzeichnis: (Ebene 1/2, Titel ohne Auszeichnung, Anker).
pub type TocEintrag = (u8, String, String);

/// Ein Knoten des zerlegten Textes (entspricht je einer Ausgabezeile des Originals).
#[derive(Debug, Clone, PartialEq)]
pub enum Knoten {
    /// Rohe HTML-Zeile für Titelei und Seitenumbruch (`<div class="imprint">`, `</div>` …).
    Html(String),
    /// Absatz aus zusammengefügten Zeilen (noch Inline-Markdown; `<br />` markiert Zeilenumbrüche).
    Absatz(String),
    /// Waagerechte Linie (`---`).
    Linie,
    /// Eigenständiges Bild: (Bildunterschrift roh, Pfad).
    Bild { unterschrift: String, pfad: String },
    /// Tabelle als Rohzellen (Trennzeilen `|---|` sind enthalten und werden beim Ausgeben übergangen).
    Tabelle(Vec<Vec<String>>),
    /// Überschrift mit Ebene 1–5, Rohtitel, Anker und Werkstatt-Rubrik (nur Ebene 3).
    Ueberschrift { ebene: u8, roh: String, id: String, rubrik: Option<String> },
    /// Beginn einer Liste (geordnet = nummeriert).
    ListeAuf(bool),
    /// Ende einer Liste.
    ListeZu(bool),
    /// Listenpunkt (Inline-Markdown).
    Punkt(String),
    /// Codeblock mit Sprache aus dem Zaun.
    Code { sprache: String, code: String },
}

/// Ergebnis der Zerlegung einer Datei.
#[derive(Debug, Clone, Default)]
pub struct Abschnitt {
    pub knoten: Vec<Knoten>,
    pub toc: Vec<TocEintrag>,
}

static RE_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`([^`]+)`").unwrap());
static RE_LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap());
static RE_FETT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*\*([^*]+)\*\*").unwrap());
static RE_KURSIV: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*([^*]+)\*").unwrap());
static RE_STERNE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*+([^*]+)\*+").unwrap());
static RE_LINIE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-{3,}$").unwrap());
static RE_BILD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^!\[([^\]]*)\]\(([^)]+)\)$").unwrap());
static RE_KOPF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(#{1,5})\s+(.*)$").unwrap());
static RE_UL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)[*-]\s+(.*)$").unwrap());
static RE_OL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)\d+\.\s+(.*)$").unwrap());
static RE_TRENNER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^:?-{2,}:?$").unwrap());
static RE_SLUG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^a-z0-9]+").unwrap());

/// `html.escape(s, quote=True)`.
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#x27;")
}

/// `html.escape(s, quote=False)`.
pub fn escape_ohne_anfz(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Anker aus einem Titel (wie `slugify` im Original).
pub fn slugify(text: &str) -> String {
    let mut t = text.to_lowercase();
    for (a, b) in [("ä", "ae"), ("ö", "oe"), ("ü", "ue"), ("ß", "ss")] {
        t = t.replace(a, b);
    }
    let t = RE_SLUG.replace_all(&t, "-");
    let t = t.trim_matches('-');
    if t.is_empty() { "abschnitt".to_string() } else { t.to_string() }
}

/// Inline-Markdown (Code, Fett, Kursiv, Links) als XHTML – wie `format_inline`.
pub fn format_inline(text: &str) -> String {
    let mut spans: Vec<String> = Vec::new();
    let text = RE_CODE.replace_all(text, |c: &regex::Captures| {
        let idx = spans.len();
        spans.push(format!("<code>{}</code>", escape(&c[1])));
        format!("\x00CODE{idx}\x00")
    });
    let text = escape_ohne_anfz(&text);
    let text = RE_LINK.replace_all(&text, |c: &regex::Captures| format!("<a href=\"{}\">{}</a>", escape(&c[2]), &c[1]));
    let text = RE_FETT.replace_all(&text, "<strong>$1</strong>");
    let mut text = RE_KURSIV.replace_all(&text, "<em>$1</em>").into_owned();
    for (idx, span) in spans.iter().enumerate() {
        text = text.replace(&format!("\x00CODE{idx}\x00"), span);
    }
    text
}

/// Titel ohne Code- und Sternchen-Auszeichnung (für Anker und Inhaltsverzeichnis).
pub fn reiner_titel(roh: &str) -> String {
    let t = RE_CODE.replace_all(roh, "$1");
    RE_STERNE.replace_all(&t, "$1").into_owned()
}

/// Zeilen wie Pythons `str.splitlines()`: trennt an \n, \r\n, \r und weiteren Zeilenenden
/// (Seitenvorschub \x0c, \x0b, \x1c–\x1e, \u{85}, \u{2028}, \u{2029}); ein Ende am Schluss erzeugt keine leere Zeile.
pub fn splitlines(text: &str) -> Vec<&str> {
    let mut zeilen = Vec::new();
    let mut start = 0;
    let mut zeichen = text.char_indices().peekable();
    while let Some((i, c)) = zeichen.next() {
        if matches!(c, '\n' | '\r' | '\x0b' | '\x0c' | '\x1c' | '\x1d' | '\x1e' | '\u{85}' | '\u{2028}' | '\u{2029}') {
            zeilen.push(&text[start..i]);
            let mut ende = i + c.len_utf8();
            if c == '\r' && zeichen.peek().map(|&(_, n)| n) == Some('\n') {
                zeichen.next();
                ende += 1;
            }
            start = ende;
        }
    }
    if start < text.len() {
        zeilen.push(&text[start..]);
    }
    zeilen
}

/// Zerlegt eine Markdown-Datei. `rubriken`: (Titelanfang, Klasse) der Werkstatt-Rubriken in fester Reihenfolge.
pub fn zerlegen(md: &str, id_prefix: &str, rubriken: &[(String, String)]) -> Abschnitt {
    const HTML_ZEILEN: [&str; 5] =
        ["<div class=\"half-title\">", "<div class=\"title-page\">", "<div class=\"imprint\">", "</div>", "<div class=\"page-break\"></div>"];

    let mut out: Vec<Knoten> = Vec::new();
    let mut toc: Vec<TocEintrag> = Vec::new();
    let mut benutzt: HashSet<String> = HashSet::new();

    let mut im_code = false;
    let mut sprache = String::new();
    let mut code_zeilen: Vec<&str> = Vec::new();
    let mut in_ul = false;
    let mut in_ol = false;
    let mut in_tabelle = false;
    let mut tabelle: Vec<Vec<String>> = Vec::new();
    let mut absatz: Vec<String> = Vec::new();

    fn absatz_leeren(absatz: &mut Vec<String>, out: &mut Vec<Knoten>) {
        if !absatz.is_empty() {
            let verbunden = absatz.iter().map(|z| z.trim()).collect::<Vec<_>>().join(" ");
            out.push(Knoten::Absatz(verbunden));
            absatz.clear();
        }
    }
    fn listen_schliessen(in_ul: &mut bool, in_ol: &mut bool, out: &mut Vec<Knoten>) {
        if *in_ul {
            out.push(Knoten::ListeZu(false));
            *in_ul = false;
        }
        if *in_ol {
            out.push(Knoten::ListeZu(true));
            *in_ol = false;
        }
    }
    fn tabelle_leeren(in_tabelle: &mut bool, tabelle: &mut Vec<Vec<String>>, out: &mut Vec<Knoten>) {
        if !*in_tabelle {
            return;
        }
        if !tabelle.is_empty() {
            out.push(Knoten::Tabelle(std::mem::take(tabelle)));
        }
        *in_tabelle = false;
        tabelle.clear();
    }

    for zeile in splitlines(md) {
        // Python `str.strip()` entfernt Leerraum an beiden Enden
        let s = zeile.trim();

        if s.starts_with("```") {
            absatz_leeren(&mut absatz, &mut out);
            listen_schliessen(&mut in_ul, &mut in_ol, &mut out);
            tabelle_leeren(&mut in_tabelle, &mut tabelle, &mut out);
            if !im_code {
                im_code = true;
                sprache = s.strip_prefix("```").unwrap_or(s).trim().to_string();
                code_zeilen.clear();
            } else {
                out.push(Knoten::Code { sprache: std::mem::take(&mut sprache), code: code_zeilen.join("\n") });
                im_code = false;
                code_zeilen.clear();
            }
            continue;
        }
        if im_code {
            code_zeilen.push(zeile);
            continue;
        }

        if HTML_ZEILEN.contains(&s) {
            absatz_leeren(&mut absatz, &mut out);
            listen_schliessen(&mut in_ul, &mut in_ol, &mut out);
            tabelle_leeren(&mut in_tabelle, &mut tabelle, &mut out);
            out.push(Knoten::Html(s.to_string()));
            continue;
        }
        if s.is_empty() {
            absatz_leeren(&mut absatz, &mut out);
            listen_schliessen(&mut in_ul, &mut in_ol, &mut out);
            tabelle_leeren(&mut in_tabelle, &mut tabelle, &mut out);
            continue;
        }
        if RE_LINIE.is_match(s) {
            absatz_leeren(&mut absatz, &mut out);
            listen_schliessen(&mut in_ul, &mut in_ol, &mut out);
            tabelle_leeren(&mut in_tabelle, &mut tabelle, &mut out);
            out.push(Knoten::Linie);
            continue;
        }
        if let Some(c) = RE_BILD.captures(s) {
            absatz_leeren(&mut absatz, &mut out);
            listen_schliessen(&mut in_ul, &mut in_ol, &mut out);
            tabelle_leeren(&mut in_tabelle, &mut tabelle, &mut out);
            out.push(Knoten::Bild { unterschrift: c[1].trim().to_string(), pfad: c[2].trim().to_string() });
            continue;
        }
        if s.starts_with('|') && s.ends_with('|') {
            absatz_leeren(&mut absatz, &mut out);
            listen_schliessen(&mut in_ul, &mut in_ol, &mut out);
            in_tabelle = true;
            // Wie Python `stripped[1:-1]`: eine Zeile aus nur „|“ ergibt eine leere Zelle
            let innen = if s.len() >= 2 { &s[1..s.len() - 1] } else { "" };
            tabelle.push(innen.split('|').map(str::to_string).collect());
            continue;
        } else if in_tabelle {
            tabelle_leeren(&mut in_tabelle, &mut tabelle, &mut out);
        }
        if let Some(c) = RE_KOPF.captures(s) {
            absatz_leeren(&mut absatz, &mut out);
            listen_schliessen(&mut in_ul, &mut in_ol, &mut out);
            let ebene = c[1].len() as u8;
            let roh = c[2].trim().to_string();
            let rein = reiner_titel(&roh);
            let basis = format!("{id_prefix}{}", slugify(&rein));
            let mut id = basis.clone();
            let mut zaehler = 2;
            while benutzt.contains(&id) {
                id = format!("{basis}-{zaehler}");
                zaehler += 1;
            }
            benutzt.insert(id.clone());
            if ebene <= 2 {
                toc.push((ebene, rein.clone(), id.clone()));
            }
            let rubrik = (ebene == 3)
                .then(|| rubriken.iter().find(|(name, _)| rein.starts_with(name.as_str())).map(|(_, kl)| kl.clone()))
                .flatten();
            out.push(Knoten::Ueberschrift { ebene, roh, id, rubrik });
            continue;
        }
        if let Some(c) = RE_UL.captures(zeile) {
            absatz_leeren(&mut absatz, &mut out);
            if in_ol {
                out.push(Knoten::ListeZu(true));
                in_ol = false;
            }
            if !in_ul {
                out.push(Knoten::ListeAuf(false));
                in_ul = true;
            }
            out.push(Knoten::Punkt(c[2].trim().to_string()));
            continue;
        }
        if let Some(c) = RE_OL.captures(zeile) {
            absatz_leeren(&mut absatz, &mut out);
            if in_ul {
                out.push(Knoten::ListeZu(false));
                in_ul = false;
            }
            if !in_ol {
                out.push(Knoten::ListeAuf(true));
                in_ol = true;
            }
            out.push(Knoten::Punkt(c[2].trim().to_string()));
            continue;
        }
        if zeile.ends_with("  ") {
            absatz.push(format!("{s}<br />"));
        } else {
            absatz.push(s.to_string());
        }
    }
    absatz_leeren(&mut absatz, &mut out);
    listen_schliessen(&mut in_ul, &mut in_ol, &mut out);
    tabelle_leeren(&mut in_tabelle, &mut tabelle, &mut out);

    Abschnitt { knoten: out, toc }
}

/// Das HTML eines Abschnitts (wie die Ausgabe von `convert_markdown_to_html`).
/// `code_html(code, sprache)` liefert den hervorgehobenen Codeblock.
pub fn html(abschnitt: &Abschnitt, code_html: &dyn Fn(&str, &str) -> String) -> String {
    let mut out: Vec<String> = Vec::new();
    for k in &abschnitt.knoten {
        match k {
            Knoten::Html(s) => out.push(s.clone()),
            Knoten::Absatz(t) => out.push(format!("<p>{}</p>", format_inline(t))),
            Knoten::Linie => out.push("<hr />".to_string()),
            Knoten::Bild { unterschrift, pfad } => {
                let bildunterschrift =
                    if unterschrift.is_empty() { String::new() } else { format!("<figcaption>{}</figcaption>", format_inline(unterschrift)) };
                out.push(format!(
                    "<figure class=\"book-figure\"><img src=\"{}\" alt=\"{}\" />{bildunterschrift}</figure>",
                    escape(pfad),
                    escape(unterschrift)
                ));
            }
            Knoten::Tabelle(zeilen) => {
                out.push("<table class=\"book-table\">".to_string());
                out.push("<thead><tr>".to_string());
                for zelle in &zeilen[0] {
                    out.push(format!("<th>{}</th>", format_inline(zelle.trim())));
                }
                out.push("</tr></thead><tbody>".to_string());
                for z in &zeilen[1..] {
                    if z.iter().all(|c| RE_TRENNER.is_match(c.trim())) {
                        continue;
                    }
                    out.push("<tr>".to_string());
                    for zelle in z {
                        out.push(format!("<td>{}</td>", format_inline(zelle.trim())));
                    }
                    out.push("</tr>".to_string());
                }
                out.push("</tbody></table>".to_string());
            }
            Knoten::Ueberschrift { ebene, roh, id, rubrik } => {
                let klasse = rubrik.as_ref().map(|r| format!(" class=\"rubrik-heading rubrik-{r}\"")).unwrap_or_default();
                out.push(format!("<h{ebene} id=\"{id}\"{klasse}>{}</h{ebene}>", format_inline(roh)));
            }
            Knoten::ListeAuf(geordnet) => out.push(if *geordnet { "<ol>" } else { "<ul>" }.to_string()),
            Knoten::ListeZu(geordnet) => out.push(if *geordnet { "</ol>" } else { "</ul>" }.to_string()),
            Knoten::Punkt(t) => out.push(format!("<li>{}</li>", format_inline(t))),
            Knoten::Code { sprache, code } => out.push(code_html(code, sprache)),
        }
    }
    out.join("\n").replace("&lt;br /&gt;", "<br />")
}

/// Kopfzeile eines Codeblocks: Sprache in Großbuchstaben, sonst „CODE“.
pub fn sprach_etikett(sprache: &str) -> String {
    let s = sprache.trim().to_lowercase();
    if s.is_empty() || s == "text" { "CODE".to_string() } else { escape(&s.to_uppercase()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rubriken() -> Vec<(String, String)> {
        vec![("Bughunter".to_string(), "bughunter".to_string()), ("Klartext".to_string(), "klartext".to_string())]
    }

    #[test]
    fn inline_wie_python() {
        assert_eq!(
            format_inline("Das **fette** und *kursive* `a<b>` mit [Link](https://x.de?a=1&b=2) & mehr"),
            "Das <strong>fette</strong> und <em>kursive</em> <code>a&lt;b&gt;</code> mit <a href=\"https://x.de?a=1&amp;amp;b=2\">Link</a> &amp; mehr"
        );
        assert_eq!(format_inline("`it's`"), "<code>it&#x27;s</code>");
    }

    #[test]
    fn zeilen_wie_python() {
        assert_eq!(splitlines("a\nb\r\nc\rd\x0ce\n"), ["a", "b", "c", "d", "e"]);
        assert_eq!(splitlines("a\n\nb"), ["a", "", "b"]);
        assert!(splitlines("").is_empty());
    }

    #[test]
    fn slugs() {
        assert_eq!(slugify("Kapitel 3: Hallo, Werkstatt!"), "kapitel-3-hallo-werkstatt");
        assert_eq!(slugify("Größe & Übermaß"), "groesse-uebermass");
        assert_eq!(slugify("!!!"), "abschnitt");
    }

    #[test]
    fn bloecke_und_inhaltsverzeichnis() {
        let md = "# Teil\n\n## Kapitel `x`\nText  \nweiter\n\n### Bughunter: Fehler\n- a\n- b\n1. eins\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n![Abb](images/a.png)\n\n```dart\nvoid main() {}\n```\n## Kapitel `x`\n";
        let a = zerlegen(md, "s2_", &rubriken());
        assert_eq!(a.toc, vec![(1, "Teil".into(), "s2_teil".into()), (2, "Kapitel x".into(), "s2_kapitel-x".into()), (2, "Kapitel x".into(), "s2_kapitel-x-2".into())]);
        let h = html(&a, &|c, s| format!("[{s}:{c}]"));
        assert!(h.contains("<p>Text<br /> weiter</p>"), "{h}");
        assert!(h.contains("<h3 id=\"s2_bughunter-fehler\" class=\"rubrik-heading rubrik-bughunter\">Bughunter: Fehler</h3>"));
        assert!(h.contains("<ul>\n<li>a</li>\n<li>b</li>\n</ul>\n<ol>\n<li>eins</li>\n</ol>"), "{h}");
        assert!(h.contains("<thead><tr>\n<th>A</th>\n<th>B</th>\n</tr></thead><tbody>\n<tr>\n<td>1</td>\n<td>2</td>\n</tr>\n</tbody></table>"), "{h}");
        assert!(h.contains("<figure class=\"book-figure\"><img src=\"images/a.png\" alt=\"Abb\" /><figcaption>Abb</figcaption></figure>"));
        assert!(h.contains("[dart:void main() {}]"));
    }
}
