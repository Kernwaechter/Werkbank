//! # Schreibansicht im Ulysses-Stil
//!
//! Hier befindet sich der dreispaltige Arbeitsplatz:
//! 1. Spalte: Bibliothek (Bände und Teile des Buches)
//! 2. Spalte: Blattliste (Kapitel des gewählten Teils mit Vorschau und Wortzahlen)
//! 3. Spalte: Editor (zentrierter Schreibbereich mit Iced-Texteditor und Markdown-Hervorhebung)
//! 4. Spalte: Inspektor (rechts einblendbar mit Metadaten, Wortziel und Querverweisen)

use crate::thema;
use crate::ui::navigation::tooltip_knopf;
use iced::widget::{column, container, progress_bar, row, scrollable, slider, space, text};
use crate::ui::fokus::{button, regler_mit_pfeilen};
use crate::editor as text_editor;
use crate::ui::kontext::{Ziel, rechtsklick};
use iced::{Alignment, Element, Fill, Length, Padding, Pixels};
use werkbank_kern::{KapitelStatus, Projekt};

/// Kennung des Editors, damit Hinweise der Sprachprüfung ihn fokussieren können.
pub fn editor_id() -> iced::widget::Id {
    iced::widget::Id::new("kapitel-editor")
}

/// Erzeugt die Hauptschreibansicht abhängig von Fokus-Modus und Inspektor-Zustand.
#[allow(clippy::too_many_arguments)]
pub fn ansicht<'a, Message: Clone + 'a>(
    projekt: &'a Projekt,
    aktiver_teil_id: &'a str,
    aktives_kapitel_id: &'a str,
    editor_content: &'a text_editor::Content,
    edit_tick: usize,
    fokus_modus: bool,
    seitenleiste_sichtbar: bool,
    inspektor_sichtbar: bool,
    vorschau_sichtbar: bool,
    on_teil_klick: impl Fn(String) -> Message + Copy + 'a,
    on_kapitel_klick: impl Fn(String) -> Message + Copy + 'a,
    on_editor_aktion: impl Fn(text_editor::Action) -> Message + 'a,
    on_neues_kapitel: Message,
    on_neuer_teil: Message,
    on_ziel_setzen: impl Fn(usize) -> Message + 'a,
    on_status_aendern: impl Fn(KapitelStatus) -> Message + 'a,
    on_springe_zu_zeile: impl Fn(usize, usize) -> Message + Copy + 'a,
    on_querverweis_klick: impl Fn(String) -> Message + Copy + 'a,
    sprach_box: Element<'a, Message>,
    inspektor_tab: u8,
    on_inspektor_tab: impl Fn(u8) -> Message + Copy + 'a,
    on_inspektor_ziehen: Message,
    hl: crate::markierung::Einstellung,
    vorschau: Element<'a, Message>,
    erklaerung: crate::editor::Erklaerung<'a>,
    on_wort_merken: impl Fn(String) -> Message + 'a,
    on_kontext: impl Fn(Ziel, iced::Point) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let mut haupt_reihe = row![].width(Fill).height(Fill);

    // Spalte 1 und 2 verschwinden im Fokus-Modus oder per Seitenleisten-Schalter
    if seitenleiste_sichtbar && !fokus_modus {
        let bibliothek = spalte_bibliothek(projekt, aktiver_teil_id, on_teil_klick, on_neuer_teil, on_kontext);
        haupt_reihe = haupt_reihe.push(bibliothek).push(thema::trennlinie_senkrecht());

        let blattliste = spalte_blattliste(
            projekt,
            aktiver_teil_id,
            aktives_kapitel_id,
            on_kapitel_klick,
            on_neues_kapitel,
            on_kontext,
        );
        haupt_reihe = haupt_reihe.push(blattliste).push(thema::trennlinie_senkrecht());
    }

    // Spalte 3: Der zentrale Editor (optional geteilt mit Live-Vorschau)
    let editor_spalte = spalte_editor(
        projekt,
        aktives_kapitel_id,
        editor_content,
        edit_tick,
        vorschau_sichtbar,
        on_editor_aktion,
        on_springe_zu_zeile,
        hl,
        vorschau,
        erklaerung,
        on_wort_merken,
    );
    haupt_reihe = haupt_reihe.push(editor_spalte);

    // Spalte 4: Inspektor rechts (sofern eingeblendet und nicht im Fokus-Modus)
    if inspektor_sichtbar && !fokus_modus {
        let inspektor = spalte_inspektor(
            projekt,
            aktives_kapitel_id,
            on_ziel_setzen,
            on_status_aendern,
            on_querverweis_klick,
            sprach_box,
            inspektor_tab,
            on_inspektor_tab,
        );
        // Griff zum Verbreitern und Verschmälern der Detailspalte: sichtbar nur als feine Linie,
        // greifbar aber auf 5 px Breite
        let griff = iced::widget::mouse_area(
            container(thema::trennlinie_senkrecht()).padding(Padding { left: 2.0, right: 2.0, top: 0.0, bottom: 0.0 }),
        )
        .on_press(on_inspektor_ziehen)
        .interaction(iced::mouse::Interaction::ResizingHorizontally);
        haupt_reihe = haupt_reihe.push(griff).push(inspektor);
    }

    haupt_reihe.into()
}

