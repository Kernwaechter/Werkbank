//! # Handbuch und „Über die Werkbank“

use crate::Message;
use crate::thema;
use crate::ui::dialog::fenster;
use crate::ui::fokus::button;
use iced::widget::{column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Fill, Length};
use werkbank_kern::lizenz::LizenzStatus;

/// Das eingebaute Handbuch.
pub const HANDBUCH: &str = include_str!("../../../../assets/handbuch.md");
/// Lizenzen aller Fremdbausteine (erzeugt mit `packaging/lizenzen.py`).
pub const LIZENZEN: &str = include_str!("../../../../assets/lizenzen.txt");
/// Öffentlicher Lizenzschlüssel (Platzhalter, bis einer eingerichtet ist).
pub const OEFFENTLICHER_SCHLUESSEL: &str = include_str!("../../../../assets/lizenz-oeffentlich.key");

pub fn handbuch() -> Element<'static, Message> {
    fenster(
        column![
            row![
                text("Handbuch").size(18).font(thema::fett()).color(thema::text_hell()),
                space::horizontal(),
                text("Esc schließt").size(11).color(thema::text_gedimmt()),
            ]
            .align_y(Alignment::Center),
            container(crate::ui::vorschau::lesetext(HANDBUCH, Message::Nichts))
                .height(Length::Fixed(620.0))
                .width(Fill),
        ]
        .spacing(10),
        940.0,
        30.0,
    )
}

/// Statuszeile der Lizenz (Text und Farbe), für „Über die Werkbank“ und die Einstellungen.
pub fn status_text(status: &LizenzStatus) -> (String, iced::Color) {
    match status {
        LizenzStatus::NichtEingerichtet => ("Die Lizenzprüfung ist in dieser Ausgabe nicht eingerichtet. Alle Funktionen sind frei.".to_string(), thema::text_gedimmt()),
        LizenzStatus::KeineLizenz => ("Noch nicht aktiviert. Trage E-Mail-Adresse und Lizenzschlüssel ein.".to_string(), thema::warnung_orange()),
        LizenzStatus::Gueltig { name, edition, bis } => (
            format!(
                "Lizenziert für {name}{}{}",
                if edition.is_empty() { String::new() } else { format!(" ({edition})") },
                bis.map(|d| format!(", gültig bis {}", d.format("%d.%m.%Y"))).unwrap_or_default()
            ),
            thema::erfolg_gruen(),
        ),
        LizenzStatus::Test { tage_uebrig } => (
            format!("Testversion: noch {tage_uebrig} {} bis zur Aktivierung.", if *tage_uebrig == 1 { "Tag" } else { "Tage" }),
            thema::warnung_orange(),
        ),
        LizenzStatus::TestAbgelaufen => ("Die Testzeit von vier Wochen ist vorbei.".to_string(), thema::fehler_rot()),
        LizenzStatus::Abgelaufen(d) => (format!("Die Lizenz ist am {} abgelaufen.", d.format("%d.%m.%Y")), thema::fehler_rot()),
        LizenzStatus::Ungueltig(f) => (format!("Lizenz ungültig: {f}"), thema::fehler_rot()),
    }
}

/// Das Programmzeichen für „Über die Werkbank“.
const LOGO: &[u8] = include_bytes!("../../../../assets/icons/256.png");

