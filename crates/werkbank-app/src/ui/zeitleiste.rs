//! # Zeitleiste
//!
//! Frühere Stände eines Kapitels (und der Papierkorb) in einem großen Fenster: links die Stände,
//! nach Tagen gegliedert und zu Schreibsitzungen zusammengefasst; rechts der gewählte Stand mit
//! Vorschau, Änderungen gegenüber jetzt und den Aktionen zum Zurückholen.

use crate::Message;
use crate::thema;
use crate::ui::dialog::{abdunkeln, haupt_knopf, neben_knopf};
use crate::ui::fokus::button;
use chrono::{DateTime, Datelike, Duration, Local, Timelike};
use iced::widget::text::Span;
use iced::widget::{column, container, rich_text, row, scrollable, space, span, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Fill, Font, Length, Size};
use std::collections::HashSet;
use werkbank_kern::diff::{Aenderung, Art, Block, Stueck, Vergleich};
use werkbank_kern::verlauf::{AufraeumPlan, KorbEintrag, Stand, StandArt};

/// Stände mit weniger Abstand gehören zur selben Schreibsitzung.
const SITZUNG_PAUSE_MIN: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reiter {
    Staende,
    Papierkorb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ansicht {
    /// Nur die geänderten Stellen.
    Aenderungen,
    /// Der ganze Text mit Markierungen.
    Ganz,
    /// Älterer Stand und aktueller Text nebeneinander.
    Nebeneinander,
}

/// Zustand der Zeitleiste.
pub struct Zeitleiste {
    pub kapitel_id: String,
    pub titel: String,
    pub staende: Vec<Stand>,
    /// Aktueller Text des Kapitels (Editorstand).
    pub aktuell: String,
    pub ungespeichert: bool,
    pub reiter: Reiter,
    pub ansicht: Ansicht,
    pub auswahl: Option<usize>,
    /// Aufgeklappte Sitzungen (Index des jeweils neuesten Standes).
    pub offen: HashSet<usize>,
    /// Text des gewählten Standes und Vergleich mit dem aktuellen.
    pub alter_text: Option<String>,
    pub vergleich: Option<Vergleich>,
    pub name: String,
    pub aufraeumen: Option<AufraeumPlan>,
    pub uebersicht: (usize, u64),
    pub korb: Vec<KorbEintrag>,
    pub korb_auswahl: Option<usize>,
    pub korb_text: String,
    /// Der erste Klick auf „Endgültig löschen“ fragt nach.
    pub korb_sicher: bool,
}

impl Zeitleiste {
    pub fn gruppen(&self) -> Vec<(usize, usize)> {
        let mut gruppen: Vec<(usize, usize)> = Vec::new();
        for i in 0..self.staende.len() {
            match gruppen.last_mut() {
                Some((_, ende)) if (self.staende[*ende - 1].zeit - self.staende[i].zeit) < Duration::minutes(SITZUNG_PAUSE_MIN) => *ende = i + 1,
                _ => gruppen.push((i, i + 1)),
            }
        }
        gruppen
    }

    /// Wörter dieses Standes minus Wörter des nächstälteren.
    pub fn zuwachs(&self, i: usize) -> Option<i64> {
        let jetzt = self.staende.get(i)?.woerter? as i64;
        let davor = self.staende.get(i + 1)?.woerter? as i64;
        Some(jetzt - davor)
    }

    pub fn aktuell_woerter(&self) -> usize {
        self.aktuell.split_whitespace().count()
    }
}

// ------------------------------------------------------------------ Hilfen

const WOCHENTAGE: [&str; 7] = ["Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag", "Sonntag"];

/// „heute, 14:22“, „gestern, 19:30“, „Montag, 02.11., 10:05“ oder „02.11.2025, 10:05“.
pub fn zeit_text(zeit: DateTime<Local>) -> String {
    let jetzt = Local::now();
    let tage = (jetzt.date_naive() - zeit.date_naive()).num_days();
    let uhr = format!("{:02}:{:02}", zeit.hour(), zeit.minute());
    match tage {
        0 => format!("heute, {uhr}"),
        1 => format!("gestern, {uhr}"),
        2..=6 => format!("{}, {}, {uhr}", WOCHENTAGE[zeit.weekday().num_days_from_monday() as usize], zeit.format("%d.%m.")),
        _ if zeit.year() == jetzt.year() => format!("{}, {uhr}", zeit.format("%d.%m.")),
        _ => format!("{}, {uhr}", zeit.format("%d.%m.%Y")),
    }
}

fn abschnitt(zeit: DateTime<Local>) -> &'static str {
    match (Local::now().date_naive() - zeit.date_naive()).num_days() {
        i64::MIN..=0 => "Heute",
        1 => "Gestern",
        2..=6 => "Diese Woche",
        7..=30 => "Letzte 30 Tage",
        _ => "Älter",
    }
}

