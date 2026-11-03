//! # Inspektor: Tempo, Stil und Text
//!
//! - **Tempo:** Fortschritt, Schreibzeit und -tempo in diesem Kapitel, Restzeit bis zum Wortziel, Lesedauer
//!   und der Satzrhythmus als kleines Säulendiagramm.
//! - **Stil:** Wie lebendig oder steril der Text wirkt (Satzrhythmus, Füllformeln, Verbindungswörter,
//!   persönliche Stimme …) mit Hinweisen zum Überarbeiten. Kein KI-Detektor.
//! - **Text:** Gliederung, Notizen, Rechtschreibung, Sprachprüfung.

use crate::thema;
use crate::ui::fokus::button;
use crate::{AppState, Message};
use iced::widget::{column, container, progress_bar, row, space, text};
use iced::{Alignment, Color, Element, Fill, Length};
use std::sync::Arc;
use werkbank_kern::stil::{self, Stilprofil, Urteil};
use werkbank_kern::{markdown, Kapitel};

pub fn ansicht(state: &AppState) -> Element<'_, Message> {
    let Some(k) = state.projekt.kapitel_finden(&state.aktives_kapitel_id) else {
        return text("Kein Kapitel gewählt").size(12).color(thema::text_gedimmt()).into();
    };
    match state.inspektor_tab {
        1 => tempo(state, k),
        2 => stil_ansicht(state, k),
        _ => text_tab(state, k),
    }
}

fn abschnitt<'a>(titel: &'a str, inhalt: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(titel).size(12).color(thema::text_gedimmt()), inhalt.into()].spacing(6).into()
}

fn zeile<'a>(name: &'a str, wert: String) -> Element<'a, Message> {
    row![text(name).size(11).color(thema::text_gedimmt()), space::horizontal(), text(wert).size(11).color(thema::text_hell())]
        .align_y(Alignment::Center)
        .into()
}

fn dauer(sekunden: f32) -> String {
    let min = (sekunden / 60.0).round() as u64;
    if min >= 60 { format!("{} h {:02} min", min / 60, min % 60) } else { format!("{min} min") }
}

/// Profil des offenen Kapitels, zwischengespeichert bis sich der Text ändert.
fn profil(state: &AppState, k: &Kapitel) -> Arc<Stilprofil> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (&k.id, &k.inhalt).hash(&mut h);
    let schluessel = h.finish();
    let mut cache = state.stil_cache.borrow_mut();
    if let Some((s, p)) = cache.as_ref()
        && *s == schluessel
    {
        return p.clone();
    }
    let p = Arc::new(stil::analysieren(&k.inhalt));
    *cache = Some((schluessel, p.clone()));
    p
}

/// Säulen der Satzlängen in Textreihenfolge: kurze Sätze in der Akzentfarbe, lange in Orange.
fn rhythmus<'a>(laengen: &[usize]) -> Element<'a, Message> {
    const SAEULEN: usize = 72;
    if laengen.is_empty() {
        return text("Zu wenig Text").size(11).color(thema::text_gedimmt()).into();
    }
    let gruppe = laengen.len().div_ceil(SAEULEN).max(1);
    let mut r = row![].spacing(1).align_y(Alignment::End).height(Length::Fixed(46.0));
    for teil in laengen.chunks(gruppe) {
        let mittel = teil.iter().sum::<usize>() as f32 / teil.len() as f32;
        let hoehe = (mittel / 40.0).clamp(0.05, 1.0) * 44.0;
        let farbe = if mittel >= 25.0 { thema::warnung_orange() } else if mittel <= 8.0 { thema::akzent() } else { thema::text_gedimmt() };
        r = r.push(
            container(space::horizontal().width(Length::Fixed(2.0)))
                .height(Length::Fixed(hoehe))
                .style(move |_t: &iced::Theme| container::Style { background: Some(iced::Background::Color(farbe)), ..Default::default() }),
        );
    }
    column![
        container(r).padding(6).width(Fill).style(thema::infobox_stil),
        text("Höhe = Satzlänge · blau kurz (≤ 8 Wörter), grau mittel, orange lang (≥ 25)").size(10).color(thema::text_gedimmt()),
    ]
    .spacing(3)
    .into()
}

