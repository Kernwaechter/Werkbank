//! # Iced GUI für das Lizenzkey-Tool (Werkbank Lizenzgeber)
//!
//! Moderne, grafische Desktop-Oberfläche zum Ausstellen, Prüfen und Sichern
//! von Ed25519-Lizenzschlüsseln und Verwalten des Kundenregisters.

use chrono::Local;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Alignment, Background, Border, Color, Element, Length, Task, Theme};
use std::path::PathBuf;
use std::time::Instant;

use crate::{
    datenordner, hat_schluesselpaar, kunden_mail_text, lizenzschluessel_ausstellen,
    lizenzschluessel_pruefen, oeffentlich_laden, register_lesen, register_loeschen, register_sichern,
    schluesselpaar_anlegen, vollsicherung_schreiben, vollsicherung_wiederherstellen, vorlage_einspielen, Eintrag,
    LizenzStatus, PASSWORT_MINDESTLAENGE,
};

// Farbpalette im Stil der Werkbank
const FARBE_HINTERGRUND: Color = Color::from_rgb(0.08, 0.09, 0.11);
const FARBE_PANEL: Color = Color::from_rgb(0.12, 0.14, 0.18);
const FARBE_PANEL_HERVOR: Color = Color::from_rgb(0.16, 0.18, 0.23);
const FARBE_RAHMEN: Color = Color::from_rgb(0.20, 0.24, 0.30);
const FARBE_TEXT_HAUPT: Color = Color::from_rgb(0.92, 0.94, 0.96);
const FARBE_TEXT_GEDRAENGT: Color = Color::from_rgb(0.60, 0.65, 0.72);
const FARBE_RUST_ORANGE: Color = Color::from_rgb(0.88, 0.34, 0.15);
const FARBE_RUST_ORANGE_HOVER: Color = Color::from_rgb(0.94, 0.42, 0.22);
const FARBE_STAHLBLAU: Color = Color::from_rgb(0.22, 0.65, 0.90);
const FARBE_GRUEN: Color = Color::from_rgb(0.18, 0.72, 0.40);
const FARBE_ROT: Color = Color::from_rgb(0.88, 0.25, 0.25);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Ausstellen,
    Pruefen,
    Register,
    Status,
}

pub struct App {
    ordner: PathBuf,
    hat_schluessel: bool,
    oeffentlicher_schluessel: Option<String>,
    aktiver_tab: Tab,

    // Tab 1: Ausstellen
    ausstellen_email: String,
    ausstellen_name: String,
    ausstellen_bestellung: String,
    ausstellen_betrag: String,
    ausstellen_edition: String,
    letzter_schluessel: Option<String>,
    letzte_kunden_mail: Option<String>,
    ausstellen_hinweis: Option<(String, bool)>, // (Nachricht, ist_fehler)

    // Tab 2: Prüfen
    pruefen_email: String,
    pruefen_schluessel: String,
    pruefen_ergebnis: Option<(String, bool)>, // (Ergebnis, ist_gueltig)

    // Tab 3: Register
    suchbegriff: String,
    register_eintraege: Vec<Eintrag>,
    ausgewaehlter_eintrag: Option<Eintrag>,
    /// Eintrag, bei dem gerade „Wirklich löschen?“ gefragt wird.
    loesch_frage: Option<Eintrag>,

    // Tab 4: Status & Sicherung
    status_hinweis: Option<(String, bool)>,
    sicherung_passwort: String,
    sicherung_passwort2: String,

    // Toast / Feedback
    kopiert_meldung: Option<(String, Instant)>,
}

#[derive(Debug, Clone)]
pub enum Message {
    TabWaehlen(Tab),

    // Ausstellen
    EmailGeaendert(String),
    NameGeaendert(String),
    BestellungGeaendert(String),
    BetragGeaendert(String),
    EditionGeaendert(String),
    AusstellenAusfuehren,

    // Prüfen
    PruefenEmailGeaendert(String),
    PruefenSchluesselGeaendert(String),
    PruefenAusfuehren,

    // Register
    SuchbegriffGeaendert(String),
    EintragAuswaehlen(Eintrag),
    RegisterNeuLaden,
    LoeschenFragen(Eintrag),
    LoeschenAbbrechen,
    LoeschenBestaetigt,
    SicherungPasswort(String),
    SicherungPasswort2(String),
    VollsicherungWaehlen,
    VollsicherungZiel(Option<PathBuf>),
    WiederherstellenWaehlen,
    WiederherstellenQuelle(Option<PathBuf>),

    // Status / Backup / Init
    SchluesselpaarNeuAnlegen,
    RegisterSichernKlick,

    // Clipboard & Toast
    InZwischenablage(String, String), // (Text, Bezeichnung)
}

