//! # Bauen
//!
//! Startet die Bauskripte des Bandes (Tolino, Amazon KDP, Lese-PDF), zeigt die
//! Ausgabe live, die erzeugten Dateien und die Vorprüfungen vor dem Satz.

use crate::thema;
use crate::{AppState, Message};
use iced::font::Font;
use iced::widget::{pick_list, column, container, row, scrollable, space, text};
use crate::ui::fokus::{liste_mit_pfeilen, toggler, button};
use iced::{Alignment, Element, Fill, Length, Padding, Pixels};
use std::path::PathBuf;
use std::time::SystemTime;

/// Zustand der Bauansicht.
#[derive(Debug, Default)]
pub struct BauZustand {
    pub befehle: Vec<crate::bau::BauBefehl>,
    pub log: Vec<String>,
    /// Index des laufenden Befehls.
    pub laeuft: Option<usize>,
    pub abbruch: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub start: Option<SystemTime>,
    pub letzter_code: Option<Option<i32>>,
    pub dateien: Vec<(PathBuf, u64)>,
    /// Nativer Satz läuft.
    pub satz_laeuft: bool,
    /// Meldung zum letzten nativen Satzlauf.
    pub satz_meldung: String,
    /// Erzeugte Dateien des nativen Satzes (Pfad, Beschreibung).
    pub satz_dateien: Vec<(PathBuf, String)>,
}

fn karte<'a>(inhalt: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(inhalt).padding(14).width(Fill).style(thema::infobox_stil).into()
}

