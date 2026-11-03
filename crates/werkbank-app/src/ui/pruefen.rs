//! # Prüfen
//!
//! Code-Prüfer für die Rust- und Dart/Flutter-Beispiele, Sprachprüfung über das ganze Buch
//! und Druckregeln (Länge der Codezeilen).

use crate::thema;
use crate::{AppState, Message};
use iced::font::Font;
use iced::widget::{column, container, progress_bar, row, scrollable, space, text};
use crate::ui::fokus::{button};
use iced::{Alignment, Element, Fill, Padding, Pixels};
use werkbank_kern::codepruefung::Ergebnis;

/// Zustand des Code-Prüfers.
#[derive(Debug, Clone, Default)]
pub struct CodeStatus {
    pub beispiele: Vec<werkbank_kern::codepruefung::Beispiel>,
    pub ergebnisse: Vec<Option<Ergebnis>>,
    pub laeuft: bool,
}

/// Zustand der Sprachprüfung über das ganze Buch.
#[derive(Debug, Clone, Default)]
pub struct BuchSprache {
    pub zeilen: Vec<(String, String, usize)>,
    pub gesamt: usize,
    pub laeuft: bool,
    pub fehler: Option<String>,
}

fn karte<'a>(inhalt: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(inhalt).padding(14).width(Fill).style(thema::infobox_stil).into()
}

fn link<'a>(beschriftung: String, nachricht: Message) -> Element<'a, Message> {
    button(text(beschriftung).size(11))
        .padding([2, 6])
        .width(Fill)
        .style(thema::symbol_knopf_stil(false))
        .on_press(nachricht)
        .into()
}

