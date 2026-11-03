//! # Navigationsleiste der Werkbank
//!
//! Eine einzige, ruhige Zeile am oberen Fensterrand: links der Band (mit Öffnen), in der Mitte die
//! Arbeitsbereiche als Segment-Schalter, rechts Speicherzustand, Suche (`Strg+K`) und – nur beim
//! Schreiben – die Ansichts-Schalter. Was gerade nicht gebraucht wird, erscheint nicht.

use crate::thema;
use iced::widget::{column, container, pick_list, row, space, text, tooltip};
use crate::ui::fokus::{button, liste_mit_pfeilen};
use iced::{Alignment, Element, Fill, Padding};

/// Die vier Hauptbereiche der Werkbank.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Arbeitsbereich {
    /// Drei-Spalten-Editor für den eigentlichen Schreibprozess.
    #[default]
    Schreiben,
    /// Build-Pipeline für Tolino, Amazon KDP und Webpublishing.
    Bauen,
    /// Sprachprüfung (Hunspell, LanguageTool) und Code-Prüfung der Buchbeispiele.
    Pruefen,
    /// Stephen-King-Tagespensum, Schreibtempo und Buchmetriken.
    Zahlen,
    /// Cover-Designer mit Vorlage, Vorschau und Export.
    Cover,
}

impl Arbeitsbereich {
    /// Liefert die Beschriftung für den Reiter.
    pub fn titel(&self) -> &'static str {
        match self {
            Self::Schreiben => "Schreiben",
            Self::Bauen => "Bauen",
            Self::Pruefen => "Prüfen",
            Self::Zahlen => "Zahlen",
            Self::Cover => "Cover",
        }
    }
}

/// Zustand der Ansichts-Schalter in der Kopfleiste.
#[derive(Debug, Clone, Copy)]
pub struct Ansicht {
    /// Bibliothek und Blattliste sichtbar.
    pub seitenleiste: bool,
    /// Live-Vorschau neben dem Editor sichtbar.
    pub vorschau: bool,
    /// Inspektor rechts sichtbar.
    pub inspektor: bool,
    /// Ablenkungsfreier Fokus-Modus aktiv.
    pub fokus: bool,
}

/// Nachrichten, die die Kopfleiste auslösen kann.
pub struct Aktionen<Message> {
    pub speichern: Message,
    pub oeffnen: Message,
    pub palette: Message,
    pub seitenleiste: Message,
    pub vorschau: Message,
    pub inspektor: Message,
    pub fokus: Message,
    pub einstellungen: Message,
}

