//! # Einstellungen
//!
//! Schwebendes Fenster (`Strg+,`) mit fünf Seiten: Allgemein (Lizenz, Speichern, Pensum),
//! Editor (Darstellung), Prüfung (Rechtschreibung, LanguageTool, Ausnahmen – mit
//! Installationsanleitung über das Info-Symbol), Werkzeuge und Tastenkürzel.

use crate::Message;
use crate::einstellungen::{Einstellungen, SPRACHEN};
use crate::languagetool::ServerStatus;
use crate::thema;
use iced::font::Font;
use iced::widget::{column, container, mouse_area, opaque, pick_list, row, scrollable, slider, space, text, text_input};
use crate::ui::fokus::{button, liste_mit_pfeilen, regler_mit_pfeilen, toggler};
use iced::{Alignment, Element, Fill, Padding};
use std::fmt;
use werkbank_kern::sprache::Ausnahmen;

/// Betriebssystem für die Installationsanleitung.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Betriebssystem {
    Linux,
    Windows,
    MacOs,
}

impl Betriebssystem {
    pub const ALLE: [Self; 3] = [Self::Linux, Self::Windows, Self::MacOs];

    /// Das System, auf dem die Werkbank gerade läuft.
    pub fn aktuell() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }
}

impl fmt::Display for Betriebssystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Linux => "Linux",
            Self::Windows => "Windows",
            Self::MacOs => "macOS",
        })
    }
}

/// Zustand des Einstellungsfensters.
#[derive(Debug, Clone, Copy)]
pub struct Fenster {
    /// Installationsanleitung eingeblendet?
    pub info_offen: bool,
    /// Welches System die Anleitung zeigt.
    pub system: Betriebssystem,
    /// Installationshinweise für ein Werkzeug: 0 = keine, 1 = Pandoc, 2 = Ghostscript, 3 = Rust, 4 = Dart/Flutter.
    pub werkzeug_info: u8,
    /// Gewählte Seite (siehe [`SEITEN`]).
    pub seite: u8,
}

/// Seiten des Einstellungsfensters: (Titel, Kurzbeschreibung für die Befehlssuche).
pub const SEITEN: [(&str, &str); 5] = [
    ("Allgemein", "Lizenz, automatisch speichern, Tagespensum"),
    ("Editor", "Design, Schrift, Zeilenabstand, Spaltenbreite"),
    ("Prüfung", "Rechtschreibung, LanguageTool, Ausnahmen"),
    ("Werkzeuge", "Pandoc, Typst, Ghostscript, Java, Rust, Dart/Flutter, Skripte"),
    ("Tastenkürzel", "Kürzel ändern und zurücksetzen"),
];

impl Default for Fenster {
    fn default() -> Self {
        Self {
            info_offen: false,
            system: Betriebssystem::aktuell(),
            werkzeug_info: 0,
            seite: 0,
        }
    }
}

/// Ein Schritt der Anleitung: Erklärung plus optional Befehle zum Kopieren.
struct Schritt {
    titel: &'static str,
    befehle: &'static [&'static str],
}

const ZIP_URL: &str = "https://languagetool.org/download/LanguageTool-stable.zip";

fn anleitung(system: Betriebssystem) -> Vec<Schritt> {
    let verbinden = if crate::einstellungen::im_snap() {
        Schritt {
            titel: "Snap-Version: Die Werkbank kann Java nicht selbst starten. Im Terminal in den entpackten \
                    Ordner wechseln (zum Beispiel cd ~/LanguageTool-6.6) und den Server starten. Das Fenster \
                    offen lassen, solange du schreibst. Danach bei „Server“ die Adresse http://localhost:8081 \
                    eintragen und „Verbindung testen“ wählen.",
            befehle: &["java -cp languagetool-server.jar org.languagetool.server.HTTPServer --port 8081"],
        }
    } else {
        Schritt {
            titel: "In der Werkbank unter „Lokaler Server“ den entpackten Ordner wählen (darin liegt \
                    languagetool-server.jar) und „Server starten“ klicken. Nach etwa 10–30 Sekunden \
                    steht die Verbindung auf „Erreichbar“.",
            befehle: &[],
        }
    };
    let docker = Schritt {
        titel: "Alternative ohne Java: Server als Docker-Container betreiben. Dann bei der \
                Server-Adresse http://localhost:8081 eintragen und nicht „Server starten“ benutzen.",
        befehle: &["docker run -d --name languagetool --restart unless-stopped -p 8081:8010 erikvl87/languagetool"],
    };
    let manuell = Schritt {
        titel: "Alternative: Server von Hand im Terminal starten, im entpackten Ordner:",
        befehle: &["java -cp languagetool-server.jar org.languagetool.server.HTTPServer --port 8081"],
    };

    match system {
        Betriebssystem::Linux => vec![
            Schritt {
                titel: "Java 17 oder neuer installieren (eine Zeile, je nach Distribution):",
                befehle: &[
                    "sudo apt install default-jre",
                    "sudo dnf install java-latest-openjdk",
                    "sudo pacman -S jre-openjdk",
                ],
            },
            Schritt {
                titel: "LanguageTool herunterladen und entpacken:",
                befehle: &[
                    "curl -LO https://languagetool.org/download/LanguageTool-stable.zip",
                    "unzip LanguageTool-stable.zip -d ~/",
                ],
            },
            verbinden,
            manuell,
            docker,
        ],
        Betriebssystem::Windows => vec![
            Schritt {
                titel: "Java 17 oder neuer installieren (PowerShell):",
                befehle: &["winget install EclipseAdoptium.Temurin.21.JRE"],
            },
            Schritt {
                titel: "LanguageTool herunterladen und entpacken (PowerShell). Alternativ die ZIP-Datei \
                        im Browser laden und per Rechtsklick „Alle extrahieren“ wählen:",
                befehle: &[
                    "Invoke-WebRequest https://languagetool.org/download/LanguageTool-stable.zip -OutFile LanguageTool.zip",
                    "Expand-Archive LanguageTool.zip -DestinationPath C:\\LanguageTool",
                ],
            },
            verbinden,
            Schritt {
                titel: "Alternative: Server von Hand in der Eingabeaufforderung starten, im entpackten Ordner:",
                befehle: &["java -cp languagetool-server.jar org.languagetool.server.HTTPServer --port 8081"],
            },
            docker,
        ],
        Betriebssystem::MacOs => vec![
            Schritt {
                titel: "Java 17 oder neuer installieren (Homebrew):",
                befehle: &["brew install --cask temurin@21"],
            },
            Schritt {
                titel: "LanguageTool herunterladen und entpacken:",
                befehle: &[
                    "curl -LO https://languagetool.org/download/LanguageTool-stable.zip",
                    "unzip LanguageTool-stable.zip -d ~/",
                ],
            },
            verbinden,
            manuell,
            docker,
        ],
    }
}

