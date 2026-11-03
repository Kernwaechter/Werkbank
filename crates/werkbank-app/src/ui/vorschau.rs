//! # Live-Vorschau für Markdown-Inhalte
//!
//! Stellt eine gerenderte Vorschau des aktuellen Kapitels bereit.
//! Bietet synchrone visuelle Verknüpfung mit dem Editor:
//! 1. Der Abschnitt, in dem sich der Cursor aktuell befindet, wird dezent hervorgehoben.
//! 2. Ein Klick auf ein beliebiges Wort oder einen Block in der Vorschau
//!    springt im Editor unmittelbar an die entsprechende Quelltextzeile.

use crate::thema;
use iced::widget::{button, column, container, row, scrollable, space, text};
use iced::{Alignment, Element, Fill, Padding, Pixels};

/// Ein geparster Textblock aus dem Markdown-Quelltext mit Zeileninformation.
#[derive(Debug, Clone)]
pub enum MarkdownBlock {
    /// Überschrift mit Ebene (1 bis 4), Text und Quellzeile.
    Ueberschrift {
        ebene: usize,
        text: String,
        zeile: usize,
    },
    /// Rust-Codeblock mit Codezeilen und Quellzeilenbereich.
    CodeBlock {
        code: Vec<String>,
        start_zeile: usize,
        end_zeile: usize,
    },
    /// Zitat oder Hinweisblock (`> ...`).
    Zitat {
        zeilen: Vec<String>,
        start_zeile: usize,
        end_zeile: usize,
    },
    /// Normaler Fließtext-Absatz mit Quellzeilenbereich.
    Absatz {
        text: String,
        start_zeile: usize,
        end_zeile: usize,
    },
    /// Listenpunkt mit Quellzeile.
    ListenPunkt { text: String, zeile: usize },
}

