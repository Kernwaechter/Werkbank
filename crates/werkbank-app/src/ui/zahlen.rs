//! # Statistik und Schreibtempo
//!
//! Echte Zahlen aus der Schreibstatistik des Bandes: Tagespensum, Tempo,
//! Streak, Heatmap, Prognose, Fortschritt je Teil und Textqualität.

use crate::thema;
use crate::{AppState, Message};
use chrono::Local;
use iced::widget::{column, container, progress_bar, row, scrollable, space, text};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Padding, Pixels};
use werkbank_kern::metriken::{self, STANDARD_FLOSKELN};
use werkbank_kern::statistik::Statistik;
use werkbank_kern::{KapitelStatus, markdown};

fn karte<'a>(inhalt: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(inhalt).padding(16).width(Fill).style(thema::gruppe_stil).into()
}

fn kennzahl<'a>(titel: &'a str, wert: String, hinweis: String) -> Element<'a, Message> {
    container(
        column![
            text(titel).size(11).color(thema::text_gedimmt()),
            text(wert).size(24).font(thema::fett()).color(thema::text_hell()),
            text(hinweis).size(10).color(thema::text_gedimmt()),
        ]
        .spacing(2),
    )
    .padding(14)
    .width(Length::FillPortion(1))
    .style(thema::gruppe_stil)
    .into()
}

fn minuten(sekunden: u64) -> String {
    if sekunden >= 3600 {
        format!("{} h {:02} min", sekunden / 3600, sekunden % 3600 / 60)
    } else {
        format!("{} min", sekunden / 60)
    }
}

fn heat_farbe(netto: i64, pensum: i64) -> Color {
    if netto < 0 {
        return Color::TRANSPARENT; // Zukunft
    }
    let stufe = match netto {
        0 => 0,
        n if n < pensum / 4 => 1,
        n if n < pensum / 2 => 2,
        n if n < pensum => 3,
        _ => 4,
    };
    let mische = |t: f32| {
        let (a, b) = (thema::karte_hover(), thema::akzent());
        Color { r: a.r + (b.r - a.r) * t, g: a.g + (b.g - a.g) * t, b: a.b + (b.b - a.b) * t, a: 1.0 }
    };
    [thema::karte_hover(), mische(0.30), mische(0.55), mische(0.80), thema::akzent()][stufe]
}

fn heatmap(statistik: &Statistik, pensum: i64) -> Element<'_, Message> {
    let heute = Local::now().date_naive();
    let wochen = statistik.heatmap(heute, 26);
    let mut spalten = row![].spacing(3);
    for woche in wochen {
        let mut tage = column![].spacing(3);
        for (_, netto) in woche {
            tage = tage.push(
                container(space::horizontal().width(Length::Fixed(13.0)))
                    .height(13)
                    .style(move |_t: &iced::Theme| container::Style {
                        background: Some(Background::Color(heat_farbe(netto, pensum))),
                        border: Border { radius: 3.0.into(), ..Default::default() },
                        ..Default::default()
                    }),
            );
        }
        spalten = spalten.push(tage);
    }
    let beschriftung = column![
        text("Mo").size(9), text("").size(9), text("Mi").size(9), text("").size(9), text("Fr").size(9), text("").size(9), text("So").size(9)
    ]
    .spacing(3)
    .height(Length::Shrink);
    row![beschriftung.width(20), spalten].spacing(6).into()
}