fn groesse_text(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0).replace('.', ",")
    } else {
        format!("{:.0} KB", (bytes as f64 / 1024.0).max(1.0))
    }
}

fn woerter_text(n: usize) -> String {
    format!("{} {}", thema::tausender(n), if n == 1 { "Wort" } else { "Wörter" })
}

fn zuwachs_element<'a>(d: Option<i64>, max: i64) -> Element<'a, Message> {
    let Some(d) = d else {
        return space::horizontal().width(0).into();
    };
    let (zeichen, farbe) = match d {
        0 => ("±0".to_string(), thema::text_gedimmt()),
        d if d > 0 => (format!("+{}", thema::tausender(d as usize)), thema::erfolg_gruen()),
        d => (format!("−{}", thema::tausender(d.unsigned_abs() as usize)), thema::fehler_rot()),
    };
    // Kleiner Balken: Länge nach der Größe der Änderung (Wurzel, damit kleine Änderungen sichtbar bleiben)
    let laenge = if d == 0 || max == 0 { 0.0 } else { ((d.abs() as f32 / max as f32).sqrt() * 34.0).max(3.0) };
    column![
        text(zeichen).size(12).color(farbe),
        container(space::horizontal().width(laenge).height(3)).style(move |_| container::Style {
            background: Some(Background::Color(farbe)),
            border: Border { radius: 2.0.into(), ..Default::default() },
            ..Default::default()
        }),
    ]
    .spacing(3)
    .align_x(Alignment::End)
    .into()
}

fn flaeche(farbe: Color, radius: f32) -> impl Fn(&iced::Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(farbe)),
        border: Border { color: thema::linie_farbe(), width: 1.0, radius: radius.into() },
        ..Default::default()
    }
}

fn gedimmt<'a>(t: impl text::IntoFragment<'a>, groesse: u32) -> Element<'a, Message> {
    text(t).size(groesse).color(thema::text_gedimmt()).into()
}

// ------------------------------------------------------------------ Ansicht

pub fn ansicht<'a>(z: &'a Zeitleiste, fenster: Size) -> Element<'a, Message> {
    let breite = (fenster.width - 48.0).clamp(560.0, 1140.0);
    let hoehe = (fenster.height - 64.0).clamp(420.0, 780.0);

    let tab = |name: String, reiter: Reiter| -> Element<'a, Message> {
        button(text(name).size(13))
            .padding([5, 14])
            .style(thema::tab_knopf_stil(z.reiter == reiter))
            .on_press(Message::ZeitReiter(reiter))
            .into()
    };
    let kopf = container(
        row![
            text("Zeitleiste").size(17).font(thema::fett()).color(thema::text_hell()),
            text(format!("· {}", z.titel)).size(14).color(thema::text_gedimmt()),
            space::horizontal(),
            row![
                tab(format!("Stände ({})", z.staende.len()), Reiter::Staende),
                tab(if z.korb.is_empty() { "Papierkorb".to_string() } else { format!("Papierkorb ({})", z.korb.len()) }, Reiter::Papierkorb),
            ]
            .spacing(4),
            space::horizontal(),
            neben_knopf("Schließen  Esc", Message::DialogSchliessen),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([14, 20]);

    let koerper: Element<'a, Message> = match z.reiter {
        Reiter::Staende => row![
            container(links(z)).width(330).height(Fill),
            thema::trennlinie_senkrecht(),
            container(rechts(z)).width(Fill).height(Fill),
        ]
        .height(Fill)
        .into(),
        Reiter::Papierkorb => row![
            container(korb_links(z)).width(330).height(Fill),
            thema::trennlinie_senkrecht(),
            container(korb_rechts(z)).width(Fill).height(Fill),
        ]
        .height(Fill)
        .into(),
    };

    let box_ = container(column![kopf, thema::trennlinie(), koerper])
        .width(Length::Fixed(breite))
        .height(Length::Fixed(hoehe))
        .clip(true)
        .style(thema::dialog_stil);
    abdunkeln(box_.into(), 30.0)
}

