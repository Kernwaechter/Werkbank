//! # Satzvorschau
//!
//! Zeigt das offene Kapitel so, wie es gesetzt wird: Markdown → Pandoc → Typst → Seiten
//! (ohne Pandoc setzt der eingebaute Übersetzer). Die Seiten erneuern sich kurz nach dem Tippen.

use crate::Message;
use crate::thema;
use crate::ui::fokus::button;
use iced::widget::{column, container, image, row, scrollable, space, text};
use iced::{Alignment, Element, Fill, Padding};

/// Zustand der Satzvorschau.
#[derive(Default)]
pub struct SatzVorschau {
    /// Gesetzte Seiten als Bilder.
    pub seiten: Vec<image::Handle>,
    /// Seitenzahl des ganzen Kapitels.
    pub seitenzahl: usize,
    /// Womit gesetzt wurde.
    pub quelle: String,
    /// Hinweise des Setzers.
    pub hinweise: Vec<String>,
    /// Fehlermeldung, falls nichts gesetzt werden konnte.
    pub fehler: Option<String>,
    /// Läuft gerade ein Satz?
    pub laeuft: bool,
    /// Laufende Nummer, damit nur das neueste Ergebnis zählt.
    pub version: u64,
    /// Stattdessen die schnelle Text-Vorschau zeigen.
    pub text_modus: bool,
}

pub fn ansicht<'a>(v: &'a SatzVorschau, pandoc_da: bool) -> Element<'a, Message> {
    // Zwei ruhige Zeilen statt einer gedrängten: oben Titel und Umschalter, darunter der Stand
    let kopf = column![
        row![
            thema::abschnitt_titel("Satzvorschau"),
            space::horizontal(),
            crate::ui::navigation::tooltip_knopf(
                button(text("Text").size(11).wrapping(iced::widget::text::Wrapping::None))
                    .padding([2, 8])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::VorschauText(true)),
                "Zur schnellen Text-Vorschau wechseln",
            ),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(if v.laeuft {
            "setze …".to_string()
        } else if v.quelle.is_empty() {
            "wird gesetzt …".to_string()
        } else {
            format!("{} · {} Seiten", v.quelle, v.seitenzahl)
        })
        .size(11)
        .color(if v.laeuft { thema::warnung_orange() } else { thema::text_gedimmt() }),
    ]
    .spacing(4);

    let mut seiten = column![].spacing(14).width(Fill);
    if let Some(f) = &v.fehler {
        seiten = seiten.push(text(format!("Nicht gesetzt: {f}")).size(12).color(thema::fehler_rot()));
    }
    if !pandoc_da {
        seiten = seiten.push(
            text("Pandoc fehlt: Es setzt der eingebaute Übersetzer. Für denselben Weg wie die Bauskripte Pandoc installieren (Einstellungen → Werkzeuge, Symbol ⓘ).")
                .size(11)
                .color(thema::warnung_orange()),
        );
    }
    for h in v.hinweise.iter().take(3) {
        seiten = seiten.push(text(h).size(10).color(thema::text_gedimmt()));
    }
    for s in &v.seiten {
        seiten = seiten.push(
            container(image(s.clone()).width(Fill))
                .padding(1)
                .style(thema::infobox_stil),
        );
    }
    if v.seitenzahl > v.seiten.len() {
        seiten = seiten.push(text(format!("… weitere {} Seiten werden nicht angezeigt", v.seitenzahl - v.seiten.len())).size(11).color(thema::text_gedimmt()));
    }

    container(column![container(kopf).padding(Padding::from([8, 12])), scrollable(container(seiten).padding(Padding::from([4, 12]))).height(Fill)])
        .width(Fill)
        .height(Fill)
        .style(thema::editor_bereich_stil)
        .into()
}
