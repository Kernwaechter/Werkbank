//! # Cover-Designer
//!
//! Vorlage mit Eingabefeldern, Farbe je Text, Hintergrundbild über den ganzen Umschlag oder nur die
//! Vorderseite (zugeschnitten skaliert), Kästen mit wählbarer Deckkraft hinter Rücken- und Rückseitentext
//! und einer Vorschau in Farbe und in Graustufen (E-Reader). Export als PNG, SVG und PDF.

use crate::fokus_hilfe::{auswahl_mit_pfeilen, regler};
use crate::thema;
use crate::ui::fokus::{button, toggler};
use crate::{AppState, Message};
use iced::widget::{column, container, image, row, scrollable, space, text, text_input};
use iced::{Alignment, Element, Fill, Length, Padding};
use std::sync::Arc;
use werkbank_kern::cover::{self, CoverDaten};

/// Feld der Cover-Eingaben.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverFeld {
    Titel,
    Untertitel,
    Bandzeile,
    Autor,
    Hintergrund,
    Text,
    Akzent,
    FarbeBandzeile,
    FarbeTitel,
    FarbeUntertitel,
    FarbeAutor,
    FarbeRueckseite,
    FarbeRuecken,
    RueckenKastenFarbe,
    RueckseiteKastenFarbe,
    Seiten,
    Klappentext,
}

/// Regler der Cover-Vorlage (Werte 0 bis 100 in Prozent).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverRegler {
    Abdunkeln,
    RueckenDeckkraft,
    RueckseiteDeckkraft,
    /// Schriftgröße des Klappentextes in Punkt (nicht Prozent).
    RueckseiteSchrift,
}

/// Auswahlfelder der Cover-Vorlage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverAuswahl {
    BildModus,
    Ausrichtung,
}

/// Zustand der Cover-Ansicht.
#[derive(Default)]
pub struct CoverZustand {
    pub daten: CoverDaten,
    /// Vorschau in Farbe und als Graustufen-Fassung für E-Reader.
    pub vorschau: Option<image::Handle>,
    pub vorschau_grau: Option<image::Handle>,
    /// Originalpixel der Vorschau, damit sich die Graustufen ohne neuen Satz ändern.
    pub pixel: Option<(u32, u32, Arc<Vec<u8>>)>,
    pub version: u64,
    pub umschlag: bool,
    /// Graustufen mit 16 Stufen wie bei E-Ink-Displays.
    pub eink: bool,
    pub portal: usize,
    pub meldung: String,
    pub dateien: Vec<std::path::PathBuf>,
    /// Pixelmaße des gewählten Hintergrundbildes.
    pub bild_masse: Option<(u32, u32)>,
    /// Satz des Klappentextes auf der Rückseite (Größe, Passt-Prüfung) für das gewählte Portal.
    pub klappentext: Option<werkbank_satz::buchbau::umschlag::Bericht>,
    /// Liegt im Bandordner ein fertiges `cover.png`?
    pub cover_png: bool,
}

impl std::fmt::Debug for CoverZustand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CoverZustand")
    }
}

/// RGBA-Bild einer Vorschau.
pub type Bild = (u32, u32, Arc<Vec<u8>>);

fn feld<'a>(name: &'a str, wert: &str, f: CoverFeld) -> Element<'a, Message> {
    column![
        text(name).size(11).color(thema::text_gedimmt()),
        text_input("", wert).on_input(move |t| Message::CoverFeld(f, t)).size(13).padding(6),
    ]
    .spacing(3)
    .into()
}

fn parse_farbe(s: &str) -> Option<iced::Color> {
    let f = cover::farbe(s, "");
    let hex = f.strip_prefix('#')?;
    let voll = if hex.len() == 3 { hex.chars().flat_map(|c| [c, c]).collect::<String>() } else { hex.to_string() };
    let z = |i: usize| u8::from_str_radix(&voll[i..i + 2], 16).ok();
    Some(iced::Color::from_rgb8(z(0)?, z(2)?, z(4)?))
}