const SCHRIFTARTEN: [&str; 3] = ["Serif (Libertinus, wie im Buchsatz)", "Sans (System)", "Monospace"];

fn schriftart_name(kennung: &str) -> &'static str {
    match kennung {
        "sans" => SCHRIFTARTEN[1],
        "mono" => SCHRIFTARTEN[2],
        _ => SCHRIFTARTEN[0],
    }
}

fn schriftart_kennung(name: &str) -> &'static str {
    if name == SCHRIFTARTEN[1] { "sans" } else if name == SCHRIFTARTEN[2] { "mono" } else { "serif" }
}

fn abschnitt<'a>(titel: &'a str) -> Element<'a, Message> {
    thema::abschnitt_titel(titel)
}

/// Eine Tafel zusammengehöriger Einstellungen.
fn tafel<'a>(inhalt: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(inhalt).padding(16).width(Fill).style(thema::gruppe_stil).into()
}

fn beschriftet<'a>(
    titel: &'a str,
    feld: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![text(titel).size(11).color(thema::text_gedimmt()), feld.into()]
        .spacing(3)
        .into()
}

/// Angaben für den Abschnitt „Lizenz“.
pub struct LizenzFelder<'a> {
    pub status: &'a werkbank_kern::lizenz::LizenzStatus,
    pub email: &'a str,
    pub schluessel: &'a str,
    pub meldung: Option<&'a str>,
    /// Rückfrage „Lizenz deaktivieren?“ ist offen.
    pub frage: bool,
}