impl App {
    pub fn neu() -> (Self, Task<Message>) {
        let ordner = datenordner();
        vorlage_einspielen(&ordner);
        let hat_schluessel = hat_schluesselpaar(&ordner);
        let oeffentlicher_schluessel = oeffentlich_laden(&ordner).ok();
        let register_eintraege = register_lesen(&ordner);

        (
            Self {
                ordner,
                hat_schluessel,
                oeffentlicher_schluessel,
                aktiver_tab: Tab::Ausstellen,

                ausstellen_email: String::new(),
                ausstellen_name: String::new(),
                ausstellen_bestellung: String::new(),
                ausstellen_betrag: "49,00 €".to_string(),
                ausstellen_edition: "Einzelplatz".to_string(),
                letzter_schluessel: None,
                letzte_kunden_mail: None,
                ausstellen_hinweis: None,

                pruefen_email: String::new(),
                pruefen_schluessel: String::new(),
                pruefen_ergebnis: None,

                suchbegriff: String::new(),
                register_eintraege,
                ausgewaehlter_eintrag: None,
                loesch_frage: None,

                status_hinweis: None,
                sicherung_passwort: String::new(),
                sicherung_passwort2: String::new(),
                kopiert_meldung: None,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TabWaehlen(tab) => {
                self.aktiver_tab = tab;
                if tab == Tab::Register {
                    self.register_eintraege = register_lesen(&self.ordner);
                }
                Task::none()
            }

            // --- Ausstellen ---
            Message::EmailGeaendert(val) => {
                self.ausstellen_email = val;
                Task::none()
            }
            Message::NameGeaendert(val) => {
                self.ausstellen_name = val;
                Task::none()
            }
            Message::BestellungGeaendert(val) => {
                self.ausstellen_bestellung = val;
                Task::none()
            }
            Message::BetragGeaendert(val) => {
                self.ausstellen_betrag = val;
                Task::none()
            }
            Message::EditionGeaendert(val) => {
                self.ausstellen_edition = val;
                Task::none()
            }
            Message::AusstellenAusfuehren => {
                if !self.hat_schluessel {
                    self.ausstellen_hinweis = Some((
                        "Fehler: Kein privater Schlüssel vorhanden. Bitte zuerst unter „Status & Sicherung“ initialisieren."
                            .to_string(),
                        true,
                    ));
                    return Task::none();
                }

                match lizenzschluessel_ausstellen(
                    &self.ordner,
                    &self.ausstellen_email,
                    &self.ausstellen_name,
                    &self.ausstellen_bestellung,
                    &self.ausstellen_betrag,
                    &self.ausstellen_edition,
                ) {
                    Ok((schluessel, war_vorhanden, sicherung_nr)) => {
                        let mail_text = kunden_mail_text(
                            &self.ausstellen_name,
                            &self.ausstellen_email,
                            &schluessel,
                            &self.ausstellen_edition,
                        );
                        self.letzter_schluessel = Some(schluessel.clone());
                        self.letzte_kunden_mail = Some(mail_text);
                        self.register_eintraege = register_lesen(&self.ordner);

                        if war_vorhanden {
                            self.ausstellen_hinweis = Some((
                                format!(
                                    "Hinweis: Für „{}“ war bereits ein Schlüssel im Register hinterlegt. Er wurde unverändert neu berechnet.",
                                    self.ausstellen_email.trim()
                                ),
                                false,
                            ));
                        } else {
                            self.ausstellen_hinweis = Some((
                                format!(
                                    "✓ Lizenzschlüssel erfolgreich erzeugt, im Register eingetragen und gesichert (Sicherung #{sicherung_nr:04})."
                                ),
                                false,
                            ));
                        }
                    }
                    Err(f) => {
                        self.ausstellen_hinweis = Some((format!("Fehler: {f}"), true));
                    }
                }
                Task::none()
            }

            // --- Prüfen ---
            Message::PruefenEmailGeaendert(val) => {
                self.pruefen_email = val;
                Task::none()
            }
            Message::PruefenSchluesselGeaendert(val) => {
                self.pruefen_schluessel = val;
                Task::none()
            }
            Message::PruefenAusfuehren => {
                let status = lizenzschluessel_pruefen(
                    &self.ordner,
                    &self.pruefen_email,
                    &self.pruefen_schluessel,
                );
                match status {
                    LizenzStatus::Gueltig { name, edition, bis } => {
                        let bis_str = bis
                            .map(|d| format!(" (gültig bis {d})"))
                            .unwrap_or_else(|| " (unbefristet)".to_string());
                        self.pruefen_ergebnis = Some((
                            format!("✓ Gültig: Lizenz für „{name}“ [{edition}]{bis_str}."),
                            true,
                        ));
                    }
                    LizenzStatus::Ungueltig(grund) => {
                        self.pruefen_ergebnis = Some((format!("✗ Ungültig: {grund}"), false));
                    }
                    LizenzStatus::NichtEingerichtet => {
                        self.pruefen_ergebnis = Some((
                            "✗ Ungültig: Kein öffentlicher Schlüssel im Datenordner gefunden.".to_string(),
                            false,
                        ));
                    }
                    other => {
                        self.pruefen_ergebnis = Some((format!("Status: {other:?}"), false));
                    }
                }
                Task::none()
            }

            // --- Register ---
            Message::SuchbegriffGeaendert(val) => {
                self.suchbegriff = val;
                Task::none()
            }
            Message::EintragAuswaehlen(e) => {
                self.ausgewaehlter_eintrag = Some(e.clone());
                self.ausstellen_email = e.email.clone();
                self.ausstellen_name = e.name.clone();
                self.ausstellen_bestellung = e.bestellung.clone();
                self.ausstellen_betrag = e.betrag.clone();
                self.letzter_schluessel = Some(e.schluessel.clone());
                self.letzte_kunden_mail = Some(kunden_mail_text(&e.name, &e.email, &e.schluessel, &e.art));
                self.aktiver_tab = Tab::Ausstellen;
                Task::none()
            }
            Message::RegisterNeuLaden => {
                self.register_eintraege = register_lesen(&self.ordner);
                Task::none()
            }

            Message::LoeschenFragen(e) => {
                self.loesch_frage = Some(e);
                Task::none()
            }
            Message::LoeschenAbbrechen => {
                self.loesch_frage = None;
                Task::none()
            }
            Message::LoeschenBestaetigt => {
                if let Some(e) = self.loesch_frage.take() {
                    match register_loeschen(&self.ordner, &e) {
                        Ok(_) => {
                            self.register_eintraege = register_lesen(&self.ordner);
                            if self.ausgewaehlter_eintrag.as_ref() == Some(&e) {
                                self.ausgewaehlter_eintrag = None;
                            }
                            self.kopiert_meldung = Some((
                                format!("✓ Eintrag für {} gelöscht (alter Stand als Sicherung abgelegt)", e.email),
                                Instant::now(),
                            ));
                        }
                        Err(f) => {
                            self.kopiert_meldung = Some((format!("Löschen fehlgeschlagen: {f}"), Instant::now()));
                        }
                    }
                }
                Task::none()
            }

            // --- Status & Sicherung ---
            Message::SicherungPasswort(v) => {
                self.sicherung_passwort = v;
                Task::none()
            }
            Message::SicherungPasswort2(v) => {
                self.sicherung_passwort2 = v;
                Task::none()
            }
            Message::VollsicherungWaehlen => {
                if self.sicherung_passwort.chars().count() < PASSWORT_MINDESTLAENGE {
                    self.status_hinweis = Some((format!("Das Passwort braucht mindestens {PASSWORT_MINDESTLAENGE} Zeichen."), true));
                    return Task::none();
                }
                if self.sicherung_passwort != self.sicherung_passwort2 {
                    self.status_hinweis = Some(("Die beiden Passwörter stimmen nicht überein.".to_string(), true));
                    return Task::none();
                }
                let name = format!("werkbank-lizenzgeber-{}.wls", Local::now().format("%Y%m%d-%H%M%S"));
                Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title("Vollsicherung speichern (beliebiges Laufwerk)")
                            .set_file_name(name)
                            .add_filter("Sicherung des Lizenzgebers", &["wls"])
                            .save_file()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::VollsicherungZiel,
                )
            }
            Message::VollsicherungZiel(None) => Task::none(),
            Message::VollsicherungZiel(Some(ziel)) => {
                let ziel = if ziel.extension().is_none() { ziel.with_extension("wls") } else { ziel };
                self.status_hinweis = Some(match vollsicherung_schreiben(&self.ordner, &ziel, &self.sicherung_passwort) {
                    Ok((n, bytes)) => (
                        format!(
                            "✓ {n} Dateien verschlüsselt gesichert ({:.1} KB):\n  {}\nBewahre Datei und Passwort getrennt auf. Ohne das Passwort lässt sich die Sicherung nicht öffnen.",
                            bytes as f64 / 1024.0,
                            ziel.display()
                        ),
                        false,
                    ),
                    Err(f) => (format!("Sicherungsfehler: {f}"), true),
                });
                Task::none()
            }
            Message::WiederherstellenWaehlen => {
                if self.sicherung_passwort.is_empty() {
                    self.status_hinweis = Some(("Gib oben das Passwort der Sicherung ein.".to_string(), true));
                    return Task::none();
                }
                Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Sicherung zum Wiederherstellen wählen")
                            .add_filter("Sicherung des Lizenzgebers", &["wls"])
                            .pick_file()
                            .await
                            .map(|h| h.path().to_path_buf())
                    },
                    Message::WiederherstellenQuelle,
                )
            }
            Message::WiederherstellenQuelle(None) => Task::none(),
            Message::WiederherstellenQuelle(Some(quelle)) => {
                self.status_hinweis = Some(match vollsicherung_wiederherstellen(&quelle, &self.sicherung_passwort, &self.ordner) {
                    Ok(n) => {
                        self.hat_schluessel = hat_schluesselpaar(&self.ordner);
                        self.oeffentlicher_schluessel = oeffentlich_laden(&self.ordner).ok();
                        self.register_eintraege = register_lesen(&self.ordner);
                        (format!("✓ {n} Dateien wiederhergestellt nach {}", self.ordner.display()), false)
                    }
                    Err(f) => (format!("Wiederherstellung fehlgeschlagen: {f}"), true),
                });
                Task::none()
            }