fn tempo<'a>(state: &'a AppState, k: &'a Kapitel) -> Element<'a, Message> {
    let woerter = k.wortzahl();
    let rest = k.wortziel.saturating_sub(woerter);
    let fortschritt = k.ziel_fortschritt();
    let zeit = state.statistik.kapitel.get(&k.id);
    let wph = zeit.and_then(|z| z.woerter_pro_stunde());

    let mut schreiben = column![
        progress_bar(0.0..=100.0, fortschritt).girth(5.0).style(progress_bar::primary),
        zeile("Fortschritt", format!("{woerter} von {} Wörtern ({fortschritt:.0} %)", k.wortziel)),
        zeile("Noch offen", if rest == 0 { "Ziel erreicht".to_string() } else { format!("{rest} Wörter") }),
    ]
    .spacing(4);
    match (zeit, wph) {
        (Some(z), Some(t)) => {
            schreiben = schreiben
                .push(zeile("Schreibzeit hier", dauer(z.sekunden as f32)))
                .push(zeile("Tempo (netto)", format!("{t:.0} Wörter / Std.")));
            if rest > 0 && t > 1.0 {
                schreiben = schreiben.push(zeile("Bis zum Ziel", format!("≈ {}", dauer(rest as f32 / t * 3600.0))));
            }
        }
        _ => {
            schreiben = schreiben.push(
                text("Die Schreibzeit dieses Kapitels wird gemessen, sobald du hier tippst (Pausen über fünf Minuten zählen nicht).")
                    .size(10)
                    .color(thema::text_gedimmt()),
            );
        }
    }

    let p = profil(state, k);
    let code_zeilen = markdown::zeilen(&k.inhalt).iter().filter(|z| z.im_code && !z.text.trim_start().starts_with("```")).count();
    let lese_sek = p.woerter as f32 / 200.0 * 60.0 + code_zeilen as f32 * 8.0;
    let lesen = column![
        zeile("Lesedauer", format!("≈ {}", dauer(lese_sek))),
        text(format!("Fließtext {} Wörter bei 200 Wörtern/Min. plus {code_zeilen} Codezeilen à 8 s", p.woerter)).size(10).color(thema::text_gedimmt()),
    ]
    .spacing(3);

    let mut satz = column![].spacing(6);
    if p.urteil == Urteil::ZuWenigText {
        satz = satz.push(text("Zu wenig Fließtext für den Satzrhythmus.").size(11).color(thema::text_gedimmt()));
    } else {
        satz = satz
            .push(zeile("Ø Satzlänge", format!("{:.1} Wörter", p.satzlaenge_mittel)))
            .push(zeile("Kurze Sätze (≤ 8)", format!("{:.0} %", p.kurz_anteil * 100.0)))
            .push(zeile("Lange Sätze (≥ 25)", format!("{:.0} %", p.lang_anteil * 100.0)))
            .push(rhythmus(&p.satzlaengen));
    }

    column![
        abschnitt("Schreibtempo", schreiben),
        abschnitt("Lesetempo", lesen),
        abschnitt("Satzrhythmus", satz),
    ]
    .spacing(16)
    .width(Fill)
    .into()
}