// -------------------------------------------------------------- Stände links

fn links<'a>(z: &'a Zeitleiste) -> Element<'a, Message> {
    let max = (0..z.staende.len()).filter_map(|i| z.zuwachs(i)).map(i64::abs).max().unwrap_or(0);
    let mut liste = column![].spacing(2).padding([10, 10]);

    // Jetzt
    let delta = z.staende.first().and_then(|s| s.woerter).map(|w| z.aktuell_woerter() as i64 - w as i64);
    liste = liste.push(
        container(
            row![
                text("●").size(10).color(thema::akzent()),
                column![
                    text(if z.ungespeichert { "Jetzt · ungespeichert" } else { "Jetzt" }).size(13).font(thema::fett()),
                    gedimmt(woerter_text(z.aktuell_woerter()), 11),
                ]
                .spacing(1),
                space::horizontal(),
                zuwachs_element(delta, max),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([7, 10])
        .width(Fill),
    );

    if z.staende.is_empty() {
        liste = liste.push(container(gedimmt(
            "Noch keine früheren Stände. Sie entstehen, sobald du speicherst und sich der Text geändert hat.",
            12,
        ))
        .padding([10, 10]));
    }

    let mut letzter_abschnitt = "";
    for (start, ende) in z.gruppen() {
        let ab = abschnitt(z.staende[start].zeit);
        if ab != letzter_abschnitt {
            letzter_abschnitt = ab;
            liste = liste.push(container(thema::abschnitt_titel(ab)).padding(iced::Padding::from([12, 10]).bottom(4)));
        }
        if ende - start == 1 {
            liste = liste.push(eintrag(z, start, false, max));
            continue;
        }
        let aufgeklappt = z.offen.contains(&start);
        let erste = z.staende[start].zeit;
        let letzte = z.staende[ende - 1].zeit;
        let summe: i64 = (start..ende).filter_map(|i| z.zuwachs(i)).sum();
        liste = liste.push(
            button(
                row![
                    text(if aufgeklappt { "▾" } else { "▸" }).size(12).color(thema::text_gedimmt()).width(12),
                    column![
                        text(format!("Schreibsitzung {:02}:{:02}–{:02}:{:02}", letzte.hour(), letzte.minute(), erste.hour(), erste.minute()))
                            .size(13),
                        gedimmt(format!("{} Stände", ende - start), 11),
                    ]
                    .spacing(1),
                    space::horizontal(),
                    zuwachs_element(Some(summe), max),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([7, 10])
            .style(thema::palette_eintrag_stil(false))
            .on_press(Message::ZeitSitzung(start)),
        );
        if aufgeklappt {
            for i in start..ende {
                liste = liste.push(eintrag(z, i, true, max));
            }
        }
    }

    // Fuß: Speicherbedarf und Verwaltung
    let (anzahl, bytes) = z.uebersicht;
    let fuss = container(
        column![
            gedimmt(format!("{anzahl} Stände · {} belegt", groesse_text(bytes)), 11),
            row![
                neben_knopf("Aufräumen …", Message::ZeitAufraeumenPlan),
                neben_knopf("Im Dateimanager zeigen", Message::ZeitOrdner),
            ]
            .spacing(6),
        ]
        .spacing(8),
    )
    .padding([10, 14])
    .width(Fill)
    .style(thema::dialog_fuss_stil);

    column![scrollable(liste).height(Fill), thema::trennlinie(), fuss].into()
}

fn eintrag<'a>(z: &'a Zeitleiste, i: usize, eingerueckt: bool, max: i64) -> Element<'a, Message> {
    let s = &z.staende[i];
    let titel = match &s.name {
        Some(n) => format!("★ {n}"),
        None if eingerueckt => format!("{:02}:{:02}", s.zeit.hour(), s.zeit.minute()),
        None => zeit_text(s.zeit),
    };
    let art = match s.art {
        StandArt::Sicherung => format!("Sicherung Nr. {}", s.nummer),
        StandArt::Auto => "Automatisch gesichert".to_string(),
    };
    let unter = match s.woerter {
        Some(w) => format!("{art} · {}", woerter_text(w)),
        None => format!("{art} · Kapitel fehlt"),
    };
    // Mit Namen steht die Zeit in der Nebenzeile
    let unter = match (&s.name, eingerueckt) {
        (Some(_), true) => format!("{:02}:{:02} · {unter}", s.zeit.hour(), s.zeit.minute()),
        (Some(_), false) => format!("{} · {unter}", zeit_text(s.zeit)),
        (None, _) => unter,
    };
    let kopf = column![text(titel).size(13), gedimmt(unter, 11)].spacing(1);
    let innen = row![
        text(if z.auswahl == Some(i) { "●" } else { "○" }).size(10).color(thema::akzent()),
        kopf,
        space::horizontal(),
        zuwachs_element(z.zuwachs(i), max),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    let knopf = button(innen)
        .width(Fill)
        .padding([7, 10])
        .style(thema::palette_eintrag_stil(z.auswahl == Some(i)))
        .on_press(Message::ZeitAuswahl(i));
    if eingerueckt {
        row![space::horizontal().width(18), knopf].into()
    } else {
        knopf.into()
    }
}

// ------------------------------------------------------------ Vorschau rechts

fn rechts<'a>(z: &'a Zeitleiste) -> Element<'a, Message> {
    if let Some(plan) = &z.aufraeumen {
        return aufraeumen_ansicht(plan, z.uebersicht);
    }
    let Some(s) = z.auswahl.and_then(|i| z.staende.get(i)) else {
        return container(gedimmt("Wähle links einen Stand, um ihn zu sehen und zurückzuholen.", 13))
            .center(Fill)
            .into();
    };

    let (kaemen, fielen) = z.vergleich.as_ref().map_or((0, 0), Vergleich::woerter);
    let anzahl = z.vergleich.as_ref().map_or(0, Vergleich::anzahl);
    let zusammenfassung = if z.alter_text.is_none() {
        "Dieses Kapitel gab es in dem Stand noch nicht.".to_string()
    } else if anzahl == 0 {
        "Dieser Stand ist gleich dem aktuellen Text.".to_string()
    } else {
        format!(
            "Gegenüber jetzt: {} {}, davon {} kämen zurück und {} gingen verloren.",
            anzahl,
            if anzahl == 1 { "Änderung" } else { "Änderungen" },
            woerter_text(kaemen),
            woerter_text(fielen),
        )
    };

    let titelzeile = row![
        column![
            text(format!("Stand von {}", zeit_text(s.zeit))).size(16).font(thema::fett()).color(thema::text_hell()),
            gedimmt(
                format!(
                    "{} · {}",
                    match s.art {
                        StandArt::Sicherung => format!("Sicherung Nr. {}", s.nummer),
                        StandArt::Auto => "Automatisch gesichert".to_string(),
                    },
                    groesse_text(s.groesse)
                ),
                11
            ),
        ]
        .spacing(2),
        space::horizontal(),
        row![ansichts_knopf(z, Ansicht::Aenderungen, "Änderungen"), ansichts_knopf(z, Ansicht::Ganz, "Ganzer Text"), ansichts_knopf(z, Ansicht::Nebeneinander, "Nebeneinander")]
            .spacing(2),
    ]
    .align_y(Alignment::Center);

    // Benennen (nur Sicherungen; Autosave-Stände werden irgendwann ersetzt)
    let benennen: Element<'a, Message> = if s.art == StandArt::Sicherung {
        row![
            text_input("Stand benennen, z. B. „Vor Lektorat“", &z.name)
                .on_input(Message::ZeitName)
                .on_submit(Message::ZeitBenennen)
                .size(12)
                .padding([5, 9])
                .width(Fill),
            neben_knopf(if z.name.trim().is_empty() && s.name.is_some() { "Namen entfernen" } else { "★ Benennen" }, Message::ZeitBenennen),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    } else {
        gedimmt("Automatisch gesicherte Stände lassen sich nicht benennen; sie werden nach einiger Zeit ersetzt.", 11)
    };

    let kopfbereich = container(column![titelzeile, gedimmt(zusammenfassung, 12), benennen].spacing(8)).padding([14, 20]);

    let inhalt: Element<'a, Message> = match (&z.vergleich, &z.alter_text) {
        (Some(v), Some(_)) => vorschau(v, z.ansicht),
        _ => container(gedimmt("Keine Vorschau.", 13)).center(Fill).into(),
    };

    let legende = row![
        text("Grün unterstrichen: kommt zurück").size(11).color(thema::erfolg_gruen()),
        text("Rot durchgestrichen: geht verloren").size(11).color(thema::fehler_rot()),
    ]
    .spacing(16);

    let kann = z.alter_text.is_some();
    let mut aktionen = row![].spacing(6).align_y(Alignment::Center);
    aktionen = aktionen.push(legende).push(space::horizontal());
    if kann {
        aktionen = aktionen
            .push(neben_knopf("Als Kopie einfügen", Message::ZeitKopie))
            .push(neben_knopf("Text kopieren", Message::ZeitKopieren))
            .push(neben_knopf("Als Datei …", Message::ZeitExport))
            .push(haupt_knopf("Diesen Stand wiederherstellen", Message::ZeitWiederherstellen));
    }
    let fuss = container(
        column![
            aktionen,
            gedimmt("Beim Wiederherstellen wird der aktuelle Text vorher selbst als Stand gesichert; nichts geht verloren.", 11),
        ]
        .spacing(6),
    )
    .padding([10, 20])
    .width(Fill)
    .style(thema::dialog_fuss_stil);

    column![kopfbereich, thema::trennlinie(), container(inhalt).height(Fill), thema::trennlinie(), fuss].into()
}

