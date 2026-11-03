//! # Live-Vorschau für Markdown-Inhalte
//!
//! Stellt eine gerenderte Vorschau des aktuellen Kapitels bereit.
//! Bietet synchrone visuelle Verknüpfung mit dem Editor:
//! 1. Der Abschnitt, in dem sich der Cursor aktuell befindet, wird dezent hervorgehoben.
//! 2. Ein Klick auf ein beliebiges Wort oder einen Block in der Vorschau
//!    springt im Editor unmittelbar an die entsprechende Quelltextzeile.

use crate::thema;
use iced::widget::{button, column, container, image, row, scrollable, space, text};
use iced::{Alignment, Element, Fill, FillPortion, Padding, Pixels};
use std::path::Path;

/// Ein geparster Textblock aus dem Markdown-Quelltext mit Zeileninformation.
///
/// Die Regeln folgen dem Buchbau (`build_book.py`): Bilder stehen als eigene Zeile
/// `![Unterschrift](images/datei.png)`, HTML-Hüllen wie `<div class="imprint">` sind nur
/// Satzanweisungen, `<div class="page-break"></div>` ist ein Seitenumbruch.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownBlock {
    /// Überschrift mit Ebene (1 bis 6), Text und Quellzeile.
    Ueberschrift { ebene: usize, text: String, zeile: usize },
    /// Codeblock mit Sprache, Codezeilen und Quellzeilenbereich.
    CodeBlock { sprache: String, code: Vec<String>, start_zeile: usize, end_zeile: usize },
    /// Zitat oder Hinweisblock (`> ...`).
    Zitat { zeilen: Vec<String>, start_zeile: usize, end_zeile: usize },
    /// Normaler Fließtext-Absatz mit Quellzeilenbereich.
    Absatz { text: String, start_zeile: usize, end_zeile: usize },
    /// Listenpunkt: Marke („•“ oder „3.“), Einzugsebene, Text und Quellzeile.
    ListenPunkt { marke: String, ebene: usize, text: String, zeile: usize },
    /// Eigenständiges Bild mit Pfad (wie im Text) und Bildunterschrift.
    Bild { pfad: String, unterschrift: String, zeile: usize },
    /// Tabelle: Kopfzeile und Datenzeilen (Trennzeile `|---|` entfällt).
    Tabelle { zeilen: Vec<Vec<String>>, start_zeile: usize, end_zeile: usize },
    /// Seitenumbruch (`<div class="page-break"></div>`).
    Seitenumbruch { zeile: usize },
}

/// Ebene einer Überschrift (`#` bis `######` mit Leerzeichen danach).
fn ueberschrift(z: &str) -> Option<(usize, &str)> {
    let n = z.chars().take_while(|c| *c == '#').count();
    ((1..=6).contains(&n) && z[n..].starts_with(' ')).then(|| (n, z[n..].trim()))
}

/// Listenpunkt: (Marke, Text) für `- `, `* `, `+ ` oder `12. `.
fn listenpunkt(z: &str) -> Option<(String, &str)> {
    for p in ["- ", "* ", "+ "] {
        if let Some(rest) = z.strip_prefix(p) {
            return Some(("•".to_string(), rest));
        }
    }
    let ziffern = z.chars().take_while(char::is_ascii_digit).count();
    (ziffern > 0 && ziffern <= 3 && z[ziffern..].starts_with(". ")).then(|| (z[..ziffern + 1].to_string(), &z[ziffern + 2..]))
}

/// Eigenständige Bildzeile `![Unterschrift](pfad)`.
fn bildzeile(z: &str) -> Option<(String, String)> {
    let rest = z.strip_prefix("![")?;
    let (alt, rest) = rest.split_once("](")?;
    let pfad = rest.strip_suffix(')')?;
    // Titel in Anführungszeichen (`bild.png "Titel"`) gehört nicht zum Pfad
    let pfad = pfad.split(" \"").next().unwrap_or(pfad).trim();
    (!pfad.is_empty()).then(|| (alt.trim().to_string(), pfad.to_string()))
}

/// Reine HTML-Zeile (Hülle, Kommentar), die im Buch nur den Satz steuert.
fn html_zeile(z: &str) -> bool {
    (z.starts_with('<') && z.ends_with('>') && z[1..].chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '/' || c == '!'))
        || (z.starts_with("<!--") && z.ends_with("-->"))
}

