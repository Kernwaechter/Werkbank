//! EPUB 3 aus einem Buchprojekt.

use crate::markdown::kapitelnummern;
use epub_builder::{EpubBuilder, EpubContent, EpubVersion, ReferenceType, ZipLibrary};
use pulldown_cmark::{Event, Options, Parser, Tag, html};
use std::collections::HashMap;
use std::path::Path;
use werkbank_kern::Projekt;

const STANDARD_CSS: &str = "body{font-family:serif;line-height:1.45;margin:0 4%}h1,h2,h3,h4{font-family:sans-serif;line-height:1.2;page-break-after:avoid}h1{font-size:1.9em;text-align:center;margin:3em 0 1em}h2{font-size:1.45em;margin:1.6em 0 .6em}pre{background:#f2f2f2;padding:.6em;overflow-x:auto;white-space:pre-wrap;font-size:.82em}code{font-family:monospace;font-size:.92em}table{border-collapse:collapse}td,th{border:1px solid #888;padding:.25em .5em}blockquote{border-left:3px solid #aaa;margin-left:0;padding-left:1em;color:#444}img{max-width:100%}";

fn xhtml(titel: &str, koerper: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"de\" lang=\"de\">\n<head><meta charset=\"utf-8\"/><title>{}</title><link rel=\"stylesheet\" type=\"text/css\" href=\"stylesheet.css\"/></head>\n<body>\n{koerper}\n</body>\n</html>\n",
        html_maskieren(titel)
    )
}

fn html_maskieren(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn mime(pfad: &str) -> &'static str {
    match pfad.rsplit('.').next().map(str::to_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        Some("gif") => "image/gif",
        _ => "application/octet-stream",
    }
}

