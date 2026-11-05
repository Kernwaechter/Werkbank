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
    /// Buchbau nach `buchbau.yaml`: (Buchordner, Bandnummer, Untertitel), wenn der Band dazu gehört.
    pub buchbau: Option<(PathBuf, u32, String)>,
    pub buchbau_laeuft: bool,
    pub buchbau_meldung: String,
    pub buchbau_hinweise: Vec<String>,
    pub buchbau_dateien: Vec<(PathBuf, u64)>,
    /// Gestaltung der Markdown-Elemente (aus .werkbank/satzstil.json).
    pub stil: werkbank_satz::gestaltung::Satzstil,
    /// Gewähltes Element der Gestaltung (Schlüssel, leer = Fließtext).
    pub stil_element: String,
    /// Gestaltungsfeld ausgeklappt.
    pub stil_offen: bool,
    /// Musterseite in Farbe (Web-PDF) statt in Graustufen (Druck).
    pub muster_farbig: bool,
    /// Zähler, damit nur die neueste Musterseite zählt.
    pub muster_version: u64,
    /// Musterseite als Bild.
    pub muster: Vec<iced::widget::image::Handle>,
    pub muster_meldung: String,
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
        text("Buch bauen").size(22).font(thema::fett()).color(thema::text_hell()),
        text("„Buch nativ setzen“ erzeugt alles ohne Python und Bash: Druck-PDFs je Portal (Schwarzweiß), Lese-/Web-PDF in Farbe und EPUB. Vorhandene Bauskripte des Bandes laufen weiter, sind aber optional.")
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
                text(&p.name).size(14).font(thema::fett()).color(thema::text_hell()),
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
    let mut werkzeuge = column![text("Voraussetzungen der Bauskripte (nur für Skripte nötig, nicht für den nativen Satz)").size(14).font(thema::fett()).color(thema::text_hell())].spacing(4);
    for (n, ok, pfad) in [werkzeug("python3"), werkzeug("typst"), werkzeug("pandoc")] {
        let (ok, pfad) = if n == "python3" && cfg!(windows) {
            let (prog, vor) = crate::bau::programm_aufloesen("python3");
            let gefunden = prog != "python3" || ok;
            (gefunden, if gefunden { format!("{prog} {}", vor.join(" ")) } else { "nicht gefunden (winget install Python.Python.3.12)".to_string() })
        } else {
            (ok, pfad)
        };
        werkzeuge = werkzeuge.push(punkt(n, pfad, ok));
    }
    if cfg!(windows) {
        let (prog, _) = crate::bau::programm_aufloesen("bash");
        let gefunden = prog != "bash";
        werkzeuge = werkzeuge.push(punkt("bash (Git für Windows)", if gefunden { prog } else { "nicht gefunden (winget install Git.Git)".to_string() }, gefunden));
    }

    // Befehle
    let mut befehle = column![
        row![
            text("Bauskripte des Bandes (optional)").size(14).font(thema::fett()).color(thema::text_hell()),
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
            text("Keine Bauskripte im Bandordner gefunden. Das ist in Ordnung: „Buch nativ setzen“ oben braucht keine. Skripte (build.sh, alle.py, build.py, scripts/build_book.py) oder eigene Befehle in .werkbank/bauen.json werden sonst hier angeboten.")
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

    // Buchbau nach buchbau.yaml (ersetzt das Bauskript des Buchs)
    let buchbau: Option<Element<'_, Message>> = bau.buchbau.as_ref().map(|(buch, nummer, untertitel)| {
        let mut spalte = column![
            row![
                text("Buchbau nach buchbau.yaml (ohne Skripte)").size(14).font(thema::fett()).color(thema::text_hell()),
                space::horizontal(),
                button(text(if bau.buchbau_laeuft { "Baue …".to_string() } else { format!("Band {nummer} bauen") }).size(12))
                    .padding([6, 14])
                    .style(thema::primaer_knopf_stil)
                    .on_press_maybe((!bau.buchbau_laeuft).then_some(Message::BuchbauStarten)),
            ]
            .align_y(Alignment::Center),
            text(format!(
                "{untertitel}: Print-PDF in Farbe (mit Cover) und in Graustufen (Seitenzahl durch 4 teilbar), EPUB 3 und die Ordner tolino/, kdp/ und webpublishing/ mit Umschlägen, E-Book-Covern und Hinweisen. Alles nativ – ohne Python, Chrome oder Pygments; Ghostscript wird nur für CMYK-Umschläge genutzt, wenn es installiert ist."
            ))
            .size(11)
            .color(thema::text_gedimmt()),
        ]
        .spacing(6);
        if !bau.buchbau_meldung.is_empty() {
            spalte = spalte.push(text(&bau.buchbau_meldung).size(11).color(thema::text_hell()));
        }
        for h in &bau.buchbau_hinweise {
            spalte = spalte.push(text(h).size(11).color(thema::warnung_orange()));
        }
        for (p, groesse) in &bau.buchbau_dateien {
            let name = p.strip_prefix(buch).unwrap_or(p).display().to_string();
            spalte = spalte.push(
                button(row![text(name).size(12), space::horizontal(), text(format!("{:.0} KB", *groesse as f64 / 1024.0)).size(11).color(thema::text_gedimmt())].align_y(Alignment::Center))
                    .width(Fill)
                    .padding([3, 8])
                    .style(thema::symbol_knopf_stil(false))
                    .on_press(Message::DateiOeffnen(p.clone())),
            );
        }
        karte(spalte)
    });

    // Nativer Satz
    let mut satz = column![
        row![
            text("Nativer Satz (Typst und EPUB, ohne Skripte)").size(14).font(thema::fett()).color(thema::text_hell()),
            space::horizontal(),
            button(text("Stil bearbeiten …").size(12)).padding([6, 14]).style(thema::sekundaer_knopf_stil).on_press(Message::SatzStilBearbeiten),
            button(text(if bau.satz_laeuft { "Setze …" } else { "Buch nativ setzen" }).size(12))
                .padding([6, 14])
                .style(thema::primaer_knopf_stil)
                .on_press_maybe((!bau.satz_laeuft).then_some(Message::SatzStarten)),
        ]
        .align_y(Alignment::Center),
        text("Setzt je Portal ein Schwarzweiß-Druck-PDF (mit PDF/X-3-Graustufen, wenn Ghostscript da ist), ein farbiges Lese-/Web-PDF und ein EPUB 3 in .werkbank/satz/. Kästen (::: info … :::) und Codeblöcke setzt der Stil; „Stil bearbeiten“ legt .werkbank/satz-stil.typ an (Schrift, Ränder, Überschriften, Kästen). Das Ergebnis ist nicht identisch mit der Ausgabe der Skripte.")
            .size(11)
            .color(thema::text_gedimmt()),
    ]
    .spacing(6);
    satz = satz.push(gestaltung(state));
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
    if cfg!(windows) && state.werkzeuge.ghostscript.is_none() {
        satz = satz.push(
            row![
                text("Für PDF/X-3 (Graustufen-Druckdatei) fehlt Ghostscript (ghostscript.com, „choco install ghostscript“ oder „scoop install ghostscript“). Ohne es entstehen die Druck-PDFs trotzdem.").size(11).color(thema::warnung_orange()),
                space::horizontal(),
                button(text("Erneut prüfen").size(11)).padding([4, 10]).style(thema::sekundaer_knopf_stil).on_press(Message::WerkzeugePruefen),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }
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
        row![text("Ausgabe").size(14).font(thema::fett()).color(thema::text_hell()), space::horizontal(), text(status.0).size(11).color(status.1)],
        container(scrollable(log).anchor_bottom().width(Fill).height(Length::Fixed(260.0)))
            .padding(8)
            .style(thema::codeblock_container_stil)
            .width(Fill),
    ]
    .spacing(6);

    // Ergebnisdateien
    let mut dateien = column![text("Erzeugte Dateien").size(14).font(thema::fett()).color(thema::text_hell())].spacing(3);
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
        text("Vorprüfung vor dem Satz").size(14).font(thema::fett()).color(thema::text_hell()),
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

    let mut inhalt = column![kopf];
    if let Some(b) = buchbau {
        inhalt = inhalt.push(b);
    }
    let inhalt = inhalt.extend([karte(satz), portale.into(), karte(befehle), karte(werkzeuge), karte(ausgabe), karte(dateien), karte(vor)])
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

/// Gestaltung der Markdown-Elemente: Element wählen, Felder ausfüllen, Musterseite ansehen.
fn gestaltung(state: &AppState) -> Element<'_, Message> {
    use werkbank_satz::gestaltung::{ELEMENTE, feld_name, felder_von};
    let bau = &state.bau;
    let kopf = button(text(if bau.stil_offen { "▾ Gestaltung der Elemente (Zitate, Code, Überschriften, Kästen …)" } else { "▸ Gestaltung der Elemente (Zitate, Code, Überschriften, Kästen …)" }).size(13))
        .padding([6, 10])
        .style(thema::sekundaer_knopf_stil)
        .on_press(Message::SatzStilOffen);
    if !bau.stil_offen {
        return kopf.into();
    }
    let vorlagen = werkbank_satz::gestaltung::vorlagen();
    let vorlage_namen: Vec<&'static str> = vorlagen.iter().map(|v| v.name).collect();
    let aktive = vorlagen.iter().find(|v| v.stil.formate == bau.stil.formate).map(|v| (v.name, v.beschreibung));
    let vorlagenzeile = column![
        row![
            text("Vorlage").size(11).color(thema::text_gedimmt()).width(130),
            pick_list(vorlage_namen, aktive.map(|(n, _)| n), |n: &str| Message::SatzStilVorlage(n.to_string()))
                .placeholder("Eigene Gestaltung")
                .text_size(12)
                .padding([4, 8])
                .style(thema::auswahl_stil)
                .menu_style(thema::auswahl_menue_stil),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(aktive.map_or("Eine Vorlage ersetzt die aktuelle Gestaltung; danach lässt sich jedes Element weiter ändern (die Vorlage selbst bleibt unverändert).", |(_, b)| b)).size(10).color(thema::text_gedimmt()),
    ]
    .spacing(4);
    let schluessel = if bau.stil_element.is_empty() { "text" } else { bau.stil_element.as_str() };
    let namen: Vec<&'static str> = ELEMENTE.iter().map(|(_, n)| *n).collect();
    let gewaehlt = ELEMENTE.iter().find(|(k, _)| *k == schluessel).map(|(_, n)| *n);
    let format = bau.stil.format(schluessel);

    let mut felder = column![].spacing(6);
    for feld in felder_von(schluessel) {
        let (name, hinweis) = feld_name(feld);
        let wert = format.feld(feld);
        let eingabe: Element<Message> = match *feld {
            "fett" | "kursiv" | "rand_seite" => {
                let optionen: Vec<&'static str> = if *feld == "rand_seite" { vec!["Standard", "links", "rundum", "keine"] } else { vec!["Standard", "ja", "nein"] };
                let aktuell = if wert.is_empty() { "Standard" } else { optionen.iter().copied().find(|o| *o == wert).unwrap_or("Standard") };
                let f = *feld;
                pick_list(optionen, Some(aktuell), move |w: &str| Message::SatzStilFeld(f.to_string(), if w == "Standard" { String::new() } else { w.to_string() }))
                    .text_size(12)
                    .padding([4, 8])
                    .style(thema::auswahl_stil)
                    .menu_style(thema::auswahl_menue_stil)
                    .into()
            }
            _ => {
                let f = *feld;
                let farbfeld = matches!(*feld, "textfarbe" | "fuellung" | "rand");
                let gueltig = wert.trim().is_empty() || !farbfeld || werkbank_satz::gestaltung::farbe_lesen(wert).is_some();
                let muster = werkbank_satz::gestaltung::farbe_lesen(wert).map_or(iced::Color::TRANSPARENT, |(r, g, b)| iced::Color::from_rgb8(r, g, b));
                let eingabefeld = iced::widget::text_input("Standard", wert).on_input(move |t| Message::SatzStilFeld(f.to_string(), t)).size(12).padding(5);
                if farbfeld {
                    row![
                        eingabefeld,
                        container(space::horizontal().width(24)).height(24).style(move |_t: &iced::Theme| container::Style {
                            background: Some(iced::Background::Color(muster)),
                            border: iced::Border { color: if gueltig { thema::rahmen_farbe() } else { thema::fehler_rot() }, width: 1.0, radius: 3.0.into() },
                            ..Default::default()
                        }),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .into()
                } else {
                    eingabefeld.into()
                }
            }
        };
        felder = felder.push(
            row![
                text(name).size(11).color(thema::text_gedimmt()).width(130),
                container(eingabe).width(220),
                text(hinweis).size(10).color(thema::text_gedimmt()),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    }

    let muster: Element<Message> = if let Some(bild) = bau.muster.first() {
        iced::widget::image(bild.clone()).width(Length::Fixed(330.0)).into()
    } else {
        text(if bau.muster_meldung.is_empty() { "Musterseite wird gesetzt …" } else { bau.muster_meldung.as_str() }).size(11).color(thema::text_gedimmt()).into()
    };
    let links = column![
        vorlagenzeile,
        row![
            text("Element").size(11).color(thema::text_gedimmt()).width(130),
            pick_list(namen, gewaehlt, |n: &str| Message::SatzStilElement(ELEMENTE.iter().find(|(_, x)| *x == n).map_or("text", |(k, _)| *k).to_string()))
                .text_size(12)
                .padding([4, 8])
                .style(thema::auswahl_stil)
                .menu_style(thema::auswahl_menue_stil),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        felder,
        row![
            button(text("Dieses Element zurücksetzen").size(11)).padding([4, 10]).style(thema::sekundaer_knopf_stil).on_press(Message::SatzStilZuruecksetzen(false)),
            button(text("Alles zurücksetzen").size(11)).padding([4, 10]).style(thema::sekundaer_knopf_stil).on_press(Message::SatzStilZuruecksetzen(true)),
        ]
        .spacing(8),
        toggler(bau.muster_farbig).label("Muster in Farbe (Web-PDF); aus = Graustufen wie im Druck").size(15).text_size(12).on_toggle(Message::SatzStilMusterFarbig),
        text("Leere Felder behalten die Voreinstellung. Farben als #rrggbb; im Druck werden sie zu Grau. Gespeichert in .werkbank/satzstil.json. Mehr Möglichkeiten: „Stil bearbeiten“ (Typst).")
            .size(10)
            .color(thema::text_gedimmt()),
    ]
    .spacing(8)
    .width(Fill);
    column![kopf, row![links, container(muster).padding(4).style(thema::codeblock_container_stil)].spacing(14)].spacing(8).into()
}
