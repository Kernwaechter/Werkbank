//! Kleine Bausteine für Formulare: beschrifteter Regler und Auswahlliste, beide mit Tastaturbedienung.

use crate::Message;
use crate::thema;
use crate::ui::fokus::{liste_mit_pfeilen, regler_mit_pfeilen};
use iced::widget::{column, pick_list, row, slider, text};
use iced::{Alignment, Element};

/// Regler mit Beschriftung und Wert; `aendern` bekommt den neuen Wert.
pub fn regler<'a>(
    name: &'a str,
    wert: u32,
    min: u32,
    max: u32,
    einheit: &'a str,
    aendern: impl Fn(u32) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    column![
        row![text(name).size(11).color(thema::text_gedimmt()), text(format!("{wert} {einheit}")).size(11).color(thema::text_hell())].spacing(8),
        regler_mit_pfeilen(
            slider(min..=max, wert.clamp(min, max), aendern).style(thema::regler_stil),
            wert,
            1,
            min,
            max,
            aendern,
        ),
    ]
    .spacing(3)
    .align_x(Alignment::Start)
    .into()
}

/// Beschriftete Auswahlliste mit festen Einträgen.
pub fn auswahl_mit_pfeilen<'a>(
    name: &'a str,
    optionen: &'static [&'static str],
    aktuell: &'static str,
    waehlen: impl Fn(&'static str) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    column![
        text(name).size(11).color(thema::text_gedimmt()),
        liste_mit_pfeilen(
            pick_list(optionen, Some(aktuell), waehlen)
                .text_size(12)
                .padding([5, 8])
                .style(thema::auswahl_stil)
                .menu_style(thema::auswahl_menue_stil),
            optionen.to_vec(),
            Some(aktuell),
            waehlen,
        ),
    ]
    .spacing(3)
    .into()
}