/// Spalte 1: Bibliothek – das Buch als Regal: Titel oben, darunter die Teile als flache Liste.
fn spalte_bibliothek<'a, Message: Clone + 'a>(
    projekt: &'a Projekt,
    aktiver_teil_id: &'a str,
    on_teil_klick: impl Fn(String) -> Message + 'a,
    on_neuer_teil: Message,
    on_kontext: impl Fn(Ziel, iced::Point) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let buch_info = column![
        text(&projekt.metadaten.titel).size(15).font(thema::fett()).color(thema::text_hell()),
        text(&projekt.metadaten.untertitel).size(12).color(thema::text_gedimmt()),
        text(format!(
            "{} Wörter · {:.0} Normseiten",
            thema::tausender(projekt.gesamtwortzahl()),
            projekt.normseiten()
        ))
        .size(11)
        .color(thema::text_gedimmt()),
    ]
    .spacing(3)
    .padding([0, 8]);

    let mut teile_spalte = column![].spacing(1).width(Fill);
    for teil in &projekt.teile {
        let aktiv = teil.id == aktiver_teil_id;
        let eintrag = row![
            text("▤").size(13).color(if aktiv { thema::akzent() } else { thema::text_gedimmt() }),
            text(&teil.titel).size(13).color(thema::text_hell()).width(Fill),
            text(teil.kapitel.len().to_string()).size(11).color(thema::text_gedimmt()),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let id = teil.id.clone();
        teile_spalte = teile_spalte.push(
            tooltip_knopf_text(
                rechtsklick(
                    button(eintrag)
                        .padding([6, 8])
                        .width(Fill)
                        .style(thema::listen_eintrag_stil(aktiv))
                        .on_press(on_teil_klick(teil.id.clone())),
                    move |p| on_kontext(Ziel::Teil(id.clone()), p),
                ),
                format!(
                    "{} Wörter in {} Kapiteln  ·  Rechtsklick: Umbenennen, Verschieben …",
                    thema::tausender(teil.gesamtwortzahl()),
                    teil.kapitel.len()
                ),
            ),
        );
    }

    let abschnitt_kopf = row![
        container(thema::abschnitt_titel("Teile")).padding([0, 8]),
        space::horizontal(),
        plus_symbol("Neuer Buchteil", on_neuer_teil),
    ]
    .align_y(Alignment::Center);

    container(
        column![
            buch_info,
            space::vertical().height(10),
            abschnitt_kopf,
            scrollable(teile_spalte).height(Fill),
        ]
        .spacing(6)
        .height(Fill),
    )
    .width(250)
    .padding(Padding::from([16, 10]))
    .style(thema::bibliothek_stil)
    .into()
}

/// Spalte 2: Blattliste – die Kapitel des Teils mit Titel, lesbarer Vorschau, Stand und Wortziel.
fn spalte_blattliste<'a, Message: Clone + 'a>(
    projekt: &'a Projekt,
    aktiver_teil_id: &'a str,
    aktives_kapitel_id: &'a str,
    on_kapitel_klick: impl Fn(String) -> Message + 'a,
    on_neues_kapitel: Message,
    on_kontext: impl Fn(Ziel, iced::Point) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let aktiver_teil = projekt
        .teile
        .iter()
        .find(|t| t.id == aktiver_teil_id)
        .or_else(|| projekt.teile.first());

    let (teil_titel, kapitel_liste, teil_woerter) = match aktiver_teil {
        Some(t) => (t.titel.as_str(), &t.kapitel[..], t.gesamtwortzahl()),
        None => ("Kein Teil ausgewählt", &[][..], 0),
    };

    let kopf = row![
        column![
            text(teil_titel).size(15).font(thema::fett()).color(thema::text_hell()),
            text(format!("{} Kapitel · {} Wörter", kapitel_liste.len(), thema::tausender(teil_woerter)))
            .size(11)
            .color(thema::text_gedimmt()),
        ]
        .spacing(3)
        .width(Fill),
        plus_symbol("Neues Kapitel  ·  Strg+Umschalt+N", on_neues_kapitel),
    ]
    .spacing(8)
    .align_y(Alignment::Start)
    .padding([0, 8]);

    let mut liste_spalte = column![].spacing(2).width(Fill);
    if kapitel_liste.is_empty() {
        liste_spalte = liste_spalte.push(
            container(
                text("Noch keine Kapitel. Mit + legst du das erste an.").size(12).color(thema::text_gedimmt()),
            )
            .padding([12, 8]),
        );
    }
    for kapitel in kapitel_liste {
        let aktiv = kapitel.id == aktives_kapitel_id;
        let fortschritt = kapitel.ziel_fortschritt();
        let farbe = thema::status_farbe(kapitel.status);
        let vorschau = kapitel.vorschautext(120);

        let mut karte = column![text(&kapitel.titel).size(14).font(thema::fett()).color(thema::text_hell())].spacing(5).width(Fill);
        if !vorschau.is_empty() {
            karte = karte.push(text(vorschau).size(12).color(thema::text_gedimmt()));
        }
        karte = karte.push(
            row![
                container(text(kapitel.status.name()).size(10)).padding([1, 7]).style(thema::marke_stil(farbe)),
                space::horizontal(),
                text(format!("{} / {}", thema::tausender(kapitel.wortzahl()), thema::tausender(kapitel.wortziel)))
                    .size(10)
                    .color(thema::text_gedimmt()),
                container(progress_bar(0.0..=100.0, fortschritt).girth(3.0).style(progress_bar::primary)).width(44),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );

        let id = kapitel.id.clone();
        liste_spalte = liste_spalte.push(rechtsklick(
            button(karte)
                .padding([10, 10])
                .width(Fill)
                .style(thema::listen_eintrag_stil(aktiv))
                .on_press(on_kapitel_klick(kapitel.id.clone())),
            move |p| on_kontext(Ziel::Kapitel(id.clone()), p),
        ));
    }

    container(
        column![
            kopf,
            space::vertical().height(8),
            scrollable(liste_spalte).height(Fill),
        ]
        .spacing(4)
        .height(Fill),
    )
    .width(300)
    .padding(Padding::from([16, 10]))
    .style(thema::flaeche_stil)
    .into()
}

/// Spalte 3: Zentraler Schreibbereich mit zentriertem Texteditor (optional mit geteilter Live-Vorschau).
#[allow(clippy::too_many_arguments)]
fn spalte_editor<'a, Message: Clone + 'a>(
    projekt: &'a Projekt,
    aktives_kapitel_id: &'a str,
    editor_content: &'a text_editor::Content,
    edit_tick: usize,
    vorschau_sichtbar: bool,
    on_editor_aktion: impl Fn(text_editor::Action) -> Message + 'a,
    _on_springe_zu_zeile: impl Fn(usize, usize) -> Message + Copy + 'a,
    hl: crate::markierung::Einstellung,
    vorschau: Element<'a, Message>,
    erklaerung: crate::editor::Erklaerung<'a>,
    on_wort_merken: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let aktives_kapitel = projekt.kapitel_finden(aktives_kapitel_id);

    let (titel, pfad_str, _inhalt_str, wortzahl, zeichen, lesedauer, ziel, fortschritt) =
        match aktives_kapitel {
            Some(k) => (
                k.titel.as_str(),
                k.dateipfad.to_string_lossy().to_string(),
                k.inhalt.as_str(),
                k.wortzahl(),
                k.zeichenzahl(),
                k.lesedauer_minuten(),
                k.wortziel,
                k.ziel_fortschritt(),
            ),
            None => (
                "Kein Kapitel geöffnet",
                String::new(),
                "",
                0,
                0,
                0,
                2000,
                0.0,
            ),
        };

    // Kopf: nur, wo man ist (Teil › Kapitel) und wie lang es ist; alles Weitere steht im Inspektor
    let teil_titel = aktives_kapitel
        .and_then(|k| projekt.teile.iter().find(|t| t.kapitel.iter().any(|x| x.id == k.id)))
        .map(|t| t.titel.as_str())
        .unwrap_or("");
    let _ = (&pfad_str, zeichen, lesedauer);
    let breadcrumb_leiste = row![
        text(if teil_titel.is_empty() { String::new() } else { format!("{teil_titel}  ›") })
            .size(12)
            .color(thema::text_gedimmt())
            .wrapping(iced::widget::text::Wrapping::None),
        text(titel).size(12).color(thema::text_hell()).wrapping(iced::widget::text::Wrapping::None).width(Fill),
        text(format!("{} Wörter", thema::tausender(wortzahl))).size(12).color(thema::text_gedimmt()),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .padding([9, 20]);

    let _ = edit_tick;

    // Iced Native Text Editor mit reaktiver Markdown-Hervorhebung; ohne Rahmen, der Text steht frei
    let editor_widget = text_editor::TextEditor::new(editor_content)
        .id(editor_id())
        .placeholder("Hier beginnt dein Kapitel. Schreib einfach los …")
        .height(Fill)
        .padding(Padding::from([32, 40]))
        .size(Pixels(thema::editor_schrift()))
        .font(thema::editor_font(false))
        .line_height(iced::widget::text::LineHeight::Relative(thema::zeilenabstand()))
        .schreibmaschine(thema::schreibmaschine())
        .wellen(hl.wellen.clone())
        .style(thema::schreibflaeche_stil)
        .erklaerung(erklaerung)
        .rechtsklick(move |wort, art| (art == crate::markierung::WELLE_RECHTSCHREIBUNG).then(|| on_wort_merken(wort)))
        .highlight_with::<crate::markierung::Markierer>(hl, crate::markierung::format)
        .on_action(on_editor_aktion);

    // Fußzeile: Position, Wortziel, Hinweis auf die Befehlssuche – leise und klein
    let cursor = editor_content.cursor();
    let zeile = cursor.position.line + 1;
    let spalte = cursor.position.column + 1;

    let fusszeile = row![
        text(format!("Zeile {zeile} · Spalte {spalte}")).size(11).color(thema::text_gedimmt()),
        space::horizontal(),
        text(format!("Ziel {} / {}", thema::tausender(wortzahl), thema::tausender(ziel))).size(11).color(thema::text_gedimmt()),
        container(progress_bar(0.0..=100.0, fortschritt).girth(3.0).style(progress_bar::primary)).width(Length::Fixed(90.0)),
        text(format!("{fortschritt:.0} %")).size(11).color(thema::text_gedimmt()),
        space::horizontal(),
        text("Strg+K Befehle").size(11).color(thema::text_gedimmt()),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .padding([6, 20]);

    // Zentraler Bereich: geteilt mit Vorschau oder fokussiert zentriert
    let mitte: Element<'a, Message> = if vorschau_sichtbar {
        let vorschau_widget = vorschau;

        row![
            container(editor_widget)
                .width(Length::FillPortion(1))
                .height(Fill),
            thema::trennlinie_senkrecht(),
            container(vorschau_widget)
                .width(Length::FillPortion(1))
                .height(Fill),
        ]
        .width(Fill)
        .height(Fill)
        .into()
    } else {
        container(
            container(editor_widget)
                .max_width(Pixels(thema::editor_spalte()))
                .width(Fill)
                .height(Fill),
        )
        .center_x(Fill)
        .height(Fill)
        .width(Fill)
        .into()
    };

    container(
        column![breadcrumb_leiste, mitte, fusszeile,]
            .height(Fill)
            .width(Fill),
    )
    .style(thema::editor_bereich_stil)
    .width(Fill)
    .height(Fill)
    .into()
}

/// Spalte 4: Inspektor mit Metadaten, Wortziel und Querverweisen.
#[allow(clippy::too_many_arguments)]
fn spalte_inspektor<'a, Message: Clone + 'a>(
    projekt: &'a Projekt,
    aktives_kapitel_id: &'a str,
    on_ziel_setzen: impl Fn(usize) -> Message + 'a,
    on_status_aendern: impl Fn(KapitelStatus) -> Message + 'a,
    on_querverweis_klick: impl Fn(String) -> Message + Copy + 'a,
    sprach_box: Element<'a, Message>,
    tab: u8,
    on_tab: impl Fn(u8) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let ziel: std::rc::Rc<dyn Fn(usize) -> Message + 'a> = std::rc::Rc::new(on_ziel_setzen);
    let status_fn: std::rc::Rc<dyn Fn(KapitelStatus) -> Message + 'a> = std::rc::Rc::new(on_status_aendern);
    let aktives_kapitel = projekt.kapitel_finden(aktives_kapitel_id);

    // Segment-Schalter: Details · Tempo · Stil · Text
    let mut segmente = row![].spacing(2);
    for (nr, name) in ["Details", "Tempo", "Stil", "Text"].iter().enumerate() {
        let nr = nr as u8;
        segmente = segmente.push(
            button(text(*name).size(12).center())
                .padding([4, 6])
                .width(Fill)
                .style(thema::segment_stil(tab == nr))
                .on_press(on_tab(nr)),
        );
    }
    let reiter = container(segmente).padding(2).width(Fill).style(thema::segment_rahmen_stil);
    let mut inhalt = column![reiter, space::vertical().height(4)].spacing(18).width(Fill);

    if let Some(k) = aktives_kapitel {
        // Wortziel: der Stand groß, darunter der Fortschritt und der Regler (250er-Schritte)
        let ziel_box = column![
            thema::abschnitt_titel("Wortziel"),
            row![
                text(thema::tausender(k.wortzahl())).size(24).font(thema::fett()).color(thema::text_hell()),
                text(format!("/ {} Wörter", thema::tausender(k.wortziel))).size(12).color(thema::text_gedimmt()),
                space::horizontal(),
                text(format!("{:.0} %", k.ziel_fortschritt())).size(12).color(thema::akzent()),
            ]
            .spacing(6)
            .align_y(Alignment::End),
            progress_bar(0.0..=100.0, k.ziel_fortschritt()).girth(5.0).style(progress_bar::primary),
            regler_mit_pfeilen(
                slider(250..=6000u32, k.wortziel.clamp(250, 6000) as u32, {
                    let f = ziel.clone();
                    move |w| f(w as usize)
                })
                .step(250u32)
                .style(thema::regler_stil),
                k.wortziel.clamp(250, 6000) as u32,
                250,
                250,
                6000,
                move |w| ziel(w as usize),
            ),
        ]
        .spacing(8);

        // Bearbeitungsstand: vier Marken, ein Klick setzt den Stand
        let mut stand = column![].spacing(2);
        let mut reihe = row![].spacing(2);
        for (i, w) in STATUS_WAHL.into_iter().enumerate() {
            let aktiv = w.0 == k.status;
            let farbe = thema::status_farbe(w.0);
            let f = status_fn.clone();
            reihe = reihe.push(
                button(
                    row![text("●").size(9).color(farbe), text(w.0.name()).size(12)]
                        .spacing(6)
                        .align_y(Alignment::Center),
                )
                .padding([5, 8])
                .width(Fill)
                .style(thema::segment_stil(aktiv))
                .on_press(f(w.0)),
            );
            if !i.is_multiple_of(2) {
                stand = stand.push(std::mem::replace(&mut reihe, row![].spacing(2)));
            }
        }
        let status_box = column![
            thema::abschnitt_titel("Bearbeitungsstand"),
            container(stand).padding(2).style(thema::segment_rahmen_stil),
        ]
        .spacing(8);

        // Textstatistik
        let statistik_box = column![
            thema::abschnitt_titel("Text"),
            zeile_metrik("Wörter", format!("{}", k.wortzahl())),
            zeile_metrik("Zeichen gesamt", format!("{}", k.zeichenzahl())),
            zeile_metrik(
                "Ohne Leerzeichen",
                format!("{}", k.zeichen_ohne_leerzeichen())
            ),
            zeile_metrik("Absätze", format!("{}", k.absatz_anzahl())),
            {
                let (rust, dart) = (k.rust_codebloecke_anzahl(), k.dart_codebloecke_anzahl());
                match (rust, dart) {
                    (_, 0) => zeile_metrik("Rust-Codeblöcke", rust.to_string()),
                    (0, _) => zeile_metrik("Dart-Codeblöcke", dart.to_string()),
                    _ => zeile_metrik("Codeblöcke Rust / Dart", format!("{rust} / {dart}")),
                }
            },
        ]
        .spacing(5);

        // Querverweise: anklickbare Links, die zum Zielkapitel springen
        let querverweise = k.querverweise();
        let refs_inhalt: Element<'a, Message> = if querverweise.is_empty() {
            text("Keine Querverweise im Text")
                .size(11)
                .color(thema::text_gedimmt())
                .into()
        } else {
            let mut ref_spalte = column![].spacing(2);
            for r in querverweise {
                let ziel = r.clone();
                ref_spalte = ref_spalte.push(
                    button(if projekt.kapitel_finden(&r).is_some() {
                        text(format!("→ @kap:{}", r)).size(11)
                    } else {
                        text(format!("✗ @kap:{} (Ziel fehlt)", r)).size(11).color(thema::fehler_rot())
                    })
                        .padding([2, 6])
                        .style(thema::link_knopf_stil)
                        .on_press(on_querverweis_klick(ziel)),
                );
            }
            ref_spalte.into()
        };

        let refs_box = column![
            thema::abschnitt_titel("Querverweise (@kap:)"),
            refs_inhalt,
        ]
        .spacing(4);

        if tab == 0 {
            inhalt = inhalt.push(ziel_box).push(status_box).push(statistik_box).push(refs_box);
        } else {
            inhalt = inhalt.push(sprach_box);
        }
    } else {
        inhalt = inhalt.push(
            text("Kein Kapitel gewählt")
                .size(12)
                .color(thema::text_gedimmt()),
        );
    }

    container(scrollable(container(inhalt).padding(Padding::ZERO.right(10))).height(Fill))
        .width(thema::inspektor_breite())
        .padding(Padding::from([14, 14]).right(4))
        .style(thema::flaeche_stil)
        .into()
}

/// Kapitelstatus als Eintrag der Auswahlliste.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StatusWahl(KapitelStatus);

impl std::fmt::Display for StatusWahl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.name())
    }
}

