//! # Schreibansicht im Ulysses-Stil
//!
//! Hier befindet sich der dreispaltige Arbeitsplatz:
//! 1. Spalte: Bibliothek (Bände und Teile des Buches)
//! 2. Spalte: Blattliste (Kapitel des gewählten Teils mit Vorschau und Wortzahlen)
//! 3. Spalte: Editor (zentrierter Schreibbereich mit Iced-Texteditor und Markdown-Hervorhebung)
//! 4. Spalte: Inspektor (rechts einblendbar mit Metadaten, Wortziel und Querverweisen)

use crate::thema;
use crate::ui::navigation::tooltip_knopf;
use iced::widget::{column, container, pick_list, progress_bar, row, scrollable, slider, space, text};
use crate::ui::fokus::{button, liste_mit_pfeilen, regler_mit_pfeilen};
use crate::editor as text_editor;
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
) -> Element<'a, Message> {
    let mut haupt_reihe = row![].width(Fill).height(Fill);

    // Spalte 1 und 2 verschwinden im Fokus-Modus oder per Seitenleisten-Schalter
    if seitenleiste_sichtbar && !fokus_modus {
        let bibliothek = spalte_bibliothek(projekt, aktiver_teil_id, on_teil_klick, on_neuer_teil);
        haupt_reihe = haupt_reihe.push(bibliothek);

        let blattliste = spalte_blattliste(
            projekt,
            aktiver_teil_id,
            aktives_kapitel_id,
            on_kapitel_klick,
            on_neues_kapitel,
        );
        haupt_reihe = haupt_reihe.push(blattliste);
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
        // Griff zum Verbreitern und Verschmälern der Detailspalte
        let griff = iced::widget::mouse_area(
            container(space().width(Length::Fixed(2.0)).height(Fill))
                .padding(Padding { left: 2.0, right: 2.0, top: 0.0, bottom: 0.0 })
                .style(|_| container::Style {
                    background: Some(iced::Background::Color(thema::rahmen_farbe())),
                    ..Default::default()
                }),
        )
        .on_press(on_inspektor_ziehen)
        .interaction(iced::mouse::Interaction::ResizingHorizontally);
        haupt_reihe = haupt_reihe.push(griff).push(inspektor);
    }

    haupt_reihe.into()
}