fn ansichts_knopf<'a>(z: &Zeitleiste, a: Ansicht, name: &'a str) -> Element<'a, Message> {
    button(text(name).size(12))
        .padding([4, 10])
        .style(thema::tab_knopf_stil(z.ansicht == a))
        .on_press(Message::ZeitAnsicht(a))
        .into()
}

fn aufraeumen_ansicht<'a>(plan: &'a AufraeumPlan, uebersicht: (usize, u64)) -> Element<'a, Message> {
    let mut spalte = column![
        text("Aufräumen").size(16).font(thema::fett()).color(thema::text_hell()),
        gedimmt(
            "Die Regel: Die letzten 7 Tage bleiben vollständig erhalten. Danach bleibt je Tag der letzte Stand, \
             nach 90 Tagen je Woche. Benannte Stände (★) bleiben immer.",
            12,
        ),
    ]
    .spacing(8);
    if plan.zu_loeschen.is_empty() {
        spalte = spalte.push(text(format!("Es gibt nichts aufzuräumen. {} Stände belegen {}.", uebersicht.0, groesse_text(uebersicht.1))).size(13));
        spalte = spalte.push(row![neben_knopf("Zurück", Message::ZeitAufraeumenAbbruch)]);
    } else {
        spalte = spalte
            .push(
                container(
                    column![
                        text(format!("{} Stände würden gelöscht", plan.zu_loeschen.len())).size(15).color(thema::fehler_rot()),
                        gedimmt(format!("Das gibt {} frei. {} Stände bleiben.", groesse_text(plan.frei), plan.bleiben), 12),
                    ]
                    .spacing(3),
                )
                .padding(14)
                .width(Fill)
                .style(flaeche(thema::karte_hintergrund(), 8.0)),
            )
            .push(
                row![
                    neben_knopf("Abbrechen", Message::ZeitAufraeumenAbbruch),
                    iced::widget::button(text("Jetzt aufräumen").size(13))
                        .padding([6, 14])
                        .style(thema::gefahr_knopf_stil)
                        .on_press(Message::ZeitAufraeumenJa),
                ]
                .spacing(8),
            );
    }
    container(spalte.max_width(560)).padding(24).into()
}