fn punkt<'a>(titel: &'a str, beschreibung: String, ok: bool) -> Element<'a, Message> {
    row![
        text(if ok { "✓" } else { "!" }).size(13).color(if ok { thema::erfolg_gruen() } else { thema::warnung_orange() }),
        text(titel).size(12).color(thema::text_hell()).width(220),
        text(beschreibung).size(12).color(thema::text_gedimmt()),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

pub fn ansicht(state: &AppState) -> Element<'_, Message> {
    let projekt = &state.projekt;
    let bau = &state.bau;
    let seiten = projekt.buchseiten_schaetzung() as u32;

    let kopf = column![
        text("Verlags- und Build-Pipeline").size(20).color(thema::text_hell()),
        text("Die vorhandenen Bauskripte des Bandes laufen im Hintergrund; Ausgabe und Ergebnis erscheinen hier.")
            .size(13)
            .color(thema::text_gedimmt()),
    ]
    .spacing(4);

    // Portale mit berechneter Rückenbreite
    let mut portale = row![].spacing(14);
    for p in &state.portale.portale {
        let (ub, uh) = p.umschlag_mm(seiten);
        portale = portale.push(karte(
            column![
                text(&p.name).size(15).color(thema::text_hell()),
                text(&p.beschreibung).size(12).color(thema::text_gedimmt()),
                text(format!(
                    "Druck {:.0} × {:.0} mm · Cover E-Book {} × {} px",
                    p.druck_format_mm.0, p.druck_format_mm.1, p.cover_ebook_px.0, p.cover_ebook_px.1
                ))
                .size(11)
                .color(thema::akzent()),
                text(format!(
                    "≈ {seiten} Seiten → Rücken {:.1} mm, Umschlag {ub:.1} × {uh:.1} mm (Beschnitt {} mm)",
                    p.ruecken_mm(seiten),
                    p.beschnitt_mm
                ))
                .size(11)
                .color(thema::text_hell()),
                text(format!("Quelle: {} · Stand {}", p.quelle, p.stand)).size(10).color(thema::text_gedimmt()),
            ]
            .spacing(5),
        ));
    }

    // Werkzeuge
    let werkzeug = |name: &'static str| {
        let pfad = crate::bau::programm_finden(name);
        (name, pfad.is_some(), pfad.map_or("nicht gefunden".to_string(), |p| p.display().to_string()))
    };
    let mut werkzeuge = column![text("Voraussetzungen").size(13).color(thema::text_hell())].spacing(4);
    for (n, ok, pfad) in [werkzeug("python3"), werkzeug("typst"), werkzeug("pandoc")] {
        werkzeuge = werkzeuge.push(punkt(n, pfad, ok));
    }

    // Befehle
    let mut befehle = column![
        row![
            text("Bauen").size(15).color(thema::text_hell()),
            space::horizontal(),
            if bau.laeuft.is_some() {
                button(text("Abbrechen").size(12)).padding([6, 14]).style(thema::sekundaer_knopf_stil).on_press(Message::BauAbbrechen)
            } else {
                button(text("Anlegen der Portalregeln als Datei").size(11))
                    .padding([4, 10])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::PortaleAnlegen)
            },
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(6);
    if bau.befehle.is_empty() {
        befehle = befehle.push(
            text("Keine Bauskripte gefunden (build.sh, alle.py, build.py oder scripts/build_book.py). Eigene Befehle gehören in .werkbank/bauen.json.")
                .size(12)
                .color(thema::warnung_orange()),
        );
    }
    for (i, b) in bau.befehle.iter().enumerate() {
        befehle = befehle.push(
            row![
                button(text(&b.name).size(12))
                    .padding([6, 14])
                    .width(240)
                    .style(if i == 0 { thema::primaer_knopf_stil } else { thema::sekundaer_knopf_stil })
                    .on_press_maybe(bau.laeuft.is_none().then_some(Message::BauStarten(i))),
                text(format!("{}   in {}", b.zeile(), b.ordner.display())).size(10).color(thema::text_gedimmt()),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
    }

    // Nativer Satz
    let mut satz = column![
        row![
            text("Nativer Satz (Typst und EPUB, ohne Skripte)").size(13).color(thema::text_hell()),
            space::horizontal(),
            button(text(if bau.satz_laeuft { "Setze …" } else { "Buch nativ setzen" }).size(12))
                .padding([6, 14])
                .style(thema::primaer_knopf_stil)
                .on_press_maybe((!bau.satz_laeuft).then_some(Message::SatzStarten)),
        ]
        .align_y(Alignment::Center),
        text("Setzt je Portal ein Schwarzweiß-Druck-PDF, dazu ein farbiges Lese-PDF und ein EPUB 3 in .werkbank/satz/. Das Ergebnis ist nicht identisch mit der Ausgabe der Skripte.")
            .size(11)
            .color(thema::text_gedimmt()),
    ]
    .spacing(6);
    let e = &state.einstellungen;
    const STELLUNGEN: [&str; 2] = ["Außen, abwechselnd (gerade links, ungerade rechts)", "Immer in der Mitte"];
    const ZAEHLUNGEN: [&str; 2] = ["Titelei römisch, Haupttext beginnt mit 1", "Durchgehend ab dem Titelblatt"];
    let stellung = if e.seitenzahl == "mitte" { STELLUNGEN[1] } else { STELLUNGEN[0] };
    let zaehlung = if e.zaehlung == "durchgehend" { ZAEHLUNGEN[1] } else { ZAEHLUNGEN[0] };
    let waehle = |s: &'static str| move |wahl: &str| Message::SatzSeitenzahl(if wahl == STELLUNGEN[1] { "mitte" } else { s }.to_string());
    let waehle_z = move |wahl: &str| Message::SatzZaehlung(if wahl == ZAEHLUNGEN[1] { "durchgehend" } else { "roemisch" }.to_string());
    satz = satz.push(
        column![
            row![
                column![
                    text("Seitenzahl").size(11).color(thema::text_gedimmt()),
                    liste_mit_pfeilen(
                        pick_list(&STELLUNGEN[..], Some(stellung), waehle("aussen")).text_size(12).padding([4, 8]).style(thema::auswahl_stil).menu_style(thema::auswahl_menue_stil),
                        STELLUNGEN.to_vec(),
                        Some(stellung),
                        waehle("aussen"),
                    ),
                ]
                .spacing(3),
                column![
                    text("Zählung").size(11).color(thema::text_gedimmt()),
                    liste_mit_pfeilen(
                        pick_list(&ZAEHLUNGEN[..], Some(zaehlung), waehle_z).text_size(12).padding([4, 8]).style(thema::auswahl_stil).menu_style(thema::auswahl_menue_stil),
                        ZAEHLUNGEN.to_vec(),
                        Some(zaehlung),
                        waehle_z,
                    ),
                ]
                .spacing(3),
            ]
            .spacing(16),
            toggler(e.teile_rechts).label("Teile beginnen auf einer rechten Seite (Leerseiten ohne Zahl)").size(15).text_size(12).on_toggle(Message::SatzTeileRechts),
            toggler(e.kapitel_rechts).label("Auch Kapitel beginnen rechts").size(15).text_size(12).on_toggle(Message::SatzKapitelRechts),
            toggler(e.pdfx).label(if state.werkzeuge.ghostscript.is_some() { "Am Ende PDF/X-3 erzeugen (Ghostscript, Graustufen)" } else { "Am Ende PDF/X-3 erzeugen (Ghostscript fehlt, siehe Einstellungen)" }).size(15).text_size(12).on_toggle(Message::SatzPdfx),
            toggler(e.teil_titelseiten).label("Jeder Teil bekommt eine eigene Titelseite (Rückseite leer)").size(15).text_size(12).on_toggle(Message::SatzTeiltitel),
            toggler(e.gerade_seitenzahl).label("Gerade Gesamtseitenzahl (Leerseite am Ende, wenn nötig)").size(15).text_size(12).on_toggle(Message::SatzGerade),
            toggler(e.titelei_zahlen).label("Seitenzahl auch auf Schmutztitel, Titelblatt, Impressum und Teiltiteln").size(15).text_size(12).on_toggle(Message::SatzTitelei),
            text("Seitenzahlen stehen auf jeder Textseite, auch auf Kapitelanfängen; Leerseiten bleiben leer. Das Inhaltsverzeichnis übernimmt immer die gedruckten Zahlen.")
                .size(10)
                .color(thema::text_gedimmt()),
        ]
        .spacing(6),
    );
    if !bau.satz_meldung.is_empty() {
        satz = satz.push(text(&bau.satz_meldung).size(11).color(thema::text_gedimmt()));
    }
    for (p, info) in &bau.satz_dateien {
        satz = satz.push(
            button(row![text(p.file_name().and_then(|n| n.to_str()).unwrap_or("?")).size(12), space::horizontal(), text(info).size(11).color(thema::text_gedimmt())].align_y(Alignment::Center))
                .width(Fill)
                .padding([3, 8])
                .style(thema::symbol_knopf_stil(false))
                .on_press(Message::DateiOeffnen(p.clone())),
        );
    }

    // Ausgabe
    let status = match (bau.laeuft, bau.letzter_code) {
        (Some(_), _) => ("läuft …".to_string(), thema::warnung_orange()),
        (None, Some(Some(0))) => ("erfolgreich beendet".to_string(), thema::erfolg_gruen()),
        (None, Some(Some(c))) => (format!("beendet mit Fehlercode {c}"), thema::fehler_rot()),
        (None, Some(None)) => ("abgebrochen oder nicht gestartet".to_string(), thema::warnung_orange()),
        (None, None) => ("noch kein Lauf".to_string(), thema::text_gedimmt()),
    };
    let mut log = column![].spacing(0);
    for z in bau.log.iter().rev().take(400).collect::<Vec<_>>().into_iter().rev() {
        log = log.push(text(z).size(11).font(Font::MONOSPACE));
    }
    let ausgabe = column![
        row![text("Ausgabe").size(13).color(thema::text_hell()), space::horizontal(), text(status.0).size(11).color(status.1)],
        container(scrollable(log).anchor_bottom().width(Fill).height(Length::Fixed(260.0)))
            .padding(8)
            .style(thema::codeblock_container_stil)
            .width(Fill),
    ]
    .spacing(6);

    // Ergebnisdateien
    let mut dateien = column![text("Erzeugte Dateien").size(13).color(thema::text_hell())].spacing(3);
    if bau.dateien.is_empty() {
        dateien = dateien.push(text("Nach einem Lauf erscheinen hier die neuen Dateien aus out/.").size(11).color(thema::text_gedimmt()));
    }
    for (p, groesse) in &bau.dateien {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        dateien = dateien.push(
            button(
                row![
                    text(name).size(12),
                    space::horizontal(),
                    text(format!("{:.1} MB", *groesse as f32 / 1_048_576.0)).size(11).color(thema::text_gedimmt()),
                ]
                .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([3, 8])
            .style(thema::symbol_knopf_stil(false))
            .on_press(Message::DateiOeffnen(p.clone())),
        );
    }

    // Vorprüfung
    let grenze = state.portale.code_zeilenlaenge;
    let zu_lang: usize = projekt.teile.iter().flat_map(|t| t.kapitel.iter()).map(|k| k.ueberlange_codezeilen_ab(grenze).len()).sum();
    let fehlende: usize = projekt
        .teile
        .iter()
        .flat_map(|t| t.kapitel.iter())
        .flat_map(|k| k.querverweise())
        .filter(|id| projekt.kapitel_finden(id).is_none())
        .count();
    let vor = column![
        text("Vorprüfung vor dem Satz").size(13).color(thema::text_hell()),
        punkt("Codezeilen-Länge", if zu_lang == 0 { format!("alle ≤ {grenze} Zeichen") } else { format!("{zu_lang} Zeilen länger als {grenze} Zeichen (Prüfen → Druckregeln)") }, zu_lang == 0),
        punkt("Querverweise", if fehlende == 0 { "alle @kap:-Ziele vorhanden".to_string() } else { format!("{fehlende} Verweise ohne Ziel") }, fehlende == 0),
        punkt("Gliederung", format!("{} Teile, {} Kapitel", projekt.teile.len(), projekt.kapitel_anzahl()), true),
        punkt(
            "ISBN und Metadaten",
            format!("Print-ISBN {} · Autor {}", projekt.metadaten.isbn_print.as_deref().unwrap_or("nicht vergeben"), projekt.metadaten.autor),
            projekt.metadaten.isbn_print.is_some(),
        ),
        punkt("Ungespeicherte Änderungen", if crate::dateien::hat_ungespeichertes(state) { "ja – vor dem Bauen speichern (Strg+S)".to_string() } else { "keine".to_string() }, !crate::dateien::hat_ungespeichertes(state)),
    ]
    .spacing(5);

    let inhalt = column![kopf, portale, karte(werkzeuge), karte(satz), karte(befehle), karte(ausgabe), karte(dateien), karte(vor)]
        .spacing(12)
        .max_width(Pixels(1000.0))
        .width(Fill);

    container(scrollable(inhalt).height(Fill))
        .center_x(Fill)
        .padding(Padding::from([20, 30]))
        .style(thema::editor_bereich_stil)
        .width(Fill)
        .height(Fill)
        .into()
}