/// Markdown → XHTML-Körper; Bilder werden gesammelt und auf `bilder/<name>` umgeschrieben,
/// `@kap:`-Verweise werden zu Verweisen auf die Kapiteldatei.
fn koerper(md: &str, nummern: &HashMap<String, usize>, dateinamen: &HashMap<String, String>, bilder: &mut Vec<String>) -> String {
    let mut text = String::new();
    let mut rest = md;
    while let Some(p) = rest.find("@kap:") {
        text.push_str(&rest[..p]);
        let nach = &rest[p + 5..];
        let l = nach.find(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_')).unwrap_or(nach.len());
        let id = &nach[..l];
        let schon = text.ends_with("Kapitel ") || text.ends_with("Kapiteln ");
        match (nummern.get(id), dateinamen.get(id)) {
            (Some(n), Some(f)) if schon => text.push_str(&format!("[{n}]({f})")),
            (Some(n), Some(f)) => text.push_str(&format!("[Kapitel {n}]({f})")),
            _ => text.push_str(&format!("Kapitel „{id}“")),
        }
        rest = &nach[l..];
    }
    text.push_str(rest);

    let mut opt = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    opt.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    let ereignisse = Parser::new_ext(&text, opt).map(|e| match e {
        Event::Start(Tag::Image { link_type, dest_url, title, id }) if !dest_url.starts_with("http") => {
            let pfad = dest_url.to_string();
            let name = Path::new(&pfad).file_name().and_then(|n| n.to_str()).unwrap_or("bild").to_string();
            if !bilder.contains(&pfad) {
                bilder.push(pfad);
            }
            Event::Start(Tag::Image { link_type, dest_url: format!("bilder/{name}").into(), title, id })
        }
        andere => andere,
    });
    let mut aus = String::new();
    html::push_html(&mut aus, ereignisse);
    aus
}

/// Schreibt ein EPUB 3. `cover_png`: Vorderseite als PNG (optional).
pub fn epub_schreiben(projekt: &Projekt, cover_png: Option<Vec<u8>>, ziel: &Path) -> Result<(), String> {
    let fehler = |e: epub_builder::Error| format!("EPUB: {e}");
    let m = &projekt.metadaten;
    let nummern = kapitelnummern(projekt);

    // Dateinamen je Kapitel vorab festlegen, damit Verweise auflösbar sind
    let mut dateinamen: HashMap<String, String> = HashMap::new();
    for (ti, teil) in projekt.teile.iter().enumerate() {
        for (ki, k) in teil.kapitel.iter().enumerate() {
            dateinamen.insert(k.id.clone(), format!("t{:02}-k{:02}.xhtml", ti + 1, ki + 1));
        }
    }

    let mut epub = EpubBuilder::new(ZipLibrary::new().map_err(fehler)?).map_err(fehler)?;
    epub.epub_version(EpubVersion::V30);
    epub.metadata("title", &m.titel).map_err(fehler)?;
    epub.metadata("author", &m.autor).map_err(fehler)?;
    epub.metadata("lang", "de").map_err(fehler)?;
    if let Some(isbn) = &m.isbn_print {
        epub.metadata("description", format!("{} – ISBN {}", m.untertitel, isbn)).map_err(fehler)?;
    }
    let css = std::fs::read_to_string(projekt.wurzelverzeichnis.join("epub.css")).unwrap_or_else(|_| STANDARD_CSS.to_string());
    epub.stylesheet(css.as_bytes()).map_err(fehler)?;
    if let Some(png) = cover_png {
        epub.add_cover_image("cover.png", png.as_slice(), "image/png").map_err(fehler)?;
    }
    epub.add_content(
        EpubContent::new(
            "titel.xhtml",
            xhtml(&m.titel, &format!("<h1>{}</h1><p style=\"text-align:center\">{}</p><p style=\"text-align:center\">{}</p><p style=\"text-align:center\">{}</p>", html_maskieren(&m.titel), html_maskieren(&m.untertitel), html_maskieren(&m.band_bezeichnung), html_maskieren(&m.autor))).as_bytes(),
        )
        .title("Titel")
        .reftype(ReferenceType::TitlePage),
    )
    .map_err(fehler)?;

    let mut bilder: Vec<String> = Vec::new();
    for (ti, teil) in projekt.teile.iter().enumerate() {
        let teil_datei = format!("t{:02}.xhtml", ti + 1);
        epub.add_content(
            EpubContent::new(&teil_datei, xhtml(&teil.titel, &koerper(&teil.praeambel, &nummern, &dateinamen, &mut bilder)).as_bytes())
                .title(teil.titel.clone())
                .reftype(ReferenceType::Text),
        )
        .map_err(fehler)?;
        for k in &teil.kapitel {
            let inhalt = match nummern.get(&k.id) {
                Some(n) if k.inhalt.starts_with("## ") => {
                    let (kopf, rest) = k.inhalt.split_once('\n').unwrap_or((&k.inhalt, ""));
                    let titel = kopf.find("]:").map_or(kopf.trim_start_matches("## ").trim(), |i| kopf[i + 2..].trim());
                    format!("## Kapitel {n}: {titel}\n{rest}")
                }
                _ => k.inhalt.clone(),
            };
            let titel = inhalt.lines().next().unwrap_or("").trim_start_matches('#').trim().to_string();
            let datei = &dateinamen[&k.id];
            epub.add_content(
                EpubContent::new(datei, xhtml(&titel, &koerper(&inhalt, &nummern, &dateinamen, &mut bilder)).as_bytes())
                    .title(if titel.is_empty() { k.titel.clone() } else { titel })
                    .level(2),
            )
            .map_err(fehler)?;
        }
    }

    for pfad in bilder {
        let quelle = projekt.wurzelverzeichnis.join(pfad.trim_start_matches('/'));
        if let Ok(daten) = std::fs::read(&quelle) {
            let name = Path::new(&pfad).file_name().and_then(|n| n.to_str()).unwrap_or("bild");
            epub.add_resource(format!("bilder/{name}"), daten.as_slice(), mime(&pfad)).map_err(fehler)?;
        }
    }

    if let Some(eltern) = ziel.parent() {
        std::fs::create_dir_all(eltern).map_err(|e| e.to_string())?;
    }
    let mut datei = std::fs::File::create(ziel).map_err(|e| format!("{}: {e}", ziel.display()))?;
    epub.generate(&mut datei).map_err(fehler)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use werkbank_kern::{Kapitel, Teil};

    #[test]
    fn schreibt_gueltiges_epub() {
        let o = std::env::temp_dir().join("werkbank-satz-epub");
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(&o).unwrap();
        let mut p = Projekt::neu(&o);
        p.metadaten.titel = "Testbuch & Co".into();
        let mut t = Teil::neu("t1", "Teil 1: Anfang");
        t.praeambel = "# Teil 1: Anfang {-}\n\n".into();
        t.kapitel_hinzufuegen(Kapitel::neu("eins", "Eins", "t.md", "## Kapitel [eins]: Erstes\n\nText, siehe @kap:zwei.\n\n```rust\nlet x = 1 < 2;\n```\n"));
        t.kapitel_hinzufuegen(Kapitel::neu("zwei", "Zwei", "t.md", "## Kapitel [zwei]: Zweites\n\nEnde.\n"));
        p.teile.push(t);
        let ziel = o.join("buch.epub");
        epub_schreiben(&p, None, &ziel).unwrap();

        let mut zip = zip_lesen(&ziel);
        assert_eq!(zip.0[0], "mimetype");
        let k1 = zip.1("OEBPS/t01-k01.xhtml");
        assert!(k1.contains("<h2>Kapitel 1: Erstes</h2>"), "{k1}");
        assert!(k1.contains("href=\"t01-k02.xhtml\">Kapitel 2</a>"), "Verweis aufgelöst: {k1}");
        assert!(k1.contains("1 &lt; 2"), "Code maskiert");
    }

    /// Minimaler Zip-Leser über `epub-builder`s eigene Abhängigkeit ist nicht öffentlich; wir prüfen per `unzip`-freier Suche.
    fn zip_lesen(pfad: &Path) -> (Vec<String>, impl FnMut(&str) -> String) {
        let daten = std::fs::read(pfad).unwrap();
        let namen = {
            let mut n = Vec::new();
            let mut i = 0;
            while let Some(p) = daten[i..].windows(4).position(|w| w == [0x50, 0x4b, 0x03, 0x04]) {
                let kopf = i + p;
                let l = u16::from_le_bytes([daten[kopf + 26], daten[kopf + 27]]) as usize;
                n.push(String::from_utf8_lossy(&daten[kopf + 30..kopf + 30 + l]).to_string());
                i = kopf + 4;
            }
            n
        };
        let pfad = pfad.to_path_buf();
        (namen, move |name: &str| {
            // Inhalt über das Systemprogramm `unzip` lesen, falls vorhanden
            let aus = std::process::Command::new("unzip").arg("-p").arg(&pfad).arg(name).output().expect("unzip nötig für den Test");
            let mut s = String::new();
            aus.stdout.as_slice().read_to_string(&mut s).unwrap();
            s
        })
    }
}