fn ist_seitenumbruch(z: &str) -> bool {
    z.contains("page-break") && z.starts_with('<')
}

fn ist_tabellenzeile(z: &str) -> bool {
    z.starts_with('|') && z.ends_with('|') && z.len() > 1
}

fn trennlinie(z: &str) -> bool {
    let nur = |c: char| z.len() >= 3 && z.chars().all(|x| x == c || x == ' ');
    nur('-') || nur('*') || nur('_')
}

/// Beginnt mit dieser Zeile ein neuer Block (dann endet ein laufender Absatz)?
fn blockanfang(z: &str) -> bool {
    z.is_empty()
        || z.starts_with("```")
        || z.starts_with("~~~")
        || z.starts_with('>')
        || z.starts_with(":::")
        || ueberschrift(z).is_some()
        || listenpunkt(z).is_some()
        || bildzeile(z).is_some()
        || html_zeile(z)
        || ist_tabellenzeile(z)
        || trennlinie(z)
}

/// Parst den Markdown-Text zeilenweise in Blöcke mit Quellzeilen-Zuordnung.
pub fn parse_bloecke(inhalt: &str) -> Vec<MarkdownBlock> {
    let mut bloecke = Vec::new();
    let zeilen: Vec<&str> = inhalt.lines().collect();
    let mut i = 0;

    while i < zeilen.len() {
        let zeile = zeilen[i];
        let getrimmt = zeile.trim();

        if getrimmt.is_empty() || trennlinie(getrimmt) || getrimmt.starts_with(":::") {
            i += 1;
            continue;
        }

        // Codeblöcke (``` oder ~~~, mit Sprache)
        if getrimmt.starts_with("```") || getrimmt.starts_with("~~~") {
            let zaun = &getrimmt[..3];
            let sprache = getrimmt[3..].split(|c: char| c.is_whitespace() || c == ',' || c == '{').next().unwrap_or("").to_string();
            let start_zeile = i;
            i += 1;
            let mut code = Vec::new();
            while i < zeilen.len() && !zeilen[i].trim().starts_with(zaun) {
                code.push(zeilen[i].to_string());
                i += 1;
            }
            let end_zeile = i;
            i = (i + 1).min(zeilen.len().max(i));
            bloecke.push(MarkdownBlock::CodeBlock { sprache, code, start_zeile, end_zeile });
            continue;
        }

        // HTML-Hüllen sind Satzanweisungen; nur der Seitenumbruch wird sichtbar
        if html_zeile(getrimmt) {
            if ist_seitenumbruch(getrimmt) {
                bloecke.push(MarkdownBlock::Seitenumbruch { zeile: i });
            }
            i += 1;
            continue;
        }

        if let Some((ebene, rest)) = ueberschrift(getrimmt) {
            bloecke.push(MarkdownBlock::Ueberschrift { ebene, text: rest.to_string(), zeile: i });
            i += 1;
            continue;
        }

        if let Some((unterschrift, pfad)) = bildzeile(getrimmt) {
            bloecke.push(MarkdownBlock::Bild { pfad, unterschrift, zeile: i });
            i += 1;
            continue;
        }

        if ist_tabellenzeile(getrimmt) {
            let start_zeile = i;
            let mut tabelle = Vec::new();
            while i < zeilen.len() && ist_tabellenzeile(zeilen[i].trim()) {
                let z = zeilen[i].trim();
                let zellen: Vec<String> = z[1..z.len() - 1].split('|').map(|c| c.trim().to_string()).collect();
                let ist_trenner = zellen.iter().all(|c| !c.is_empty() && c.chars().all(|x| x == '-' || x == ':'));
                if !ist_trenner {
                    tabelle.push(zellen);
                }
                i += 1;
            }
            bloecke.push(MarkdownBlock::Tabelle { zeilen: tabelle, start_zeile, end_zeile: i.saturating_sub(1) });
            continue;
        }

        if getrimmt.starts_with('>') {
            let start_zeile = i;
            let mut zitat = Vec::new();
            while i < zeilen.len() && zeilen[i].trim().starts_with('>') {
                zitat.push(zeilen[i].trim().trim_start_matches('>').trim().to_string());
                i += 1;
            }
            bloecke.push(MarkdownBlock::Zitat { zeilen: zitat, start_zeile, end_zeile: i.saturating_sub(1) });
            continue;
        }

        if let Some((marke, rest)) = listenpunkt(getrimmt) {
            let einzug = zeile.len() - zeile.trim_start().len();
            bloecke.push(MarkdownBlock::ListenPunkt { marke, ebene: einzug / 2, text: rest.to_string(), zeile: i });
            i += 1;
            continue;
        }

        // Fließtext-Absatz bis zur nächsten Leerzeile oder zum nächsten Block;
        // zwei Leerzeichen am Zeilenende erzwingen einen Zeilenumbruch (wie im Buchbau)
        let start_zeile = i;
        let mut text = String::new();
        while i < zeilen.len() && (i == start_zeile || !blockanfang(zeilen[i].trim())) {
            let z = zeilen[i];
            if !text.is_empty() && !text.ends_with('\n') {
                text.push(' ');
            }
            text.push_str(z.trim());
            if z.ends_with("  ") {
                text.push('\n');
            }
            i += 1;
        }
        bloecke.push(MarkdownBlock::Absatz { text: text.trim_end().to_string(), start_zeile, end_zeile: i.saturating_sub(1) });
    }

    bloecke
}

