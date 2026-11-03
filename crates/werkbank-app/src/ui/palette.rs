//! # Befehlspalette
//!
//! Schwebendes Suchfenster (`Strg+K`): Befehle, Ansichten, Bücher und
//! Kapitel lassen sich tippend finden und per Pfeiltasten und Eingabe
//! ausführen. Ersetzt die Flut einzelner Knöpfe durch einen Ort für alles.

use crate::thema;
use iced::widget::{Id, column, container, mouse_area, opaque, row, scrollable, space, text, text_input};
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
        .size(16)
        .padding([10, 12]);

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
                    text(z.titel).size(13),
                    space::horizontal(),
                    text(z.hinweis).size(11).color(thema::text_gedimmt()),
                ]
                .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([7, 12])
            .style(thema::palette_eintrag_stil(i == auswahl))
            .on_press(on_waehlen(i)),
        );
    }

    let fenster = container(
        column![
            feld,
            scrollable(liste).height(Length::Shrink),
            text("↑↓ wählen   ⏎ ausführen   Esc schließen")
                .size(10)
                .color(thema::text_gedimmt()),
        ]
        .spacing(8),
    )
    .padding(12)
    .max_width(560)
    .style(thema::palette_box_stil);

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