// --------------------------------------------------------------- Textansicht

#[derive(Clone, Copy, PartialEq, Eq)]
enum Seite {
    Beide,
    Alt,
    Neu,
}

fn schrift() -> Font {
    thema::editor_font(false)
}

fn stueck_spans(stuecke: &[Stueck], seite: Seite) -> Vec<Span<'static, (), Font>> {
    let gruen = Color { a: 0.24, ..thema::erfolg_gruen() };
    let rot = Color { a: 0.22, ..thema::fehler_rot() };
    stuecke
        .iter()
        .filter(|s| match (s.art, seite) {
            (Art::Gleich, _) | (_, Seite::Beide) => true,
            (Art::Alt, Seite::Alt) | (Art::Neu, Seite::Neu) => true,
            _ => false,
        })
        .map(|s| {
            let basis = span(s.text.clone()).font(schrift()).size(14).color(thema::text_hell());
            match s.art {
                Art::Gleich => basis,
                Art::Alt => basis.background(gruen).underline(true),
                Art::Neu => basis.background(rot).strikethrough(true).color(thema::text_gedimmt()),
            }
        })
        .collect()
}

fn absatz<'a>(spans: Vec<Span<'static, (), Font>>) -> Element<'a, Message> {
    rich_text(spans).size(14).line_height(iced::widget::text::LineHeight::Relative(1.5)).width(Fill).into()
}