const STATUS_WAHL: [StatusWahl; 4] = [
    StatusWahl(KapitelStatus::Entwurf),
    StatusWahl(KapitelStatus::InArbeit),
    StatusWahl(KapitelStatus::Ueberarbeitet),
    StatusWahl(KapitelStatus::Fertig),
];

/// Hängt einem Element eine Hinweisblase mit berechnetem Text an.
fn tooltip_knopf_text<'a, Message: Clone + 'a>(knopf: impl Into<Element<'a, Message>>, hinweis: String) -> Element<'a, Message> {
    iced::widget::tooltip(
        knopf,
        container(text(hinweis).size(11)).padding([4, 8]).style(thema::tooltip_stil),
        iced::widget::tooltip::Position::Right,
    )
    .gap(6)
    .into()
}

/// Dezentes „+“ in Abschnittsköpfen, mit Tooltip statt beschrifteter Knopfzeile.
fn plus_symbol<'a, Message: Clone + 'a>(
    hinweis: &'static str,
    nachricht: Message,
) -> Element<'a, Message> {
    tooltip_knopf(
        button(text("+").size(16).center())
            .padding([0, 8])
            .style(thema::symbol_knopf_stil(false))
            .on_press(nachricht),
        hinweis,
    )
}

/// Hilfszeile für die Metriken-Tabelle.
fn zeile_metrik<'a, Message: 'a>(bezeichnung: &'static str, wert: String) -> Element<'a, Message> {
    row![
        text(bezeichnung).size(11).color(thema::text_gedimmt()),
        space::horizontal(),
        text(wert).size(11).color(thema::text_hell()),
    ]
    .align_y(Alignment::Center)
    .into()
}