fn stil_ansicht<'a>(state: &'a AppState, k: &'a Kapitel) -> Element<'a, Message> {
    let p = profil(state, k);
    if p.urteil == Urteil::ZuWenigText {
        return column![
            text("Zu wenig Text").size(14).color(thema::text_hell()),
            text(format!("Für eine Einschätzung braucht das Kapitel mindestens 150 Wörter Fließtext und 8 Sätze (hier {} Wörter, {} Sätze). Code, Überschriften und Listen zählen nicht mit.", p.woerter, p.saetze))
                .size(11)
                .color(thema::text_gedimmt()),
        ]
        .spacing(6)
        .into();
    }

    let farbe = match p.urteil {
        Urteil::Lebendig => thema::erfolg_gruen(),
        Urteil::Ueberwiegend => thema::akzent(),
        Urteil::Gleichfoermig => thema::warnung_orange(),
        _ => thema::fehler_rot(),
    };
    let kopf = column![
        row![
            text(format!("{}", p.punktzahl)).size(34).color(farbe),
            column![text(p.urteil.name()).size(13).color(farbe), text("Natürlichkeit von 100").size(10).color(thema::text_gedimmt())].spacing(1),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        progress_bar(0.0..=100.0, p.punktzahl as f32).girth(6.0).style(if p.punktzahl >= 70 { progress_bar::success } else if p.punktzahl >= 52 { progress_bar::primary } else if p.punktzahl >= 36 { progress_bar::warning } else { progress_bar::danger }),
        text("Misst, wie abwechslungsreich und persönlich der Text geschrieben ist. Das ist kein KI-Detektor und kein Urteil über den Inhalt; Fachtexte erreichen bei „Stimme“ naturgemäß weniger.").size(10).color(thema::text_gedimmt()),
    ]
    .spacing(6);

    let mut kennzahlen = column![].spacing(8);
    for z in &p.kennzahlen {
        let stil_balken = if z.einstufung >= 0.66 { progress_bar::success } else if z.einstufung >= 0.33 { progress_bar::warning } else { progress_bar::danger };
        let mut spalte = column![
            row![text(z.name).size(11).color(thema::text_hell()), space::horizontal(), text(format!("{:.0} %", z.einstufung * 100.0)).size(10).color(thema::text_gedimmt())],
            progress_bar(0.0..=1.0, z.einstufung).girth(3.0).style(stil_balken),
            text(z.wert.clone()).size(10).color(thema::text_gedimmt()),
        ]
        .spacing(2);
        if let Some(t) = z.tipp {
            spalte = spalte.push(text(t).size(10).color(thema::warnung_orange()));
        }
        kennzahlen = kennzahlen.push(spalte);
    }

    let mut inhalt = column![kopf, abschnitt("Merkmale", kennzahlen)].spacing(16).width(Fill);
    if !p.floskeln.is_empty() {
        let mut liste = column![].spacing(2);
        for (phrase, n) in p.floskeln.iter().take(10) {
            liste = liste.push(
                button(row![text(format!("„{phrase}“")).size(11), space::horizontal(), text(format!("{n}×")).size(11).color(thema::text_gedimmt())].align_y(Alignment::Center))
                    .width(Fill)
                    .padding([2, 6])
                    .style(thema::symbol_knopf_stil(false))
                    .on_press(Message::SucheNach(phrase.trim_end_matches(" … sondern auch").replace("nicht nur … ", "nicht nur").to_string())),
            );
        }
        inhalt = inhalt.push(abschnitt("Füllformeln im Text (Klick sucht sie)", liste));
    }
    if !p.satzanfaenge.is_empty() && p.satzanfaenge[0].1 >= 4 {
        let s: Vec<String> = p.satzanfaenge.iter().take(4).map(|(w, n)| format!("„{w}“ {n}×")).collect();
        inhalt = inhalt.push(abschnitt("Häufigste Satzanfänge", text(s.join(" · ")).size(11).color(thema::text_gedimmt())));
    }
    inhalt.into()
}

fn text_tab<'a>(state: &'a AppState, k: &'a Kapitel) -> Element<'a, Message> {
    use crate::editor as text_editor;
    let mut spalte = column![].spacing(12).width(Fill);

    // Gliederung
    let mut gliederung = column![text("Gliederung").size(12).color(thema::text_gedimmt())].spacing(1);
    let ueberschriften = markdown::ueberschriften(&k.inhalt);
    if ueberschriften.is_empty() {
        gliederung = gliederung.push(text("Keine Überschriften").size(11).color(thema::text_gedimmt()));
    }
    for u in ueberschriften {
        let einzug = (u.ebene.saturating_sub(2)) as f32 * 12.0;
        let titel: String = u.text.chars().take(48).collect();
        gliederung = gliederung.push(
            button(row![space::horizontal().width(einzug), text(titel).size(11)])
                .padding([2, 4])
                .width(Fill)
                .style(thema::symbol_knopf_stil(false))
                .on_press(Message::SpringeZuZeile { zeile: u.zeile, spalte: 0 }),
        );
    }
    spalte = spalte.push(gliederung);

    // Notizen
    spalte = spalte.push(
        column![
            text("Notizen zum Kapitel").size(12).color(thema::text_gedimmt()),
            container(
                text_editor::TextEditor::new(&state.notizen_content)
                    .placeholder("Schreibabsicht, offene Punkte …")
                    .on_action(Message::NotizenAktion)
                    .size(12)
                    .height(110),
            )
            .style(thema::infobox_stil),
        ]
        .spacing(4),
    );

    // Rechtschreibung: unbekannte Wörter des Kapitels
    if let Some(wb) = &state.woerterbuch
        && state.einstellungen.rechtschreibung
    {
        let eigene: std::collections::HashSet<String> = state.ausnahmen.woerter.iter().cloned().collect();
        let unbekannt = crate::rechtschreibung::unbekannte_woerter(wb, &k.inhalt, &eigene);
        let mut box_ = column![text(format!("Rechtschreibung ({} unbekannte Wörter)", unbekannt.len())).size(12).color(thema::text_gedimmt())].spacing(2);
        if unbekannt.is_empty() {
            box_ = box_.push(text("Alles bekannt.").size(11).color(thema::erfolg_gruen()));
        }
        for (wort, n) in unbekannt.into_iter().take(12) {
            let vorschlag = wb.erster_vorschlag(&wort).map(|v| format!(" → {v}")).unwrap_or_default();
            box_ = box_.push(
                button(text(format!("{wort} ×{n}{vorschlag}")).size(11))
                    .padding([2, 4])
                    .width(Fill)
                    .style(thema::symbol_knopf_stil(false))
                    .on_press(Message::WortMerken(wort)),
            );
        }
        spalte = spalte.push(box_);
        spalte = spalte.push(text("Ein Klick merkt ein Wort als richtig (gilt für den ganzen Band).").size(10).color(thema::text_gedimmt()));
    }

    spalte
        .push(crate::ui::sprache::inspektor_box(state.einstellungen.sprachpruefung, &state.treffer, &state.pruef_status))
        .into()
}

#[allow(dead_code)]
fn unbenutzt(_: Color) {}