fn gleich_absatz<'a>(t: &str) -> Element<'a, Message> {
    absatz(vec![span(t.to_string()).font(schrift()).size(14).color(thema::text_hell())])
}

fn passage_kopf<'a>(nr: usize, gesamt: usize, a: &Aenderung) -> Element<'a, Message> {
    let alt: usize = a.stuecke.iter().filter(|s| s.art == Art::Alt).map(|s| s.text.split_whitespace().count()).sum();
    let neu: usize = a.stuecke.iter().filter(|s| s.art == Art::Neu).map(|s| s.text.split_whitespace().count()).sum();
    row![
        text(format!("Änderung {} von {gesamt}", nr + 1)).size(11).color(thema::text_gedimmt()),
        text(format!("+{alt} kämen zurück · −{neu} gingen verloren")).size(11).color(thema::text_gedimmt()),
        space::horizontal(),
        button(text("Diese Passage übernehmen").size(12))
            .padding([3, 10])
            .style(thema::sekundaer_knopf_stil)
            .on_press(Message::ZeitPassage(nr)),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn vorschau<'a>(v: &Vergleich, ansicht: Ansicht) -> Element<'a, Message> {
    let gesamt = v.anzahl();
    if gesamt == 0 && ansicht == Ansicht::Aenderungen {
        return container(gedimmt("Kein Unterschied zum aktuellen Text.", 13)).center(Fill).into();
    }
    let mut spalte = column![].spacing(12).padding([16, 20]);
    let mut nr = 0;
    for b in &v.bloecke {
        match b {
            Block::Gleich(t) => {
                if ansicht == Ansicht::Ganz {
                    spalte = spalte.push(gleich_absatz(t));
                } else if ansicht == Ansicht::Nebeneinander {
                    spalte = spalte.push(row![gleich_absatz(t), gleich_absatz(t)].spacing(24));
                }
            }
            Block::Aenderung(a) => {
                let inhalt: Element<'a, Message> = if ansicht == Ansicht::Nebeneinander {
                    row![absatz(stueck_spans(&a.stuecke, Seite::Alt)), absatz(stueck_spans(&a.stuecke, Seite::Neu))].spacing(24).into()
                } else {
                    absatz(stueck_spans(&a.stuecke, Seite::Beide))
                };
                spalte = spalte.push(
                    container(column![passage_kopf(nr, gesamt, a), inhalt].spacing(8))
                        .padding(12)
                        .width(Fill)
                        .style(flaeche(thema::karte_hintergrund(), 8.0)),
                );
                nr += 1;
            }
        }
    }
    let kopf: Element<'a, Message> = if ansicht == Ansicht::Nebeneinander {
        container(row![gedimmt("Gewählter Stand", 11), gedimmt("Jetzt", 11)].spacing(24)).padding(iced::Padding::from([8, 20]).bottom(0)).into()
    } else {
        space::vertical().height(0).into()
    };
    column![kopf, scrollable(spalte).height(Fill)].into()
}

