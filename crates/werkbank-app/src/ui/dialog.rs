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
        .padding(22)
        .max_width(breite)
        .style(thema::dialog_stil);
    abdunkeln(box_.into(), oben)
}

/// Abgedunkelter Hintergrund mit dem Dialog oben mittig; Klick daneben schließt.
fn abdunkeln<'a>(box_: Element<'a, Message>, oben: f32) -> Element<'a, Message> {
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

/// Einheitlicher Dialog: Titel, kurze Erklärung, Inhalt und eine abgesetzte Fußleiste mit
/// Tastenhinweis links und den Knöpfen rechts (Hauptaktion ganz rechts).
pub fn rahmen<'a>(
    titel: &'a str,
    erklaerung: Option<String>,
    inhalt: Option<Element<'a, Message>>,
    hinweis: &'a str,
    knoepfe: Vec<Element<'a, Message>>,
    breite: f32,
    oben: f32,
) -> Element<'a, Message> {
    let mut kopf = column![text(titel).size(17).font(thema::fett()).color(thema::text_hell())].spacing(6);
    if let Some(e) = erklaerung {
        kopf = kopf.push(text(e).size(12).color(thema::text_gedimmt()));
    }
    if let Some(i) = inhalt {
        kopf = kopf.push(space::vertical().height(6)).push(i);
    }
    let mut leiste = row![text(hinweis).size(11).color(thema::text_gedimmt()), space::horizontal()]
        .spacing(8)
        .align_y(Alignment::Center);
    for k in knoepfe {
        leiste = leiste.push(k);
    }
    let box_ = container(
        column![
            container(kopf).padding(Padding::from([22, 24]).bottom(18)),
            container(leiste).padding([12, 16]).width(Fill).style(thema::dialog_fuss_stil),
        ],
    )
    .max_width(breite)
    .style(thema::dialog_stil);
    abdunkeln(box_.into(), oben)
}

/// Hauptaktion eines Dialogs (gefüllt, Akzent).
pub fn haupt_knopf<'a>(name: &'a str, nachricht: Message) -> Element<'a, Message> {
    button(text(name).size(13)).padding([6, 16]).style(thema::primaer_knopf_stil).on_press(nachricht).into()
}

/// Nebenaktion eines Dialogs (z. B. Abbrechen).
pub fn neben_knopf<'a>(name: &'a str, nachricht: Message) -> Element<'a, Message> {
    button(text(name).size(13)).padding([6, 14]).style(thema::sekundaer_knopf_stil).on_press(nachricht).into()
}

/// Eingabedialog.
pub fn eingabe_ansicht<'a>(e: &'a Eingabe) -> Element<'a, Message> {
    let (erklaerung, aktion) = match e.zweck {
        EingabeZweck::NeuesKapitel => ("Das Kapitel kommt ans Ende des gewählten Teils. Den Titel kannst du später ändern.", "Anlegen"),
        EingabeZweck::NeuerTeil => ("Ein Teil fasst Kapitel zusammen und wird als eigene Datei gespeichert.", "Anlegen"),
        EingabeZweck::KapitelUmbenennen => ("Ändert die Überschrift des Kapitels im Text.", "Umbenennen"),
        EingabeZweck::TeilUmbenennen => ("Ändert den Titel des Buchteils.", "Umbenennen"),
    };
    rahmen(
        e.zweck.titel(),
        Some(erklaerung.to_string()),
        Some(
            text_input("Titel", &e.text)
                .id(eingabe_id())
                .on_input(Message::EingabeText)
                .on_submit(Message::EingabeOk)
                .size(15)
                .padding([9, 12])
                .into(),
        ),
        "↵ bestätigt · Esc bricht ab",
        vec![neben_knopf("Abbrechen", Message::DialogSchliessen), haupt_knopf(aktion, Message::EingabeOk)],
        480.0,
        110.0,
    )
}

/// Bestätigungsdialog.
pub fn dialog_ansicht<'a>(d: &'a Dialog) -> Element<'a, Message> {
    let knoepfe = d
        .knoepfe
        .iter()
        .enumerate()
        .map(|(i, (name, _, haupt))| {
            // Zerstörerische Aktionen stehen rot da, damit niemand sie versehentlich für „OK“ hält
            let gefaehrlich = ["Löschen", "Entfernen", "Verwerfen", "Trotzdem beenden"].contains(&name.as_str());
            if gefaehrlich {
                button(text(name).size(13)).padding([6, 14]).style(thema::gefahr_knopf_stil).on_press(Message::DialogAktion(i)).into()
            } else if *haupt {
                haupt_knopf(name, Message::DialogAktion(i))
            } else {
                neben_knopf(name, Message::DialogAktion(i))
            }
        })
        .collect();
    let hinweis = if d.knoepfe.len() <= 1 { "Esc schließt" } else { "Esc bricht ab" };
    rahmen(&d.titel, Some(d.text.clone()), None, hinweis, knoepfe, 480.0, 130.0)
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
    rahmen(
        "Verlauf",
        Some(format!(
            "Ein Klick holt den Text von „{kapitel}“ aus dem gewählten Stand zurück. \
             Er wird erst beim Speichern übernommen; der aktuelle Stand bleibt als Snapshot erhalten."
        )),
        Some(container(scrollable(zeilen).height(Length::Shrink)).max_height(420).into()),
        "Esc schließt",
        vec![neben_knopf("Schließen", Message::DialogSchliessen)],
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
    let feld = |name: &'a str, wert: &'a str, platzhalter: &'a str, f: MetaFeld| -> Element<'a, Message> {
        column![
            text(name).size(11).color(thema::text_gedimmt()),
            text_input(platzhalter, wert).on_input(move |t| Message::MetaAendern(f, t)).size(13).padding([7, 10]),
        ]
        .spacing(4)
        .width(Fill)
        .into()
    };
    let formular = column![
        feld("Titel", &m.titel, "Titel des Buches", MetaFeld::Titel),
        feld("Untertitel", &m.untertitel, "optional", MetaFeld::Untertitel),
        row![
            feld("Band", &m.band_bezeichnung, "z. B. Band 1", MetaFeld::Band),
            feld("Jahr", &m.jahr, "z. B. 2026", MetaFeld::Jahr),
        ]
        .spacing(12),
        row![
            feld("Autor", &m.autor, "Vor- und Nachname", MetaFeld::Autor),
            feld("ISBN (Druck)", m.isbn_print.as_deref().unwrap_or(""), "978-…", MetaFeld::Isbn),
        ]
        .spacing(12),
    ]
    .spacing(12);
    rahmen(
        "Buchdaten",
        Some(format!("Erscheinen auf Titelei, Cover und in den Metadaten von E-Book und Druck. Gespeichert in {ziel}.")),
        Some(formular.into()),
        "Esc bricht ab",
        vec![neben_knopf("Abbrechen", Message::DialogSchliessen), haupt_knopf("Speichern", Message::MetaSpeichern)],
        560.0,
        60.0,
    )
}