/// Farbfeld; leer bedeutet „Standard“ (`ersatz`).
fn farbfeld<'a>(name: &'a str, wert: &str, f: CoverFeld, ersatz: &str) -> Element<'a, Message> {
    let wirksam = if wert.trim().is_empty() { ersatz } else { wert };
    let farbe = parse_farbe(wirksam).unwrap_or(iced::Color::TRANSPARENT);
    let gueltig = wert.trim().is_empty() || !cover::farbe(wert, "").is_empty();
    column![
        text(name).size(11).color(thema::text_gedimmt()),
        row![
            text_input("Standard", wert).on_input(move |t| Message::CoverFeld(f, t)).size(13).padding(6),
            container(space::horizontal().width(24)).height(26).style(move |_t: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(farbe)),
                border: iced::Border {
                    color: if gueltig { thema::rahmen_farbe() } else { thema::fehler_rot() },
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            }),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .spacing(3)
    .into()
}

fn abschnitt<'a>(titel: &'a str, inhalt: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![text(titel).size(12).color(thema::akzent()), inhalt.into()].spacing(8).into()
}

/// Prüft eine ISBN-Eingabe: (in Ordnung, Hinweistext).
pub fn isbn_pruefung(eingabe: &str, barcode_an: bool) -> (bool, String) {
    use werkbank_satz::buchbau::barcode::isbn_ziffern;
    let t = eingabe.trim();
    if t.is_empty() {
        return (false, "Keine ISBN: Auf der Rückseite bleibt ein weißes Feld (Tolino: für den Barcode aus dem Tolino-Dashboard).".to_string());
    }
    let ziffern = t.chars().filter(|c| c.is_ascii_digit()).count();
    if isbn_ziffern(t).is_some() {
        let zusatz = if barcode_an { "Tolino: Der Barcode wird gesetzt. KDP: KDP druckt den Barcode selbst in die Freifläche." } else { "Der Barcode-Bereich ist oben ausgeschaltet, die ISBN wird nur gespeichert." };
        (true, format!("Gültige ISBN-13. {zusatz}"))
    } else if ziffern != 13 {
        (false, format!("Eine ISBN-13 hat 13 Ziffern, hier sind es {ziffern}."))
    } else if !t.replace(['-', ' '], "").starts_with("978") && !t.replace(['-', ' '], "").starts_with("979") {
        (false, "Eine ISBN-13 beginnt mit 978 oder 979.".to_string())
    } else {
        (false, "Die Prüfziffer (letzte Ziffer) stimmt nicht: Vertippt?".to_string())
    }
}

/// ISBN der Druckausgabe direkt im Cover-Bereich, mit Prüfung und Knopf zu allen Buchdaten.
fn isbn_feld(state: &AppState) -> Element<'_, Message> {
    let isbn = state.projekt.metadaten.isbn_print.as_deref().unwrap_or("");
    let (ok, hinweis) = isbn_pruefung(isbn, state.cover.daten.barcode);
    column![
        text("ISBN der Druckausgabe (für den Barcode; steht in den Buchdaten)").size(11).color(thema::text_gedimmt()),
        row![
            text_input("978-3-…", isbn).on_input(Message::CoverIsbn).size(13).padding(6),
            button(text("Buchdaten …").size(12)).padding([5, 8]).style(thema::link_knopf_stil).on_press(Message::MetaOeffnen),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
        row![
            text(if ok { "✓" } else { "!" }).size(11).color(if ok { thema::erfolg_gruen() } else { thema::warnung_orange() }),
            text(hinweis).size(11).color(thema::text_gedimmt()),
        ]
        .spacing(6),
    ]
    .spacing(3)
    .into()
}

/// Statt eines zweiten Textfeldes: Hinweis auf `klappentext.md`, die einzige Quelle der Rückseite.
fn klappentext_hinweis(state: &AppState) -> Element<'_, Message> {
    let datei = state.band.ordner.as_ref().map(|o| o.join("klappentext.md")).filter(|p| p.is_file());
    let mut spalte = column![
        text("Rückseitentext: klappentext.md im Bandordner").size(11).color(thema::text_gedimmt()),
        text("Aufbau: Dachzeile, # Schlagzeile, Absätze, ## Titel + Aufzählung = Kasten, ## Titel + Text = Infoblock (z. B. Autor), nach --- die Fußzeile ({seiten} = Seitenzahl)")
            .size(10)
            .color(thema::text_gedimmt()),
    ]
    .spacing(3);
    spalte = match datei {
        Some(p) => spalte.push(button(text("klappentext.md öffnen").size(12)).padding([3, 8]).style(thema::link_knopf_stil).on_press(Message::DateiOeffnen(p))),
        None => spalte.push(text("klappentext.md fehlt im Bandordner.").size(11).color(thema::warnung_orange())),
    };
    spalte.into()
}