// ------------------------------------------------------------------ Papierkorb

fn korb_links<'a>(z: &'a Zeitleiste) -> Element<'a, Message> {
    let mut liste = column![].spacing(2).padding([10, 10]);
    if z.korb.is_empty() {
        liste = liste.push(container(gedimmt("Der Papierkorb ist leer. Entfernte Buchteile liegen hier, bis du sie endgültig löschst.", 12)).padding(10));
    }
    for (i, k) in z.korb.iter().enumerate() {
        liste = liste.push(
            button(
                column![
                    text(&k.titel).size(13),
                    gedimmt(format!("Entfernt {} · {}", zeit_text(k.zeit), woerter_text(k.woerter)), 11),
                ]
                .spacing(1),
            )
            .width(Fill)
            .padding([7, 10])
            .style(thema::palette_eintrag_stil(z.korb_auswahl == Some(i)))
            .on_press(Message::KorbAuswahl(i)),
        );
    }
    scrollable(liste).height(Fill).into()
}

fn korb_rechts<'a>(z: &'a Zeitleiste) -> Element<'a, Message> {
    let Some(k) = z.korb_auswahl.and_then(|i| z.korb.get(i)) else {
        return container(gedimmt("Wähle links einen entfernten Teil.", 13)).center(Fill).into();
    };
    let vorschau: String = z.korb_text.chars().take(6000).collect();
    let kopf = container(
        column![
            text(&k.titel).size(16).font(thema::fett()).color(thema::text_hell()),
            gedimmt(format!("{} · {} · {}", k.datei, woerter_text(k.woerter), groesse_text(k.groesse)), 11),
        ]
        .spacing(2),
    )
    .padding([14, 20]);
    let loeschen: Element<'a, Message> = iced::widget::button(text(if z.korb_sicher { "Wirklich endgültig löschen?" } else { "Endgültig löschen" }).size(13))
        .padding([6, 14])
        .style(thema::gefahr_knopf_stil)
        .on_press(Message::KorbLoeschen)
        .into();
    let fuss = container(
        row![
            gedimmt("Zurückgeholt wird der Teil ans Ende des Buches.", 11),
            space::horizontal(),
            loeschen,
            haupt_knopf("Zurückholen", Message::KorbZurueck),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([10, 20])
    .width(Fill)
    .style(thema::dialog_fuss_stil);
    column![
        kopf,
        thema::trennlinie(),
        scrollable(container(gleich_absatz(&vorschau)).padding([16, 20])).height(Fill),
        thema::trennlinie(),
        fuss,
    ]
    .into()
}