/// Erzeugt die obere Navigationsleiste: eine ruhige Zeile mit Band links, den Bereichen als
/// Segment-Schalter in der Mitte und Suche, Speichern und Ansichts-Schaltern rechts.
#[allow(clippy::too_many_arguments)]
pub fn kopfleiste<'a, Message: Clone + 'a, Quelle: ToString + PartialEq + Clone + 'a>(
    aktiver_bereich: Arbeitsbereich,
    ist_geaendert: bool,
    ansicht: Ansicht,
    speicher_zeitstempel: &'a str,
    buecher: Vec<Quelle>,
    aktuelles_buch: Quelle,
    on_buch_wechsel: impl Fn(Quelle) -> Message + 'a,
    on_bereich_wechsel: impl Fn(Arbeitsbereich) -> Message + Copy + 'a,
    aktionen: Aktionen<Message>,
    test_hinweis: Option<(String, Message)>,
) -> Element<'a, Message> {
    let schreiben = aktiver_bereich == Arbeitsbereich::Schreiben;

    // Links: Seitenleisten-Schalter (nur beim Schreiben), Band und Öffnen
    let wechsel = std::rc::Rc::new(on_buch_wechsel);
    let wechsel2 = wechsel.clone();
    let optionen = buecher.clone();
    let buch_auswahl = liste_mit_pfeilen(
        pick_list(buecher, Some(aktuelles_buch.clone()), move |b| wechsel(b))
            .text_size(13)
            .padding([5, 10])
            .style(thema::band_auswahl_stil)
            .menu_style(thema::auswahl_menue_stil),
        optionen,
        Some(aktuelles_buch),
        move |b| wechsel2(b),
    );
    let mut links = row![].spacing(6).align_y(Alignment::Center);
    if schreiben {
        links = links.push(symbol_schalter("◧", ansicht.seitenleiste, "Bibliothek und Blattliste  ·  Strg+B", aktionen.seitenleiste));
    }
    links = links.push(buch_auswahl).push(tooltip_knopf(
        button(text("Öffnen …").size(12))
            .padding([5, 8])
            .style(thema::symbol_knopf_stil(false))
            .on_press(aktionen.oeffnen),
        "Ordner mit Buchprojekt öffnen  ·  Strg+O",
    ));

    // Mitte: die Arbeitsbereiche als Segment-Schalter
    let mut segmente = row![].spacing(2);
    for b in [Arbeitsbereich::Schreiben, Arbeitsbereich::Bauen, Arbeitsbereich::Pruefen, Arbeitsbereich::Zahlen, Arbeitsbereich::Cover] {
        segmente = segmente.push(
            button(text(b.titel()).size(13).center())
                .padding([4, 16])
                .style(thema::segment_stil(b == aktiver_bereich))
                .on_press(on_bereich_wechsel(b)),
        );
    }
    let mitte = container(segmente).padding(2).style(thema::segment_rahmen_stil);

    // Rechts: Hinweise, Speicherzustand, Suche, Ansichten, Einstellungen
    let mut rechts = row![].spacing(6).align_y(Alignment::Center);
    if let Some((beschriftung, nachricht)) = test_hinweis {
        rechts = rechts.push(
            button(text(beschriftung).size(11).color(thema::warnung_orange()))
                .padding([4, 8])
                .style(thema::symbol_knopf_stil(false))
                .on_press(nachricht),
        );
    }
    // Speichern: bei Änderungen ein deutlicher Knopf, sonst nur ein leiser Zustand
    let speichern: Element<'a, Message> = if ist_geaendert {
        tooltip_knopf(
            button(text("● Speichern").size(12))
                .padding([5, 12])
                .style(thema::primaer_knopf_stil)
                .on_press(aktionen.speichern),
            "Speichern und die alte Fassung als nummerierte Sicherung ablegen  ·  Strg+S",
        )
    } else {
        let status = if speicher_zeitstempel.is_empty() { "Gespeichert" } else { speicher_zeitstempel };
        tooltip_knopf(
            button(text(status).size(11).color(thema::text_gedimmt()).wrapping(iced::widget::text::Wrapping::None))
                .padding([5, 8])
                .style(thema::symbol_knopf_stil(false))
                .on_press(aktionen.speichern),
            "Alles gespeichert. Klick speichert erneut  ·  Strg+S",
        )
    };
    rechts = rechts.push(container(speichern).max_width(300));
    rechts = rechts.push(tooltip_knopf(
        button(
            row![text("⌕").size(14), text("Suchen").size(12), space::horizontal().width(18), text("Strg+K").size(10)]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .padding([4, 10])
        .style(thema::suchfeld_knopf_stil)
        .on_press(aktionen.palette),
        "Befehl, Kapitel oder Einstellung finden  ·  Strg+K",
    ));
    if schreiben {
        rechts = rechts
            .push(symbol_schalter("◫", ansicht.vorschau, "Live-Vorschau  ·  Strg+P", aktionen.vorschau))
            .push(symbol_schalter("◨", ansicht.inspektor, "Inspektor  ·  Strg+I", aktionen.inspektor))
            .push(symbol_schalter("◎", ansicht.fokus, "Fokus-Modus  ·  F11", aktionen.fokus));
    }
    rechts = rechts.push(symbol_schalter("⚙", false, "Einstellungen  ·  Strg+,", aktionen.einstellungen));

    let zeile = row![
        container(links).width(Fill),
        mitte,
        container(rechts).width(Fill).align_x(iced::alignment::Horizontal::Right),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    column![
        container(zeile).padding(Padding::from([7, 12])).width(Fill).style(thema::flaeche_stil),
        thema::trennlinie(),
    ]
    .into()
}

/// Umschalter als kompaktes Symbol mit Tooltip statt beschriftetem Knopf.
fn symbol_schalter<'a, Message: Clone + 'a>(
    symbol: &'static str,
    aktiv: bool,
    hinweis: &'static str,
    nachricht: Message,
) -> Element<'a, Message> {
    tooltip_knopf(
        button(text(symbol).size(16).center())
            .padding([2, 8])
            .style(thema::symbol_knopf_stil(aktiv))
            .on_press(nachricht),
        hinweis,
    )
}

/// Hängt einem Knopf eine Hinweisblase an.
pub fn tooltip_knopf<'a, Message: Clone + 'a>(
    knopf: impl Into<Element<'a, Message>>,
    hinweis: &'static str,
) -> Element<'a, Message> {
    tooltip(
        knopf,
        container(text(hinweis).size(11))
            .padding([4, 8])
            .style(thema::tooltip_stil),
        tooltip::Position::Bottom,
    )
    .gap(4)
    .into()
}
