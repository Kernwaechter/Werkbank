//! # Dialoge
//!
//! Kleine schwebende Fenster: Texteingabe (neues Kapitel, Umbenennen),
//! Bestätigung (Löschen, Beenden) und der Verlauf mit früheren Ständen.

use crate::Message;
use crate::thema;
use iced::widget::{Id, column, container, mouse_area, opaque, row, scrollable, space, text, text_input};
use crate::ui::fokus::{button};
use iced::{Alignment, Element, Fill, Length, Padding};
use werkbank_kern::speicher::Snapshot;

/// Wofür der Eingabedialog gerade dient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EingabeZweck {
    NeuesKapitel,
    NeuerTeil,
    KapitelUmbenennen,
    TeilUmbenennen,
}

impl EingabeZweck {
    pub fn titel(self) -> &'static str {
        match self {
            Self::NeuesKapitel => "Neues Kapitel",
            Self::NeuerTeil => "Neuer Buchteil",
            Self::KapitelUmbenennen => "Kapitel umbenennen",
            Self::TeilUmbenennen => "Teil umbenennen",
        }
    }
}

/// Zustand eines Eingabedialogs.
#[derive(Debug, Clone)]
pub struct Eingabe {
    pub zweck: EingabeZweck,
    pub text: String,
}

/// Bestätigungsdialog mit frei wählbaren Knöpfen: (Beschriftung, Nachricht, hervorgehoben).
#[derive(Debug, Clone)]
pub struct Dialog {
    pub titel: String,
    pub text: String,
    pub knoepfe: Vec<(String, Message, bool)>,
}

/// Kennung des Eingabefelds (für den Fokus beim Öffnen).
pub fn eingabe_id() -> Id {
    Id::new("dialog-eingabe")
}

/// Rahmen für ein schwebendes Fenster; Klick daneben schließt (`Message::DialogSchliessen`).
pub fn fenster<'a>(inhalt: impl Into<Element<'a, Message>>, breite: f32, oben: f32) -> Element<'a, Message> {
    let box_ = container(inhalt)
        .padding(18)
        .max_width(breite)
        .style(thema::palette_box_stil);
    mouse_area(
        container(opaque(box_))
            .width(Fill)
            .height(Fill)
            .padding(Padding::ZERO.top(oben))
            .center_x(Fill)
            .align_y(Alignment::Start)
            .style(thema::overlay_dimm_stil),
    )
    .on_press(Message::DialogSchliessen)
    .into()
}

/// Eingabedialog.
pub fn eingabe_ansicht<'a>(e: &'a Eingabe) -> Element<'a, Message> {
    fenster(
        column![
            text(e.zweck.titel()).size(16).color(thema::text_hell()),
            text_input("Titel eingeben …", &e.text)
                .id(eingabe_id())
                .on_input(Message::EingabeText)
                .on_submit(Message::EingabeOk)
                .size(15)
                .padding([8, 10]),
            row![
                text("Eingabetaste bestätigt · Esc bricht ab").size(11).color(thema::text_gedimmt()),
                space::horizontal(),
                button(text("OK").size(12))
                    .padding([5, 16])
                    .style(thema::primaer_knopf_stil)
                    .on_press(Message::EingabeOk),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(10),
        460.0,
        120.0,
    )
}

/// Bestätigungsdialog.
pub fn dialog_ansicht<'a>(d: &'a Dialog) -> Element<'a, Message> {
    let mut knoepfe = row![space::horizontal()].spacing(8);
    for (i, (name, _, haupt)) in d.knoepfe.iter().enumerate() {
        knoepfe = knoepfe.push(
            button(text(name).size(12))
                .padding([6, 14])
                .style(if *haupt { thema::primaer_knopf_stil } else { thema::sekundaer_knopf_stil })
                .on_press(Message::DialogAktion(i)),
        );
    }
    fenster(
        column![
            text(&d.titel).size(16).color(thema::text_hell()),
            text(&d.text).size(12).color(thema::text_gedimmt()),
            knoepfe,
        ]
        .spacing(12),
        480.0,
        140.0,
    )
}

/// Verlauf: frühere Stände der Datei, aus der das Kapitel stammt.
pub fn verlauf_ansicht<'a>(kapitel: &'a str, liste: &'a [Snapshot]) -> Element<'a, Message> {
    let mut zeilen = column![].spacing(2);
    if liste.is_empty() {
        zeilen = zeilen.push(
            text("Noch keine früheren Stände. Sie entstehen beim Speichern, sobald sich die Datei ändert.")
                .size(12)
                .color(thema::text_gedimmt()),
        );
    }
    for (i, s) in liste.iter().enumerate() {
        zeilen = zeilen.push(
            button(
                row![
                    text(&s.zeit).size(13),
                    space::horizontal(),
                    text(format!("{:.0} KB", s.groesse as f32 / 1024.0)).size(11).color(thema::text_gedimmt()),
                ]
                .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([6, 10])
            .style(thema::palette_eintrag_stil(false))
            .on_press(Message::VerlaufWiederherstellen(i)),
        );
    }
    fenster(
        column![
            text("Verlauf").size(16).color(thema::text_hell()),
            text(format!(
                "Ein Klick holt den Text von „{kapitel}“ aus dem gewählten Stand zurück. \
                 Er wird erst beim Speichern übernommen; der aktuelle Stand bleibt als Snapshot erhalten."
            ))
            .size(11)
            .color(thema::text_gedimmt()),
            scrollable(zeilen).height(Length::Shrink),
        ]
        .spacing(10),
        560.0,
        80.0,
    )
}

/// Feld des Buchdaten-Dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetaFeld {
    Titel,
    Untertitel,
    Band,
    Autor,
    Jahr,
    Isbn,
}

/// Dialog für Titel, Untertitel, Band, Autor, Jahr und ISBN.
pub fn meta_ansicht<'a>(m: &'a werkbank_kern::BandMetadaten, ziel: &'a str) -> Element<'a, Message> {
    let feld = |name: &'a str, wert: &'a str, f: MetaFeld| -> Element<'a, Message> {
        column![
            text(name).size(11).color(thema::text_gedimmt()),
            text_input("", wert).on_input(move |t| Message::MetaAendern(f, t)).size(13).padding(6),
        ]
        .spacing(3)
        .into()
    };
    fenster(
        column![
            text("Buchdaten").size(16).color(thema::text_hell()),
            feld("Titel", &m.titel, MetaFeld::Titel),
            feld("Untertitel", &m.untertitel, MetaFeld::Untertitel),
            feld("Band", &m.band_bezeichnung, MetaFeld::Band),
            feld("Autor", &m.autor, MetaFeld::Autor),
            feld("Jahr", &m.jahr, MetaFeld::Jahr),
            feld("ISBN Print", m.isbn_print.as_deref().unwrap_or(""), MetaFeld::Isbn),
            row![
                text(format!("Gespeichert in {ziel}")).size(10).color(thema::text_gedimmt()),
                space::horizontal(),
                button(text("Speichern").size(12))
                    .padding([5, 16])
                    .style(thema::primaer_knopf_stil)
                    .on_press(Message::MetaSpeichern),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(8),
        520.0,
        60.0,
    )
}
