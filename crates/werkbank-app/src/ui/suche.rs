//! # Suchen und Ersetzen
//!
//! Schwebendes Fenster (`Strg+F` im Kapitel, `Strg+Umschalt+F` in allen Bänden des Buchs).
//! Die Trefferliste zeigt jede Zeile vorher und nachher; ein Klick springt an die Stelle,
//! `F3` geht zum nächsten Treffer. „Alle ersetzen“ verlangt eine zweite Bestätigung mit der Anzahl.

use crate::Message;
use crate::thema;
use crate::ui::dialog::fenster;
use crate::ui::fokus::{button, toggler};
use iced::widget::{container, Id, column, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Fill, Length};
use werkbank_kern::suche::{BandFund, Fund, Optionen};

/// Wo gesucht wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Umfang {
    /// Nur im geöffneten Kapitel.
    #[default]
    Kapitel,
    /// In allen Dateien des geöffneten Bandes.
    Band,
    /// In allen Bänden des Buchs (Ordner nebeneinander).
    Buch,
}

/// Zustand der Suche.
#[derive(Debug, Clone, Default)]
pub struct Suche {
    pub muster: String,
    pub ersatz: String,
    pub umfang: Umfang,
    pub gross_klein: bool,
    pub ganzes_wort: bool,
    pub regex: bool,
    /// Funde im geöffneten Band (im Speicher, auch ungespeicherte Änderungen).
    pub funde: Vec<Fund>,
    /// Funde in den anderen Bänden (von der Festplatte gelesen).
    pub andere: Vec<BandFund>,
    pub auswahl: usize,
    /// Fehler im Suchausdruck.
    pub fehler: Option<String>,
    /// „Alle ersetzen“ wartet auf die zweite Bestätigung.
    pub bestaetigen: bool,
}

impl Suche {
    pub fn optionen(&self) -> Optionen {
        Optionen { gross_klein: self.gross_klein, ganzes_wort: self.ganzes_wort, regex: self.regex }
    }

    /// Anzahl aller Treffer.
    pub fn anzahl(&self) -> usize {
        self.funde.len() + self.andere.len()
    }

    /// Anzahl der Dateien (Kapitelgruppen), in denen Treffer liegen.
    pub fn kapitel_anzahl(&self) -> usize {
        let mut ids: Vec<(&str, &str)> = self.funde.iter().map(|f| ("", f.kapitel_id.as_str())).collect();
        ids.extend(self.andere.iter().map(|b| (b.band.as_str(), b.fund.kapitel_id.as_str())));
        ids.sort_unstable();
        ids.dedup();
        ids.len()
    }
}

/// Kennung des Suchfelds.
pub fn feld_id() -> Id {
    Id::new("suchfeld")
}

/// Zeigt höchstens so viele Treffer an.
const SICHTBAR: usize = 150;

fn umfang_knopf(name: &'static str, aktiv: bool, u: Umfang) -> Element<'static, Message> {
    button(text(name).size(12)).padding([4, 10]).style(thema::segment_stil(aktiv)).on_press(Message::SucheUmfang(u)).into()
}

fn eintrag<'a>(i: usize, auswahl: usize, kopf: String, f: &'a Fund, mit_vorschau: bool) -> Element<'a, Message> {
    let mut inhalt = column![text(kopf).size(10).color(thema::text_gedimmt()), text(&f.kontext).size(12)].spacing(1);
    if mit_vorschau && f.kontext != f.kontext_neu {
        inhalt = inhalt.push(text(format!("→ {}", f.kontext_neu)).size(12).color(thema::akzent()));
    }
    button(inhalt).width(Fill).padding([4, 8]).style(thema::palette_eintrag_stil(i == auswahl)).on_press(Message::SucheSpringe(i)).into()
}

