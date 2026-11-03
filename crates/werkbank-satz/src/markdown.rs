//! Übersetzung von Markdown in Typst-Markup.

use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::HashMap;
use std::path::Path;
use werkbank_kern::Projekt;

/// Wo die Seitenzahl steht.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Seitenzahl {
    /// Immer unten in der Mitte.
    Mitte,
    /// Unten außen, abwechselnd: gerade Seiten links, ungerade rechts (wie bei Romanen und Sachbüchern).
    #[default]
    Aussen,
}

/// Wie gezählt wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Zaehlung {
    /// Titelei (bis einschließlich Vorwort) in römischen Ziffern, der Haupttext beginnt mit 1.
    #[default]
    VorspannRoemisch,
    /// Durchgehend arabisch ab dem ersten Blatt (Schmutztitel = Seite 1).
    Durchgehend,
}

/// Einstellungen eines Satzlaufs.
#[derive(Debug, Clone, PartialEq)]
pub struct SatzOptionen {
    /// Seitenformat (Breite, Höhe) in Millimetern.
    pub format_mm: (f32, f32),
    /// Farbig (Codehervorhebung) oder rein Schwarzweiß (Druck).
    pub farbig: bool,
    /// Grundschriftgröße in Punkt.
    pub schriftgroesse: f32,
    /// Stellung der Seitenzahl.
    pub seitenzahl: Seitenzahl,
    /// Zählweise.
    pub zaehlung: Zaehlung,
    /// Teile (und Haupttext, Titelei-Abschnitte) beginnen auf einer rechten, ungeraden Seite;
    /// dafür entstehen Leerseiten ohne Seitenzahl. Im Druck üblich.
    pub teile_rechts: bool,
    /// Auch jedes Kapitel beginnt auf einer rechten Seite.
    pub kapitel_rechts: bool,
    /// Schmutztitel, Titelblatt, Impressum und Teiltitel zeigen ebenfalls eine Seitenzahl (sonst nicht, wie üblich).
    pub titelei_zahlen: bool,
    /// Jeder Teil bekommt eine eigene Titelseite (Zwischentitel) auf einer rechten Seite, dahinter eine Leerseite.
    pub teil_titelseiten: bool,
    /// Das Buch endet auf einer geraden Seite (Druckereien verlangen eine gerade Seitenzahl); nötigenfalls folgt eine Leerseite.
    pub gerade_seitenzahl: bool,
}

impl SatzOptionen {
    /// Schwarzweiß-Druck in einem Format: Zahlen außen, römische Titelei, Teile rechts.
    pub fn druck(format_mm: (f32, f32)) -> Self {
        Self {
            format_mm,
            farbig: false,
            schriftgroesse: 10.0,
            seitenzahl: Seitenzahl::Aussen,
            zaehlung: Zaehlung::VorspannRoemisch,
            teile_rechts: true,
            kapitel_rechts: false,
            titelei_zahlen: false,
            teil_titelseiten: true,
            gerade_seitenzahl: true,
        }
    }

    /// Farbiges Lese-PDF (B5) für den Bildschirm: keine Leerseiten.
    pub fn lesen() -> Self {
        Self {
            format_mm: (176.0, 250.0),
            farbig: true,
            schriftgroesse: 10.5,
            seitenzahl: Seitenzahl::Mitte,
            zaehlung: Zaehlung::VorspannRoemisch,
            teile_rechts: false,
            kapitel_rechts: false,
            titelei_zahlen: false,
            teil_titelseiten: false,
            gerade_seitenzahl: false,
        }
    }

    /// Teiltitelseiten ein- oder ausschalten.
    pub fn mit_teiltitelseiten(mut self, an: bool) -> Self {
        self.teil_titelseiten = an;
        self
    }

    /// Gerade Gesamtseitenzahl (Leerseite am Ende) ein- oder ausschalten.
    pub fn mit_gerader_seitenzahl(mut self, an: bool) -> Self {
        self.gerade_seitenzahl = an;
        self
    }

    /// Stellung der Seitenzahl wählen.
    pub fn mit_seitenzahl(mut self, s: Seitenzahl) -> Self {
        self.seitenzahl = s;
        self
    }

    /// Zählweise wählen.
    pub fn mit_zaehlung(mut self, z: Zaehlung) -> Self {
        self.zaehlung = z;
        self
    }