/// Abschnitt „Lizenz“: Status, Felder für E-Mail-Adresse und Schlüssel, Aktivieren und Kaufen.
fn lizenz_abschnitt<'a>(l: LizenzFelder<'a>) -> Element<'a, Message> {
    use werkbank_kern::lizenz::LizenzStatus;
    let (status_text, farbe) = crate::ui::hilfe::status_text(l.status);
    let mut spalte = column![abschnitt("Lizenz"), text(status_text).size(13).color(farbe)].spacing(8);
    if matches!(l.status, LizenzStatus::Gueltig { .. } | LizenzStatus::NichtEingerichtet) {
        if let Some(m) = l.meldung {
            spalte = spalte.push(text(m).size(12).color(thema::text_gedimmt()));
        }
        if matches!(l.status, LizenzStatus::Gueltig { .. }) {
            if l.frage {
                spalte = spalte
                    .push(
                        text("Der Schlüssel wird von diesem Gerät entfernt und die Werkbank hier gesperrt. Danach öffnet sich eine E-Mail an den Anbieter, damit die neue Person einen eigenen Schlüssel bekommt (Lizenzvereinbarung, Abschnitt 4a).")
                            .size(12)
                            .color(thema::text_gedimmt()),
                    )
                    .push(
                        row![
                            button(text("Ja, deaktivieren").size(12)).padding([5, 12]).style(thema::primaer_knopf_stil).on_press(Message::LizenzDeaktivieren),
                            button(text("Abbrechen").size(12)).padding([5, 12]).style(thema::sekundaer_knopf_stil).on_press(Message::LizenzDeaktivierenFrage(false)),
                        ]
                        .spacing(8),
                    );
            } else {
                spalte = spalte.push(
                    button(text("Lizenz weitergeben (deaktivieren) …").size(12))
                        .padding([5, 12])
                        .style(thema::sekundaer_knopf_stil)
                        .on_press(Message::LizenzDeaktivierenFrage(true)),
                );
            }
        }
        return spalte.into();
    }
    spalte = spalte
        .push(beschriftet(
            "E-Mail-Adresse des Kaufs",
            text_input("name@beispiel.de", l.email).on_input(Message::LizenzEmail).size(13).padding(6),
        ))
        .push(beschriftet(
            "Lizenzschlüssel",
            text_input("WB1-…", l.schluessel)
                .on_input(Message::LizenzSchluessel)
                .on_submit(Message::LizenzAktivieren)
                .size(13)
                .padding(6),
        ))
        .push(
            row![
                button(text("Lizenzschlüssel aktivieren").size(12)).padding([5, 12]).style(thema::primaer_knopf_stil).on_press(Message::LizenzAktivieren),
                button(text("Kaufen").size(12)).padding([5, 12]).style(thema::sekundaer_knopf_stil).on_press(Message::KaufenOeffnen),
                button(text("Lizenzdatei wählen …").size(12)).padding([5, 12]).style(thema::sekundaer_knopf_stil).on_press(Message::LizenzWaehlen),
                text(l.meldung.unwrap_or("")).size(12).color(thema::warnung_orange()),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
    spalte.into()
}

/// Rendert das Einstellungsfenster als Overlay.
#[allow(clippy::too_many_arguments)]
pub fn ansicht<'a>(
    werkzeuge: &'a crate::werkzeuge::WerkzeugStatus,
    aufnahme: Option<&'a str>,
    wb_status: &'a str,
    e: &'a Einstellungen,
    fenster: Fenster,
    status: &'a ServerStatus,
    server_laeuft: bool,
    ausnahmen: &'a Ausnahmen,
    lizenz: LizenzFelder<'a>,
) -> Element<'a, Message> {
    let titelzeile = row![
        text("Einstellungen").size(18).font(thema::fett()).color(thema::text_hell()),
        space::horizontal(),
        crate::ui::navigation::tooltip_knopf(
            button(text("✕").size(13))
                .padding([3, 9])
                .style(thema::symbol_knopf_stil(false))
                .on_press(Message::EinstellungenSchliessen),
            "Schließen  ·  Esc",
        ),
    ]
    .align_y(Alignment::Center);

    let mut seiten = row![].spacing(2);
    for (nr, (name, _)) in SEITEN.iter().enumerate() {
        let nr = nr as u8;
        seiten = seiten.push(
            button(text(*name).size(13).center())
                .padding([5, 14])
                .style(thema::segment_stil(fenster.seite == nr))
                .on_press(Message::EinstellungenSeite(nr)),
        );
    }
    let seiten = container(container(seiten).padding(2).style(thema::segment_rahmen_stil)).center_x(Fill);

    let (status_text, status_farbe) = match status {
        ServerStatus::Unbekannt => ("● Noch nicht geprüft".to_string(), thema::text_gedimmt()),
        ServerStatus::Pruefe => ("● Teste Verbindung …".to_string(), thema::warnung_orange()),
        ServerStatus::Startet => ("● Server startet … (kann bis zu 30 s dauern)".to_string(), thema::warnung_orange()),
        ServerStatus::Erreichbar(info) => (format!("● Erreichbar – {info}"), thema::erfolg_gruen()),
        ServerStatus::Fehler(f) => (format!("● {f}"), thema::fehler_rot()),
    };

    let ueberschrift = row![
        abschnitt("Sprachprüfung (LanguageTool)"),
        crate::ui::navigation::tooltip_knopf(
            button(text("ⓘ").size(15))
                .padding([0, 6])
                .style(thema::symbol_knopf_stil(fenster.info_offen))
                .on_press(Message::EinstellungenInfoToggle),
            "Installationsanleitung für LanguageTool",
        ),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let darstellung = column![
        abschnitt("Darstellung"),
        toggler(e.hell)
            .label("Helles Design „Papier“ (sonst „Dämmerung“)")
            .size(16)
            .text_size(12)
            .on_toggle(Message::HellToggle),
        toggler(e.schreibmaschine)
            .label("Schreibmaschinen-Modus (Cursorzeile bleibt in der Mitte)")
            .size(16)
            .text_size(12)
            .on_toggle(Message::SchreibmaschineToggle),
        beschriftet(
            "Schriftart im Editor",
            liste_mit_pfeilen(
                pick_list(&SCHRIFTARTEN[..], Some(schriftart_name(&e.schriftart)), |n: &str| Message::SchriftartSetzen(schriftart_kennung(n).to_string()))
                    .text_size(12)
                    .padding([5, 8])
                    .style(thema::auswahl_stil)
                    .menu_style(thema::auswahl_menue_stil),
                SCHRIFTARTEN.to_vec(),
                Some(schriftart_name(&e.schriftart)),
                |n: &str| Message::SchriftartSetzen(schriftart_kennung(n).to_string()),
            ),
        ),
        beschriftet(
            "Zeilenabstand",
            row![
                regler_mit_pfeilen(
                    slider(12..=20u32, (e.zeilenabstand * 10.0).round() as u32, |w| Message::ZeilenabstandSetzen(w as f32 / 10.0))
                        .style(thema::regler_stil),
                    (e.zeilenabstand * 10.0).round() as u32, 1, 12, 20,
                    |w| Message::ZeilenabstandSetzen(w as f32 / 10.0),
                ),
                text(format!("{:.1} ×", e.zeilenabstand)).size(12),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ),
        beschriftet(
            "Schriftgröße im Editor",
            row![
                regler_mit_pfeilen(
                    slider(12..=26u32, e.schriftgroesse.round() as u32, |w| Message::SchriftSetzen(w as f32))
                        .style(thema::regler_stil),
                    e.schriftgroesse.round() as u32, 1, 12, 26,
                    |w| Message::SchriftSetzen(w as f32),
                ),
                text(format!("{:.0} pt", e.schriftgroesse)).size(12),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ),
        beschriftet(
            "Breite der Textspalte",
            row![
                regler_mit_pfeilen(
                    slider(560..=1400u32, e.spaltenbreite.round() as u32, |w| Message::SpalteSetzen(w as f32))
                        .step(20u32)
                        .style(thema::regler_stil),
                    e.spaltenbreite.round() as u32, 20, 560, 1400,
                    |w| Message::SpalteSetzen(w as f32),
                ),
                text(format!("{:.0} px", e.spaltenbreite)).size(12),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ),
    ]
    .spacing(8);

    let editor = column![
        abschnitt("Schreiben"),
        toggler(e.autospeichern)
            .label("Automatisch speichern (nach 8 Sekunden Pause, mit Snapshots)")
            .size(16)
            .text_size(12)
            .on_toggle(Message::AutospeichernToggle),
        beschriftet(
            "Tagespensum",
            row![
                regler_mit_pfeilen(
                    slider(250..=5000u32, e.tagespensum.clamp(250, 5000) as u32, |w| Message::PensumSetzen(w as usize))
                        .step(250u32)
                        .style(thema::regler_stil),
                    e.tagespensum.clamp(250, 5000) as u32, 250, 250, 5000,
                    |w| Message::PensumSetzen(w as usize),
                ),
                text(format!("{} Wörter", e.tagespensum)).size(12),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ),
    ]
    .spacing(8);

    let rechtschreibung = column![
        abschnitt("Rechtschreibung im Text (offline, Hunspell)"),
        toggler(e.rechtschreibung)
            .label("Unbekannte Wörter im Editor rot markieren")
            .size(16)
            .text_size(12)
            .on_toggle(Message::RechtschreibungToggle),
        beschriftet(
            "Ordner mit Wörterbüchern (leer = übliche Orte)",
            text_input("z. B. /usr/share/hunspell", &e.woerterbuch_ordner)
                .on_input(Message::WoerterbuchOrdner)
                .size(12)
                .padding(6),
        ),
        text(wb_status).size(11).color(thema::text_gedimmt()),
    ]
    .spacing(6);

    // `warnung`: gefunden, aber nur eingeschränkt brauchbar (Text in Klammern)
    let werkzeug_zeile = |name: &'static str, version: &Option<String>, nr: u8, hinweis: &'static str, warnung: Option<&'static str>| -> Element<'a, Message> {
        let (status, farbe) = match version {
            Some(v) if warnung.is_some() => (format!("✓ {v}  ({})", warnung.unwrap_or_default()), thema::warnung_orange()),
            Some(v) => (format!("✓ {v}"), thema::erfolg_gruen()),
            None if werkzeuge.geprueft => ("✗ nicht gefunden".to_string(), thema::fehler_rot()),
            None => ("prüfe …".to_string(), thema::text_gedimmt()),
        };
        column![
            row![
                text(name).size(12).color(thema::text_hell()).width(110),
                text(status).size(12).color(farbe),
                space::horizontal(),
                crate::ui::navigation::tooltip_knopf(
                    button(text("ⓘ").size(15))
                        .padding([0, 6])
                        .style(thema::symbol_knopf_stil(fenster.werkzeug_info == nr))
                        .on_press(Message::WerkzeugInfoToggle(nr)),
                    "So installierst du es (Windows, macOS, Linux)",
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(hinweis).size(10).color(thema::text_gedimmt()),
        ]
        .spacing(2)
        .into()
    };
    let pandoc_alt = werkzeuge.pandoc.is_some() && !werkzeuge.pandoc_brauchbar();
    let dart_flutter = match (&werkzeuge.flutter, &werkzeuge.dart) {
        (Some(f), Some(d)) => Some(format!("Flutter {f} · Dart {d}")),
        (Some(f), None) => Some(format!("Flutter {f}")),
        (None, Some(d)) => Some(format!("Dart {d}")),
        (None, None) => None,
    };
    let mut werkzeug_spalte = column![
        row![
            abschnitt("Externe Werkzeuge (Weg der Bauskripte)"),
            space::horizontal(),
            button(text("Erneut prüfen").size(10)).padding([2, 8]).style(thema::sekundaer_knopf_stil).on_press(Message::WerkzeugePruefen),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(8);
    if crate::einstellungen::im_snap() {
        werkzeug_spalte = werkzeug_spalte.push(
            text("Snap-Version: Der Snap läuft in einer abgeschirmten Umgebung und kann Programme deines Systems nicht starten. Pandoc, das Typst-Programm, Ghostscript, Rust, Dart und Flutter bleiben deshalb \"nicht gefunden\", auch wenn du sie installierst; der Rust- und Dart-Code-Prüfer ist im Snap nicht nutzbar. Bash- und sh-Beispiele prüft der Skript-Prüfer weiterhin. Der Buchsatz (Druck-PDF, Lese-PDF, EPUB, Cover) ist eingebaut und braucht sie nicht. Brauchst du den Code-Prüfer für Rust oder Dart, die PDF/X-3-Druckdatei (Ghostscript), die Pandoc-Vorschau oder eigene Bauskripte, nimm das DEB-, RPM- oder Flatpak-Paket von werkbank.jurzel-apps.de. Die Sprachprüfung läuft über einen eigenen Server, siehe Seite „Prüfung“.")
                .size(11)
                .color(thema::warnung_orange()),
        );
    }
    werkzeug_spalte = werkzeug_spalte.push(column![
        werkzeug_zeile(
            "Pandoc",
            &werkzeuge.pandoc,
            1,
            "Wandelt Markdown nach Typst. Die Satzvorschau geht damit denselben Weg wie die Bauskripte.",
            pandoc_alt.then_some("zu alt: ab Version 3 nötig"),
        ),
        match &werkzeuge.typst_eingebaut {
            Some(Ok(t)) => werkzeug_zeile(
                "Typst",
                &Some(format!("eingebaut, Test bestanden ({t}){}", werkzeuge.typst.as_ref().map(|v| format!(" · Programm {v}")).unwrap_or_default())),
                5,
                "Der Satz der Bücher (PDF) steckt im Programm; das Typst-Programm ist nur für die alten Bauskripte nötig.",
                None,
            ),
            Some(Err(e)) => werkzeug_zeile("Typst", &Some(format!("Selbsttest fehlgeschlagen: {e}")), 5, "Der eingebaute Satz arbeitet nicht; Bücher lassen sich so nicht setzen.", Some("Fehler")),
            None => werkzeug_zeile("Typst", &None, 5, "Der Satz der Bücher (PDF) steckt im Programm.", None),
        },
        werkzeug_zeile("Ghostscript", &werkzeuge.ghostscript, 2, "Erzeugt aus dem Druck-PDF die PDF/X-3-Datei (Graustufen) für die Portale.", None),
        beschriftet(
            "Ordner von Ghostscript (leer = automatisch suchen)",
            text_input(if cfg!(windows) { "z. B. C:\\Program Files\\gs" } else { "z. B. /usr/local/gs" }, &e.gs_ordner)
                .on_input(Message::GsOrdner)
                .size(12)
                .padding(6),
        ),
        abschnitt("Code-Prüfer"),
        werkzeug_zeile("Rust", &werkzeuge.rustc, 3, "rustc prüft die Rust-Beispiele des Buches auf Typfehler (Bereich Prüfen).", None),
        if crate::einstellungen::im_snap() {
            column![
                row![text("Java").size(12).color(thema::text_hell()).width(110), text("läuft außerhalb des Snaps").size(12).color(thema::text_gedimmt())].spacing(8),
                text("Java und LanguageTool installierst und startest du selbst im Terminal (siehe Seite „Prüfung“). Der Snap sieht Java nicht, deshalb gibt es hier keine Prüfung.").size(10).color(thema::text_gedimmt()),
            ]
            .spacing(2)
            .into()
        } else {
            werkzeug_zeile(
                "Java",
                &werkzeuge.java,
                7,
                "Java 17 oder neuer lässt den LanguageTool-Server laufen (Sprachprüfung). Ohne Java läuft nur die Rechtschreibung.",
                (werkzeuge.java.is_some() && !werkzeuge.java_brauchbar()).then_some("zu alt: ab Version 17 nötig"),
            )
        },
        werkzeug_zeile(
            "Skripte",
            &(!werkzeuge.skript.is_empty()).then(|| werkzeuge.skript.join(" · ")),
            6,
            "Prüft bash-, sh-, zsh-, fish- und powershell-Beispiele auf Syntaxfehler, ohne sie auszuführen. Jede installierte Shell zählt; ShellCheck ist optional.",
            None,
        ),
        werkzeug_zeile(
            "Dart / Flutter",
            &dart_flutter,
            4,
            "dart analyze prüft die Dart- und Flutter-Beispiele des Buches (Bereich Prüfen).",
            (werkzeuge.dart.is_some() && werkzeuge.flutter.is_none()).then_some("ohne Flutter: Flutter-Beispiele werden übersprungen"),
        ),
    ]
    .spacing(8));
    if fenster.werkzeug_info != 0 {
        werkzeug_spalte = werkzeug_spalte.push(werkzeug_anleitung(fenster.werkzeug_info, fenster.system));
    }

    let mut tasten = column![abschnitt("Tastenkürzel (Ändern, dann gewünschte Tasten drücken · Esc bricht ab)")].spacing(4);
    for (id, name, _) in crate::kuerzel::BEFEHLE {
        let aktuell = crate::kuerzel::kuerzel_von(e, id);
        let wartet = aufnahme == Some(*id);
        tasten = tasten.push(
            row![
                text(*name).size(11).width(Fill),
                text(if wartet { "Taste drücken …".to_string() } else { crate::kuerzel::anzeige(aktuell) })
                    .size(11)
                    .color(if wartet { thema::warnung_orange() } else { thema::text_gedimmt() }),
                button(text("Ändern").size(10))
                    .padding([2, 8])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::KuerzelAufnehmen((*id).to_string())),
                button(text("Standard").size(10))
                    .padding([2, 8])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::KuerzelZuruecksetzen((*id).to_string())),
                button(text("Leeren").size(10))
                    .padding([2, 8])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::KuerzelLeeren((*id).to_string())),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }

    let allgemein = column![
        toggler(e.sprachpruefung)
            .label("Sprachprüfung verwenden")
            .size(16)
            .text_size(12)
            .on_toggle(Message::LtAktiv),
        toggler(e.automatisch)
            .label("Automatisch nach kurzer Schreibpause prüfen")
            .size(16)
            .text_size(12)
            .on_toggle(Message::LtAutomatisch),
        beschriftet(
            "Sprache",
            liste_mit_pfeilen(
                pick_list(&SPRACHEN[..], Some(e.sprache.as_str()), |s: &str| Message::LtSprache(s.to_string()))
                    .text_size(12)
                    .padding([5, 8])
                    .style(thema::auswahl_stil)
                    .menu_style(thema::auswahl_menue_stil),
                SPRACHEN.to_vec(),
                Some(e.sprache.as_str()),
                |s: &str| Message::LtSprache(s.to_string()),
            ),
        ),
    ]
    .spacing(8);

    let verbindung = column![
        abschnitt("Server"),
        beschriftet(
            "Adresse  (Eingabetaste testet die Verbindung)",
            text_input("http://localhost:8081", &e.server_url)
                .on_input(Message::LtUrl)
                .on_submit(Message::LtVerbindungTesten)
                .size(12)
                .padding(6),
        ),
        row![
            text(status_text).size(11).color(status_farbe),
            space::horizontal(),
            button(text("Verbindung testen").size(11))
                .padding([3, 8])
                .style(thema::sekundaer_knopf_stil)
                .on_press(Message::LtVerbindungTesten),
        ]
        .align_y(Alignment::Center),
    ]
    .spacing(6);

    let lokal = column![
        abschnitt("Lokaler Server (von der Werkbank gestartet)"),
        beschriftet(
            "LanguageTool-Ordner (Eingabetaste startet den Server)",
            row![
                text_input("z. B. /home/du/LanguageTool-6.6", &e.lt_ordner)
                    .on_input(Message::LtOrdner)
                    .on_submit(Message::LtServerStarten)
                    .size(12)
                    .padding(6),
                button(text("Wählen …").size(11))
                    .padding([6, 10])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::LtOrdnerWaehlen),
            ]
            .spacing(6),
        ),
        beschriftet(
            "Java-Programm",
            text_input("java", &e.java)
                .on_input(Message::LtJava)
                .size(12)
                .padding(6),
        ),
        toggler(e.server_autostart)
            .label("Server beim Start der Werkbank mitstarten")
            .size(16)
            .text_size(12)
            .on_toggle(Message::LtAutostart),
        row![
            if server_laeuft {
                button(text("Server stoppen").size(12))
                    .padding([6, 12])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::LtServerStoppen)
            } else {
                button(text("Server starten").size(12))
                    .padding([6, 12])
                    .style(thema::primaer_knopf_stil)
                    .on_press_maybe((!e.lt_ordner.trim().is_empty()).then_some(Message::LtServerStarten))
            },
            text(if server_laeuft {
                "Läuft – wird beim Beenden der Werkbank mit beendet"
            } else {
                "Beim Beenden der Werkbank wird er automatisch gestoppt"
            })
            .size(11)
            .color(thema::text_gedimmt()),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    ]
    .spacing(6);

    // Snap: Java und LanguageTool liegen nicht im Paket und sind aus der Sandbox nicht startbar
    let lokal = if crate::einstellungen::im_snap() {
        column![
            abschnitt("LanguageTool im Snap"),
            text("Der Snap enthält LanguageTool und Java nicht. Du installierst sie selbst und startest den Server in einem Terminal; die Werkbank verbindet sich dann über die Adresse oben (http://localhost:8081). Die Schritte stehen unter dem Info-Symbol ⓘ neben „Sprachprüfung (LanguageTool)“.")
                .size(11)
                .color(thema::text_gedimmt()),
            text("Kurz: 1. Java 17 oder neuer installieren (sudo apt install default-jre). 2. LanguageTool laden und entpacken (https://languagetool.org/download/LanguageTool-stable.zip). 3. Im entpackten Ordner: java -cp languagetool-server.jar org.languagetool.server.HTTPServer --port 8081. 4. Hier „Verbindung testen“.")
                .size(11)
                .color(thema::text_gedimmt()),
        ]
        .spacing(6)
    } else {
        lokal
    };

    let mut ausnahmen_spalte = column![abschnitt("Ausnahmen für dieses Buch")].spacing(6);
    if ausnahmen.regeln.is_empty() && ausnahmen.woerter.is_empty() {
        ausnahmen_spalte = ausnahmen_spalte.push(
            text("Noch keine. Im Inspektor lassen sich Wörter merken und Regeln ausschalten.")
                .size(11)
                .color(thema::text_gedimmt()),
        );
    }
    if !ausnahmen.regeln.is_empty() {
        let mut chips = row![].spacing(4);
        for r in &ausnahmen.regeln {
            chips = chips.push(
                button(text(format!("{r}  ×")).size(10))
                    .padding([2, 6])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::AusnahmeRegelEntfernen(r.clone())),
            );
        }
        ausnahmen_spalte = ausnahmen_spalte.push(beschriftet("Ausgeschaltete Regeln", chips.wrap()));
    }
    if !ausnahmen.woerter.is_empty() {
        let mut chips = row![].spacing(4);
        for w in &ausnahmen.woerter {
            chips = chips.push(
                button(text(format!("{w}  ×")).size(10))
                    .padding([2, 6])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::AusnahmeWortEntfernen(w.clone())),
            );
        }
        ausnahmen_spalte = ausnahmen_spalte.push(beschriftet("Eigene Wörter", chips.wrap()));
    }

    // Seiteninhalt: jede Seite zeigt nur ihre Tafeln
    let mut seite = column![].spacing(14).width(Fill);
    match fenster.seite {
        1 => seite = seite.push(tafel(darstellung)),
        2 => {
            seite = seite
                .push(tafel(rechtschreibung))
                .push(tafel(column![ueberschrift, allgemein].spacing(8)))
                .push(tafel(verbindung))
                .push(tafel(lokal));
            if fenster.info_offen {
                seite = seite.push(info_panel(fenster.system));
            }
            seite = seite.push(tafel(ausnahmen_spalte));
        }
        3 => seite = seite.push(tafel(werkzeug_spalte)),
        4 => seite = seite.push(tafel(tasten)),
        _ => seite = seite.push(tafel(lizenz_abschnitt(lizenz))).push(tafel(editor)),
    }

    let fuss = row![
        text("Änderungen gelten sofort · Esc schließt").size(11).color(thema::text_gedimmt()),
        space::horizontal(),
        button(text("Fertig").size(13)).padding([6, 18]).style(thema::primaer_knopf_stil).on_press(Message::EinstellungenSchliessen),
    ]
    .align_y(Alignment::Center);

    let box_ = container(
        column![
            container(column![titelzeile, seiten].spacing(14)).padding(Padding::from([18, 22]).bottom(10)),
            container(scrollable(container(seite).padding(Padding::from([4, 22]).right(26))).height(Fill)).height(Fill),
            container(fuss).padding([12, 16]).width(Fill).style(thema::dialog_fuss_stil),
        ]
        .height(Fill),
    )
    .max_width(720)
    .max_height(700)
    .height(Fill)
    .style(thema::dialog_stil);

    mouse_area(
        container(opaque(box_))
            .width(Fill)
            .height(Fill)
            .padding(Padding::from([40, 20]))
            .center_x(Fill)
            .align_y(Alignment::Start)
            .style(thema::overlay_dimm_stil),
    )
    .on_press(Message::EinstellungenSchliessen)
    .into()
}

/// Installationshinweise für Pandoc (1), Ghostscript (2), Rust (3), Dart/Flutter (4) oder Typst (5).
fn werkzeug_anleitung<'a>(welches: u8, system: Betriebssystem) -> Element<'a, Message> {
    let (name, url, zeilen): (&str, &str, Vec<(&str, &str)>) = match (welches, system) {
        (1, Betriebssystem::Windows) => ("Pandoc", "https://pandoc.org/installing.html", vec![
            ("Mit winget (PowerShell):", "winget install --id JohnMacFarlane.Pandoc"),
            ("Oder mit Chocolatey:", "choco install pandoc"),
            ("Oder den MSI-Installer von der Webseite laden und ausführen.", ""),
        ]),
        (1, Betriebssystem::MacOs) => ("Pandoc", "https://pandoc.org/installing.html", vec![
            ("Mit Homebrew:", "brew install pandoc"),
            ("Oder das pkg-Paket von der Webseite laden und öffnen.", ""),
        ]),
        (1, Betriebssystem::Linux) => ("Pandoc", "https://pandoc.org/installing.html", vec![
            ("Debian, Ubuntu, Mint (apt):", "sudo apt install pandoc"),
            ("Fedora, RHEL (dnf, rpm-Pakete):", "sudo dnf install pandoc"),
            ("Arch, Manjaro (pacman):", "sudo pacman -S pandoc-cli"),
            ("Ist die Version unter 3: die aktuelle .deb-Datei von den GitHub-Releases laden:", "sudo apt install ./pandoc-*-amd64.deb"),
        ]),
        (3, Betriebssystem::Windows) => ("Rust", "https://rustup.rs", vec![
            ("Mit winget (PowerShell), danach ein neues Terminal öffnen:", "winget install --id Rustlang.Rustup"),
            ("Oder rustup-init.exe von der Webseite laden und ausführen.", ""),
        ]),
        (3, Betriebssystem::MacOs) => ("Rust", "https://rustup.rs", vec![
            ("Mit rustup (Terminal):", "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"),
        ]),
        (3, Betriebssystem::Linux) => ("Rust", "https://rustup.rs", vec![
            ("Mit rustup (empfohlen, aktuelle Version für Edition 2024):", "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"),
            ("Oder aus den Paketquellen (oft ältere Version), Debian/Ubuntu:", "sudo apt install rustc"),
            ("Fedora (dnf):", "sudo dnf install rust"),
            ("Arch (pacman):", "sudo pacman -S rustup"),
        ]),
        (4, Betriebssystem::Windows) => ("Flutter", "https://docs.flutter.dev/get-started/install", vec![
            ("Das Flutter-SDK als ZIP von der Webseite laden, nach C:\\src\\flutter entpacken und C:\\src\\flutter\\bin zum PATH hinzufügen.", ""),
            ("Oder mit Chocolatey:", "choco install flutter"),
            ("Flutter bringt Dart mit. Nur Dart (ohne Flutter-Beispiele): https://dart.dev/get-dart", ""),
        ]),
        (4, Betriebssystem::MacOs) => ("Flutter", "https://docs.flutter.dev/get-started/install", vec![
            ("Mit Homebrew:", "brew install --cask flutter"),
            ("Flutter bringt Dart mit. Nur Dart (ohne Flutter-Beispiele):", "brew tap dart-lang/dart && brew install dart"),
        ]),
        (4, Betriebssystem::Linux) => ("Flutter", "https://docs.flutter.dev/get-started/install", vec![
            ("Mit Snap:", "sudo snap install flutter --classic"),
            ("Oder per Git in den Home-Ordner:", "git clone https://github.com/flutter/flutter.git -b stable ~/flutter"),
            ("Flutter bringt Dart mit. Die Werkbank findet es auch unter ~/flutter oder ~/Development/flutter und über FLUTTER_ROOT.", ""),
        ]),
        (7, Betriebssystem::Windows) => ("Java", "https://adoptium.net/", vec![
            ("Java 17 oder neuer, mit winget (PowerShell), danach ein neues Fenster öffnen:", "winget install --id EclipseAdoptium.Temurin.21.JRE"),
            ("Oder den Installer von adoptium.net laden und ausführen.", ""),
        ]),
        (7, Betriebssystem::MacOs) => ("Java", "https://adoptium.net/", vec![
            ("Java 17 oder neuer mit Homebrew:", "brew install --cask temurin@21"),
        ]),
        (7, Betriebssystem::Linux) => ("Java", "https://adoptium.net/", vec![
            ("Debian, Ubuntu, Mint (apt):", "sudo apt install default-jre"),
            ("Fedora, RHEL (dnf):", "sudo dnf install java-latest-openjdk"),
            ("Arch, Manjaro (pacman):", "sudo pacman -S jre-openjdk"),
            ("Prüfen, ob die Version reicht (17 oder höher):", "java -version"),
        ]),
        (6, Betriebssystem::Windows) => ("Shells und ShellCheck", "https://learn.microsoft.com/powershell/", vec![
            ("Die Skript-Prüfung ist optional. Sie nutzt, was installiert ist. Für Bash-Beispiele Git für Windows (bringt bash mit):", "winget install --id Git.Git"),
            ("PowerShell 7 (pwsh):", "winget install --id Microsoft.PowerShell"),
            ("ShellCheck (für zusätzliche Hinweise bei Bash/sh):", "winget install --id koalaman.shellcheck"),
        ]),
        (6, Betriebssystem::MacOs) => ("Shells und ShellCheck", "https://www.shellcheck.net/", vec![
            ("Bash und Zsh sind vorhanden. ShellCheck, Fish und PowerShell mit Homebrew:", "brew install shellcheck fish"),
            ("PowerShell:", "brew install --cask powershell"),
        ]),
        (6, Betriebssystem::Linux) => ("Shells und ShellCheck", "https://www.shellcheck.net/", vec![
            ("Bash und sh sind fast überall vorhanden. Weitere Shells und ShellCheck, Debian, Ubuntu, Mint (apt):", "sudo apt install zsh fish shellcheck"),
            ("Fedora, RHEL (dnf):", "sudo dnf install zsh fish ShellCheck"),
            ("Arch, Manjaro (pacman):", "sudo pacman -S zsh fish shellcheck"),
            ("PowerShell (pwsh), zum Beispiel mit Snap:", "sudo snap install powershell --classic"),
        ]),
        (5, Betriebssystem::Windows) => ("Typst", "https://typst.app/open-source/", vec![
            ("Der Satz ist eingebaut, ein Typst-Programm brauchst du nur für die alten Bauskripte. Mit winget (PowerShell):", "winget install --id Typst.Typst"),
            ("Oder mit Scoop:", "scoop install typst"),
        ]),
        (5, Betriebssystem::MacOs) => ("Typst", "https://typst.app/open-source/", vec![
            ("Der Satz ist eingebaut, ein Typst-Programm brauchst du nur für die alten Bauskripte. Mit Homebrew:", "brew install typst"),
        ]),
        (5, Betriebssystem::Linux) => ("Typst", "https://typst.app/open-source/", vec![
            ("Der Satz ist eingebaut, ein Typst-Programm brauchst du nur für die alten Bauskripte. Mit Cargo:", "cargo install --locked typst-cli"),
            ("Oder mit Snap:", "sudo snap install typst"),
            ("Arch (pacman):", "sudo pacman -S typst"),
        ]),
        (_, Betriebssystem::Windows) => ("Ghostscript", "https://ghostscript.com/releases/gsdnld.html", vec![
            ("Den Installer (Windows 64 bit) von der Webseite laden und ausführen. Die Werkbank findet ihn in C:\\Program Files\\gs. Das Programm heißt unter Windows gswin64c. (Ghostscript gibt es nicht in winget.)", ""),
            ("Oder mit Chocolatey (Administrator-PowerShell):", "choco install ghostscript"),
            ("Oder mit Scoop:", "scoop install ghostscript"),
        ]),
        (_, Betriebssystem::MacOs) => ("Ghostscript", "https://ghostscript.com/releases/gsdnld.html", vec![("Mit Homebrew:", "brew install ghostscript")]),
        (_, Betriebssystem::Linux) => ("Ghostscript", "https://ghostscript.com/releases/gsdnld.html", vec![
            ("Debian, Ubuntu, Mint (apt):", "sudo apt install ghostscript"),
            ("Fedora, RHEL (dnf, rpm-Pakete):", "sudo dnf install ghostscript"),
            ("Arch, Manjaro (pacman):", "sudo pacman -S ghostscript"),
        ]),
    };
    let mut tabs = row![].spacing(4);
    for s in Betriebssystem::ALLE {
        tabs = tabs.push(
            button(text(s.to_string()).size(12))
                .padding([4, 12])
                .style(thema::tab_knopf_stil(s == system))
                .on_press(Message::EinstellungenSystem(s)),
        );
    }
    let mut inhalt = column![
        text(format!("{name} installieren")).size(14).color(thema::text_hell()),
        tabs,
    ]
    .spacing(8);
    for (beschreibung, befehl) in zeilen {
        inhalt = inhalt.push(text(beschreibung).size(12).color(thema::text_hell()));
        if !befehl.is_empty() {
            inhalt = inhalt.push(
                container(
                    row![
                        text(befehl).size(11).font(Font::MONOSPACE).width(Fill),
                        button(text("Kopieren").size(10))
                            .padding([1, 6])
                            .style(thema::link_knopf_stil)
                            .on_press(Message::Kopieren(befehl.to_string())),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .padding([6, 8])
                .style(thema::codeblock_container_stil),
            );
        }
    }
    if system == Betriebssystem::Windows {
        let id = match welches {
            1 => Some("JohnMacFarlane.Pandoc"),
            3 => Some("Rustlang.Rustup"),
            5 => Some("Typst.Typst"),
            7 => Some("EclipseAdoptium.Temurin.21.JRE"),
            4 | 6 => None,
            _ => None, // Ghostscript ist nicht in winget
        };
        if let Some(id) = id {
            inhalt = inhalt.push(
                button(text(format!("{name} jetzt mit winget installieren")).size(12))
                    .padding([6, 14])
                    .style(thema::primaer_knopf_stil)
                    .on_press(Message::WerkzeugInstallieren(id)),
            );
        }
    }
    inhalt = inhalt
        .push(
            row![
                text("Webseite:").size(11).color(thema::text_gedimmt()),
                button(text(url).size(11))
                    .padding([0, 4])
                    .style(thema::link_knopf_stil)
                    .on_press(Message::LinkOeffnen(url.to_string())),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .push(text(match welches {
            3 => "Danach in den Einstellungen „Erneut prüfen“ wählen. Die Werkbank findet rustc im Suchpfad (PATH) und unter ~/.cargo/bin.",
            _ => "Danach in den Einstellungen „Erneut prüfen“ wählen. Die Werkbank findet das Programm im Suchpfad (PATH).",
        }).size(11).color(thema::text_gedimmt()));
    container(inhalt).padding(14).width(Fill).style(thema::infobox_stil).into()
}

fn info_panel<'a>(system: Betriebssystem) -> Element<'a, Message> {
    let mut tabs = row![].spacing(4);
    for s in Betriebssystem::ALLE {
        tabs = tabs.push(
            button(text(s.to_string()).size(12))
                .padding([4, 12])
                .style(thema::tab_knopf_stil(s == system))
                .on_press(Message::EinstellungenSystem(s)),
        );
    }

    let mut schritte = column![].spacing(10);
    for (i, schritt) in anleitung(system).into_iter().enumerate() {
        let mut block = column![
            text(format!("{}. {}", i + 1, schritt.titel))
                .size(12)
                .color(thema::text_hell()),
        ]
        .spacing(4);
        for befehl in schritt.befehle {
            block = block.push(
                container(
                    row![
                        text(*befehl).size(11).font(Font::MONOSPACE).width(Fill),
                        button(text("Kopieren").size(10))
                            .padding([1, 6])
                            .style(thema::link_knopf_stil)
                            .on_press(Message::Kopieren((*befehl).to_string())),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .padding([6, 8])
                .style(thema::codeblock_container_stil),
            );
        }
        schritte = schritte.push(block);
    }

    let hinweise = column![
        text("Hinweise").size(12).color(thema::text_gedimmt()),
        text("• LanguageTool läuft ausschließlich auf deinem Rechner; dein Manuskript wird nicht ins Internet gesendet.")
            .size(11)
            .color(thema::text_gedimmt()),
        text("• Die Download-Adresse lautet " .to_string() + ZIP_URL + ". Die Datei ist etwa 250 MB groß.")
            .size(11)
            .color(thema::text_gedimmt()),
        text("• Für bessere Treffer bei Grammatik und Stil gibt es optional N-Gramm-Daten (mehrere GB), siehe dev.languagetool.org.")
            .size(11)
            .color(thema::text_gedimmt()),
        text("• Codeblöcke, Inline-Code, Link-Adressen und @kap:-Verweise werden nicht geprüft.")
            .size(11)
            .color(thema::text_gedimmt()),
    ]
    .spacing(3);

    container(
        column![
            text(format!("Installationsanleitung für {system}")).size(14).color(thema::text_hell()),
            tabs,
            schritte,
            hinweise,
        ]
        .spacing(10),
    )
    .padding(14)
    .width(Fill)
    .style(thema::infobox_stil)
    .into()
}