pub fn ansicht(state: &AppState) -> Element<'_, Message> {
    let projekt = &state.projekt;
    let s = &state.statistik;
    let heute = Local::now().date_naive();
    let pensum = state.einstellungen.tagespensum.max(1);

    let tag = s.tag(heute);
    let netto_heute = tag.netto().max(0);
    let fortschritt = (netto_heute as f32 / pensum as f32 * 100.0).clamp(0.0, 100.0);
    let tempo = s
        .woerter_pro_stunde(heute, 1)
        .or_else(|| s.woerter_pro_stunde(heute, 7))
        .map_or("noch keine Messung".to_string(), |t| format!("{t:.0} Wörter / Std."));

    let kopf = column![
        text("Statistik & Schreibtempo").size(22).font(thema::fett()).color(thema::text_hell()),
        text(if state.band.ordner.is_some() {
            "Gespeichert in .werkbank/statistik.json des Bandes. Gezählt werden Netto-Wörter: neu geschrieben minus gestrichen."
        } else {
            "Demo-Buch: Die Statistik wird nicht gespeichert."
        })
        .size(12)
        .color(thema::text_gedimmt()),
    ]
    .spacing(4);

    let heute_karte = karte(
        column![
            row![
                text("Tagespensum").size(14).font(thema::fett()).color(thema::text_hell()),
                space::horizontal(),
                text(format!("{netto_heute} / {pensum} Wörter ({fortschritt:.0}%)"))
                    .size(12)
                    .color(thema::akzent()),
            ]
            .align_y(Alignment::Center),
            progress_bar(0.0..=100.0, fortschritt).girth(6.0).style(progress_bar::primary),
            row![
                text(format!("Schreibzeit heute: {}", minuten(tag.sekunden))).size(11).color(thema::text_gedimmt()),
                space::horizontal(),
                text(format!("Tempo: {tempo}")).size(11).color(thema::text_hell()),
            ]
            .align_y(Alignment::Center),
            text(format!(
                "Heute: {} neu, {} gestrichen, {} Sitzung{}",
                tag.hinzu, tag.entfernt, tag.sitzungen, if tag.sitzungen == 1 { "" } else { "en" }
            ))
            .size(11)
            .color(thema::text_gedimmt()),
        ]
        .spacing(6),
    );

    // Prognose: Wie lange bis alle Kapitel ihr Wortziel erreicht haben?
    let rest: usize = projekt
        .teile
        .iter()
        .flat_map(|t| t.kapitel.iter())
        .map(|k| k.wortziel.saturating_sub(k.wortzahl()))
        .sum();
    let schnitt = s.schnitt(heute, 14);
    let prognose = Statistik::prognose(rest, schnitt, heute).map_or(
        "keine Prognose (zu wenig Schreibtempo)".to_string(),
        |d| d.format("%d.%m.%Y").to_string(),
    );

    let kennzahlen = row![
        kennzahl("Letzte 7 Tage", format!("{}", s.netto_summe(heute, 7)), format!("{} Schreibzeit", minuten(s.sekunden_summe(heute, 7)))),
        kennzahl("Letzte 30 Tage", format!("{}", s.netto_summe(heute, 30)), format!("Ø {:.0} / Tag", s.schnitt(heute, 30))),
        kennzahl("Streak", format!("{} Tage", s.streak(heute)), "Tage in Folge mit Netto > 0".to_string()),
        kennzahl("Fertig laut Tempo", prognose, format!("noch {rest} Wörter bis zu allen Wortzielen")),
    ]
    .spacing(10);

    let heatmap_karte = karte(
        column![
            text("Aktivität der letzten 26 Wochen").size(14).font(thema::fett()).color(thema::text_hell()),
            scrollable(heatmap(s, pensum as i64)).direction(scrollable::Direction::Horizontal(Default::default())),
            text(format!("Dunkel = wenig, hell = Pensum von {pensum} Wörtern erreicht")).size(10).color(thema::text_gedimmt()),
        ]
        .spacing(8),
    );

    // Fortschritt je Teil mit Statusampel
    let mut teile = column![text("Fortschritt je Teil").size(14).font(thema::fett()).color(thema::text_hell())].spacing(12);
    for t in &projekt.teile {
        let zaehle = |st: KapitelStatus| t.kapitel.iter().filter(|k| k.status == st).count();
        let ziel: usize = t.kapitel.iter().map(|k| k.wortziel).sum();
        let woerter = t.gesamtwortzahl();
        let anteil = if ziel == 0 { 0.0 } else { (woerter as f32 / ziel as f32 * 100.0).min(100.0) };
        // Titel in eigener Zeile (darf umbrechen), darunter die Stände als farbige Punkte – nur die vorhandenen
        let mut staende = row![].spacing(10).align_y(Alignment::Center);
        for (st, name) in [
            (KapitelStatus::Entwurf, "Entwurf"),
            (KapitelStatus::InArbeit, "in Arbeit"),
            (KapitelStatus::Ueberarbeitet, "überarbeitet"),
            (KapitelStatus::Fertig, "fertig"),
        ] {
            let n = zaehle(st);
            if n > 0 {
                staende = staende.push(
                    row![text("●").size(9).color(thema::status_farbe(st)), text(format!("{n} {name}")).size(11).color(thema::text_gedimmt())]
                        .spacing(4)
                        .align_y(Alignment::Center),
                );
            }
        }
        teile = teile.push(
            column![
                text(&t.titel).size(13).color(thema::text_hell()),
                row![staende, space::horizontal(), text(format!("{anteil:.0} %")).size(11).color(thema::text_gedimmt())]
                    .align_y(Alignment::Center),
                progress_bar(0.0..=100.0, anteil).girth(4.0).style(progress_bar::primary),
            ]
            .spacing(4),
        );
    }

    // Buchstatistik
    let (mut code_woerter, mut fliess_woerter) = (0usize, 0usize);
    let mut gesamttext = String::new();
    for k in projekt.teile.iter().flat_map(|t| t.kapitel.iter()) {
        for z in markdown::zeilen(&k.inhalt) {
            let n = z.text.split_whitespace().count();
            if z.im_code { code_woerter += n } else { fliess_woerter += n }
        }
        gesamttext.push_str(&k.inhalt);
        gesamttext.push('\n');
    }
    let code_anteil = code_woerter as f32 / (code_woerter + fliess_woerter).max(1) as f32 * 100.0;
    let buch = karte(
        column![
            text("Buch").size(14).font(thema::fett()).color(thema::text_hell()),
            metrik("Wörter", projekt.gesamtwortzahl().to_string()),
            metrik("Zeichen", projekt.gesamtzeichenzahl().to_string()),
            metrik("Normseiten (1.800 Zeichen)", format!("{:.1}", projekt.normseiten())),
            metrik("Seiten im Druck (Schätzung)", projekt.buchseiten_schaetzung().to_string()),
            metrik("Kapitel", projekt.kapitel_anzahl().to_string()),
            metrik("Codeanteil gegen Fließtext", format!("{code_anteil:.0} % Code")),
        ]
        .spacing(4),
    );

    // Textqualität: aktuelles Kapitel und ganzes Buch
    let aktuell = projekt.kapitel_finden(&state.aktives_kapitel_id).map(|k| k.inhalt.as_str()).unwrap_or("");
    let a_kap = metriken::text_analysieren(aktuell);
    let a_buch = metriken::text_analysieren(&gesamttext);
    let qualitaet = |titel: &'static str, a: &metriken::TextAnalyse| -> Element<'static, Message> {
        column![
            text(titel).size(12).color(thema::akzent()),
            metrik("Lesbarkeit (Flesch-Amstad)", format!("{:.0}", a.flesch_amstad)),
            metrik("Einstufung", a.grad.map_or("–".to_string(), |g| g.name().to_string())),
            metrik("Sätze", a.saetze.to_string()),
            metrik("Ø Satzlänge", format!("{:.1} Wörter", a.durchschnittliche_satzlaenge)),
            metrik("Kurz / mittel / lang / Bandwurm", format!("{} / {} / {} / {}", a.saetze_kurz, a.saetze_mittel, a.saetze_lang, a.saetze_ueberlang)),
        ]
        .spacing(4)
        .width(Fill)
        .into()
    };
    let floskeln = metriken::begriffe_zaehlen(aktuell, STANDARD_FLOSKELN);
    let mut floskel_liste = column![text("Floskeln und Füllwörter im offenen Kapitel").size(12).color(thema::akzent())].spacing(3);
    if floskeln.is_empty() {
        floskel_liste = floskel_liste.push(text("Keine gefunden.").size(11).color(thema::text_gedimmt()));
    }
    for (b, n) in floskeln.into_iter().take(10) {
        floskel_liste = floskel_liste.push(metrik_dyn(b, n.to_string()));
    }
    let text_karte = karte(
        column![
            text("Textqualität").size(14).font(thema::fett()).color(thema::text_hell()),
            row![qualitaet("Offenes Kapitel", &a_kap), qualitaet("Ganzes Buch", &a_buch), floskel_liste.width(Fill)].spacing(20),
        ]
        .spacing(10),
    );

    let inhalt = column![
        kopf,
        heute_karte,
        kennzahlen,
        heatmap_karte,
        row![karte(teile), buch].spacing(10),
        text_karte,
    ]
    .spacing(12)
    .max_width(Pixels(1100.0))
    .width(Fill);

    container(scrollable(inhalt).height(Fill))
        .center_x(Fill)
        .padding(Padding::from([20, 30]))
        .style(thema::editor_bereich_stil)
        .width(Fill)
        .height(Fill)
        .into()
}

fn metrik<'a>(name: &'static str, wert: String) -> Element<'a, Message> {
    row![text(name).size(11).color(thema::text_gedimmt()), space::horizontal(), text(wert).size(11).color(thema::text_hell())]
        .align_y(Alignment::Center)
        .into()
}

fn metrik_dyn<'a>(name: String, wert: String) -> Element<'a, Message> {
    row![text(name).size(11).color(thema::text_gedimmt()), space::horizontal(), text(wert).size(11).color(thema::text_hell())]
        .align_y(Alignment::Center)
        .into()
}