/// Spalte 1: Bibliothek mit Buchtitel, Teilen und Gesamtwortzahl.
fn spalte_bibliothek<'a, Message: Clone + 'a>(
    projekt: &'a Projekt,
    aktiver_teil_id: &'a str,
    on_teil_klick: impl Fn(String) -> Message + 'a,
    on_neuer_teil: Message,
) -> Element<'a, Message> {
    let buch_info = column![
        text(&projekt.metadaten.titel)
            .size(15)
            .color(thema::text_hell()),
        text(&projekt.metadaten.untertitel)
            .size(12)
            .color(thema::text_gedimmt()),
        row![
            text(format!("{} Wörter", projekt.gesamtwortzahl()))
                .size(11)
                .color(thema::akzent()),
            text("•").size(10).color(thema::text_gedimmt()),
            text(format!("{:.1} Normseiten", projekt.normseiten()))
                .size(11)
                .color(thema::text_gedimmt()),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .spacing(4);

    let mut teile_spalte = column![].spacing(6).width(Fill);

    for teil in &projekt.teile {
        let aktiv = teil.id == aktiver_teil_id;
        let kapitel_anzahl = teil.kapitel.len();
        let wortzahl = teil.gesamtwortzahl();

        let karten_inhalt = column![
            row![
                text(&teil.titel).size(13).color(thema::text_hell()),
                space::horizontal(),
                text(format!("{}", kapitel_anzahl))
                    .size(11)
                    .color(thema::text_gedimmt()),
            ]
            .align_y(Alignment::Center),
            text(format!("{} Wörter gesamt", wortzahl))
                .size(11)
                .color(thema::text_gedimmt()),
        ]
        .spacing(4)
        .width(Fill);

        let teil_btn = button(karten_inhalt)
            .padding([8, 10])
            .width(Fill)
            .style(if aktiv {
                thema::kapitel_karte_aktiv_stil
            } else {
                thema::kapitel_karte_stil
            })
            .on_press(on_teil_klick(teil.id.clone()));

        teile_spalte = teile_spalte.push(teil_btn);
    }

    let abschnitt_kopf = row![
        text("TEILE").size(10).color(thema::text_gedimmt()),
        space::horizontal(),
        plus_symbol("Neuer Buchteil", on_neuer_teil),
    ]
    .align_y(Alignment::Center);

    container(
        column![
            buch_info,
            space::vertical().height(6),
            abschnitt_kopf,
            scrollable(teile_spalte).height(Fill),
        ]
        .spacing(8)
        .height(Fill),
    )
    .width(260)
    .padding(12)
    .style(thema::seitenleiste_stil)
    .into()
}

/// Spalte 2: Blattliste (Kapitelübersicht des aktuellen Teils).
fn spalte_blattliste<'a, Message: Clone + 'a>(
    projekt: &'a Projekt,
    aktiver_teil_id: &'a str,
    aktives_kapitel_id: &'a str,
    on_kapitel_klick: impl Fn(String) -> Message + 'a,
    on_neues_kapitel: Message,
) -> Element<'a, Message> {
    let aktiver_teil = projekt
        .teile
        .iter()
        .find(|t| t.id == aktiver_teil_id)
        .or_else(|| projekt.teile.first());

    let (teil_titel, kapitel_liste) = match aktiver_teil {
        Some(t) => (t.titel.as_str(), &t.kapitel[..]),
        None => ("Kein Teil ausgewählt", &[][..]),
    };

    let kopf = row![
        text(teil_titel).size(14).color(thema::text_hell()),
        space::horizontal(),
        text(format!("{} Kapitel", kapitel_liste.len()))
            .size(11)
            .color(thema::text_gedimmt()),
        plus_symbol("Neues Kapitel  ·  Strg+Umschalt+N", on_neues_kapitel),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut liste_spalte = column![].spacing(8).width(Fill);

    for kapitel in kapitel_liste {
        let aktiv = kapitel.id == aktives_kapitel_id;
        let fortschritt = kapitel.ziel_fortschritt();

        let karte_inhalt = column![
            row![
                text(&kapitel.titel).size(13).color(thema::text_hell()),
                space::horizontal(),
                text(kapitel.status.name())
                    .size(10)
                    .color(thema::text_gedimmt()),
            ]
            .align_y(Alignment::Center),
            text(kapitel.vorschautext(65))
                .size(11)
                .color(thema::text_gedimmt()),
            row![
                text(format!("{} / {} W.", kapitel.wortzahl(), kapitel.wortziel))
                    .size(10)
                    .color(thema::text_gedimmt()),
                space::horizontal(),
                text(format!("{:.0}%", fortschritt))
                    .size(10)
                    .color(thema::akzent()),
            ]
            .align_y(Alignment::Center),
            progress_bar(0.0..=100.0, fortschritt)
                .girth(3.0)
                .style(progress_bar::primary),
        ]
        .spacing(4)
        .width(Fill);

        let karten_btn = button(karte_inhalt)
            .padding([8, 10])
            .width(Fill)
            .style(if aktiv {
                thema::kapitel_karte_aktiv_stil
            } else {
                thema::kapitel_karte_stil
            })
            .on_press(on_kapitel_klick(kapitel.id.clone()));

        liste_spalte = liste_spalte.push(karten_btn);
    }

    container(
        column![
            kopf,
            space::vertical().height(6),
            scrollable(liste_spalte).height(Fill),
        ]
        .spacing(8)
        .height(Fill),
    )
    .width(290)
    .padding(12)
    .style(thema::seitenleiste_stil)
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

    // Obere Leiste des Editors (Breadcrumb, Live-Zähler und Vorschau-Toggle)
    let breadcrumb_leiste = row![
        text(titel).size(14).color(thema::text_hell()),
        text("—").size(12).color(thema::text_gedimmt()),
        text(pfad_str).size(11).color(thema::text_gedimmt()),
        space::horizontal(),
        text(format!("{} Wörter", wortzahl))
            .size(12)
            .color(thema::akzent()),
        text("•").size(11).color(thema::text_gedimmt()),
        text(format!("{} Zeichen", zeichen))
            .size(11)
            .color(thema::text_gedimmt()),
        text("•").size(11).color(thema::text_gedimmt()),
        text(format!("~{} Min. Lesezeit", lesedauer))
            .size(11)
            .color(thema::text_gedimmt()),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .padding([8, 20]);

    let _ = edit_tick;

    // Iced Native Text Editor mit reaktiver Markdown-Hervorhebung
    let editor_widget = text_editor::TextEditor::new(editor_content)
        .id(editor_id())
        .placeholder("Hier beginnt dein Kapitel. Schreibe drauflos...")
        .height(Fill)
        .padding(Padding::from([28, 36]))
        .size(Pixels(thema::editor_schrift()))
        .font(thema::editor_font(false))
        .line_height(iced::widget::text::LineHeight::Relative(thema::zeilenabstand()))
        .schreibmaschine(thema::schreibmaschine())
        .wellen(hl.wellen.clone())
        .highlight_with::<crate::markierung::Markierer>(hl, crate::markierung::format)
        .on_action(on_editor_aktion);

    // Fußzeile mit Cursor-Info und Fortschritt
    let cursor = editor_content.cursor();
    let zeile = cursor.position.line + 1;
    let spalte = cursor.position.column + 1;

    let fusszeile = row![
        text(format!("Zeile {}, Spalte {}", zeile, spalte))
            .size(11)
            .color(thema::text_gedimmt()),
        space::horizontal(),
        text(format!(
            "Wortziel: {} / {} ({:.0}%)",
            wortzahl, ziel, fortschritt
        ))
        .size(11)
        .color(thema::text_gedimmt()),
        container(
            progress_bar(0.0..=100.0, fortschritt)
                .girth(4.0)
                .style(progress_bar::primary),
        )
        .width(Length::Fixed(120.0)),
        space::horizontal(),
        text("Strg+K  Befehle").size(10).color(thema::text_gedimmt()),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .padding([6, 20]);

    // Zentraler Bereich: geteilt mit Vorschau oder fokussiert zentriert
    let mitte: Element<'a, Message> = if vorschau_sichtbar {
        let vorschau_widget = vorschau;

        row![
            container(editor_widget)
                .width(Length::FillPortion(1))
                .height(Fill),
            container(space::horizontal().width(Length::Fixed(1.0)))
                .style(thema::seitenleiste_stil)
                .height(Fill),
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

    // Reiter: Details · Tempo · Stil · Text
    let mut reiter = row![].spacing(2);
    for (nr, name) in ["Details", "Tempo", "Stil", "Text"].iter().enumerate() {
        let nr = nr as u8;
        reiter = reiter.push(
            button(text(*name).size(12).center())
                .padding([4, 10])
                .width(Fill)
                .style(thema::tab_knopf_stil(tab == nr))
                .on_press(on_tab(nr)),
        );
    }
    let mut inhalt = column![reiter, space::vertical().height(2)].spacing(12).width(Fill);

    if let Some(k) = aktives_kapitel {
        // Wortziel: Regler in 250er-Schritten statt Plus/Minus-Knöpfen
        let ziel_box = column![
            row![
                text("Persönliches Wortziel")
                    .size(12)
                    .color(thema::text_gedimmt()),
                space::horizontal(),
                text(format!("{} Wörter", k.wortziel))
                    .size(13)
                    .color(thema::text_hell()),
            ]
            .align_y(Alignment::Center),
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
        .spacing(6);

        // Status: Auswahlliste statt vier Knöpfen
        let status_box = column![
            text("Bearbeitungsstand")
                .size(12)
                .color(thema::text_gedimmt()),
liste_mit_pfeilen(
            pick_list(
                STATUS_WAHL,
                Some(StatusWahl(k.status)),
                { let f = status_fn.clone(); move |w: StatusWahl| f(w.0) }
            )
            .width(Fill)
            .text_size(12)
            .padding([6, 10])
            .style(thema::auswahl_stil)
            .menu_style(thema::auswahl_menue_stil),
            STATUS_WAHL.to_vec(),
            Some(StatusWahl(k.status)),
            move |w: StatusWahl| status_fn(w.0),
        ),
        ]
        .spacing(6);

        // Textstatistik
        let statistik_box = column![
            text("Textmetriken").size(12).color(thema::text_gedimmt()),
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
        .spacing(4);

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
            text("Gefundene Verweise (@kap:)")
                .size(12)
                .color(thema::text_gedimmt()),
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

    container(scrollable(inhalt).height(Fill))
        .width(thema::inspektor_breite())
        .padding(14)
        .style(thema::seitenleiste_stil)
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