            Message::SchluesselpaarNeuAnlegen => {
                match schluesselpaar_anlegen(&self.ordner, None) {
                    Ok(_) => {
                        self.hat_schluessel = true;
                        self.oeffentlicher_schluessel = oeffentlich_laden(&self.ordner).ok();
                        self.status_hinweis = Some((
                            "✓ Neues Ed25519-Schlüsselpaar erfolgreich in privat.key und oeffentlich.key angelegt."
                                .to_string(),
                            false,
                        ));
                    }
                    Err(f) => {
                        self.status_hinweis = Some((format!("Fehler: {f}"), true));
                    }
                }
                Task::none()
            }
            Message::RegisterSichernKlick => {
                match register_sichern(&self.ordner, None) {
                    Ok((anzahl, jsonl_pfad, csv_pfad)) => {
                        self.status_hinweis = Some((
                            format!(
                                "✓ {anzahl} Kundeneinträge erfolgreich exportiert nach:\n  - {}\n  - {}",
                                jsonl_pfad.display(),
                                csv_pfad.display()
                            ),
                            false,
                        ));
                    }
                    Err(f) => {
                        self.status_hinweis = Some((format!("Sicherungsfehler: {f}"), true));
                    }
                }
                Task::none()
            }

            // --- Clipboard ---
            Message::InZwischenablage(text_val, bezeichnung) => {
                self.kopiert_meldung = Some((
                    format!("✓ {bezeichnung} in die Zwischenablage kopiert!"),
                    Instant::now(),
                ));
                iced::clipboard::write(text_val)
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let kopfzeile = self.ansicht_kopfzeile();
        let inhalt = match self.aktiver_tab {
            Tab::Ausstellen => self.ansicht_ausstellen(),
            Tab::Pruefen => self.ansicht_pruefen(),
            Tab::Register => self.ansicht_register(),
            Tab::Status => self.ansicht_status(),
        };

        let fusszeile = self.ansicht_fusszeile();

        container(
            column![
                kopfzeile,
                container(inhalt).height(Length::Fill).width(Length::Fill),
                fusszeile,
            ]
            .spacing(12)
            .padding(16),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(FARBE_HINTERGRUND)),
            ..Default::default()
        })
        .into()
    }

    fn ansicht_kopfzeile(&self) -> Element<'_, Message> {
        let titel_block = column![
            row![
                text("WERKBANK")
                    .size(20)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(FARBE_RUST_ORANGE),
                text(" · Lizenzgeber")
                    .size(20)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
                    .color(FARBE_TEXT_HAUPT),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
            text(format!("Datenordner: {}", self.ordner.display()))
                .size(11)
                .color(FARBE_TEXT_GEDRAENGT),
        ]
        .spacing(2);

        let status_pille = if self.hat_schluessel {
            container(
                row![
                    text("●").size(12).color(FARBE_GRUEN),
                    text("Ed25519 aktiv").size(12).color(FARBE_GRUEN),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([4, 10])
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.18, 0.72, 0.40, 0.12))),
                border: Border {
                    color: Color::from_rgba(0.18, 0.72, 0.40, 0.35),
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            })
        } else {
            container(
                row![
                    text("●").size(12).color(FARBE_ROT),
                    text("Kein Schlüssel").size(12).color(FARBE_ROT),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([4, 10])
            .style(|_| container::Style {
                background: Some(Background::Color(Color::from_rgba(0.88, 0.25, 0.25, 0.15))),
                border: Border {
                    color: Color::from_rgba(0.88, 0.25, 0.25, 0.4),
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            })
        };

        let tabs = row![
            self.tab_knopf("🔑 Ausstellen", Tab::Ausstellen),
            self.tab_knopf("🔍 Prüfen", Tab::Pruefen),
            self.tab_knopf(
                format!("📋 Register ({})", self.register_eintraege.len()),
                Tab::Register
            ),
            self.tab_knopf("⚙️ Status & Sicherung", Tab::Status),
        ]
        .spacing(8);

        container(
            column![
                row![titel_block, space::horizontal(), status_pille]
                    .align_y(Alignment::Center)
                    .width(Length::Fill),
                tabs,
            ]
            .spacing(12),
        )
        .padding([12, 16])
        .style(|_| container::Style {
            background: Some(Background::Color(FARBE_PANEL)),
            border: Border {
                color: FARBE_RAHMEN,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn tab_knopf<'a>(&self, beschriftung: impl iced::widget::text::IntoFragment<'a>, tab: Tab) -> Element<'a, Message> {
        let ist_aktiv = self.aktiver_tab == tab;
        button(text(beschriftung).size(13).font(iced::Font {
            weight: if ist_aktiv {
                iced::font::Weight::Bold
            } else {
                iced::font::Weight::Normal
            },
            ..Default::default()
        }))
        .padding([6, 14])
        .on_press(Message::TabWaehlen(tab))
        .style(move |_, status| {
            if ist_aktiv {
                button::Style {
                    background: Some(Background::Color(FARBE_RUST_ORANGE)),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                let hover = matches!(status, button::Status::Hovered);
                button::Style {
                    background: Some(Background::Color(if hover {
                        FARBE_PANEL_HERVOR
                    } else {
                        Color::TRANSPARENT
                    })),
                    text_color: if hover {
                        Color::WHITE
                    } else {
                        FARBE_TEXT_GEDRAENGT
                    },
                    border: Border {
                        color: if hover {
                            FARBE_RAHMEN
                        } else {
                            Color::TRANSPARENT
                        },
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }
            }
        })
        .into()
    }

    // --- Tab 1: Ausstellen ---
    fn ansicht_ausstellen(&self) -> Element<'_, Message> {
        let eingaben_form = column![
            text("Neuen Lizenzschlüssel ausstellen")
                .size(16)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(FARBE_TEXT_HAUPT),
            text("Der Schlüssel wird deterministisch per Ed25519 signiert und dauerhaft im Kundenregister vermerkt.")
                .size(12)
                .color(FARBE_TEXT_GEDRAENGT),
            space::vertical().height(6),
            row![
                column![
                    text("E-Mail-Adresse des Kunden (Pflichtfeld):").size(12).color(FARBE_TEXT_HAUPT),
                    text_input("z. B. susanne@beispiel.de", &self.ausstellen_email)
                        .on_input(Message::EmailGeaendert)
                        .padding(8)
                        .size(13),
                ]
                .spacing(4)
                .width(Length::FillPortion(3)),
                column![
                    text("Vollständiger Name (optional):").size(12).color(FARBE_TEXT_HAUPT),
                    text_input("z. B. Susanne Becker", &self.ausstellen_name)
                        .on_input(Message::NameGeaendert)
                        .padding(8)
                        .size(13),
                ]
                .spacing(4)
                .width(Length::FillPortion(2)),
            ]
            .spacing(12),
            row![
                column![
                    text("Bestell- / Rechnungs-Nr.:").size(12).color(FARBE_TEXT_HAUPT),
                    text_input("z. B. cs_live_1234 oder PO-2026-9", &self.ausstellen_bestellung)
                        .on_input(Message::BestellungGeaendert)
                        .padding(8)
                        .size(13),
                ]
                .spacing(4)
                .width(Length::FillPortion(2)),
                column![
                    text("Betrag (€):").size(12).color(FARBE_TEXT_HAUPT),
                    text_input("49,00 €", &self.ausstellen_betrag)
                        .on_input(Message::BetragGeaendert)
                        .padding(8)
                        .size(13),
                ]
                .spacing(4)
                .width(Length::FillPortion(1)),
                column![
                    text("Edition / Art:").size(12).color(FARBE_TEXT_HAUPT),
                    text_input("Einzelplatz", &self.ausstellen_edition)
                        .on_input(Message::EditionGeaendert)
                        .padding(8)
                        .size(13),
                ]
                .spacing(4)
                .width(Length::FillPortion(2)),
            ]
            .spacing(12),
            space::vertical().height(4),
            row![
                button(
                    text("🚀 Lizenzschlüssel generieren & im Register speichern")
                        .size(13)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                )
                .on_press(Message::AusstellenAusfuehren)
                .padding([10, 20])
                .style(|_, status| {
                    let hover = matches!(status, button::Status::Hovered);
                    button::Style {
                        background: Some(Background::Color(if hover {
                            FARBE_RUST_ORANGE_HOVER
                        } else {
                            FARBE_RUST_ORANGE
                        })),
                        text_color: Color::WHITE,
                        border: Border {
                            radius: 6.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
                space::horizontal(),
            ],
        ]
        .spacing(10);

        let mut inhalt = column![self.karten_container(eingaben_form)].spacing(14);

        if let Some((meldung, ist_fehler)) = &self.ausstellen_hinweis {
            let farbe = if *ist_fehler { FARBE_ROT } else { FARBE_GRUEN };
            let hinweis_box = container(
                text(meldung).size(12).color(farbe)
            )
            .padding([8, 14])
            .style(move |_| container::Style {
                background: Some(Background::Color(if *ist_fehler {
                    Color::from_rgba(0.88, 0.25, 0.25, 0.12)
                } else {
                    Color::from_rgba(0.18, 0.72, 0.40, 0.12)
                })),
                border: Border {
                    color: farbe,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });
            inhalt = inhalt.push(hinweis_box);
        }

        if let Some(schluessel) = &self.letzter_schluessel {
            let ergebnis_card = column![
                row![
                    text("Ausgestellter Lizenzschlüssel:")
                        .size(14)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(FARBE_TEXT_HAUPT),
                    space::horizontal(),
                    button(text("📋 Schlüssel kopieren").size(12))
                        .on_press(Message::InZwischenablage(
                            schluessel.clone(),
                            "Lizenzschlüssel".to_string()
                        ))
                        .padding([4, 12])
                        .style(|_, status| {
                            let hover = matches!(status, button::Status::Hovered);
                            button::Style {
                                background: Some(Background::Color(if hover {
                                    FARBE_PANEL_HERVOR
                                } else {
                                    Color::from_rgba(0.22, 0.65, 0.90, 0.15)
                                })),
                                text_color: FARBE_STAHLBLAU,
                                border: Border {
                                    color: FARBE_STAHLBLAU,
                                    width: 1.0,
                                    radius: 6.0.into(),
                                },
                                ..Default::default()
                            }
                        }),
                ]
                .align_y(Alignment::Center),
                container(
                    text(schluessel)
                        .size(15)
                        .font(iced::Font::MONOSPACE)
                        .color(FARBE_STAHLBLAU)
                )
                .padding([10, 14])
                .width(Length::Fill)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgb(0.06, 0.07, 0.09))),
                    border: Border {
                        color: FARBE_RAHMEN,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }),
                space::vertical().height(6),
                row![
                    text("Fertige Auslieferungs-E-Mail an den Kunden:")
                        .size(13)
                        .font(iced::Font {
                            weight: iced::font::Weight::Bold,
                            ..Default::default()
                        })
                        .color(FARBE_TEXT_HAUPT),
                    space::horizontal(),
                    if let Some(mail) = &self.letzte_kunden_mail {
                        button(text("✉️ E-Mail-Text kopieren").size(12))
                            .on_press(Message::InZwischenablage(
                                mail.clone(),
                                "Kunden-E-Mail".to_string()
                            ))
                            .padding([4, 12])
                            .style(|_, status| {
                                let hover = matches!(status, button::Status::Hovered);
                                button::Style {
                                    background: Some(Background::Color(if hover {
                                        FARBE_PANEL_HERVOR
                                    } else {
                                        Color::from_rgba(0.88, 0.34, 0.15, 0.15)
                                    })),
                                    text_color: FARBE_RUST_ORANGE,
                                    border: Border {
                                        color: FARBE_RUST_ORANGE,
                                        width: 1.0,
                                        radius: 6.0.into(),
                                    },
                                    ..Default::default()
                                }
                            })
                    } else {
                        button(text(""))
                    },
                ]
                .align_y(Alignment::Center),
                if let Some(mail) = &self.letzte_kunden_mail {
                    container(
                        scrollable(
                            text(mail)
                                .size(11)
                                .font(iced::Font::MONOSPACE)
                                .color(FARBE_TEXT_GEDRAENGT)
                        )
                        .height(Length::Fixed(140.0))
                    )
                    .padding([10, 14])
                    .width(Length::Fill)
                    .style(|_| container::Style {
                        background: Some(Background::Color(Color::from_rgb(0.06, 0.07, 0.09))),
                        border: Border {
                            color: FARBE_RAHMEN,
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    })
                } else {
                    container(text(""))
                },
            ]
            .spacing(8);

            inhalt = inhalt.push(self.karten_container(ergebnis_card));
        }

        scrollable(inhalt.spacing(12)).into()
    }

    // --- Tab 2: Prüfen ---
    fn ansicht_pruefen(&self) -> Element<'_, Message> {
        let pruef_form = column![
            text("Lizenzschlüssel überprüfen")
                .size(16)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(FARBE_TEXT_HAUPT),
            text("Gegenprobe: Prüft offline mit dem öffentlichen Schlüssel, ob ein Schlüssel zur E-Mail-Adresse passt.")
                .size(12)
                .color(FARBE_TEXT_GEDRAENGT),
            space::vertical().height(6),
            column![
                text("E-Mail-Adresse:").size(12).color(FARBE_TEXT_HAUPT),
                text_input("z. B. kunde@beispiel.de", &self.pruefen_email)
                    .on_input(Message::PruefenEmailGeaendert)
                    .padding(8)
                    .size(13),
            ]
            .spacing(4),
            column![
                text("Lizenzschlüssel (Bindestriche & Leerzeichen werden ignoriert):")
                    .size(12)
                    .color(FARBE_TEXT_HAUPT),
                text_input("WB1-XXXXX-XXXXX-...", &self.pruefen_schluessel)
                    .on_input(Message::PruefenSchluesselGeaendert)
                    .padding(8)
                    .size(13),
            ]
            .spacing(4),
            space::vertical().height(4),
            button(
                text("🔍 Lizenzschlüssel jetzt prüfen")
                    .size(13)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
            )
            .on_press(Message::PruefenAusfuehren)
            .padding([8, 18])
            .style(|_, status| {
                let hover = matches!(status, button::Status::Hovered);
                button::Style {
                    background: Some(Background::Color(if hover {
                        Color::from_rgb(0.28, 0.72, 0.98)
                    } else {
                        FARBE_STAHLBLAU
                    })),
                    text_color: Color::WHITE,
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }),
        ]
        .spacing(10);

        let mut inhalt = column![self.karten_container(pruef_form)].spacing(14);

        if let Some((ergebnis, ist_gueltig)) = &self.pruefen_ergebnis {
            let farbe = if *ist_gueltig { FARBE_GRUEN } else { FARBE_ROT };
            let ergebnis_box = container(
                row![
                    text(if *ist_gueltig { "✓" } else { "✗" }).size(18).color(farbe),
                    text(ergebnis).size(13).color(farbe),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([12, 16])
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(if *ist_gueltig {
                    Color::from_rgba(0.18, 0.72, 0.40, 0.12)
                } else {
                    Color::from_rgba(0.88, 0.25, 0.25, 0.12)
                })),
                border: Border {
                    color: farbe,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            });
            inhalt = inhalt.push(ergebnis_box);
        }

        scrollable(inhalt).into()
    }

    // --- Tab 3: Register ---
    fn ansicht_register(&self) -> Element<'_, Message> {
        let such_zeile = row![
            text_input("🔍 Suchen nach E-Mail, Name oder Bestellnummer...", &self.suchbegriff)
                .on_input(Message::SuchbegriffGeaendert)
                .padding(8)
                .size(13)
                .width(Length::FillPortion(4)),
            button(text("🔄 Neu laden").size(12))
                .on_press(Message::RegisterNeuLaden)
                .padding([8, 14])
                .style(|_, status| {
                    let hover = matches!(status, button::Status::Hovered);
                    button::Style {
                        background: Some(Background::Color(if hover {
                            FARBE_PANEL_HERVOR
                        } else {
                            FARBE_PANEL
                        })),
                        text_color: FARBE_TEXT_HAUPT,
                        border: Border {
                            color: FARBE_RAHMEN,
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    }
                }),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let suchfilter = self.suchbegriff.trim().to_lowercase();
        let treffer: Vec<&Eintrag> = self
            .register_eintraege
            .iter()
            .rev() // Neueste zuerst
            .filter(|e| {
                if suchfilter.is_empty() {
                    return true;
                }
                e.email.to_lowercase().contains(&suchfilter)
                    || e.name.to_lowercase().contains(&suchfilter)
                    || e.bestellung.to_lowercase().contains(&suchfilter)
            })
            .collect();

        let statistik = row![
            text(format!("Gesamt: {} Lizenzen", self.register_eintraege.len()))
                .size(12)
                .color(FARBE_TEXT_GEDRAENGT),
            text(" · ").size(12).color(FARBE_TEXT_GEDRAENGT),
            text(format!("Treffer: {}", treffer.len()))
                .size(12)
                .color(FARBE_STAHLBLAU),
        ];

        let mut kunden_liste = column![].spacing(6);

        if treffer.is_empty() {
            kunden_liste = kunden_liste.push(
                container(
                    text("Keine Einträge gefunden.")
                        .size(13)
                        .color(FARBE_TEXT_GEDRAENGT)
                )
                .padding(20)
                .center_x(Length::Fill),
            );
        } else {
            for e in treffer {
                let e_clone = (*e).clone();
                let e_loeschen = (*e).clone();
                let wird_gefragt = self.loesch_frage.as_ref() == Some(e);
                let schluessel_kurz = if e.schluessel.len() > 20 {
                    format!("{}...{}", &e.schluessel[..12], &e.schluessel[e.schluessel.len() - 8..])
                } else {
                    e.schluessel.clone()
                };

                let zeile = container(
                    row![
                        column![
                            text(&e.email).size(13).font(iced::Font {
                                weight: iced::font::Weight::Bold,
                                ..Default::default()
                            }).color(FARBE_TEXT_HAUPT),
                            row![
                                text(if e.name.is_empty() { "–" } else { &e.name }).size(11).color(FARBE_TEXT_GEDRAENGT),
                                text(" · ").size(11).color(FARBE_TEXT_GEDRAENGT),
                                text(&e.zeit).size(11).color(FARBE_TEXT_GEDRAENGT),
                                if !e.betrag.is_empty() {
                                    row![
                                        text(" · ").size(11).color(FARBE_TEXT_GEDRAENGT),
                                        text(&e.betrag).size(11).color(FARBE_GRUEN),
                                    ]
                                } else {
                                    row![]
                                },
                            ],
                        ]
                        .spacing(2)
                        .width(Length::FillPortion(5)),
                        column![
                            text(schluessel_kurz).size(11).font(iced::Font::MONOSPACE).color(FARBE_STAHLBLAU),
                            text(if e.bestellung.is_empty() { "–" } else { &e.bestellung }).size(10).color(FARBE_TEXT_GEDRAENGT),
                        ]
                        .spacing(2)
                        .width(Length::FillPortion(3)),
                        row![
                            button(text("📋").size(12))
                                .on_press(Message::InZwischenablage(
                                    e.schluessel.clone(),
                                    format!("Schlüssel für {}", e.email)
                                ))
                                .padding([4, 8])
                                .style(|_, status| {
                                    let hover = matches!(status, button::Status::Hovered);
                                    button::Style {
                                        background: Some(Background::Color(if hover { FARBE_PANEL_HERVOR } else { FARBE_PANEL })),
                                        text_color: Color::WHITE,
                                        border: Border { color: FARBE_RAHMEN, width: 1.0, radius: 4.0.into() },
                                        ..Default::default()
                                    }
                                }),
                            button(text("Löschen").size(11))
                                .on_press(Message::LoeschenFragen(e_loeschen))
                                .padding([4, 10])
                                .style(|_, status| {
                                    let hover = matches!(status, button::Status::Hovered);
                                    button::Style {
                                        background: Some(Background::Color(if hover { Color::from_rgba(0.88, 0.25, 0.25, 0.25) } else { FARBE_PANEL })),
                                        text_color: FARBE_ROT,
                                        border: Border { color: FARBE_ROT, width: 1.0, radius: 4.0.into() },
                                        ..Default::default()
                                    }
                                }),
                            button(text("Details").size(11))
                                .on_press(Message::EintragAuswaehlen(e_clone))
                                .padding([4, 10])
                                .style(|_, status| {
                                    let hover = matches!(status, button::Status::Hovered);
                                    button::Style {
                                        background: Some(Background::Color(if hover { FARBE_RUST_ORANGE_HOVER } else { FARBE_RUST_ORANGE })),
                                        text_color: Color::WHITE,
                                        border: Border { radius: 4.0.into(), ..Default::default() },
                                        ..Default::default()
                                    }
                                }),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center)
                    .width(Length::Fill),
                )
                .padding([8, 12])
                .style(|_| container::Style {
                    background: Some(Background::Color(FARBE_PANEL)),
                    border: Border {
                        color: FARBE_RAHMEN,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                });

                kunden_liste = kunden_liste.push(zeile);
                if wird_gefragt {
                    kunden_liste = kunden_liste.push(
                        container(
                            column![
                                text(format!(
                                    "Eintrag für {} wirklich aus dem Register löschen? Der alte Stand bleibt als Sicherung erhalten. Der bereits verschickte Schlüssel bleibt in der Werkbank gültig, weil sie offline prüft.",
                                    e.email
                                ))
                                .size(12)
                                .color(FARBE_ROT),
                                row![
                                    button(text("Ja, löschen").size(12))
                                        .on_press(Message::LoeschenBestaetigt)
                                        .padding([5, 14])
                                        .style(|_, _| button::Style {
                                            background: Some(Background::Color(FARBE_ROT)),
                                            text_color: Color::WHITE,
                                            border: Border { radius: 4.0.into(), ..Default::default() },
                                            ..Default::default()
                                        }),
                                    button(text("Abbrechen").size(12))
                                        .on_press(Message::LoeschenAbbrechen)
                                        .padding([5, 14])
                                        .style(|_, _| button::Style {
                                            background: Some(Background::Color(FARBE_PANEL_HERVOR)),
                                            text_color: Color::WHITE,
                                            border: Border { color: FARBE_RAHMEN, width: 1.0, radius: 4.0.into() },
                                            ..Default::default()
                                        }),
                                ]
                                .spacing(8),
                            ]
                            .spacing(8),
                        )
                        .padding([8, 14])
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgba(0.88, 0.25, 0.25, 0.10))),
                            border: Border { color: FARBE_ROT, width: 1.0, radius: 6.0.into() },
                            ..Default::default()
                        }),
                    );
                }
            }
        }

        let inhalt = column![
            self.karten_container(column![such_zeile, statistik].spacing(6)),
            scrollable(kunden_liste).height(Length::Fill),
        ]
        .spacing(12);

        inhalt.into()
    }

    // --- Tab 4: Status & Sicherung ---
    fn ansicht_status(&self) -> Element<'_, Message> {
        let schluessel_status_card = column![
            text("Schlüsselpaar-Status (Ed25519)")
                .size(16)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(FARBE_TEXT_HAUPT),
            space::vertical().height(4),
            row![
                text("Privater Schlüssel (privat.key): ").size(13).color(FARBE_TEXT_GEDRAENGT),
                text(if self.hat_schluessel {
                    "✓ Vorhanden & geschützt (0600)"
                } else {
                    "✗ FEHLT"
                })
                .size(13)
                .color(if self.hat_schluessel { FARBE_GRUEN } else { FARBE_ROT }),
            ],
            row![
                text("Öffentlicher Schlüssel (oeffentlich.key): ").size(13).color(FARBE_TEXT_GEDRAENGT),
                text(if oeffentlich_laden(&self.ordner).is_ok() {
                    "✓ Vorhanden"
                } else {
                    "✗ FEHLT"
                })
                .size(13)
                .color(if oeffentlich_laden(&self.ordner).is_ok() { FARBE_GRUEN } else { FARBE_ROT }),
            ],
            space::vertical().height(6),
            if !self.hat_schluessel {
                column![
                    text("WICHTIG: Es wurde noch kein Schlüsselpaar im Datenordner angelegt.")
                        .size(12)
                        .color(FARBE_ROT),
                    button(text("⚡ Schlüsselpaar jetzt generieren").size(13))
                        .on_press(Message::SchluesselpaarNeuAnlegen)
                        .padding([8, 16])
                        .style(|_, status| {
                            let hover = matches!(status, button::Status::Hovered);
                            button::Style {
                                background: Some(Background::Color(if hover { FARBE_RUST_ORANGE_HOVER } else { FARBE_RUST_ORANGE })),
                                text_color: Color::WHITE,
                                border: Border { radius: 6.0.into(), ..Default::default() },
                                ..Default::default()
                            }
                        }),
                ]
                .spacing(8)
            } else if let Some(pub_b64) = &self.oeffentlicher_schluessel {
                column![
                    text("Öffentlicher Schlüssel (Base64 zum Einbetten in Werkbank):")
                        .size(11)
                        .color(FARBE_TEXT_GEDRAENGT),
                    container(text(pub_b64).size(12).font(iced::Font::MONOSPACE).color(FARBE_STAHLBLAU))
                        .padding([6, 10])
                        .width(Length::Fill)
                        .style(|_| container::Style {
                            background: Some(Background::Color(Color::from_rgb(0.06, 0.07, 0.09))),
                            border: Border { color: FARBE_RAHMEN, width: 1.0, radius: 4.0.into() },
                            ..Default::default()
                        }),
                    button(text("📋 Öffentlichen Schlüssel kopieren").size(12))
                        .on_press(Message::InZwischenablage(pub_b64.clone(), "Öffentlicher Schlüssel".to_string()))
                        .padding([6, 12])
                        .style(|_, status| {
                            let hover = matches!(status, button::Status::Hovered);
                            button::Style {
                                background: Some(Background::Color(if hover { FARBE_PANEL_HERVOR } else { FARBE_PANEL })),
                                text_color: Color::WHITE,
                                border: Border { color: FARBE_RAHMEN, width: 1.0, radius: 4.0.into() },
                                ..Default::default()
                            }
                        }),
                ]
                .spacing(6)
            } else {
                column![]
            },
        ]
        .spacing(8);

        let sicherung_card = column![
            text("Kundenregister exportieren (JSONL und CSV)")
                .size(16)
                .font(iced::Font {
                    weight: iced::font::Weight::Bold,
                    ..Default::default()
                })
                .color(FARBE_TEXT_HAUPT),
            text("Erstelle ein zeitgestempeltes Backup aller Lizenzen als JSONL und CSV (z. B. für Steuerberatung, Buchhaltung oder einen USB-Stick).")
                .size(12)
                .color(FARBE_TEXT_GEDRAENGT),
            space::vertical().height(6),
            button(
                text("💾 Register jetzt sichern (JSONL & CSV)")
                    .size(13)
                    .font(iced::Font {
                        weight: iced::font::Weight::Bold,
                        ..Default::default()
                    })
            )
            .on_press(Message::RegisterSichernKlick)
            .padding([8, 16])
            .style(|_, status| {
                let hover = matches!(status, button::Status::Hovered);
                button::Style {
                    background: Some(Background::Color(if hover { FARBE_PANEL_HERVOR } else { FARBE_PANEL })),
                    text_color: Color::WHITE,
                    border: Border { color: FARBE_STAHLBLAU, width: 1.0, radius: 6.0.into() },
                    ..Default::default()
                }
            }),
        ]
        .spacing(8);

        let knopf_rahmen = |_: &Theme, status: button::Status| {
            let hover = matches!(status, button::Status::Hovered);
            button::Style {
                background: Some(Background::Color(if hover { FARBE_PANEL_HERVOR } else { FARBE_PANEL })),
                text_color: Color::WHITE,
                border: Border { color: FARBE_STAHLBLAU, width: 1.0, radius: 6.0.into() },
                ..Default::default()
            }
        };
        let voll_card = column![
            text("Vollsicherung in eine Datei")
                .size(16)
                .font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() })
                .color(FARBE_TEXT_HAUPT),
            text("Sichert alles auf einmal in eine einzige, mit Passwort verschlüsselte Datei: privaten und öffentlichen Schlüssel, Kundenregister, alle Sicherungskopien und Geheimnisse. Die Datei kannst du auf jedem Laufwerk ablegen, zum Beispiel einem USB-Stick oder einer Netzfreigabe.")
                .size(12)
                .color(FARBE_TEXT_GEDRAENGT),
            text_input("Passwort (mindestens 8 Zeichen)", &self.sicherung_passwort)
                .on_input(Message::SicherungPasswort)
                .secure(true)
                .padding(8)
                .size(13),
            text_input("Passwort wiederholen", &self.sicherung_passwort2)
                .on_input(Message::SicherungPasswort2)
                .on_submit(Message::VollsicherungWaehlen)
                .secure(true)
                .padding(8)
                .size(13),
            row![
                button(text("💾 Alles in Datei sichern …").size(13).font(iced::Font { weight: iced::font::Weight::Bold, ..Default::default() }))
                    .on_press(Message::VollsicherungWaehlen)
                    .padding([8, 16])
                    .style(knopf_rahmen),
                button(text("Aus Sicherung wiederherstellen …").size(13))
                    .on_press(Message::WiederherstellenWaehlen)
                    .padding([8, 16])
                    .style(knopf_rahmen),
            ]
            .spacing(10),
            text("Das Passwort lässt sich nicht zurückholen. Schreibe es auf und bewahre es getrennt von der Sicherung auf. Wiederherstellen überschreibt nichts: Der Datenordner muss dafür leer sein.")
                .size(11)
                .color(FARBE_TEXT_GEDRAENGT),
        ]
        .spacing(8);

        let mut inhalt = column![
            self.karten_container(schluessel_status_card),
            self.karten_container(voll_card),
            self.karten_container(sicherung_card),
        ]
        .spacing(12);

        if let Some((meldung, ist_fehler)) = &self.status_hinweis {
            let farbe = if *ist_fehler { FARBE_ROT } else { FARBE_GRUEN };
            let box_el = container(text(meldung).size(12).color(farbe))
                .padding([8, 14])
                .style(move |_| container::Style {
                    background: Some(Background::Color(if *ist_fehler {
                        Color::from_rgba(0.88, 0.25, 0.25, 0.12)
                    } else {
                        Color::from_rgba(0.18, 0.72, 0.40, 0.12)
                    })),
                    border: Border { color: farbe, width: 1.0, radius: 6.0.into() },
                    ..Default::default()
                });
            inhalt = inhalt.push(box_el);
        }

        scrollable(inhalt).into()
    }

    fn ansicht_fusszeile(&self) -> Element<'_, Message> {
        let datum = Local::now().format("%d.%m.%Y").to_string();

        let linke_seite = row![
            text(format!("Werkbank Lizenzgeber · Stand {datum}"))
                .size(11)
                .color(FARBE_TEXT_GEDRAENGT),
        ];

        let rechte_seite = if let Some((msg, _)) = &self.kopiert_meldung {
            text(msg).size(12).color(FARBE_GRUEN)
        } else {
            text("Ed25519 Offline-Signaturen").size(11).color(FARBE_TEXT_GEDRAENGT)
        };

        row![linke_seite, space::horizontal(), rechte_seite]
            .padding([4, 8])
            .align_y(Alignment::Center)
            .into()
    }

    fn karten_container<'a>(&self, inhalt: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
        container(inhalt)
            .padding(16)
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(FARBE_PANEL)),
                border: Border {
                    color: FARBE_RAHMEN,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

fn titel(_app: &App) -> String {
    "Werkbank Lizenzgeber".to_string()
}

fn thema(_app: &App) -> Theme {
    Theme::Dark
}

/// Startet die grafische Iced-Benutzeroberfläche des Lizenzgebers.
pub fn start_gui() -> iced::Result {
    iced::application(App::neu, App::update, App::view)
        .title(titel)
        .theme(thema)
        .window(iced::window::Settings {
            size: iced::Size::new(960.0, 720.0),
            min_size: Some(iced::Size::new(820.0, 580.0)),
            position: iced::window::Position::Centered,
            icon: iced::window::icon::from_file_data(
                include_bytes!("../../../assets/icons/lizenzgeber/hicolor/256x256/apps/de.daehn.Werkbank.Lizenzgeber.png"),
                None,
            )
            .ok(),
            #[cfg(target_os = "linux")]
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: "de.daehn.Werkbank.Lizenzgeber".to_string(),
                ..Default::default()
            },
            ..Default::default()
        })
        .run()
}