    /// Rechte Startseiten für Teile und Kapitel festlegen.
    pub fn mit_rechten_seiten(mut self, teile: bool, kapitel: bool) -> Self {
        self.teile_rechts = teile;
        self.kapitel_rechts = kapitel;
        self
    }

    /// Seitenzahlen auch auf Schmutztitel, Titelblatt und Impressum.
    pub fn mit_titelei_zahlen(mut self, an: bool) -> Self {
        self.titelei_zahlen = an;
        self
    }
}

/// Maskiert Text für Typst-Markup.
pub fn text_maskieren(s: &str) -> String {
    let mut aus = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if matches!(c, '\\' | '#' | '$' | '*' | '_' | '`' | '~' | '<' | '>' | '@' | '[' | ']' | '/' | '-' | '+' | '=' | '{' | '}') {
            aus.push('\\');
        }
        aus.push(c);
    }
    aus
}

/// Maskiert Text für ein Typst-Zeichenkettenliteral.
pub fn zeichenkette(s: &str) -> String {
    let mut aus = String::with_capacity(s.len() + 2);
    aus.push('"');
    for c in s.chars() {
        match c {
            '\\' => aus.push_str("\\\\"),
            '"' => aus.push_str("\\\""),
            '\r' => {}
            c => aus.push(c),
        }
    }
    aus.push('"');
    aus
}

/// Nummerierung der Kapitel: Kennung → „Kapitel N“, in Buchreihenfolge.
pub fn kapitelnummern(projekt: &Projekt) -> HashMap<String, usize> {
    let mut nr = 0;
    let mut karte = HashMap::new();
    for teil in projekt.teile.iter().filter(|t| ist_hauptteil(&t.titel)) {
        for k in teil.kapitel.iter().filter(|k| k.inhalt.starts_with("## ")) {
            nr += 1;
            karte.insert(k.id.clone(), nr);
        }
    }
    karte
}

/// Teile, die mit „Teil“ beginnen, enthalten nummerierte Kapitel; Vorwort und Anhänge nicht.
pub fn ist_hauptteil(titel: &str) -> bool {
    titel.trim_start().to_lowercase().starts_with("teil")
}

/// Nummern der Teile (`# Teil [id]: …`) in Buchreihenfolge, wie `teile-plan.txt` sie festlegt.
pub fn teil_nummern(projekt: &Projekt) -> HashMap<String, usize> {
    let mut karte = HashMap::new();
    for teil in projekt.teile.iter().filter(|t| ist_hauptteil(&t.titel)) {
        if let Some(h1) = werkbank_kern::markdown::ueberschriften(&teil.praeambel).into_iter().find(|u| u.ebene == 1)
            && let (Some(a), Some(b)) = (h1.text.find('['), h1.text.find(']'))
            && a < b
        {
            let n = karte.len() + 1;
            karte.insert(h1.text[a + 1..b].to_string(), n);
        }
    }
    karte
}

/// Wie die Bauskripte (`ersetze` in `build.py`): Überschriften `Kapitel [id]:`/`Teil [id]:` bekommen ihre
/// Nummer, `@kap:id` und `@teil:id` werden durch die bloße Nummer ersetzt (davor steht „Kapitel “/„Teil “).
pub fn produktionsersetzung(text: &str, kapitel: &HashMap<String, usize>, teile: &HashMap<String, usize>) -> String {
    let mut aus = String::with_capacity(text.len());
    let mut im_code = false;
    for zeile in text.split_inclusive('\n') {
        let kern = zeile.trim_end_matches(['\n', '\r']);
        let ende = &zeile[kern.len()..];
        if kern.trim_start().starts_with("```") || kern.trim_start().starts_with("~~~") {
            im_code = !im_code;
        }
        if im_code {
            aus.push_str(zeile);
            continue;
        }
        let nummeriert = |praefix: &str, wort: &str, karte: &HashMap<String, usize>| -> Option<String> {
            let rest = kern.strip_prefix(praefix)?;
            let (id, titel) = rest.strip_prefix('[')?.split_once("]: ")?;
            karte.get(id).map(|n| format!("{praefix}{n}: {titel}{ende}").replacen(wort, wort, 1))
        };
        if let Some(neu) = nummeriert("## Kapitel ", "Kapitel", kapitel).or_else(|| nummeriert("# Teil ", "Teil", teile)) {
            aus.push_str(&neu);
            continue;
        }
        let mut z = kern.to_string();
        for (marke, karte) in [("@kap:", kapitel), ("@teil:", teile)] {
            let mut neu = String::new();
            let mut rest = z.as_str();
            while let Some(p) = rest.find(marke) {
                neu.push_str(&rest[..p]);
                let nach = &rest[p + marke.len()..];
                let l = nach.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(nach.len());
                match karte.get(&nach[..l]) {
                    Some(n) => neu.push_str(&n.to_string()),
                    None => neu.push_str("??"),
                }
                rest = &nach[l..];
            }
            neu.push_str(rest);
            z = neu;
        }
        aus.push_str(&z);
        aus.push_str(ende);
    }
    aus
}