pub fn ueber<'a>(status: &'a LizenzStatus, email: &'a str, schluessel: &'a str, meldung: Option<&'a str>) -> Element<'a, Message> {
    let (text_status, farbe) = status_text(status);
    let anzahl = LIZENZEN.lines().count();
    let mut liste = column![].spacing(1);
    for z in LIZENZEN.lines().take(600) {
        let (name, lizenz) = z.split_once('\t').unwrap_or((z, ""));
        liste = liste.push(
            row![text(name).size(10).width(Length::FillPortion(2)), text(lizenz).size(10).color(thema::text_gedimmt()).width(Length::FillPortion(3))]
                .spacing(8),
        );
    }
    let link = |name: &'a str, adresse: &str| -> Element<'a, Message> {
        button(text(name).size(12))
            .padding([5, 12])
            .style(thema::sekundaer_knopf_stil)
            .on_press(Message::LinkOeffnen(adresse.to_string()))
            .into()
    };
    let kopf = row![
        iced::widget::image(iced::widget::image::Handle::from_bytes(LOGO)).width(76).height(76),
        column![
            text("Werkbank").size(30).font(thema::fett()).color(thema::text_hell()),
            text(format!("Version {}", env!("CARGO_PKG_VERSION"))).size(13).color(thema::text_gedimmt()),
            text("Schreibwerkzeug für technische Bücher").size(13).color(thema::text_hell()),
        ]
        .spacing(3),
    ]
    .spacing(20)
    .align_y(Alignment::Center);
    let verweise = row![
        link("Homepage", crate::HOMEPAGE),
        button(text("Handbuch").size(12)).padding([5, 12]).style(thema::sekundaer_knopf_stil).on_press(Message::HandbuchOeffnen),
        link("Kontakt", crate::KONTAKT),
    ]
    .spacing(8);
    let lizenz_block = container(
        column![
            row![
                text(text_status).size(12).color(farbe).width(Fill),
                button(text("Lizenzschlüssel kaufen").size(11))
                    .padding([4, 10])
                    .style(thema::primaer_knopf_stil)
                    .on_press(Message::KaufenOeffnen),
                button(text("Lizenzdatei wählen …").size(11))
                    .padding([4, 10])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::LizenzWaehlen),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text_input("E-Mail-Adresse des Kaufs", email).on_input(Message::LizenzEmail).size(13).padding(6),
            text_input("Lizenzschlüssel (WB1-…)", schluessel)
                .on_input(Message::LizenzSchluessel)
                .on_submit(Message::LizenzAktivieren)
                .size(13)
                .padding(6),
            row![
                button(text("Aktivieren").size(11)).padding([4, 12]).style(thema::primaer_knopf_stil).on_press(Message::LizenzAktivieren),
                text(meldung.unwrap_or("")).size(11).color(thema::text_gedimmt()),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(7),
    )
    .padding(14)
    .width(Fill)
    .style(thema::gruppe_stil);
    fenster(
        column![
            kopf,
            verweise,
            thema::trennlinie(),
            column![
                text("© 2026 Volker Dähn. Alle Rechte vorbehalten.").size(12).color(thema::text_hell()),
                text("Einzelplatzlizenz. Die Lizenzvereinbarung liegt dem Programm bei (LIZENZVEREINBARUNG.md). Geschrieben in Rust mit Iced und Typst.")
                    .size(11)
                    .color(thema::text_gedimmt()),
            ]
            .spacing(3),
            lizenz_block,
            text(format!("Open-Source-Bausteine ({anzahl}), jeweils unter ihrer eigenen Lizenz")).size(12).font(thema::fett()).color(thema::text_hell()),
            container(scrollable(liste).height(Length::Fixed(190.0))).padding(8).style(thema::codeblock_container_stil),
            text("Die Schriften des Systems werden verwendet, nicht mitgeliefert. LanguageTool und die Hunspell-Wörterbücher gehören nicht zur Werkbank und stehen unter eigenen Lizenzen.")
                .size(10)
                .color(thema::text_gedimmt()),
        ]
        .spacing(12),
        720.0,
        40.0,
    )
}

/// Sperrbildschirm nach der Testzeit: nur Kaufen, Aktivieren und Speichern-und-Beenden.
pub fn gesperrt<'a>(email: &'a str, schluessel: &'a str, meldung: Option<&'a str>, ungespeichert: bool) -> Element<'a, Message> {
    let box_ = container(
        column![
            text("Die Testzeit ist abgelaufen").size(22).font(thema::fett()).color(thema::text_hell()),
            text("Du hast die Werkbank vier Wochen lang ausprobiert. Mit einem Lizenzschlüssel arbeitest du ohne Einschränkung weiter. Deine Texte bleiben unverändert auf deinem Rechner.")
                .size(13)
                .color(thema::text_gedimmt()),
            row![
                button(text("Jetzt kaufen").size(13)).padding([6, 16]).style(thema::primaer_knopf_stil).on_press(Message::KaufenOeffnen),
                text("49 € einmalig · den Schlüssel bekommst du per E-Mail").size(12).color(thema::text_gedimmt()),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            text("Schon gekauft? Trage die E-Mail-Adresse des Kaufs und den Schlüssel ein.").size(12).color(thema::text_hell()),
            text_input("E-Mail-Adresse des Kaufs", email).on_input(Message::LizenzEmail).size(13).padding(7),
            text_input("Lizenzschlüssel (WB1-…)", schluessel)
                .on_input(Message::LizenzSchluessel)
                .on_submit(Message::LizenzAktivieren)
                .size(13)
                .padding(7),
            row![
                button(text("Aktivieren").size(13)).padding([6, 16]).style(thema::primaer_knopf_stil).on_press(Message::LizenzAktivieren),
                text(meldung.unwrap_or("")).size(12).color(thema::warnung_orange()),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            row![
                button(text(if ungespeichert { "Speichern und beenden" } else { "Beenden" }).size(12))
                    .padding([5, 12])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::BeendenSpeichern),
            ],
        ]
        .spacing(12),
    )
    .padding(22)
    .max_width(560)
    .style(thema::palette_box_stil);
    container(iced::widget::opaque(box_))
        .width(Fill)
        .height(Fill)
        .padding(iced::Padding::ZERO.top(90))
        .center_x(Fill)
        .align_y(Alignment::Start)
        .style(thema::overlay_dimm_stil)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eingebauter_schluessel_ist_brauchbar() {
        // Ein Schlüssel zur falschen Adresse muss „passt nicht“ ergeben, nicht „Schlüssel fehlerhaft“ oder „nicht eingerichtet“
        let status = werkbank_kern::lizenz::schluessel_pruefen(OEFFENTLICHER_SCHLUESSEL, "test@example.org", "WB1-AAAAA");
        match status {
            LizenzStatus::Ungueltig(f) => assert!(f.contains("passen nicht zusammen"), "{f}"),
            andere => panic!("unerwartet: {andere:?}"),
        }
    }
}