pub fn ansicht(state: &AppState) -> Element<'_, Message> {
    let projekt = &state.projekt;
    let code = &state.code;

    let kopf = column![
        text("Prüfstand").size(20).color(thema::text_hell()),
        text("Codebeispiele werden geprüft (Rust mit rustc, Dart/Flutter mit dart analyze), Texte mit LanguageTool, Druckregeln gegen die Portalwerte.")
            .size(13)
            .color(thema::text_gedimmt()),
    ]
    .spacing(4);

    // ---- Code-Prüfer
    let fertig = code.ergebnisse.iter().filter(|e| e.is_some()).count();
    let ok = code.ergebnisse.iter().filter(|e| matches!(e, Some(Ergebnis::Ok))).count();
    let fehler: Vec<usize> = code
        .ergebnisse
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e, Some(Ergebnis::Fehler(_))))
        .map(|(i, _)| i)
        .collect();
    let ausschnitte = code.ergebnisse.iter().filter(|e| matches!(e, Some(Ergebnis::Ausschnitt(_)))).count();
    let uebersprungen = code.ergebnisse.iter().filter(|e| matches!(e, Some(Ergebnis::Uebersprungen(_)))).count();
    let alle_kapitel = || projekt.teile.iter().flat_map(|t| t.kapitel.iter());
    let rust_bloecke: usize = alle_kapitel().map(|k| k.rust_codebloecke_anzahl()).sum();
    let dart_bloecke: usize = alle_kapitel().map(|k| k.dart_codebloecke_anzahl()).sum();
    let fortschritt = if code.beispiele.is_empty() { 0.0 } else { fertig as f32 / code.beispiele.len() as f32 * 100.0 };

    let mut code_spalte = column![
        row![
            text("Code-Prüfer").size(15).color(thema::text_hell()),
            space::horizontal(),
            button(text(if code.laeuft { "Prüfe …" } else { "Alle Codebeispiele prüfen" }).size(12))
                .padding([6, 14])
                .style(thema::primaer_knopf_stil)
                .on_press_maybe((!code.laeuft).then_some(Message::CodePruefungStarten)),
        ]
        .align_y(Alignment::Center),
        text(if code.beispiele.is_empty() {
            let mut teile = Vec::new();
            if rust_bloecke > 0 || dart_bloecke == 0 {
                teile.push(format!(
                    "{rust_bloecke} Rust-Codeblöcke: rustc (Edition {}) prüft Typen, ohne zu linken; Beispiele mit externen Crates werden übersprungen.",
                    state.portale.edition
                ));
            }
            if dart_bloecke > 0 {
                teile.push(format!(
                    "{dart_bloecke} Dart-/Flutter-Codeblöcke: dart analyze prüft alle in einem Durchgang. Ausschnitte werden passend eingebettet; Blöcke ohne Importe bekommen die ihres Kapitels; Pakete aus den Importen holt pub get."
                ));
            }
            teile.join("  ")
        } else {
            format!(
                "{fertig} von {} geprüft · {ok} in Ordnung · {} Fehler · {ausschnitte} Ausschnitte (unvollständiger Code) · {uebersprungen} übersprungen.  Absichtliche Fehlerbeispiele mit ```rust,compile_fail bzw. ```dart,compile_fail kennzeichnen, nicht prüfbare mit ignore.",
                code.beispiele.len(),
                fehler.len()
            )
        })
        .size(11)
        .color(thema::text_gedimmt()),
        progress_bar(0.0..=100.0, fortschritt).girth(4.0).style(progress_bar::primary),
    ]
    .spacing(6);
    for &i in fehler.iter().take(40) {
        let b = &code.beispiele[i];
        let meldung = match &code.ergebnisse[i] {
            Some(Ergebnis::Fehler(m)) => m.as_str(),
            _ => "",
        };
        code_spalte = code_spalte.push(
            column![
                link(
                    format!("✗ {} · {} · Zeile {}", if b.sprache == "rust" { "Rust" } else { "Dart" }, b.kapitel_titel, b.zeile),
                    Message::KapitelZeileOeffnen { kapitel_id: b.kapitel_id.clone(), zeile: b.zeile.saturating_sub(1) },
                ),
                container(text(meldung).size(10).font(Font::MONOSPACE).color(thema::fehler_rot()))
                    .padding([2, 10]),
            ]
            .spacing(1),
        );
    }
    if fehler.len() > 40 {
        code_spalte = code_spalte.push(text(format!("… und {} weitere Fehler", fehler.len() - 40)).size(11).color(thema::text_gedimmt()));
    }

    // ---- Sprache im ganzen Buch
    let s = &state.buch_sprache;
    let mut sprache_spalte = column![
        row![
            text("Sprachprüfung im ganzen Buch").size(15).color(thema::text_hell()),
            space::horizontal(),
            button(text(if s.laeuft { "Prüfe …" } else { "Ganzes Buch prüfen" }).size(12))
                .padding([6, 14])
                .style(thema::primaer_knopf_stil)
                .on_press_maybe((!s.laeuft && state.einstellungen.sprachpruefung).then_some(Message::BuchSpracheStarten)),
        ]
        .align_y(Alignment::Center),
        text(if !state.einstellungen.sprachpruefung {
            "Die Sprachprüfung ist in den Einstellungen ausgeschaltet.".to_string()
        } else if let Some(f) = &s.fehler {
            format!("Abgebrochen: {f}")
        } else if s.gesamt == 0 {
            "Prüft jedes Kapitel nacheinander mit dem eingestellten LanguageTool-Server.".to_string()
        } else {
            format!("{} von {} Kapiteln geprüft · {} Hinweise insgesamt", s.zeilen.len(), s.gesamt, s.zeilen.iter().map(|z| z.2).sum::<usize>())
        })
        .size(11)
        .color(thema::text_gedimmt()),
    ]
    .spacing(6);
    let mut sortiert = s.zeilen.clone();
    sortiert.sort_by_key(|a| std::cmp::Reverse(a.2));
    for (id, titel, n) in sortiert.into_iter().filter(|z| z.2 > 0).take(30) {
        sprache_spalte = sprache_spalte.push(link(format!("{n} Hinweise · {titel}"), Message::KapitelPruefenOeffnen(id)));
    }

    // ---- Druckregeln
    let grenze = state.portale.code_zeilenlaenge;
    let mut druck = column![
        text(format!("Druckregeln: Codezeilen höchstens {grenze} Zeichen")).size(15).color(thema::text_hell()),
    ]
    .spacing(4);
    let mut zu_lang = 0;
    for k in projekt.teile.iter().flat_map(|t| t.kapitel.iter()) {
        let lang = k.ueberlange_codezeilen_ab(grenze);
        if let Some((erste, _)) = lang.first() {
            zu_lang += lang.len();
            if zu_lang < 400 {
                druck = druck.push(link(
                    format!("{} Zeilen zu lang · {} (ab Zeile {erste})", lang.len(), k.titel),
                    Message::KapitelZeileOeffnen { kapitel_id: k.id.clone(), zeile: erste - 1 },
                ));
            }
        }
    }
    if zu_lang == 0 {
        druck = druck.push(text("Alle Codezeilen passen.").size(12).color(thema::erfolg_gruen()));
    }

    let inhalt = column![kopf, karte(code_spalte), karte(sprache_spalte), karte(druck)]
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
