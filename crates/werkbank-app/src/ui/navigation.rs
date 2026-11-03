//! # Navigationsleiste der Werkbank
//!
//! Die zweizeilige Leiste am oberen Fensterrand organisiert das Programm übersichtlich:
//! In der oberen Zeile befinden sich Logo, Buchauswahl, Öffnen, Speicherstatus und Lizenzhinweis;
//! in der unteren Zeile der Wechsel zwischen den Kernbereichen (Schreiben, Bauen, Prüfen, Zahlen, Cover),
//! die Befehlspalette (`Strg+K`) sowie die Ansichts-Schalter.

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

/// Erzeugt die obere Navigationsleiste.
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
    // Logo und Buchauswahl links
    let wechsel = std::rc::Rc::new(on_buch_wechsel);
    let wechsel2 = wechsel.clone();
    let optionen = buecher.clone();
    let buch_auswahl = liste_mit_pfeilen(
        pick_list(buecher, Some(aktuelles_buch.clone()), move |b| wechsel(b))
            .text_size(12)
            .padding([4, 8])
            .style(thema::auswahl_stil)
            .menu_style(thema::auswahl_menue_stil),
        optionen,
        Some(aktuelles_buch),
        move |b| wechsel2(b),
    );

    let logo_bereich = row![
        text("WERKBANK").size(17).color(thema::akzent()),
        buch_auswahl,
        tooltip_knopf(
            button(text("Öffnen …").size(12))
                .padding([4, 8])
                .style(thema::symbol_knopf_stil(false))
                .on_press(aktionen.oeffnen),
            "Ordner mit Buchprojekt öffnen  ·  Strg+O",
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // Haupt-Reiter in der Mitte
    let tabs = row![
        bereichs_tab(
            Arbeitsbereich::Schreiben,
            aktiver_bereich,
            on_bereich_wechsel
        ),
        bereichs_tab(Arbeitsbereich::Bauen, aktiver_bereich, on_bereich_wechsel),
        bereichs_tab(Arbeitsbereich::Pruefen, aktiver_bereich, on_bereich_wechsel),
        bereichs_tab(Arbeitsbereich::Zahlen, aktiver_bereich, on_bereich_wechsel),
        bereichs_tab(Arbeitsbereich::Cover, aktiver_bereich, on_bereich_wechsel),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    // Speichern: ein deutlicher Knopf, der immer da ist. Jedes Speichern legt die alte Fassung
    // als nummerierte Sicherung ab; der Status daneben sagt, was zuletzt passiert ist.
    let (speicher_text, speicher_farbe) = if ist_geaendert {
        ("● Ungespeichert".to_string(), thema::warnung_orange())
    } else if !speicher_zeitstempel.is_empty() {
        (speicher_zeitstempel.to_string(), thema::erfolg_gruen())
    } else {
        ("Gespeichert".to_string(), thema::erfolg_gruen())
    };

    let status = text(speicher_text).size(11).color(speicher_farbe);
    let speichern_knopf = tooltip_knopf(
        button(text("Speichern").size(12))
            .padding([5, 14])
            .style(if ist_geaendert { thema::primaer_knopf_stil } else { thema::sekundaer_knopf_stil })
            .on_press(aktionen.speichern),
        "Speichern und die alte Fassung als nummerierte Sicherung ablegen  ·  Strg+S",
    );

    // Befehlspalette als Suchfeld: der schnellste Weg zu jeder Funktion
    let palette = button(
        row![
            text("Befehl oder Kapitel suchen …").size(12),
            space::horizontal(),
            text("Strg+K").size(11),
        ]
        .align_y(Alignment::Center),
    )
    .padding([5, 10])
    .width(250)
    .style(thema::suchfeld_knopf_stil)
    .on_press(aktionen.palette);

    let schalter = row![
        symbol_schalter(
            "◧",
            ansicht.seitenleiste,
            "Seitenleisten  ·  Strg+B",
            aktionen.seitenleiste
        ),
        symbol_schalter(
            "◫",
            ansicht.vorschau,
            "Live-Vorschau  ·  Strg+P",
            aktionen.vorschau
        ),
        symbol_schalter(
            "◨",
            ansicht.inspektor,
            "Details  ·  Strg+I",
            aktionen.inspektor
        ),
        symbol_schalter("◎", ansicht.fokus, "Fokus-Modus  ·  F11", aktionen.fokus),
        symbol_schalter("⚙", false, "Einstellungen  ·  Strg+,", aktionen.einstellungen),
    ]
    .spacing(2)
    .align_y(Alignment::Center);

    // Obere Zeile (Zeile 1): Projekt- und Dateiebene
    // Links: Logo ("WERKBANK"), Buchauswahl, Öffnen-Knopf
    // Rechts: Testversionshinweis (falls vorhanden), Speicherstatus, Speichern-Knopf
    let mut zeile1_rechts = row![].spacing(10).align_y(Alignment::Center);
    if let Some((beschriftung, nachricht)) = test_hinweis {
        zeile1_rechts = zeile1_rechts.push(
            button(text(beschriftung).size(11).color(thema::warnung_orange()))
                .padding([3, 8])
                .style(thema::sekundaer_knopf_stil)
                .on_press(nachricht),
        );
    }
    let zeile1_rechts = zeile1_rechts.push(status).push(speichern_knopf);

    let zeile1 = row![
        logo_bereich,
        space::horizontal(),
        zeile1_rechts,
    ]
    .align_y(Alignment::Center)
    .width(Fill);

    // Untere Zeile (Zeile 2): Arbeitsbereiche & Werkzeuge
    // Links: Haupt-Reiter (Schreiben, Bauen, Prüfen, Zahlen, Cover)
    // Rechts: Befehlspalette / Suchfeld (Strg+K) und Ansichtsschalter
    let zeile2_rechts = row![
        palette,
        schalter,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let zeile2 = row![
        tabs,
        space::horizontal(),
        zeile2_rechts,
    ]
    .align_y(Alignment::Center)
    .width(Fill);

    container(
        column![
            zeile1,
            zeile2,
        ]
        .spacing(6)
        .width(Fill),
    )
    .padding(Padding::from([6, 14]))
    .style(thema::kopfleiste_stil)
    .width(Fill)
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

/// Hilfsfunktion zum Rendern eines einzelnen Tabs.
fn bereichs_tab<'a, Message: Clone + 'a>(
    bereich: Arbeitsbereich,
    aktiver_bereich: Arbeitsbereich,
    on_click: impl Fn(Arbeitsbereich) -> Message + 'a,
) -> Element<'a, Message> {
    let aktiv = bereich == aktiver_bereich;
    button(text(bereich.titel()).size(13))
        .padding([6, 14])
        .style(thema::tab_knopf_stil(aktiv))
        .on_press(on_click(bereich))
        .into()
}