/// Zerlegt Inline-Markdown (`**fett**`, `*kursiv*`, `` `code` ``, `~~durch~~`, Links, Bilder,
/// Maskierungen) in formatierte Textstücke; die Steuerzeichen selbst erscheinen nicht.
pub fn inline_spans(quelle: &str, groesse: f32, farbe: iced::Color) -> Vec<iced::widget::text::Span<'static, (), iced::Font>> {
    let zeichen: Vec<char> = quelle.chars().collect();
    let mut aus = Vec::new();
    inline_lesen(&zeichen, false, false, groesse, farbe, &mut aus);
    aus
}

fn inline_stueck(
    text: &str,
    fett: bool,
    kursiv: bool,
    groesse: f32,
    farbe: iced::Color,
) -> iced::widget::text::Span<'static, (), iced::Font> {
    let mut font = thema::editor_font(fett);
    if kursiv {
        font.style = iced::font::Style::Italic;
    }
    iced::widget::span(text.to_string()).font(font).size(groesse).color(farbe)
}

fn inline_lesen(
    z: &[char],
    fett: bool,
    kursiv: bool,
    groesse: f32,
    farbe: iced::Color,
    aus: &mut Vec<iced::widget::text::Span<'static, (), iced::Font>>,
) {
    let mut puffer = String::new();
    let mut i = 0;
    let leeren = |puffer: &mut String, aus: &mut Vec<_>| {
        if !puffer.is_empty() {
            aus.push(inline_stueck(puffer, fett, kursiv, groesse, farbe));
            puffer.clear();
        }
    };
    // Position des schließenden Zeichens (einfach oder doppelt) ab `von`
    let finde = |von: usize, muster: &[char]| -> Option<usize> {
        (von..z.len().saturating_sub(muster.len() - 1)).find(|&k| z[k..k + muster.len()] == *muster)
    };
    while i < z.len() {
        let c = z[i];
        let naechst = z.get(i + 1).copied();
        match c {
            '\\' if naechst.is_some_and(|n| "\\`*_{}[]()#+-.!~>|".contains(n)) => {
                puffer.push(naechst.unwrap());
                i += 2;
            }
            '`' => {
                if let Some(ende) = finde(i + 1, &['`']) {
                    leeren(&mut puffer, aus);
                    let code: String = z[i + 1..ende].iter().collect();
                    aus.push(
                        iced::widget::span(code)
                            .font(iced::Font::MONOSPACE)
                            .size(groesse * 0.92)
                            .color(thema::text_hell())
                            .background(iced::Background::Color(thema::code_hintergrund()))
                            .padding(Padding::from([0, 3])),
                    );
                    i = ende + 1;
                } else {
                    puffer.push(c);
                    i += 1;
                }
            }
            '*' | '_' | '~' => {
                let dreifach = c != '~' && naechst == Some(c) && z.get(i + 2) == Some(&c);
                let doppelt = naechst == Some(c);
                let marke: Vec<char> = if dreifach { vec![c; 3] } else if doppelt { vec![c, c] } else { vec![c] };
                // `_` mitten im Wort und `~` einzeln sind keine Auszeichnung
                let vorher_wort = i > 0 && z[i - 1].is_alphanumeric();
                let beginnt = i + marke.len() < z.len() && !z[i + marke.len()].is_whitespace();
                let gueltig = beginnt && !(c == '_' && vorher_wort) && !(c == '~' && !doppelt);
                match gueltig.then(|| finde(i + marke.len(), &marke)).flatten() {
                    Some(ende) if ende > i + marke.len() => {
                        leeren(&mut puffer, aus);
                        let innen = &z[i + marke.len()..ende];
                        if c == '~' {
                            let t: String = innen.iter().collect();
                            aus.push(inline_stueck(&t, fett, kursiv, groesse, thema::text_gedimmt()).strikethrough(true));
                        } else {
                            inline_lesen(innen, fett || doppelt, kursiv || !doppelt || dreifach, groesse, farbe, aus);
                        }
                        i = ende + marke.len();
                    }
                    _ => {
                        puffer.push(c);
                        i += 1;
                    }
                }
            }
            '!' | '[' => {
                let bild = c == '!' && naechst == Some('[');
                let start = if bild { i + 1 } else { i };
                let schluss = (c == '[' || bild).then(|| finde(start + 1, &[']'])).flatten();
                match schluss {
                    Some(e) if z.get(e + 1) == Some(&'(') => match finde(e + 2, &[')']) {
                        Some(klammer) => {
                            leeren(&mut puffer, aus);
                            let beschriftung = &z[start + 1..e];
                            if bild {
                                let alt: String = beschriftung.iter().collect();
                                aus.push(inline_stueck(&format!("[Bild: {alt}]"), false, true, groesse, thema::text_gedimmt()));
                            } else {
                                let mut innen = Vec::new();
                                inline_lesen(beschriftung, fett, kursiv, groesse, thema::akzent(), &mut innen);
                                aus.extend(innen.into_iter().map(|s| s.underline(true)));
                            }
                            i = klammer + 1;
                        }
                        None => {
                            puffer.push(c);
                            i += 1;
                        }
                    },
                    _ => {
                        puffer.push(c);
                        i += 1;
                    }
                }
            }
            '<' if naechst.is_some_and(|n| n.is_ascii_alphabetic() || n == '/') && finde(i, &['>']).is_some() => {
                let ende = finde(i, &['>']).unwrap_or(i);
                let tag: String = z[i + 1..ende].iter().collect::<String>().to_lowercase();
                if tag.starts_with("br") {
                    puffer.push('\n');
                }
                i = ende + 1;
            }
            '{' if z[i..].iter().take_while(|&&k| k != '}').count() > 1 && z[i + 1..].iter().take_while(|&&k| k != '}').next() == Some(&'#') => {
                // Pandoc-Attribut wie `{#kap:x}` gehört nicht zum Text
                let ende = finde(i, &['}']).unwrap_or(z.len() - 1);
                i = ende + 1;
            }
            _ => {
                puffer.push(c);
                i += 1;
            }
        }
    }
    leeren(&mut puffer, aus);
}