pub fn ansicht(state: &AppState) -> Element<'_, Message> {
    let c = &state.cover;
    let d = &c.daten;
    let seiten = if d.seiten > 0 { d.seiten } else { state.satz_seiten.unwrap_or(state.projekt.buchseiten_schaetzung() as u32) };
    let portal_namen: Vec<String> = state.portale.portale.iter().map(|p| p.name.clone()).collect();
    let aktuelles = portal_namen.get(c.portal).cloned();
    let nur_vorn = d.bild_modus == "vorderseite";

    // ---- Texte
    let texte = column![
        feld("Titel", &d.titel, CoverFeld::Titel),
        feld("Untertitel", &d.untertitel, CoverFeld::Untertitel),
        feld("Bandzeile", &d.bandzeile, CoverFeld::Bandzeile),
        feld("Autor", &d.autor, CoverFeld::Autor),
        feld("Seitenzahl für den Rücken (0 = Schätzung aus dem Buch)", &if d.seiten == 0 { String::new() } else { d.seiten.to_string() }, CoverFeld::Seiten),
        klappentext_hinweis(state),
        regler("Schriftgröße der Rückseite", d.rueckseite_schrift_pt.round().clamp(6.0, 20.0) as u32, 6, 16, " pt", |w| Message::CoverRegler(CoverRegler::RueckseiteSchrift, w)),
        toggler(d.rueckseite_schrift_auto).label("Schrift verkleinern, wenn der Klappentext nicht passt").size(15).text_size(12).on_toggle(Message::CoverSchriftAuto),
        toggler(d.barcode).label("Freifläche für den Barcode auf der Rückseite").size(15).text_size(12).on_toggle(Message::CoverBarcode),
        isbn_feld(state),
    ]
    .spacing(8);

    // ---- Farben je Text
    let farben = column![
        row![
            farbfeld("Hintergrund", &d.hintergrund, CoverFeld::Hintergrund, "#1b1c1f"),
            farbfeld("Standard-Text", &d.text, CoverFeld::Text, "#f5f4f0"),
            farbfeld("Akzent", &d.akzent, CoverFeld::Akzent, "#7fa6c9"),
        ]
        .spacing(8),
        row![
            farbfeld("Bandzeile (vorn)", &d.farbe_bandzeile, CoverFeld::FarbeBandzeile, &d.akzent),
            farbfeld("Titel (vorn)", &d.farbe_titel, CoverFeld::FarbeTitel, &d.text),
            farbfeld("Untertitel (vorn)", &d.farbe_untertitel, CoverFeld::FarbeUntertitel, &d.text),
        ]
        .spacing(8),
        row![
            farbfeld("Autor (vorn)", &d.farbe_autor, CoverFeld::FarbeAutor, &d.text),
            farbfeld("Rückseitentext", &d.farbe_rueckseite, CoverFeld::FarbeRueckseite, &d.text),
            farbfeld("Rückentext", &d.farbe_ruecken, CoverFeld::FarbeRuecken, &d.text),
        ]
        .spacing(8),
    ]
    .spacing(8);

    // ---- Hintergrundbild
    const MODI: [&str; 2] = ["Über den ganzen Umschlag (Rückseite, Rücken, Vorderseite)", "Nur auf der Vorderseite"];
    const AUSRICHTUNGEN: [&str; 3] = ["Links", "Mitte", "Rechts"];
    let modus = if nur_vorn { MODI[1] } else { MODI[0] };
    let ausrichtung = match d.bild_ausrichtung.as_str() {
        "links" => AUSRICHTUNGEN[0],
        "rechts" => AUSRICHTUNGEN[2],
        _ => AUSRICHTUNGEN[1],
    };
    let bild = column![
        row![
            text(if d.bild.is_empty() { "kein Bild gewählt" } else { d.bild.as_str() }).size(12).width(Fill),
            button(text("Wählen …").size(11)).padding([4, 10]).style(thema::sekundaer_knopf_stil).on_press(Message::CoverBildWaehlen),
            button(text("Entfernen").size(11)).padding([4, 10]).style(thema::sekundaer_knopf_stil).on_press_maybe((!d.bild.is_empty()).then_some(Message::CoverBildGewaehlt(None))),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
        auswahl_mit_pfeilen("Bildbereich", &MODI, modus, |n| Message::CoverAuswahl(CoverAuswahl::BildModus, (if n == MODI[1] { "vorderseite" } else { "umschlag" }).to_string())),
        auswahl_mit_pfeilen("Sichtbarer Ausschnitt, wenn das Bild zugeschnitten wird", &AUSRICHTUNGEN, ausrichtung, |n| {
            Message::CoverAuswahl(CoverAuswahl::Ausrichtung, n.to_lowercase())
        }),
        regler("Bild abdunkeln", (d.bild_abdunkeln * 100.0).round() as u32, 0, 90, "%", |w| Message::CoverRegler(CoverRegler::Abdunkeln, w)),
    ]
    .spacing(8);

    // Auflösung: Anforderung je Portal und Bewertung des gewählten Bildes
    let mut aufloesung = column![text("Bildgröße für scharfen Druck (300 dpi) und E-Book").size(11).color(thema::text_gedimmt())].spacing(3);
    for a in cover::anforderungen(&state.portale.portale, seiten, nur_vorn) {
        aufloesung = aufloesung.push(
            row![
                text(format!("{}:", a.ziel)).size(11).color(thema::text_hell()),
                text(format!("mindestens {} × {} px", a.benoetigt.0, a.benoetigt.1)).size(11).color(thema::akzent()),
            ]
            .spacing(6),
        );
        aufloesung = aufloesung.push(text(a.erlaeuterung).size(10).color(thema::text_gedimmt()));
    }
    aufloesung = aufloesung.push(
        text("Das Bild wird so skaliert, dass es die Fläche füllt; was übersteht, wird abgeschnitten. Kleinere Bilder werden hochgerechnet (unschärfer), deshalb mindestens die genannte Größe verwenden. Wichtiges nicht an den Rand und nicht in den Rücken legen.")
            .size(10)
            .color(thema::text_gedimmt()),
    );
    if let Some((bw, bh)) = c.bild_masse {
        aufloesung = aufloesung.push(text(format!("Dein Bild: {bw} × {bh} px")).size(12).color(thema::text_hell()));
        for b in cover::bewerten((bw, bh), &state.portale.portale, seiten, nur_vorn) {
            let (zeichen, farbe) = match b.stufe {
                2 => ("✓", thema::erfolg_gruen()),
                1 => ("!", thema::warnung_orange()),
                _ => ("✗", thema::fehler_rot()),
            };
            aufloesung = aufloesung.push(
                row![text(zeichen).size(12).color(farbe), text(format!("{}: {}", b.ziel, b.text)).size(11).color(thema::text_gedimmt())].spacing(6),
            );
        }
    } else if !d.bild.is_empty() {
        aufloesung = aufloesung.push(text("Bildgröße nicht lesbar (nur PNG und JPEG).").size(11).color(thema::warnung_orange()));
    }

    // ---- Kästen
    let kaesten = column![
        text("Hinter dem Rückentext und dem Text der Rückseite steht auf Wunsch ein Kasten, damit er auf jedem Bild lesbar bleibt.")
            .size(10)
            .color(thema::text_gedimmt()),
        row![
            farbfeld("Rückenkasten: Farbe", &d.ruecken_kasten_farbe, CoverFeld::RueckenKastenFarbe, "#000000"),
            farbfeld("Rückseitenkasten: Farbe", &d.rueckseite_kasten_farbe, CoverFeld::RueckseiteKastenFarbe, "#000000"),
        ]
        .spacing(8),
        regler("Rückenkasten: Deckkraft", (d.ruecken_kasten_deckkraft * 100.0).round() as u32, 0, 100, "%", |w| Message::CoverRegler(CoverRegler::RueckenDeckkraft, w)),
        regler("Rückseitenkasten: Deckkraft", (d.rueckseite_kasten_deckkraft * 100.0).round() as u32, 0, 100, "%", |w| Message::CoverRegler(CoverRegler::RueckseiteDeckkraft, w)),
    ]
    .spacing(8);

    let formular = column![
        text("Cover-Vorlage").size(22).font(thema::fett()).color(thema::text_hell()),
        abschnitt("Texte", texte),
        abschnitt("Farben", farben),
        abschnitt("Hintergrundbild", column![bild, aufloesung].spacing(10)),
        abschnitt("Kästen für Rücken und Rückseite", kaesten),
    ]
    .spacing(16)
    .width(Length::Fixed(470.0));

    // ---- Prüfung gegen das Portal
    let mut pruefung = column![text("Prüfung").size(14).font(thema::fett()).color(thema::text_hell())].spacing(3);
    if let Some(p) = state.portale.portale.get(c.portal) {
        let (ub, uh) = p.umschlag_mm(seiten);
        let ruecken = p.ruecken_mm(seiten);
        let zeile = |ok: bool, t: String| -> Element<'_, Message> {
            row![
                text(if ok { "✓" } else { "!" }).size(12).color(if ok { thema::erfolg_gruen() } else { thema::warnung_orange() }),
                text(t).size(11).color(thema::text_gedimmt()),
            ]
            .spacing(6)
            .into()
        };
        pruefung = pruefung
            .push(zeile(seiten >= p.seiten_min && seiten <= p.seiten_max, format!("{seiten} Seiten (zulässig {}–{})", p.seiten_min, p.seiten_max)))
            .push(zeile(ruecken >= 6.0, format!("Rücken {ruecken:.1} mm{}", if ruecken >= 6.0 { " – Rückentext möglich" } else { " – zu schmal für Rückentext" })))
            .push(zeile(true, format!("Umschlag {ub:.1} × {uh:.1} mm inkl. {} mm Beschnitt", p.beschnitt_mm)))
            .push(zeile(true, format!("E-Book-Cover {} × {} px", p.cover_ebook_px.0, p.cover_ebook_px.1)))
            .push(zeile(d.barcode, if p.name.to_lowercase().contains("tolino") { "Barcode unten rechts auf der Rückseite (Tolino: 33,4 × 18,1 mm, aus der ISBN der Buchdaten; ohne gültige ISBN bleibt ein weißes Feld)".to_string() } else { "Barcode-Freifläche 50,8 × 30,5 mm auf der Rückseite (KDP druckt den Barcode selbst)".to_string() }));
        if let Some(k) = &c.klappentext {
            if k.hoehe_mm <= 0.0 {
                pruefung = pruefung.push(zeile(false, "Kein Klappentext (klappentext.md im Bandordner)".to_string()));
            } else if k.passt {
                let klein = if k.groesse_pt + 0.01 < k.gewuenscht_pt { format!(", auf {:.1} pt verkleinert (eingestellt {:.0} pt)", k.groesse_pt, k.gewuenscht_pt) } else { String::new() };
                pruefung = pruefung.push(zeile(true, format!("Klappentext passt auf die Rückseite ({:.0} von {:.0} mm, {:.1} pt{klein})", k.hoehe_mm, k.verfuegbar_mm, k.groesse_pt)));
            } else {
                pruefung = pruefung.push(
                    text(format!(
                        "! Der Klappentext ist zu lang: Er braucht {:.0} mm, auf der Rückseite sind {:.0} mm frei ({:.0} mm zu viel, bei {:.1} pt). Bitte den Text kürzen{}.",
                        k.hoehe_mm,
                        k.verfuegbar_mm,
                        k.ueberhang_mm,
                        k.groesse_pt,
                        if d.rueckseite_schrift_auto { "" } else { " oder „Schrift verkleinern“ einschalten" }
                    ))
                    .size(12)
                    .color(thema::fehler_rot()),
                );
            }
        }
    }

    let steuerung = row![
        toggler(c.umschlag).label("Gesamtumschlag zeigen").size(15).text_size(12).on_toggle(Message::CoverUmschlag),
        crate::ui::fokus::liste_mit_pfeilen(
            iced::widget::pick_list(portal_namen.clone(), aktuelles.clone(), Message::CoverPortal).text_size(12).padding([4, 8]).style(thema::auswahl_stil).menu_style(thema::auswahl_menue_stil),
            portal_namen,
            aktuelles,
            Message::CoverPortal,
        ),
        toggler(c.eink).label("Graustufen mit 16 Stufen (E-Ink)").size(15).text_size(12).on_toggle(Message::CoverEink),
        space::horizontal(),
        button(text("Alle Formate exportieren").size(12)).padding([6, 14]).style(thema::primaer_knopf_stil).on_press(Message::CoverExportieren),
    ]
    .spacing(14)
    .align_y(Alignment::Center);

    let bildfeld = |titel: &'static str, h: &Option<image::Handle>| -> Element<'_, Message> {
        let inhalt: Element<'_, Message> = match h {
            Some(h) => image(h.clone()).content_fit(iced::ContentFit::Contain).width(Fill).height(Fill).into(),
            None => text("wird erzeugt …").size(12).color(thema::text_gedimmt()).into(),
        };
        column![
            text(titel).size(11).color(thema::text_gedimmt()),
            container(inhalt).height(Length::Fixed(520.0)).width(Fill).style(thema::infobox_stil).padding(8),
        ]
        .spacing(4)
        .width(Length::FillPortion(1))
        .into()
    };
    let vorschau = row![
        bildfeld("Farbe", &c.vorschau),
        bildfeld(if c.eink { "E-Reader (16 Graustufen)" } else { "Graustufen" }, &c.vorschau_grau),
    ]
    .spacing(12);

    let mut rechts = column![steuerung, vorschau].spacing(10);
    if c.cover_png && d.bild.is_empty() && !c.umschlag {
        rechts = rechts.push(
            text("Die Vorschau zeigt das fertige cover.png aus dem Bandordner. Für einen Entwurf aus dieser Vorlage ein Hintergrundbild wählen (links unter „Hintergrundbild“) oder den Gesamtumschlag einschalten.")
                .size(11)
                .color(thema::warnung_orange()),
        );
    }
    if let Some(k) = c.klappentext.as_ref().filter(|k| k.hoehe_mm > 0.0 && !k.passt && c.umschlag) {
        rechts = rechts.push(
            text(format!("Klappentext zu lang: {:.0} mm zu viel auf der Rückseite. Text kürzen.", k.ueberhang_mm)).size(12).color(thema::fehler_rot()),
        );
    }
    rechts = rechts.push(text("Gerade bei E-Readern entscheidet die Graustufen-Fassung: Farbige Schrift auf farbigem Grund kann dort denselben Grauton haben und verschwinden.").size(10).color(thema::text_gedimmt()));
    rechts = rechts.push(pruefung);
    if !c.meldung.is_empty() {
        rechts = rechts.push(text(&c.meldung).size(11).color(thema::text_gedimmt()));
    }
    for datei in c.dateien.iter().take(10) {
        rechts = rechts.push(
            button(text(datei.display().to_string()).size(11)).padding([2, 6]).style(thema::link_knopf_stil).on_press(Message::DateiOeffnen(datei.clone())),
        );
    }

    container(scrollable(container(row![formular, rechts.width(Fill)].spacing(24)).padding(Padding::ZERO.right(16))).height(Fill))
        .padding(Padding::from([20, 30]))
        .style(thema::editor_bereich_stil)
        .width(Fill)
        .height(Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::isbn_pruefung;

    #[test]
    fn isbn_hinweise_sind_verstaendlich() {
        assert!(isbn_pruefung("978-3-9825412-0-4", true).0);
        assert!(isbn_pruefung("978-3-9825412-0-4", true).1.contains("Tolino: Der Barcode wird gesetzt"));
        assert!(isbn_pruefung("978-3-9825412-0-4", false).1.contains("ausgeschaltet"));
        let leer = isbn_pruefung("  ", true);
        assert!(!leer.0 && leer.1.contains("weißes Feld"));
        assert!(isbn_pruefung("978-3-98", true).1.contains("13 Ziffern, hier sind es 6"));
        assert!(isbn_pruefung("123-4-5678-9012-3", true).1.contains("978 oder 979"));
        assert!(isbn_pruefung("978-3-9825412-0-5", true).1.contains("Prüfziffer"));
    }
}
