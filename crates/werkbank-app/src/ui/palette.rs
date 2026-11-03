//! # Befehlspalette
//!
//! Schwebendes Suchfenster (`Strg+K`): Befehle, Ansichten, Bücher und
//! Kapitel lassen sich tippend finden und per Pfeiltasten und Eingabe
//! ausführen. Ersetzt die Flut einzelner Knöpfe durch einen Ort für alles.

use crate::thema;
use iced::widget::{Id, column, container, mouse_area, opaque, row, scrollable, text, text_input};
use crate::ui::fokus::{button};
use iced::{Alignment, Element, Fill, Length};

/// Kennung des Suchfelds, damit es beim Öffnen sofort den Fokus bekommt.
pub fn feld_id() -> Id {
    Id::new("befehlspalette-suche")
}

/// Ein Treffer der Palette: Titel, Zusatzhinweis (Kürzel oder Ort) und Auswahlzustand.
pub struct Zeile {
    pub titel: String,
    pub hinweis: String,
}

/// Rendert die Palette als Overlay über der restlichen Oberfläche.
pub fn ansicht<'a, Message: Clone + 'a>(
    suche: &'a str,
    treffer: Vec<Zeile>,
    auswahl: usize,
    on_suche: impl Fn(String) -> Message + 'a,
    on_ausfuehren: Message,
    on_waehlen: impl Fn(usize) -> Message + 'a,
    on_schliessen: Message,
) -> Element<'a, Message> {
    let feld = text_input("Befehl oder Kapitel eingeben …", suche)
        .id(feld_id())
        .on_input(on_suche)
        .on_submit(on_ausfuehren)
        .size(17)
        .padding([11, 14]);

    let mut liste = column![].spacing(2);
    if treffer.is_empty() {
        liste = liste.push(
            container(text("Keine Treffer").size(12).color(thema::text_gedimmt())).padding([8, 12]),
        );
    }
    for (i, z) in treffer.into_iter().enumerate() {
        liste = liste.push(
            button(
                row![
                    text(z.titel).size(13).width(Fill),
                    text(z.hinweis).size(11).color(thema::text_gedimmt()),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([7, 12])
            .style(thema::listen_eintrag_stil(i == auswahl))
            .on_press(on_waehlen(i)),
        );
    }

    let fenster = container(
        column![
            container(column![feld, container(scrollable(liste).height(Length::Shrink)).max_height(440)].spacing(10)).padding(12),
            container(
                text("↑↓ wählen   ⏎ ausführen   Esc schließt")
                    .size(11)
                    .color(thema::text_gedimmt()),
            )
            .padding([8, 14])
            .width(Fill)
            .style(thema::dialog_fuss_stil),
        ],
    )
    .max_width(600)
    .style(thema::dialog_stil);

    // Klick neben das Fenster schließt die Palette; das Fenster selbst schluckt Klicks.
    mouse_area(
        container(opaque(fenster))
            .width(Fill)
            .height(Fill)
            .padding(iced::Padding::ZERO.top(90))
            .center_x(Fill)
            .align_y(Alignment::Start)
            .style(thema::overlay_dimm_stil),
    )
    .on_press(on_schliessen)
    .into()
}