/// Ersetzt `@kap:id` durch „Kapitel N“ (so lösen es auch die Bauskripte auf).
pub fn verweise_aufloesen(text: &str, nummern: &HashMap<String, usize>) -> String {
    verweise_ersetzen(text, nummern)
}

fn verweise_ersetzen(text: &str, nummern: &HashMap<String, usize>) -> String {
    let mut aus = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(p) = rest.find("@kap:") {
        aus.push_str(&rest[..p]);
        let nach = &rest[p + 5..];
        let laenge = nach.find(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_')).unwrap_or(nach.len());
        let id = &nach[..laenge];
        // Steht schon „Kapitel “ davor, genügt die Nummer
        let schon = aus.ends_with("Kapitel ") || aus.ends_with("Kapiteln ");
        match nummern.get(id) {
            Some(n) if schon => aus.push_str(&n.to_string()),
            Some(n) => aus.push_str(&format!("Kapitel {n}")),
            None if schon => aus.push_str(&format!("„{id}“")),
            None => aus.push_str(&format!("Kapitel „{id}“")),
        }
        rest = &nach[laenge..];
    }
    aus.push_str(rest);
    aus
}

/// Übersetzt einen Markdown-Text in Typst-Markup.
///
/// `ebene_versatz`: Überschriftenebenen werden um diesen Wert verschoben (Teile liegen auf Ebene 1).
pub fn markdown_zu_typst(md: &str, wurzel: Option<&Path>, farbig: bool, nummern: &HashMap<String, usize>) -> String {
    let md = verweise_ersetzen(md, nummern);
    let mut opt = Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TABLES;
    opt.insert(Options::ENABLE_HEADING_ATTRIBUTES);
    let parser = Parser::new_ext(&md, opt);

    let mut aus = String::new();
    // Listenstapel: (geordnet, Einzug)
    let mut listen: Vec<bool> = Vec::new();
    let mut in_code: Option<(String, String)> = None; // (Sprache, Inhalt)
    let mut in_tabelle: Option<(usize, Vec<String>, String)> = None; // (Spalten, Zellen, aktuelle Zelle)
    let mut tabelle_kopf = false;
    let mut bild_alt: Option<(String, String)> = None; // (Pfad, Alt-Text)
    let mut zitat_tiefe = 0usize;

    let einzug = |t: usize| "  ".repeat(t);
    let anfuegen = |aus: &mut String, s: &str| {
        // Nach einem Aufruf direkt folgende „(“ oder „[“ würden als Argumente gelesen
        if s.starts_with('(') && aus.ends_with([']', ')', '}']) {
            aus.push('\\');
        }
        aus.push_str(s);
    };

    for ereignis in parser {
        if let Some((_, inhalt)) = in_code.as_mut() {
            match &ereignis {
                Event::Text(t) => {
                    inhalt.push_str(t);
                    continue;
                }
                Event::End(TagEnd::CodeBlock) => {}
                _ => continue,
            }
        }
        if let Some((_, alt)) = bild_alt.as_mut() {
            match &ereignis {
                Event::Text(t) | Event::Code(t) => {
                    alt.push_str(t);
                    continue;
                }
                Event::End(TagEnd::Image) => {}
                _ => continue,
            }
        }

        let ziel: &mut String = match in_tabelle.as_mut() {
            Some((_, _, zelle)) => zelle,
            None => &mut aus,
        };

        match ereignis {
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                if listen.is_empty() {
                    ziel.push_str("\n\n");
                } else {
                    ziel.push('\n');
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                let n = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                ziel.push_str(&format!("{} ", "=".repeat(n)));
            }
            Event::End(TagEnd::Heading(_)) => ziel.push_str("\n\n"),
            Event::Start(Tag::BlockQuote(_)) => {
                zitat_tiefe += 1;
                ziel.push_str("#block(inset: (left: 10pt), stroke: (left: 1.5pt + luma(170)))[\n");
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                zitat_tiefe = zitat_tiefe.saturating_sub(1);
                ziel.push_str("]\n\n");
            }
            Event::Start(Tag::CodeBlock(art)) => {
                let sprache = match art {
                    CodeBlockKind::Fenced(info) => info.split([' ', ',', '{']).next().unwrap_or("").trim_matches('.').to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                in_code = Some((sprache, String::new()));
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some((sprache, inhalt)) = in_code.take() {
                    let lang = if farbig && !sprache.is_empty() && sprache != "text" {
                        format!("lang: {}, ", zeichenkette(&sprache))
                    } else {
                        String::new()
                    };
                    let inhalt = inhalt.trim_end_matches('\n');
                    ziel.push_str(&format!("#raw(block: true, {lang}{})\n\n", zeichenkette(inhalt)));
                }
            }
            Event::Start(Tag::List(start)) => {
                listen.push(start.is_some());
                if listen.len() > 1 {
                    ziel.push('\n');
                }
            }
            Event::End(TagEnd::List(_)) => {
                listen.pop();
                if listen.is_empty() {
                    ziel.push('\n');
                }
            }
            Event::Start(Tag::Item) => {
                let geordnet = listen.last().copied().unwrap_or(false);
                ziel.push_str(&format!("{}{} ", einzug(listen.len().saturating_sub(1)), if geordnet { "+" } else { "-" }));
            }
            Event::End(TagEnd::Item) => {
                if !ziel.ends_with('\n') {
                    ziel.push('\n');
                }
            }
            Event::Start(Tag::Emphasis) => ziel.push_str("#emph["),
            Event::End(TagEnd::Emphasis) => ziel.push(']'),
            Event::Start(Tag::Strong) => ziel.push_str("#strong["),
            Event::End(TagEnd::Strong) => ziel.push(']'),
            Event::Start(Tag::Strikethrough) => ziel.push_str("#strike["),
            Event::End(TagEnd::Strikethrough) => ziel.push(']'),
            Event::Start(Tag::Link { dest_url, .. }) => {
                // Im Druck sind Links sinnlos, und PDF/X verbietet Link-Annotationen
                if farbig {
                    ziel.push_str(&format!("#link({})[", zeichenkette(&dest_url)));
                }
            }
            Event::End(TagEnd::Link) => {
                if farbig {
                    ziel.push(']');
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                bild_alt = Some((dest_url.to_string(), String::new()));
            }
            Event::End(TagEnd::Image) => {
                if let Some((pfad, alt)) = bild_alt.take() {
                    let vorhanden = wurzel.is_some_and(|w| w.join(pfad.trim_start_matches('/')).is_file());
                    if vorhanden {
                        ziel.push_str(&format!(
                            "#figure(image({}, width: 85%), caption: [{}])\n\n",
                            zeichenkette(&format!("/{}", pfad.trim_start_matches('/'))),
                            text_maskieren(&alt)
                        ));
                    } else {
                        ziel.push_str(&format!("#emph[\\[Bild: {}\\]]\n\n", text_maskieren(&alt)));
                    }
                }
            }
            Event::Start(Tag::Table(spalten)) => {
                in_tabelle = Some((spalten.len(), Vec::new(), String::new()));
            }
            Event::Start(Tag::TableHead) => tabelle_kopf = true,
            Event::End(TagEnd::TableHead) => tabelle_kopf = false,
            Event::Start(Tag::TableRow) => {}
            Event::End(TagEnd::TableRow) => {}
            Event::Start(Tag::TableCell) => {}
            Event::End(TagEnd::TableCell) => {
                if let Some((_, zellen, zelle)) = in_tabelle.as_mut() {
                    let inhalt = std::mem::take(zelle);
                    zellen.push(if tabelle_kopf { format!("[#strong[{}]]", inhalt.trim()) } else { format!("[{}]", inhalt.trim()) });
                }
            }
            Event::End(TagEnd::Table) => {
                if let Some((spalten, zellen, _)) = in_tabelle.take() {
                    aus.push_str(&format!(
                        "#table(columns: {spalten}, inset: 5pt, stroke: 0.4pt + luma(120), {})\n\n",
                        zellen.join(", ")
                    ));
                }
            }
            Event::Text(t) => anfuegen(ziel, &text_maskieren(&t)),
            Event::Code(c) => ziel.push_str(&format!("#{{raw({})}}", zeichenkette(&c))),
            Event::SoftBreak => ziel.push(' '),
            Event::HardBreak => ziel.push_str(" \\ "),
            Event::Rule => ziel.push_str("#line(length: 100%, stroke: 0.5pt + luma(150))\n\n"),
            Event::Html(_) | Event::InlineHtml(_) => {}
            Event::FootnoteReference(_) | Event::TaskListMarker(_) => {}
            _ => {}
        }
        let _ = zitat_tiefe;
    }
    aus
}

/// Vorlage des Dokumentkopfes; `__NAME__` wird ersetzt, damit Typst-Klammern lesbar bleiben.
const KOPF: &str = r##"#set document(title: __TITEL__, author: __AUTOR__)
#set page(
  width: __B__mm, height: __H__mm,
  margin: (inside: 22mm, outside: 16mm, top: 20mm, bottom: 24mm),
  numbering: "__ZIFFERN__",
  header: none,
  footer-descent: 35%,
  footer: context {
    // Leerseiten (Vakat) liegen zwischen einer <vor>- und der nächsten <nach>-Marke und bleiben ohne Zahl.
    let p = here().page()
    let leer = query(<vor>).any(v => {
      let spaeter = query(selector(<nach>).after(v.location()))
      let q = if spaeter.len() > 0 { spaeter.first().location().page() } else { 0 }
      p > v.location().page() and p < q
    })
    let endleer = query(<endleer>).any(m => m.location().page() == p)
    let titelseite = __TITELSEITEN_OHNE_ZAHL__ and query(<titelseite>).any(m => m.location().page() == p)
    if not leer and not endleer and not titelseite {
      let nr = text(size: 9pt, counter(page).display())
      __FUSS__
    }
  },
)
#set text(font: ("Libertinus Serif", "DejaVu Serif"), size: __GR__pt, lang: "de")
#set par(justify: true, leading: 0.62em, spacing: 0.8em)
#show raw: set text(font: ("DejaVu Sans Mono",), size: 0.78em)
#show raw.where(block: true): it => block(width: 100%, fill: luma(242), inset: 6pt, radius: 2pt, breakable: true, it)
#set heading(numbering: none)
#show link: set text(fill: __LINK__)

// Beginn auf einer rechten Seite: Leerseiten entstehen nur bei Bedarf und tragen keine Seitenzahl.
#let rechts() = {
  [#metadata("v") <vor>]
  pagebreak(weak: true, to: "odd")
  [#metadata("n") <nach>]
}
#let neue-seite() = pagebreak(weak: true)
#let teil-start() = __TEILSTART__
#let kapitel-start() = __KAPITELSTART__

#show heading.where(level: 1): it => { teil-start(); v(30mm); align(center, text(size: 24pt, weight: "bold", it.body)); v(12mm) }
// Teile (Label <teil>): auf Wunsch mit eigener Titelseite und leerer Rückseite
#show <teil>: it => {
  if __ZWISCHENTITEL__ {
    rechts()
    v(1fr)
    [#metadata("t") <titelseite>]
    align(center, text(size: 26pt, weight: "bold", it.body))
    v(1fr)
    rechts()
  } else {
    teil-start(); v(30mm); align(center, text(size: 24pt, weight: "bold", it.body)); v(12mm)
  }
}
#show heading.where(level: 2): it => { kapitel-start(); v(8mm); text(size: 17pt, weight: "bold", it.body); v(5mm) }
#show heading.where(level: 3): it => { v(4mm); text(size: 12.5pt, weight: "bold", it.body); v(2mm) }
#show heading.where(level: 4): it => { v(2mm); text(size: 11pt, weight: "bold", style: "italic", it.body); v(1mm) }

"##;

/// Ersetzt in der ersten `# `-Zeile eine Kennung `Teil [id]: Titel` durch `Teil N: Titel`.
pub fn teil_ueberschrift_nummerieren(praeambel: &str, nr: usize) -> String {
    let Some(h1) = werkbank_kern::markdown::ueberschriften(praeambel).into_iter().find(|u| u.ebene == 1) else {
        return praeambel.to_string();
    };
    let ende = praeambel[h1.byte..].find('\n').map_or(praeambel.len(), |i| h1.byte + i);
    let zeile = &praeambel[h1.byte..ende];
    let (Some(a), Some(b)) = (zeile.find('['), zeile.find("]:")) else {
        return praeambel.to_string();
    };
    if a > b {
        return praeambel.to_string();
    }
    let neu = format!("{} {nr}{}", zeile[..a].trim_end(), &zeile[b + 1..]);
    format!("{}{}{}", &praeambel[..h1.byte], neu, &praeambel[ende..])
}

/// Hängt der ersten Überschrift der Ebene 1 das Label `<teil>` an (für die Titelseite eines Teils).
fn teil_label_setzen(typst: String) -> String {
    let start = if typst.starts_with("= ") { Some(0) } else { typst.find("\n= ").map(|i| i + 1) };
    let Some(start) = start else { return typst };
    let ende = typst[start..].find('\n').map_or(typst.len(), |i| start + i);
    format!("{} <teil>{}", &typst[..ende], &typst[ende..])
}

/// Erzeugt das vollständige Typst-Dokument des Buches.
///
/// Aufbau: Schmutztitel (nur Druck), Titelblatt, Impressum, Inhaltsverzeichnis, Vorspann
/// (Teile vor dem ersten „Teil“), dann der Haupttext auf einer rechten Seite. Die Zählung folgt
/// [`Zaehlung`]; Seitenzahlen stehen auf jeder Textseite, nicht auf Leerseiten.
pub fn buch_zu_typst(projekt: &Projekt, opt: &SatzOptionen) -> String {
    buch_zu_typst_mit_ende(projekt, opt, false)
}

/// Wie [`buch_zu_typst`]; `endleerseite` hängt eine Leerseite ohne Zahl an (für eine gerade Gesamtseitenzahl).
pub fn buch_zu_typst_mit_ende(projekt: &Projekt, opt: &SatzOptionen, endleerseite: bool) -> String {
    let m = &projekt.metadaten;
    let nummern = kapitelnummern(projekt);
    let wurzel = Some(projekt.wurzelverzeichnis.as_path());
    let (b, h) = opt.format_mm;

    let fuss = match opt.seitenzahl {
        Seitenzahl::Mitte => "align(center, nr)",
        Seitenzahl::Aussen => "if calc.odd(p) { align(right, nr) } else { align(left, nr) }",
    };
    let ziffern = match opt.zaehlung {
        Zaehlung::VorspannRoemisch => "i",
        Zaehlung::Durchgehend => "1",
    };
    let rechts_oder_neu = |rechts: bool| if rechts { "rechts()" } else { "neue-seite()" };

    let mut t = KOPF
        .replace("__TITEL__", &zeichenkette(&m.titel))
        .replace("__AUTOR__", &zeichenkette(&m.autor))
        .replace("__B__", &b.to_string())
        .replace("__H__", &h.to_string())
        .replace("__ZIFFERN__", ziffern)
        .replace("__FUSS__", fuss)
        .replace("__GR__", &opt.schriftgroesse.to_string())
        .replace("__LINK__", if opt.farbig { "rgb(\"#00875a\")" } else { "black" })
        .replace("__TITELSEITEN_OHNE_ZAHL__", if opt.titelei_zahlen { "false" } else { "true" })
        .replace("__ZWISCHENTITEL__", if opt.teil_titelseiten && opt.teile_rechts { "true" } else { "false" })
        .replace("__TEILSTART__", rechts_oder_neu(opt.teile_rechts))
        .replace("__KAPITELSTART__", rechts_oder_neu(opt.kapitel_rechts && opt.teile_rechts));

    // Titelei: auf Wunsch ohne Seitenzahl (üblich)
    let fuss_titelei = if opt.titelei_zahlen { "" } else { "footer: none, " };
    let marke = if opt.teile_rechts { "#[#metadata(\"v\") <vor>]" } else { "" };

    if opt.teile_rechts {
        // Schmutztitel auf Seite 1, dahinter eine Leerseite
        t.push_str(&format!(
            "#page({fuss_titelei}header: none)[\n  #align(center + horizon)[#text(size: 22pt, weight: \"bold\")[{}]]\n  {marke}\n]\n#rechts()\n",
            text_maskieren(&m.titel)
        ));
    }
    let rechts_befehl = format!("#{}", rechts_oder_neu(opt.teile_rechts));
    let isbn_text = m.isbn_print.as_deref().map(|i| format!("ISBN {}", text_maskieren(i))).unwrap_or_default();
    t.push_str(&format!(
        r#"#page({fuss_titelei}header: none)[
  #align(center + horizon)[
    #text(size: 30pt, weight: "bold")[{titel}]
    #v(8mm)
    #text(size: 15pt)[{untertitel}]
    #v(14mm)
    #text(size: 12pt)[{band}]
    #v(30mm)
    #text(size: 14pt)[{autor}]
  ]
]
#page({fuss_titelei}header: none)[
  #v(1fr)
  #text(size: 8.5pt)[
    {titel} · {untertitel} \
    © {jahr} {autor} \
    {isbn}
  ]
  {marke}
]
{rechts_vor_inhalt}
#outline(title: [Inhalt], depth: 2, indent: 1em)
{rechts_nach_inhalt}
"#,
        titel = text_maskieren(&m.titel),
        untertitel = text_maskieren(&m.untertitel),
        band = text_maskieren(&m.band_bezeichnung),
        autor = text_maskieren(&m.autor),
        jahr = text_maskieren(&m.jahr),
        isbn = isbn_text,
        rechts_vor_inhalt = rechts_befehl,
        rechts_nach_inhalt = rechts_befehl,
    ));

    // Der Haupttext beginnt beim ersten „Teil“; gibt es keinen, gleich nach dem Inhaltsverzeichnis.
    let hat_hauptteil = projekt.teile.iter().any(|t| ist_hauptteil(&t.titel));
    let mut haupt_begonnen = false;
    let mut teil_nr = 0;

    for teil in &projekt.teile {
        if !haupt_begonnen && (ist_hauptteil(&teil.titel) || !hat_hauptteil) {
            haupt_begonnen = true;
            t.push('#');
            t.push_str(rechts_oder_neu(opt.teile_rechts));
            t.push('\n');
            if opt.zaehlung == Zaehlung::VorspannRoemisch {
                // Neue Zählung: arabisch ab 1
                t.push_str("#set page(numbering: \"1\")\n#counter(page).update(1)\n");
            }
        }
        // Präambel: enthält die Teilüberschrift (`# …`) und Einleitungstext.
        // „# Teil [id]: Titel“ wird zu „# Teil N: Titel“ (Zählung der Hauptteile).
        let praeambel = if ist_hauptteil(&teil.titel) {
            teil_nr += 1;
            teil_ueberschrift_nummerieren(&teil.praeambel, teil_nr)
        } else {
            teil.praeambel.clone()
        };
        let mut typ = markdown_zu_typst(&praeambel, wurzel, opt.farbig, &nummern);
        if ist_hauptteil(&teil.titel) {
            typ = teil_label_setzen(typ);
        }
        t.push_str(&typ);
        for k in &teil.kapitel {
            let inhalt = match nummern.get(&k.id) {
                // Aus „## Kapitel [id]: Titel“ wird „## Kapitel N: Titel“
                Some(n) if k.inhalt.starts_with("## ") => {
                    let (kopf, rest) = k.inhalt.split_once('\n').unwrap_or((&k.inhalt, ""));
                    let titel = match kopf.find("]:") {
                        Some(i) => kopf[i + 2..].trim().to_string(),
                        None => kopf.trim_start_matches("## ").trim().to_string(),
                    };
                    format!("## Kapitel {n}: {titel}\n{rest}")
                }
                _ => k.inhalt.clone(),
            };
            t.push_str(&markdown_zu_typst(&inhalt, wurzel, opt.farbig, &nummern));
        }
    }
    if endleerseite {
        // Leerseite ohne Zahl am Ende; ob sie nötig ist, entscheidet `pdf_setzen` nach dem ersten Satzlauf
        t.push_str("\n#pagebreak()\n#[#metadata(\"e\") <endleer>]\n");
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use werkbank_kern::{Kapitel, Teil};

    fn num() -> HashMap<String, usize> {
        HashMap::from([("ziel".to_string(), 7)])
    }

    #[test]
    fn teilueberschrift_wird_nummeriert() {
        assert_eq!(
            teil_ueberschrift_nummerieren("# Teil [iteratoren]: Iteratoren {-}\n\nText\n", 4),
            "# Teil 4: Iteratoren {-}\n\nText\n"
        );
        assert_eq!(teil_ueberschrift_nummerieren("# Anhang\n", 1), "# Anhang\n");
    }

    #[test]
    fn teil_label() {
        assert_eq!(teil_label_setzen("= Teil 1: A\n\nText\n".into()), "= Teil 1: A <teil>\n\nText\n");
        assert_eq!(teil_label_setzen("Vorab\n= Teil 2\n".into()), "Vorab\n= Teil 2 <teil>\n");
        assert_eq!(teil_label_setzen("== Kapitel\n".into()), "== Kapitel\n");
    }

    #[test]
    fn maskierung() {
        assert_eq!(text_maskieren("a_b #x [1] -- $5"), "a\\_b \\#x \\[1\\] \\-\\- \\$5");
        assert_eq!(zeichenkette("say \"hi\"\\n"), "\"say \\\"hi\\\"\\\\n\"");
    }

    #[test]
    fn produktionsersetzung_wie_die_skripte() {
        let kap = HashMap::from([("ownership1".to_string(), 12usize)]);
        let teile = HashMap::from([("besitz".to_string(), 3usize)]);
        let t = produktionsersetzung(
            "## Kapitel [ownership1]: Besitz\n\nSiehe Kapitel @kap:ownership1 (Teil @teil:besitz), nicht @kap:fehlt.\n\n```\n@kap:ownership1\n```\n# Teil [besitz]: Besitz\n",
            &kap,
            &teile,
        );
        assert_eq!(
            t,
            "## Kapitel 12: Besitz\n\nSiehe Kapitel 12 (Teil 3), nicht ??.\n\n```\n@kap:ownership1\n```\n# Teil 3: Besitz\n"
        );
    }

    #[test]
    fn verweis_nach_kapitel_wird_nicht_doppelt() {
        let t = markdown_zu_typst("Siehe Kapitel @kap:ziel und @kap:ziel.", None, true, &num());
        assert_eq!(t, "Siehe Kapitel 7 und Kapitel 7.\n\n");
    }

    #[test]
    fn absatz_hervorhebung_code_und_verweis() {
        let t = markdown_zu_typst("Ein *Wort* und **fett** mit `Vec<u8>` siehe @kap:ziel.\n", None, true, &num());
        assert_eq!(t, "Ein #emph[Wort] und #strong[fett] mit #{raw(\"Vec<u8>\")} siehe Kapitel 7.\n\n");
    }

    #[test]
    fn ueberschriften_listen_codeblock() {
        let md = "## Titel {-}\n\n- eins\n- zwei\n  - innen\n\n1. a\n2. b\n\n```rust\nfn main() {}\n```\n";
        let t = markdown_zu_typst(md, None, true, &num());
        assert!(t.starts_with("== Titel\n\n"), "{t}");
        assert!(t.contains("- eins\n- zwei\n  - innen\n"), "{t}");
        assert!(t.contains("+ a\n+ b\n"), "{t}");
        assert!(t.contains("#raw(block: true, lang: \"rust\", \"fn main() {}\")"));
        let sw = markdown_zu_typst(md, None, false, &num());
        assert!(sw.contains("#raw(block: true, \"fn main() {}\")"), "Druck ohne Hervorhebung");
    }

    #[test]
    fn tabelle_zitat_link_bild() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n\n> Zitat\n\n[Text](https://x.de) ![Alt](fehlt.png)\n";
        let t = markdown_zu_typst(md, None, true, &num());
        assert!(t.contains("#table(columns: 2"), "{t}");
        assert!(t.contains("[#strong[A]]"));
        assert!(t.contains("#block(inset: (left: 10pt)"));
        assert!(t.contains("#link(\"https://x.de\")[Text]"));
        assert!(t.contains("\\[Bild: Alt\\]"));
    }

    #[test]
    fn kapitelnummern_zaehlen_nur_hauptteile() {
        let mut p = Projekt::neu("/tmp/x");
        let mut vorwort = Teil::neu("v", "Vorwort");
        vorwort.kapitel_hinzufuegen(Kapitel::neu("vor", "Vorwort", "v.md", "## Vorwort\n"));
        let mut t1 = Teil::neu("t1", "Teil 1: Anfang");
        t1.kapitel_hinzufuegen(Kapitel::neu("a", "A", "t.md", "## Kapitel [a]: A\n"));
        t1.kapitel_hinzufuegen(Kapitel::neu("b", "B", "t.md", "## Kapitel [b]: B\n"));
        p.teile = vec![vorwort, t1];
        let n = kapitelnummern(&p);
        assert_eq!((n.get("a"), n.get("b"), n.get("vor")), (Some(&1), Some(&2), None));
    }
}