pub fn ansicht(s: &Suche) -> Element<'_, Message> {
    let vorschau = !s.ersatz.is_empty() || s.regex;
    let mut liste = column![].spacing(2);
    let mut gezeigt = 0;
    for (i, f) in s.funde.iter().enumerate().take(SICHTBAR) {
        let kopf = if s.umfang == Umfang::Kapitel {
            format!("Zeile {}", f.zeile + 1)
        } else {
            format!("{} · {} · Zeile {}", f.kapitel_titel, f.datei, f.zeile + 1)
        };
        liste = liste.push(eintrag(i, s.auswahl, kopf, f, vorschau));
        gezeigt += 1;
    }
    for (j, b) in s.andere.iter().enumerate() {
        if gezeigt >= SICHTBAR {
            break;
        }
        let kopf = format!("{} · {} · {} · Zeile {}", b.band, b.fund.kapitel_titel, b.fund.datei, b.fund.zeile + 1);
        liste = liste.push(eintrag(s.funde.len() + j, s.auswahl, kopf, &b.fund, vorschau));
        gezeigt += 1;
    }

    let status = if let Some(f) = &s.fehler {
        f.clone()
    } else if s.muster.is_empty() {
        "Suchbegriff eingeben".to_string()
    } else if s.anzahl() == 0 {
        "Keine Treffer".to_string()
    } else {
        format!(
            "{} Treffer in {} {}{}",
            s.anzahl(),
            s.kapitel_anzahl(),
            if s.kapitel_anzahl() == 1 { "Kapitel" } else { "Kapiteln" },
            if s.anzahl() > SICHTBAR { format!(" (die ersten {SICHTBAR} angezeigt)") } else { String::new() }
        )
    };
    let status_farbe = if s.fehler.is_some() { thema::fehler_rot() } else { thema::text_gedimmt() };

    let hat_treffer = s.anzahl() > 0 && s.fehler.is_none();
    let alle: Element<'_, Message> = if s.bestaetigen {
        row![
            button(text("Abbrechen").size(12)).padding([5, 12]).style(thema::sekundaer_knopf_stil).on_press(Message::SucheBestaetigungAus),
            button(text(format!("{} Ersetzungen jetzt ausführen", s.anzahl())).size(12))
                .padding([5, 12])
                .style(thema::primaer_knopf_stil)
                .on_press(Message::SucheErsetzenAlle),
        ]
        .spacing(8)
        .into()
    } else {
        button(text("Alle ersetzen …").size(12))
            .padding([5, 12])
            .style(thema::primaer_knopf_stil)
            .on_press_maybe(hat_treffer.then_some(Message::SucheErsetzenAlle))
            .into()
    };

    let warnung: Element<'_, Message> = if s.bestaetigen {
        let ziel = match s.umfang {
            Umfang::Buch if !s.andere.is_empty() => "in allen Bänden. Die anderen Bände werden sofort gespeichert, der geöffnete Band erst mit „Speichern“.",
            _ => "im geöffneten Band. Gespeichert wird erst mit „Speichern“.",
        };
        text(format!("{} Treffer in {} Kapiteln werden ersetzt, {} Die alte Fassung bleibt als nummerierte Sicherung erhalten.", s.anzahl(), s.kapitel_anzahl(), ziel))
            .size(11)
            .color(thema::warnung_orange())
            .into()
    } else {
        space::vertical().height(0).into()
    };

    fenster(
        column![
            row![
                text("Suchen und Ersetzen").size(18).font(thema::fett()).color(thema::text_hell()),
                space::horizontal(),
                container(
                    row![
                        umfang_knopf("Kapitel", s.umfang == Umfang::Kapitel, Umfang::Kapitel),
                        umfang_knopf("Ganzer Band", s.umfang == Umfang::Band, Umfang::Band),
                        umfang_knopf("Alle Bände", s.umfang == Umfang::Buch, Umfang::Buch),
                    ]
                    .spacing(2),
                )
                .padding(2)
                .style(thema::segment_rahmen_stil),
            ]
            .align_y(Alignment::Center),
            text_input("Suchen …", &s.muster)
                .id(feld_id())
                .on_input(Message::SucheMuster)
                .on_submit(Message::SucheSpringe(s.auswahl))
                .size(14)
                .padding([7, 10]),
            text_input("Ersetzen durch …  (bei regulären Ausdrücken: $1, $2 für Gruppen)", &s.ersatz)
                .on_input(Message::SucheErsatz)
                .size(14)
                .padding([7, 10]),
            row![
                toggler(s.gross_klein).label("Groß-/Kleinschreibung").size(15).text_size(12).on_toggle(Message::SucheGrossKlein),
                toggler(s.ganzes_wort).label("Ganzes Wort").size(15).text_size(12).on_toggle(Message::SucheGanzesWort),
                toggler(s.regex).label("Regulärer Ausdruck").size(15).text_size(12).on_toggle(Message::SucheRegex),
            ]
            .spacing(18),
            row![
                text(status).size(11).color(status_farbe),
                space::horizontal(),
                button(text("Ersetzen").size(12))
                    .padding([5, 12])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press_maybe((hat_treffer && s.auswahl < s.funde.len()).then_some(Message::SucheErsetzenEins)),
                alle,
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            warnung,
            scrollable(liste).height(Length::Fixed(300.0)),
            text("Eingabetaste springt zum Treffer · F3 weiter · Umschalt+F3 zurück · Esc schließt")
                .size(10)
                .color(thema::text_gedimmt()),
        ]
        .spacing(8),
        720.0,
        50.0,
    )
}