/// Setzt Inline-Markdown als fertiges Textelement.
fn markdown_text<'a, Message: Clone + 'a>(quelle: &str, groesse: f32, farbe: iced::Color) -> Element<'a, Message> {
    let spans = inline_spans(quelle, groesse, farbe);
    iced::widget::rich_text(spans)
        .size(groesse)
        .line_height(iced::widget::text::LineHeight::Relative(1.5))
        .into()
}

/// Kennung der scrollbaren Text-Vorschau (damit sie dem Cursor folgen kann).
pub fn scroll_id() -> iced::widget::Id {
    iced::widget::Id::new("text-vorschau")
}

/// Erzeugt die Live-Vorschau-Spalte.
///
/// `basis` ist der Bandordner; Bildpfade wie `images/x.png` gelten relativ dazu.
pub fn ansicht<'a, Message: Clone + 'a>(
    inhalt: &'a str,
    cursor_zeile: usize,
    on_springe_zu_zeile: impl Fn(usize, usize) -> Message + Copy + 'a,
    basis: Option<&'a Path>,
) -> Element<'a, Message> {
    darstellen(inhalt, cursor_zeile, on_springe_zu_zeile, true, basis)
}

/// Gesetzter Markdown-Text ohne Vorschau-Kopf, z. B. für das Handbuch (dort springt kein Klick in einen Editor).
pub fn lesetext<'a, Message: Clone + 'a>(inhalt: &'a str, nichts: Message) -> Element<'a, Message> {
    darstellen(inhalt, usize::MAX, move |_, _| nichts.clone(), false, None)
}

fn darstellen<'a, Message: Clone + 'a>(
    inhalt: &'a str,
    cursor_zeile: usize,
    on_springe_zu_zeile: impl Fn(usize, usize) -> Message + Clone + 'a,
    mit_kopf: bool,
    basis: Option<&'a Path>,
) -> Element<'a, Message> {
    let bloecke = parse_bloecke(inhalt);

    let kopf = row![
        thema::abschnitt_titel("Live-Vorschau"),
        text("•").size(10).color(thema::text_gedimmt()),
        text("Klick auf Text springt in den Editor")
            .size(11)
            .color(thema::text_gedimmt()),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([8, 16]);

    let mut spalte = column![].spacing(10).width(Fill);

    for block in bloecke {
        match block {
            MarkdownBlock::Ueberschrift {
                ebene,
                text: u_text,
                zeile,
            } => {
                let aktiv = cursor_zeile == zeile;
                let (schriftgroesse, farbe) = match ebene {
                    1 => (22, thema::akzent()),
                    2 => (18, thema::text_hell()),
                    3 => (15, thema::text_hell()),
                    4 => (14, thema::text_hell()),
                    _ => (13, thema::text_gedimmt()),
                };

                let inhalt = markdown_text::<Message>(&u_text, schriftgroesse as f32, farbe);
                let btn = button(inhalt)
                    .padding([4, 8])
                    .width(Fill)
                    .style(thema::vorschau_block_stil(aktiv))
                    .on_press(on_springe_zu_zeile(zeile, 0));

                spalte = spalte.push(btn);
            }

            MarkdownBlock::CodeBlock { sprache, code, start_zeile, end_zeile } => {
                let aktiv = cursor_zeile >= start_zeile && cursor_zeile <= end_zeile;
                let anzahl = code.len();
                let code_text = code.join("\n");
                let karten_kopf = row![
                    text(if sprache.is_empty() { "CODE".to_string() } else { sprache.to_uppercase() })
                        .size(10)
                        .color(thema::akzent()),
                    space::horizontal(),
                    text(format!("{anzahl} Zeilen")).size(10).color(thema::text_gedimmt()),
                ]
                .align_y(Alignment::Center);
                // Zeilennummern je Codeblock ab 1, wie der Buchtext sie zitiert („Zeile 5 bis 11“)
                let breite = anzahl.to_string().len();
                let nummern = (1..=anzahl.max(1)).map(|n| format!("{n:>breite$}")).collect::<Vec<_>>().join("\n");
                let kasten = container(
                    column![
                        karten_kopf,
                        row![
                            text(nummern).size(12).font(iced::Font::MONOSPACE).color(thema::text_gedimmt()),
                            text(code_text).size(12).font(iced::Font::MONOSPACE).color(thema::text_hell()).wrapping(iced::widget::text::Wrapping::None),
                        ]
                        .spacing(12),
                    ]
                    .spacing(6),
                )
                .padding(10)
                .style(thema::codeblock_container_stil)
                .width(Fill);
                let btn = button(scrollable(kasten).direction(scrollable::Direction::Horizontal(scrollable::Scrollbar::new().width(4).scroller_width(4))))
                    .padding([2, 4])
                    .width(Fill)
                    .style(thema::vorschau_block_stil(aktiv))
                    .on_press(on_springe_zu_zeile(start_zeile, 0));
                spalte = spalte.push(btn);
            }

            MarkdownBlock::Bild { pfad, unterschrift, zeile } => {
                let aktiv = cursor_zeile == zeile;
                let datei = basis.map(|b| b.join(&pfad)).filter(|p| p.is_file());
                let bild: Element<'a, Message> = match datei {
                    Some(d) => container(image(image::Handle::from_path(d)).width(Fill).content_fit(iced::ContentFit::Contain))
                        .max_width(640)
                        .into(),
                    None => text(format!("Bild nicht gefunden: {pfad}")).size(12).color(thema::fehler_rot()).into(),
                };
                let mut inhalt = column![bild].spacing(6).align_x(Alignment::Center).width(Fill);
                if !unterschrift.is_empty() {
                    inhalt = inhalt.push(
                        iced::widget::rich_text(inline_spans(&unterschrift, 12.0, thema::text_gedimmt()))
                            .size(12)
                            .align_x(iced::alignment::Horizontal::Center)
                            .width(Fill),
                    );
                }
                spalte = spalte.push(
                    button(inhalt)
                        .padding([6, 8])
                        .width(Fill)
                        .style(thema::vorschau_block_stil(aktiv))
                        .on_press(on_springe_zu_zeile(zeile, 0)),
                );
            }

            MarkdownBlock::Tabelle { zeilen, start_zeile, end_zeile } => {
                let aktiv = cursor_zeile >= start_zeile && cursor_zeile <= end_zeile;
                let spalten = zeilen.iter().map(Vec::len).max().unwrap_or(1).max(1);
                let mut tabelle = column![].spacing(0).width(Fill);
                for (nr, z) in zeilen.iter().enumerate() {
                    let mut reihe = row![].spacing(10).width(Fill);
                    for k in 0..spalten {
                        let zelle = z.get(k).map(String::as_str).unwrap_or("");
                        let farbe = thema::text_hell();
                        let inhalt: Element<'a, Message> = if nr == 0 {
                            iced::widget::rich_text(
                                inline_spans(zelle, 12.0, farbe).into_iter().map(|sp| sp.font(thema::editor_font(true))).collect::<Vec<_>>(),
                            )
                            .size(12)
                            .into()
                        } else {
                            markdown_text::<Message>(zelle, 12.0, farbe)
                        };
                        reihe = reihe.push(container(inhalt).width(FillPortion(1)));
                    }
                    tabelle = tabelle.push(container(reihe).padding([5, 8]));
                    tabelle = tabelle.push(thema::trennlinie());
                }
                spalte = spalte.push(
                    button(container(tabelle).style(thema::codeblock_container_stil).padding([2, 0]))
                        .padding([2, 4])
                        .width(Fill)
                        .style(thema::vorschau_block_stil(aktiv))
                        .on_press(on_springe_zu_zeile(start_zeile, 0)),
                );
            }

            MarkdownBlock::Seitenumbruch { zeile } => {
                spalte = spalte.push(
                    button(
                        row![
                            container(thema::trennlinie()).width(Fill),
                            text("Seitenumbruch").size(10).color(thema::text_gedimmt()),
                            container(thema::trennlinie()).width(Fill),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .padding([4, 8])
                    .width(Fill)
                    .style(thema::vorschau_block_stil(cursor_zeile == zeile))
                    .on_press(on_springe_zu_zeile(zeile, 0)),
                );
            }

            MarkdownBlock::Zitat {
                zeilen,
                start_zeile,
                end_zeile,
            } => {
                let aktiv = cursor_zeile >= start_zeile && cursor_zeile <= end_zeile;
                let mut zitat_spalte = column![].spacing(3);
                for z in zeilen {
                    zitat_spalte =
                        zitat_spalte.push(markdown_text::<Message>(&z, 12.0, thema::text_hell()));
                }

                let kasten = container(zitat_spalte)
                    .padding(Padding::from([4, 12]))
                    .width(Fill);

                let btn = button(kasten)
                    .padding([2, 4])
                    .width(Fill)
                    .style(thema::vorschau_block_stil(aktiv))
                    .on_press(on_springe_zu_zeile(start_zeile, 0));

                spalte = spalte.push(btn);
            }

            MarkdownBlock::ListenPunkt { marke, ebene, text: punkt_text, zeile } => {
                let aktiv = cursor_zeile == zeile;
                let zeilen_ansicht = row![
                    space::horizontal().width(ebene as f32 * 18.0),
                    text(marke).size(13).color(thema::akzent()).width(22),
                    markdown_text::<Message>(&punkt_text, 13.0, thema::text_hell()),
                ]
                .spacing(4)
                .align_y(Alignment::Start);

                let btn = button(zeilen_ansicht)
                    .padding([3, 6])
                    .width(Fill)
                    .style(thema::vorschau_block_stil(aktiv))
                    .on_press(on_springe_zu_zeile(zeile, 0));

                spalte = spalte.push(btn);
            }

            MarkdownBlock::Absatz {
                text: absatz_text,
                start_zeile,
                end_zeile,
            } => {
                let aktiv = cursor_zeile >= start_zeile && cursor_zeile <= end_zeile;

                let inhalt = markdown_text::<Message>(&absatz_text, 13.0, thema::text_hell());

                let btn = button(inhalt)
                    .padding([4, 8])
                    .width(Fill)
                    .style(thema::vorschau_block_stil(aktiv))
                    .on_press(on_springe_zu_zeile(start_zeile, 0));

                spalte = spalte.push(btn);
            }
        }
    }

    let zentrierter_vorschau_inhalt = container(spalte).max_width(Pixels(860.0)).width(Fill);

    let mut gesamt = column![].height(Fill).width(Fill);
    if mit_kopf {
        gesamt = gesamt.push(kopf);
    }
    container(
        gesamt.push(
            container(scrollable(zentrierter_vorschau_inhalt).id(scroll_id()).height(Fill))
                .center_x(Fill)
                .padding(Padding::from([10, 20]))
                .height(Fill)
                .width(Fill),
        ),
    )
    .style(thema::editor_bereich_stil)
    .width(Fill)
    .height(Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn klartext(q: &str) -> String {
        inline_spans(q, 13.0, iced::Color::BLACK).iter().map(|s| s.text.to_string()).collect()
    }

    #[test]
    fn steuerzeichen_verschwinden() {
        assert_eq!(klartext("Das ist **fett** und *kursiv* und `code`."), "Das ist fett und kursiv und code.");
        assert_eq!(klartext("***beides*** und ~~weg~~"), "beides und weg");
        assert_eq!(klartext("Siehe [die Doku](https://x.de/a_b) jetzt"), "Siehe die Doku jetzt");
        assert_eq!(klartext("Bild ![Abb 1](a.png) hier"), "Bild [Bild: Abb 1] hier");
        assert_eq!(klartext("Titel {#kap:x}"), "Titel ");
        assert_eq!(klartext("snake_case_name und 2 * 3 * 4"), "snake_case_name und 2 * 3 * 4");
        assert_eq!(klartext(r"maskiert \*kein Stern\*"), "maskiert *kein Stern*");
        assert_eq!(klartext("Zeile<br />neu und <kbd>Strg</kbd>, a < b"), "Zeile\nneu und Strg, a < b");
    }

    #[test]
    fn bloecke_wie_im_buchbau() {
        let md = "## Titel\nText direkt davor\n![Abbildung 1: Fenster](images/a.png)\n\n<div class=\"imprint\">\n**Impressum**\n</div>\n<div class=\"page-break\"></div>\n\n1. Erstens\n2. Zweitens\n  - eingerückt\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n```dart\nvoid main() {}\n```\n##### Klein";
        let b = parse_bloecke(md);
        assert!(matches!(&b[0], MarkdownBlock::Ueberschrift { ebene: 2, .. }));
        assert!(matches!(&b[1], MarkdownBlock::Absatz { text, .. } if text == "Text direkt davor"), "Absatz endet vor dem Bild: {b:?}");
        assert!(matches!(&b[2], MarkdownBlock::Bild { pfad, unterschrift, .. } if pfad == "images/a.png" && unterschrift == "Abbildung 1: Fenster"));
        assert!(matches!(&b[3], MarkdownBlock::Absatz { text, .. } if text == "**Impressum**"), "HTML-Hüllen verschwinden: {b:?}");
        assert!(matches!(&b[4], MarkdownBlock::Seitenumbruch { .. }));
        assert!(matches!(&b[5], MarkdownBlock::ListenPunkt { marke, .. } if marke == "1."));
        assert!(matches!(&b[7], MarkdownBlock::ListenPunkt { marke, ebene: 1, .. } if marke == "•"));
        assert!(matches!(&b[8], MarkdownBlock::Tabelle { zeilen, .. } if zeilen.len() == 2 && zeilen[1] == ["1", "2"]));
        assert!(matches!(&b[9], MarkdownBlock::CodeBlock { sprache, .. } if sprache == "dart"));
        assert!(matches!(&b[10], MarkdownBlock::Ueberschrift { ebene: 5, .. }));
    }
}