/// Parst den Markdown-Text zeilenweise in Blöcke mit Quellzeilen-Zuordnung.
pub fn parse_bloecke(inhalt: &str) -> Vec<MarkdownBlock> {
    let mut bloecke = Vec::new();
    let zeilen: Vec<&str> = inhalt.lines().collect();
    let mut i = 0;

    while i < zeilen.len() {
        let zeile = zeilen[i];
        let getrimmt = zeile.trim();

        // Leere Zeilen überspringen
        if getrimmt.is_empty() {
            i += 1;
            continue;
        }

        // Trennlinien, Pandoc-Container (`:::`) und einzeilige Kommentare gehören nicht zum Text
        let nur_zeichen = |z: char| getrimmt.len() >= 3 && getrimmt.chars().all(|c| c == z);
        if nur_zeichen('-') || nur_zeichen('*') || nur_zeichen('_') || getrimmt.starts_with(":::")
            || (getrimmt.starts_with("<!--") && getrimmt.ends_with("-->"))
        {
            i += 1;
            continue;
        }

        // Codeblöcke erkennen (```rust oder ```)
        if getrimmt.starts_with("```") {
            let start_zeile = i;
            i += 1;
            let mut code_zeilen = Vec::new();
            while i < zeilen.len() && !zeilen[i].trim().starts_with("```") {
                code_zeilen.push(zeilen[i].to_string());
                i += 1;
            }
            let end_zeile = i;
            if i < zeilen.len() {
                i += 1; // End-Backticks überspringen
            }
            bloecke.push(MarkdownBlock::CodeBlock {
                code: code_zeilen,
                start_zeile,
                end_zeile,
            });
            continue;
        }

        // Überschriften erkennen
        if let Some(rest) = getrimmt.strip_prefix("#### ") {
            bloecke.push(MarkdownBlock::Ueberschrift {
                ebene: 4,
                text: rest.to_string(),
                zeile: i,
            });
            i += 1;
            continue;
        } else if let Some(rest) = getrimmt.strip_prefix("### ") {
            bloecke.push(MarkdownBlock::Ueberschrift {
                ebene: 3,
                text: rest.to_string(),
                zeile: i,
            });
            i += 1;
            continue;
        } else if let Some(rest) = getrimmt.strip_prefix("## ") {
            bloecke.push(MarkdownBlock::Ueberschrift {
                ebene: 2,
                text: rest.to_string(),
                zeile: i,
            });
            i += 1;
            continue;
        } else if let Some(rest) = getrimmt.strip_prefix("# ") {
            bloecke.push(MarkdownBlock::Ueberschrift {
                ebene: 1,
                text: rest.to_string(),
                zeile: i,
            });
            i += 1;
            continue;
        }

        // Zitate erkennen (`> `)
        if getrimmt.starts_with('>') {
            let start_zeile = i;
            let mut zitat_zeilen = Vec::new();
            while i < zeilen.len() && zeilen[i].trim().starts_with('>') {
                let text_teil = zeilen[i]
                    .trim()
                    .strip_prefix('>')
                    .unwrap_or("")
                    .trim()
                    .to_string();
                zitat_zeilen.push(text_teil);
                i += 1;
            }
            bloecke.push(MarkdownBlock::Zitat {
                zeilen: zitat_zeilen,
                start_zeile,
                end_zeile: i.saturating_sub(1),
            });
            continue;
        }

        // Listenpunkte erkennen
        if let Some(rest) = getrimmt
            .strip_prefix("- ")
            .or_else(|| getrimmt.strip_prefix("* "))
        {
            bloecke.push(MarkdownBlock::ListenPunkt {
                text: rest.to_string(),
                zeile: i,
            });
            i += 1;
            continue;
        }

        // Fließtext-Absatz zusammenfassen bis zur nächsten Leerzeile
        let start_zeile = i;
        let mut absatz_zeilen = Vec::new();
        while i < zeilen.len()
            && !zeilen[i].trim().is_empty()
            && !zeilen[i].trim().starts_with('#')
            && !zeilen[i].trim().starts_with("```")
            && !zeilen[i].trim().starts_with('>')
            && !zeilen[i].trim().starts_with("- ")
        {
            absatz_zeilen.push(zeilen[i].trim());
            i += 1;
        }

        bloecke.push(MarkdownBlock::Absatz {
            text: absatz_zeilen.join(" "),
            start_zeile,
            end_zeile: i.saturating_sub(1),
        });
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

/// Erzeugt die Live-Vorschau-Spalte.
pub fn ansicht<'a, Message: Clone + 'a>(
    inhalt: &'a str,
    cursor_zeile: usize,
    on_springe_zu_zeile: impl Fn(usize, usize) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    darstellen(inhalt, cursor_zeile, on_springe_zu_zeile, true)
}

/// Gesetzter Markdown-Text ohne Vorschau-Kopf, z. B. für das Handbuch (dort springt kein Klick in einen Editor).
pub fn lesetext<'a, Message: Clone + 'a>(inhalt: &'a str, nichts: Message) -> Element<'a, Message> {
    darstellen(inhalt, usize::MAX, move |_, _| nichts.clone(), false)
}

fn darstellen<'a, Message: Clone + 'a>(
    inhalt: &'a str,
    cursor_zeile: usize,
    on_springe_zu_zeile: impl Fn(usize, usize) -> Message + Clone + 'a,
    mit_kopf: bool,
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

            MarkdownBlock::CodeBlock {
                code,
                start_zeile,
                end_zeile,
            } => {
                let aktiv = cursor_zeile >= start_zeile && cursor_zeile <= end_zeile;

                let code_zeilen_anzahl = code.len();
                let mut code_spalte = column![].spacing(2);
                for (rel_nr, zeile_text) in code.into_iter().enumerate() {
                    let code_zeile_nr = start_zeile + rel_nr + 1;
                    let zeilen_ansicht = row![
                        text(format!("{:2} │ ", code_zeile_nr))
                            .size(12)
                            .color(thema::text_gedimmt()),
                        text(zeile_text)
                            .size(12)
                            .color(thema::text_gedimmt()),
                    ]
                    .align_y(Alignment::Center);

                    code_spalte = code_spalte.push(zeilen_ansicht);
                }

                let karten_kopf = row![
                    text("🦀 Rust").size(11).color(thema::akzent()),
                    space::horizontal(),
                    text(format!("{} Zeilen", code_zeilen_anzahl))
                        .size(10)
                        .color(thema::text_gedimmt()),
                ]
                .align_y(Alignment::Center);

                let kasten = container(
                    column![karten_kopf, space::vertical().height(4), code_spalte].spacing(4),
                )
                .padding(10)
                .style(thema::codeblock_container_stil)
                .width(Fill);

                let btn = button(kasten)
                    .padding([2, 4])
                    .width(Fill)
                    .style(thema::vorschau_block_stil(aktiv))
                    .on_press(on_springe_zu_zeile(start_zeile, 0));

                spalte = spalte.push(btn);
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

            MarkdownBlock::ListenPunkt {
                text: punkt_text,
                zeile,
            } => {
                let aktiv = cursor_zeile == zeile;
                let zeilen_ansicht = row![
                    text("•").size(14).color(thema::akzent()),
                    markdown_text::<Message>(&punkt_text, 13.0, thema::text_hell()),
                ]
                .spacing(8)
                .align_y(Alignment::Center);

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
            container(scrollable(zentrierter_vorschau_inhalt).height(Fill))
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
    }
}
