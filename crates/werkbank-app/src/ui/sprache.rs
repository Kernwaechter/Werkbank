//! # Sprachprüfung im Inspektor
//!
//! Zeigt die LanguageTool-Hinweise zum offenen Kapitel. Ein Klick auf einen
//! Hinweis markiert die Stelle im Editor, Vorschläge ersetzen den Text direkt.

use crate::Message;
use crate::languagetool::PruefStatus;
use crate::thema;
use iced::widget::{column, container, row, space, text};
use crate::ui::fokus::{button};
use iced::{Alignment, Element, Fill};
use werkbank_kern::sprache::Treffer;

/// Höchstens so viele Hinweise werden gleichzeitig gezeichnet.
const MAX_ANZEIGE: usize = 40;

/// Baut den Abschnitt „Sprachprüfung“ für den Inspektor.
pub fn inspektor_box<'a>(
    aktiv: bool,
    treffer: &'a [Treffer],
    status: &'a PruefStatus,
) -> Element<'a, Message> {
    let (status_text, farbe) = if !aktiv {
        ("Ausgeschaltet – siehe Einstellungen".to_string(), thema::text_gedimmt())
    } else {
        match status {
            PruefStatus::Leer => ("Noch nicht geprüft".to_string(), thema::text_gedimmt()),
            PruefStatus::Laeuft => ("Prüfe …".to_string(), thema::warnung_orange()),
            PruefStatus::Fertig(0) => ("Keine Hinweise".to_string(), thema::erfolg_gruen()),
            PruefStatus::Fertig(n) => (format!("{n} Hinweise"), thema::warnung_orange()),
            PruefStatus::Fehler(e) => (e.clone(), thema::fehler_rot()),
        }
    };

    let kopf = row![
        text("Sprachprüfung").size(12).color(thema::text_gedimmt()),
        space::horizontal(),
        button(text("Prüfen").size(11))
            .padding([2, 6])
            .style(thema::link_knopf_stil)
            .on_press_maybe(aktiv.then_some(Message::KapitelPruefen)),
    ]
    .align_y(Alignment::Center);

    let mut inhalt = column![kopf, text(status_text).size(11).color(farbe)].spacing(4);

    for (i, t) in treffer.iter().enumerate().take(MAX_ANZEIGE) {
        inhalt = inhalt.push(karte(i, t));
    }
    if treffer.len() > MAX_ANZEIGE {
        inhalt = inhalt.push(
            text(format!("… und {} weitere", treffer.len() - MAX_ANZEIGE))
                .size(11)
                .color(thema::text_gedimmt()),
        );
    }

    inhalt.into()
}

fn karte(index: usize, t: &Treffer) -> Element<'_, Message> {
    let oben = button(
        column![
            text(format!("„{}“", t.fundstelle)).size(12).color(thema::warnung_orange()),
            text(&t.meldung).size(11).color(thema::text_hell()),
        ]
        .spacing(2)
        .width(Fill),
    )
    .padding([6, 8])
    .width(Fill)
    .style(thema::kapitel_karte_stil)
    .on_press(Message::TrefferAnspringen(index));

    let mut vorschlaege = row![].spacing(4);
    for (j, v) in t.ersetzungen.iter().enumerate().take(3) {
        vorschlaege = vorschlaege.push(
            button(text(v).size(11))
                .padding([2, 6])
                .style(thema::sekundaer_knopf_stil)
                .on_press(Message::TrefferErsetzen(index, j)),
        );
    }

    let mut aktionen = row![].spacing(4);
    if t.kategorie_id == "TYPOS" {
        aktionen = aktionen.push(
            button(text("Wort merken").size(10))
                .padding([1, 5])
                .style(thema::link_knopf_stil)
                .on_press(Message::TrefferWortMerken(index)),
        );
    }
    aktionen = aktionen.push(
        button(text("Regel ausschalten").size(10))
            .padding([1, 5])
            .style(thema::link_knopf_stil)
            .on_press(Message::TrefferRegelAus(index)),
    );

    container(column![oben, vorschlaege.wrap(), aktionen].spacing(4))
        .width(Fill)
        .into()
}
