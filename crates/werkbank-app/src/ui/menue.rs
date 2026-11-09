//! # Menüleiste
//!
//! Iced kennt keine Menüleiste. Die Titel (Datei, Bearbeiten, …) sind Knöpfe in einer eigenen
//! Zeile; ein offenes Menü zeichnet [`auswahl`] als schwebende Liste unter dem Titel. Die Titel
//! haben feste Breiten, daraus ergibt sich die Stelle, an der die Liste hängt. Ein Klick daneben
//! oder `Esc` schließt; fährt die Maus bei offenem Menü über einen anderen Titel, wechselt es.

use crate::Message;
use crate::thema;
use crate::ui::fokus::button;
use iced::widget::{column, container, mouse_area, opaque, row, space, text};
use iced::{Alignment, Element, Fill, Length, Padding};

/// Höhe der Leiste in Pixeln; die Menüliste beginnt direkt darunter.
pub const HOEHE: f32 = 30.0;
const RAND: f32 = 8.0;
const ABSTAND: f32 = 2.0;
const LISTE_BREITE: f32 = 330.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Menue {
    Datei,
    Bearbeiten,
    Ansicht,
    Buch,
    Hilfe,
}

impl Menue {
    pub const ALLE: [Menue; 5] = [Menue::Datei, Menue::Bearbeiten, Menue::Ansicht, Menue::Buch, Menue::Hilfe];

    pub fn titel(self) -> &'static str {
        match self {
            Self::Datei => "Datei",
            Self::Bearbeiten => "Bearbeiten",
            Self::Ansicht => "Ansicht",
            Self::Buch => "Buch",
            Self::Hilfe => "Hilfe",
        }
    }

    /// Feste Breite des Titelknopfs.
    fn breite(self) -> f32 {
        self.titel().chars().count() as f32 * 7.6 + 24.0
    }

    /// Abstand des Titels vom linken Fensterrand.
    fn links(self) -> f32 {
        let mut x = RAND;
        for m in Self::ALLE {
            if m == self {
                break;
            }
            x += m.breite() + ABSTAND;
        }
        x
    }
}

/// Ein Eintrag der Menüliste.
#[allow(clippy::large_enum_variant)]
pub enum Punkt {
    Aktion {
        name: String,
        kuerzel: String,
        nachricht: Message,
        /// `Some(an)`: Schalter mit Häkchen.
        haken: Option<bool>,
        gefaehrlich: bool,
    },
    Trenner,
}

impl Punkt {
    pub fn aktion(name: &str, kuerzel: impl Into<String>, nachricht: Message) -> Self {
        Self::Aktion { name: name.to_string(), kuerzel: kuerzel.into(), nachricht, haken: None, gefaehrlich: false }
    }

    pub fn schalter(name: &str, kuerzel: impl Into<String>, an: bool, nachricht: Message) -> Self {
        Self::Aktion { name: name.to_string(), kuerzel: kuerzel.into(), nachricht, haken: Some(an), gefaehrlich: false }
    }

    pub fn gefahr(name: &str, kuerzel: impl Into<String>, nachricht: Message) -> Self {
        Self::Aktion { name: name.to_string(), kuerzel: kuerzel.into(), nachricht, haken: None, gefaehrlich: true }
    }
}

/// Die Titelzeile über der Kopfleiste.
pub fn leiste<'a>(offen: Option<Menue>) -> Element<'a, Message> {
    let mut titel = row![].spacing(ABSTAND).align_y(Alignment::Center);
    for m in Menue::ALLE {
        let knopf = button(text(m.titel()).size(13).center())
            .width(Length::Fixed(m.breite()))
            .padding([3, 0])
            .style(thema::tab_knopf_stil(offen == Some(m)))
            .on_press(Message::MenueUmschalten(m));
        titel = titel.push(mouse_area(knopf).on_enter(Message::MenueUeber(m)));
    }
    column![
        container(titel)
            .padding(Padding::from([0, RAND as u16]))
            .height(HOEHE)
            .width(Fill)
            .align_y(Alignment::Center)
            .style(thema::flaeche_stil),
        thema::trennlinie(),
    ]
    .into()
}

/// Die schwebende Liste unter dem Titel. Die Leiste selbst bleibt frei, damit der Wechsel per Maus funktioniert.
pub fn auswahl<'a>(menue: Menue, punkte: Vec<Punkt>) -> Element<'a, Message> {
    let mut liste = column![].spacing(1).width(Fill);
    for p in punkte {
        match p {
            Punkt::Trenner => liste = liste.push(container(thema::trennlinie()).padding([4, 6])),
            Punkt::Aktion { name, kuerzel, nachricht, haken, gefaehrlich } => {
                let farbe = if gefaehrlich { thema::fehler_rot() } else { thema::text_hell() };
                let marke = match haken {
                    Some(true) => "✓",
                    _ => "",
                };
                liste = liste.push(
                    button(
                        row![
                            text(marke).size(12).color(thema::akzent()).width(16),
                            text(name).size(13).color(farbe).width(Fill),
                            text(kuerzel).size(11).color(thema::text_gedimmt()),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .width(Fill)
                    .padding([5, 10])
                    .style(thema::listen_eintrag_stil(false))
                    .on_press(Message::MenueAktion(Box::new(nachricht))),
                );
            }
        }
    }
    let box_ = container(liste).padding(5).width(Length::Fixed(LISTE_BREITE)).style(thema::palette_box_stil);
    // Oben bleibt die Leiste unberührt (Klicks dort erreichen die Titel); der Rest fängt Klicks zum Schließen ab.
    column![
        space::vertical().height(HOEHE + 1.0),
        mouse_area(
            container(opaque(box_))
                .width(Fill)
                .height(Fill)
                .padding(Padding::ZERO.left(menue.links()))
                .align_x(iced::alignment::Horizontal::Left)
                .align_y(Alignment::Start),
        )
        .on_press(Message::MenueSchliessen),
    ]
    .width(Fill)
    .height(Fill)
    .into()
}
