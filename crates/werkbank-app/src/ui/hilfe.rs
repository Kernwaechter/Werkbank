//! # Handbuch und „Über die Werkbank“

use crate::Message;
use crate::thema;
use crate::ui::dialog::fenster;
use crate::ui::fokus::button;
use iced::widget::{column, container, row, scrollable, space, text};
use iced::{Alignment, Element, Fill, Length};

/// Das eingebaute Handbuch.
pub const HANDBUCH: &str = include_str!("../../../../assets/handbuch.md");
/// Lizenzen aller Fremdbausteine (erzeugt mit `packaging/lizenzen.py`).
pub const LIZENZEN: &str = include_str!("../../../../assets/lizenzen.txt");

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

/// Das Programmzeichen für „Über die Werkbank“.
const LOGO: &[u8] = include_bytes!("../../../../assets/icons/256.png");

pub fn ueber<'a>() -> Element<'a, Message> {
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
    let spenden_block = container(
        column![
            text("Die Werkbank ist für die nicht-kommerzielle Nutzung kostenlos. Wenn sie dir beim Schreiben hilft, freue ich mich über einen Kaffee. Damit unterstützt du die Weiterentwicklung.")
                .size(12)
                .color(thema::text_hell()),
            row![button(text("☕ Buy me a coffee").size(14).font(thema::fett())).padding([7, 18]).style(thema::kaffee_knopf_stil).on_press(Message::SpendenOeffnen)],
        ]
        .spacing(8),
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
                text("© 2026 Volker Dähn. Lizenz: CC BY-NC-SA 4.0.").size(12).color(thema::text_hell()),
                text("Der Lizenztext liegt dem Programm bei (LICENSE.de.md, LICENSE.en.md). Geschrieben in Rust mit Iced und Typst.")
                    .size(11)
                    .color(thema::text_gedimmt()),
            ]
            .spacing(3),
            spenden_block,
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
