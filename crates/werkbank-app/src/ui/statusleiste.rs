//! # Statusleiste
//!
//! Schmale Zeile am unteren Fensterrand. Links der Speicherstand mit der letzten Meldung der
//! Werkbank, rechts (nur beim Schreiben) Cursorstelle und Wortzahl mit Ziel, dann der Zustand der
//! Sprachprüfung und die Lizenz. Alles Anklickbare führt dorthin, wo man es ändert.

use crate::Message;
use crate::thema;
use crate::ui::fokus::button;
use iced::widget::{container, row, space, text};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Padding};

const HOEHE: f32 = 26.0;

/// Cursorstelle, Wörter und Ziel des offenen Kapitels.
pub struct Schreibstand {
    pub zeile: usize,
    pub spalte: usize,
    pub woerter: usize,
    /// Wortziel des Kapitels; 0 = keines.
    pub ziel: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pruefung {
    Aus,
    Offline,
    Laeuft,
    Bereit,
    Hinweise(usize),
}

pub struct Daten<'a> {
    pub meldung: &'a str,
    pub ungespeichert: bool,
    /// Demo-Buch: wird nicht auf die Platte geschrieben.
    pub demo: bool,
    pub schreibstand: Option<Schreibstand>,
    pub pruefung: Pruefung,
    /// Text und ob er hervorgehoben (Testversion) werden soll.
    pub lizenz: Option<(String, bool)>,
}

fn trenner<'a>() -> Element<'a, Message> {
    container(space::horizontal().width(1).height(12))
        .style(|_| container::Style { background: Some(Background::Color(thema::linie_farbe())), ..Default::default() })
        .into()
}

fn punkt<'a>(farbe: Color) -> Element<'a, Message> {
    container(space::horizontal().width(7).height(7))
        .style(move |_| container::Style {
            background: Some(Background::Color(farbe)),
            border: Border { radius: 4.0.into(), ..Default::default() },
            ..Default::default()
        })
        .into()
}

/// Ein klickbarer Eintrag der Leiste: flach, erst beim Überfahren als Fläche sichtbar.
fn eintrag<'a>(inhalt: impl Into<Element<'a, Message>>, nachricht: Message) -> Element<'a, Message> {
    button(inhalt)
        .padding([2, 7])
        .style(thema::symbol_knopf_stil(false))
        .on_press(nachricht)
        .into()
}

pub fn leiste<'a>(d: &Daten<'a>) -> Element<'a, Message> {
    // Links: Speicherstand und letzte Meldung
    let (farbe, name, nachricht) = if d.demo {
        (thema::text_gedimmt(), "Demo-Buch", Message::OrdnerDialog)
    } else if d.ungespeichert {
        (thema::warnung_orange(), "Ungespeichert", Message::Speichern)
    } else {
        (thema::erfolg_gruen(), "Gespeichert", Message::VerlaufOeffnen)
    };
    let links = row![
        eintrag(row![punkt(farbe), text(name).size(11).color(thema::text_hell())].spacing(6).align_y(Alignment::Center), nachricht),
        container(text(d.meldung.to_string()).size(11).color(thema::text_gedimmt()).wrapping(iced::widget::text::Wrapping::None))
            .clip(true)
            .width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    // Rechts
    let mut rechts = row![].spacing(8).align_y(Alignment::Center);
    if let Some(s) = &d.schreibstand {
        rechts = rechts.push(text(format!("Zeile {}, Spalte {}", s.zeile, s.spalte)).size(11).color(thema::text_gedimmt()));
        let mut woerter = row![text(if s.ziel > 0 { format!("{} / {} Wörter", thema::tausender(s.woerter), thema::tausender(s.ziel)) } else { format!("{} Wörter", thema::tausender(s.woerter)) })
            .size(11)
            .color(thema::text_hell())]
        .spacing(7)
        .align_y(Alignment::Center);
        if s.ziel > 0 {
            let anteil = (s.woerter as f32 / s.ziel as f32).clamp(0.0, 1.0);
            let fuell = if s.woerter >= s.ziel { thema::erfolg_gruen() } else { thema::akzent() };
            woerter = woerter.push(
                container(container(space::horizontal().height(4)).width(Length::Fixed(52.0 * anteil)).style(move |_| container::Style {
                    background: Some(Background::Color(fuell)),
                    border: Border { radius: 2.0.into(), ..Default::default() },
                    ..Default::default()
                }))
                .width(52)
                .style(|_| container::Style {
                    background: Some(Background::Color(thema::linie_farbe())),
                    border: Border { radius: 2.0.into(), ..Default::default() },
                    ..Default::default()
                }),
            );
        }
        rechts = rechts.push(trenner()).push(woerter);
    }
    let (pf, pt) = match d.pruefung {
        Pruefung::Aus => (thema::text_gedimmt(), "Sprachprüfung aus".to_string()),
        Pruefung::Offline => (thema::warnung_orange(), "Sprachprüfung nicht erreichbar".to_string()),
        Pruefung::Laeuft => (thema::akzent(), "Sprachprüfung läuft …".to_string()),
        Pruefung::Bereit => (thema::erfolg_gruen(), "Sprachprüfung bereit".to_string()),
        Pruefung::Hinweise(0) => (thema::erfolg_gruen(), "Keine Hinweise".to_string()),
        Pruefung::Hinweise(n) => (thema::warnung_orange(), format!("{n} {}", if n == 1 { "Hinweis" } else { "Hinweise" })),
    };
    rechts = rechts.push(trenner()).push(eintrag(
        row![punkt(pf), text(pt).size(11).color(thema::text_hell())].spacing(6).align_y(Alignment::Center),
        Message::EinstellungenOeffnen,
    ));
    if let Some((t, hervor)) = &d.lizenz {
        let farbe = if *hervor { thema::warnung_orange() } else { thema::text_gedimmt() };
        rechts = rechts.push(trenner()).push(eintrag(text(t.clone()).size(11).color(farbe), Message::UeberOeffnen));
    }

    iced::widget::column![
        thema::trennlinie(),
        container(row![links, rechts].spacing(16).align_y(Alignment::Center))
            .padding(Padding::from([0, 10]))
            .height(HOEHE)
            .width(Fill)
            .align_y(Alignment::Center)
            .style(thema::bibliothek_stil),
    ]
    .into()
}
