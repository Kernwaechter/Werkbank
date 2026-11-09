//! # Werkbank Desktop Anwendung
//!
//! Die Hauptanwendung der Werkbank in reinem Rust mit Iced.
//! Bietet einen dreispaltigen Editor im Ulysses-Stil, eine Befehlspalette
//! (`Strg+K`) mit Tastaturkürzeln für alle Funktionen, Projektverwaltung und
//! die Dashboards für Satz, Prüfung und Schreibstatistik.

#![cfg_attr(windows, windows_subsystem = "windows")]

mod bau;
mod cover;
mod dateien;
mod editor;
mod hintergrund;
mod einstellungen;
mod fokus_hilfe;
mod kuerzel;
mod languagetool;
mod markierung;
mod rechtschreibung;
mod thema;
mod werkzeuge;
mod ui;

use chrono::Local;
use iced::keyboard::{self, Key, key::Named};
use crate::editor as text_editor;
use iced::widget::stack;
use iced::{Element, Fill, Size, Subscription, Task, Theme};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use einstellungen::Einstellungen;
use languagetool::{PruefStatus, ServerProzess, ServerStatus};
use ui::dialog::{Dialog, Eingabe, EingabeZweck};
use ui::suche::Suche;
use werkbank_kern::codepruefung::{self, Ergebnis};
use werkbank_kern::portale::Portalregeln;
use ui::zeitleiste::{self, Zeitleiste};
use werkbank_kern::sprache::{Ausnahmen, Treffer};
use werkbank_kern::statistik::Statistik;
use ui::navigation::Arbeitsbereich;
use werkbank_kern::{KapitelStatus, Projekt};

/// Ein Band (Buchprojekt-Ordner), zwischen denen die Kopfleiste umschalten kann.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Band {
    /// Anzeigename, z. B. „Rust-Werkstatt · Band 1“.
    pub name: String,
    /// Ordner des Bandes auf der Festplatte (`None` beim Demo-Buch).
    pub ordner: Option<PathBuf>,
}

impl fmt::Display for Band {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl Band {
    fn demo() -> Self {
        Self {
            name: "Demo-Buch".to_string(),
            ordner: None,
        }
    }

    fn laden(&self) -> Result<Projekt, String> {
        match &self.ordner {
            None => Ok(Projekt::beispiel_projekt()),
            Some(ordner) => Projekt::aus_verzeichnis(ordner).map_err(|e| e.to_string()),
        }
    }
}

/// Enthält der Ordner Markdown-Dateien oder eine `reihenfolge.txt`?
fn ist_buchordner(ordner: &Path) -> bool {
    ordner.join("reihenfolge.txt").is_file()
        || std::fs::read_dir(ordner).is_ok_and(|eintraege| {
            eintraege.flatten().any(|e| {
                let p = e.path();
                p.is_file() && p.extension().is_some_and(|x| x == "md")
            })
        })
}

fn ordnername(ordner: &Path) -> String {
    ordner
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| ordner.display().to_string())
}

/// Sortierschlüssel, damit „Band 2“ vor „Band 10“ steht.
fn band_sortierung(name: &str) -> (String, u32) {
    let ziffern: String = name.chars().filter(|c| c.is_ascii_digit()).collect();
    let text: String = name.chars().filter(|c| !c.is_ascii_digit()).collect();
    (text.to_lowercase(), ziffern.parse().unwrap_or(0))
}

/// Findet die Bandordner (`Band 1`, `Band_2`, …) direkt unter einem Reihenordner.
fn baende_unter(reihe: &Path) -> Vec<Band> {
    let reihenname = ordnername(reihe);
    let mut ordner: Vec<PathBuf> = std::fs::read_dir(reihe)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && ordnername(p).to_lowercase().starts_with("band")
                && ist_buchordner(p)
        })
        .collect();
    ordner.sort_by_key(|p| band_sortierung(&ordnername(p)));
    ordner
        .into_iter()
        .map(|p| Band {
            name: format!("{} · {}", reihenname, ordnername(&p).replace('_', " ")),
            ordner: Some(p),
        })
        .collect()
}

/// Liest einen gewählten Ordner ein: Reihenordner mit mehreren Bänden,
/// einzelner Bandordner (dann werden die Geschwister-Bände mitgeladen) oder ein Buch ohne Bandstruktur.
/// Liefert alle Bände und den zu öffnenden Band.
fn ordner_einlesen(ordner: &Path) -> Result<(Vec<Band>, Band), String> {
    let unter = baende_unter(ordner);
    if let Some(erster) = unter.first().cloned() {
        return Ok((unter, erster));
    }

    if ist_buchordner(ordner) {
        if let Some(eltern) = ordner.parent() {
            let geschwister = baende_unter(eltern);
            if let Some(gewaehlt) = geschwister
                .iter()
                .find(|b| b.ordner.as_deref() == Some(ordner))
                .cloned()
            {
                return Ok((geschwister, gewaehlt));
            }
        }
        let band = Band {
            name: ordnername(ordner),
            ordner: Some(ordner.to_path_buf()),
        };
        return Ok((vec![band.clone()], band));
    }

    Err(format!(
        "In „{}“ wurde kein Buchprojekt gefunden (weder Markdown-Dateien noch Band-Ordner).",
        ordner.display()
    ))
}

/// Datei, in der der zuletzt geöffnete Ordner für den nächsten Start gemerkt wird.
fn merkdatei() -> Option<PathBuf> {
    let basis = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(basis.join("werkbank").join("letzter-ordner"))
}

fn letzten_ordner_merken(ordner: &Path) {
    if let Some(datei) = merkdatei() {
        if let Some(eltern) = datei.parent() {
            let _ = std::fs::create_dir_all(eltern);
        }
        let _ = std::fs::write(datei, ordner.to_string_lossy().as_bytes());
    }
}

fn letzten_ordner_lesen() -> Option<PathBuf> {
    let text = std::fs::read_to_string(merkdatei()?).ok()?;
    let pfad = PathBuf::from(text.trim());
    pfad.is_dir().then_some(pfad)
}

/// Zustand der geöffneten Befehlspalette.
#[derive(Debug, Default)]
pub struct Palette {
    /// Aktueller Suchtext.
    pub suche: String,
    /// Index des per Pfeiltasten gewählten Treffers.
    pub auswahl: usize,
}

/// Ein Eintrag der Befehlspalette.
struct Befehl {
    titel: String,
    hinweis: String,
    aktion: Message,
}

pub fn main() -> iced::Result {
    let mut anwendung = iced::application(new, update, view)
        .title(fenstertitel)
        .window(iced::window::Settings {
            size: Size::new(1380.0, 880.0),
            position: iced::window::Position::Centered,
            icon: iced::window::icon::from_file_data(include_bytes!("../../../assets/icons/256.png"), None).ok(),
            #[cfg(target_os = "linux")]
            platform_specific: iced::window::settings::PlatformSpecific {
                application_id: "de.daehn.Werkbank".to_string(),
                ..Default::default()
            },
            exit_on_close_request: false,
            ..Default::default()
        })
        .subscription(subscription)
        .theme(theme);
    // Dieselbe Schrift wie im Buchsatz (Libertinus Serif) für den Editor
    for daten in werkbank_satz::eingebettete_schriften("Libertinus Serif") {
        anwendung = anwendung.font(daten);
    }
    anwendung.run()
}

/// Fenstertitel: Band und Speicherzustand.
fn fenstertitel(state: &AppState) -> String {
    format!(
        "{}Werkbank – {}",
        if dateien::hat_ungespeichertes(state) { "● " } else { "" },
        state.band
    )
}

/// Der Gesamtzustand der Werkbank-Anwendung.
pub struct AppState {
    /// Das aktuell geladene Buchprojekt mit allen Teilen und Kapiteln.
    pub projekt: Projekt,
    /// Welcher Band gerade geöffnet ist.
    pub band: Band,
    /// Alle Bände des geöffneten Ordners.
    pub baende: Vec<Band>,
    /// Zwischengespeicherte, zuletzt verlassene Bände (samt ungesicherten Änderungen).
    pub projekt_cache: HashMap<Band, Projekt>,
    /// Der aktive Arbeitsbereich (Schreiben, Bauen, Prüfen, Zahlen).
    pub aktiver_bereich: Arbeitsbereich,
    /// Eindeutige Kennung des aktuell in der Bibliothek gewählten Teils.
    pub aktiver_teil_id: String,
    /// Eindeutige Kennung des aktuell im Editor bearbeiteten Kapitels.
    pub aktives_kapitel_id: String,
    /// Der Textpuffer von Iceds nativem Editor-Widget.
    pub editor_content: text_editor::Content,
    /// Zeigt an, ob im Editor ungespeicherte Eingaben vorliegen.
    pub ist_geaendert: bool,
    /// Letzte Statusmeldung (Speichern, Laden, Läufe).
    pub speicher_zeitstempel: String,
    /// Fokus-Modus: blendet Seitenleisten für ablenkungsfreies Schreiben aus.
    pub fokus_modus: bool,
    /// Sichtbarkeit von Bibliothek und Blattliste.
    pub seitenleiste_sichtbar: bool,
    /// Sichtbarkeit des rechten Inspektors mit Kapitel-Metadaten.
    pub inspektor_sichtbar: bool,
    /// Sichtbarkeit der geteilten Live-Vorschau.
    pub vorschau_sichtbar: bool,
    /// Geöffnete Befehlspalette (`None` = geschlossen).
    pub palette: Option<Palette>,
    /// Offenes Kontextmenü (Rechtsklick auf Kapitel oder Teil).
    pub kontext: Option<ui::kontext::Kontext>,
    /// Geöffnetes Menü der Menüleiste.
    pub menue: Option<ui::menue::Menue>,
    /// Wörter, deren Markierung bis zum Neustart ausgeblendet ist („Ignorieren“).
    pub ignoriert: HashSet<String>,
    /// Aktuelle Fenstergröße (damit schwebende Menüs im Fenster bleiben).
    pub fenster_groesse: Size,
    /// Teile mit Änderungen, die noch nicht auf der Platte stehen.
    pub dirty: HashSet<String>,
    /// Kapitel- oder Teilstruktur wurde geändert (`reihenfolge.txt` nachführen).
    pub struktur_dirty: bool,
    /// Status, Wortziel oder Notizen wurden geändert.
    pub meta_dirty: bool,
    /// Beim nächsten Speichern immer einen Snapshot anlegen (nach Löschen, Ersetzen, Wiederherstellen).
    pub snapshot_zwang: bool,
    /// Zeitpunkt der letzten Änderung (für Autosave).
    pub letzte_aenderung: Option<Instant>,
    /// Geöffneter Eingabedialog.
    pub eingabe: Option<Eingabe>,
    /// Geöffneter Bestätigungsdialog.
    pub dialog: Option<Dialog>,
    /// Geöffnete Zeitleiste mit früheren Ständen und Papierkorb.
    pub verlauf: Option<Box<Zeitleiste>>,
    /// Schreibstatistik des aktuellen Bandes.
    pub statistik: Statistik,
    /// Statistik hat ungespeicherte Werte.
    pub statistik_dirty: bool,
    /// Zeitpunkt der letzten Eingabe (Sitzungserkennung).
    pub letzte_eingabe: Option<Instant>,
    /// Wortzahl des offenen Kapitels beim letzten Zählen (für Netto-Wörter).
    pub wortzahl_stand: usize,
    /// Portalregeln (Maße, Grenzen) des Bandes.
    pub portale: Portalregeln,
    /// Zustand des Code-Prüfers.
    pub code: ui::pruefen::CodeStatus,
    /// Ergebnis der Sprachprüfung über das ganze Buch.
    pub buch_sprache: ui::pruefen::BuchSprache,
    /// Zustand der Bauansicht.
    pub bau: ui::bauen::BauZustand,
    /// Aktiver Reiter des Inspektors (0 Details, 1 Tempo, 2 Stil, 3 Text).
    pub inspektor_tab: u8,
    /// Die Detailspalte wird gerade mit der Maus verbreitert oder verschmälert.
    pub inspektor_zieht: bool,
    pub inspektor_maus_x: Option<f32>,
    /// Zwischengespeichertes Stilprofil des offenen Kapitels.
    pub stil_cache: std::cell::RefCell<Option<(u64, Arc<werkbank_kern::stil::Stilprofil>)>>,
    /// Gefundene Werkzeuge (Pandoc, Ghostscript).
    pub werkzeuge: werkzeuge::WerkzeugStatus,
    /// Satzvorschau des offenen Kapitels.
    pub satz_vorschau: ui::satzvorschau::SatzVorschau,
    /// Zeitpunkt der letzten Änderung, solange die Satzvorschau erneuert werden muss.
    pub vorschau_faellig: Option<Instant>,
    /// Zustand des Cover-Designers.
    pub cover: ui::cover::CoverZustand,
    /// Entwurf der Buchdaten im Dialog.
    pub meta_entwurf: Option<werkbank_kern::BandMetadaten>,
    /// Suchen und Ersetzen (bleibt für `F3` erhalten, auch wenn das Fenster zu ist).
    pub suche: Option<Suche>,
    /// Ist das Suchfenster sichtbar?
    pub suche_offen: bool,
    /// Notizen des offenen Kapitels.
    pub notizen_content: text_editor::Content,
    /// Nummer der Sicherung, die das letzte Speichern angelegt hat.
    pub letzte_sicherung: Option<u32>,
    /// Seitenzahl des letzten nativen Druck-PDFs (für den Buchrücken).
    pub satz_seiten: Option<u32>,
    /// Ist „Über die Werkbank“ geöffnet?
    pub ueber_offen: bool,
    /// Ist das Handbuch geöffnet?
    pub handbuch_offen: bool,
    /// Befehl, für den gerade ein neues Kürzel aufgenommen wird.
    pub kuerzel_aufnahme: Option<String>,
    /// Programmeinstellungen (LanguageTool u. a.).
    pub einstellungen: Einstellungen,
    /// Geöffnetes Einstellungsfenster (`None` = geschlossen).
    pub einstellungen_fenster: Option<ui::einstellungen::Fenster>,
    /// Geladenes Hunspell-Wörterbuch.
    pub woerterbuch: Option<Arc<rechtschreibung::Woerterbuch>>,
    /// Statusmeldung zum Wörterbuch.
    pub woerterbuch_status: String,
    /// Zwischenspeicher der Markierungs-Einstellung (wird in `view` bei Bedarf erneuert).
    pub hl_cache: std::cell::RefCell<Option<markierung::Einstellung>>,
    /// Zustand der Verbindung zum LanguageTool-Server.
    pub server_status: ServerStatus,
    /// Von der Werkbank gestarteter LanguageTool-Server.
    pub server: Option<ServerProzess>,
    /// Hinweise der Sprachprüfung zum offenen Kapitel.
    pub treffer: Vec<Treffer>,
    /// Ergebnis des letzten Prüflaufs.
    pub pruef_status: PruefStatus,
    /// Ausnahmen (Regeln, Wörter) des aktuellen Bandes.
    pub ausnahmen: Ausnahmen,
    /// Zeitpunkt der letzten Änderung, solange eine automatische Prüfung aussteht.
    pub pruef_faellig: Option<Instant>,
    /// Zähler für Textänderungen, um sofortige Syntax-Neuformatierung anzustoßen.
    pub edit_tick: usize,
    /// Laufende Zählung für neu angelegte Kapitel.
    pub kapitel_fortlaufend: usize,
}

/// Sämtliche Ereignisse und Benutzeraktionen in der Anwendung.
#[derive(Debug, Clone)]
pub enum Message {
    /// Wechselt zwischen Schreiben, Bauen, Prüfen und Zahlen.
    BereichWechseln(Arbeitsbereich),
    /// Wechselt zu einem anderen Band des geöffneten Ordners.
    ProjektWechseln(Band),
    /// Öffnet den Ordnerdialog, um ein Buchprojekt zu wählen (`Strg+O`).
    OrdnerDialog,
    /// Ergebnis des Ordnerdialogs (`None` = abgebrochen).
    OrdnerGewaehlt(Option<PathBuf>),
    /// Wählt einen anderen Buchteil in der Bibliotheksspalte.
    TeilAusgewaehlt(String),
    /// Wählt ein Kapitel in der Blattliste und lädt seinen Text in den Editor.
    KapitelAusgewaehlt(String),
    /// Springt zum nächsten (`1`) oder vorherigen (`-1`) Kapitel des Buches.
    KapitelNachbar(i32),
    /// Springt zu dem Kapitel, auf das ein `@kap:`-Verweis zeigt.
    QuerverweisOeffnen(String),
    /// Wird bei jedem Tastenanschlag oder Cursor-Bewegen im Editor aufgerufen.
    EditorAktion(text_editor::Action),
    /// Speichert das aktuelle Kapitel (`Strg+S` oder Klick auf den Status).
    Speichern,
    /// Schaltet den Fokus-Modus (Ablenkungsfreies Schreiben) ein oder aus (`F11`).
    FokusToggle,
    /// Blendet Bibliothek und Blattliste ein oder aus (`Strg+B`).
    SeitenleisteToggle,
    /// Schaltet die rechte Kapitel-Detailspalte ein oder aus (`Strg+I`).
    InspektorToggle,
    /// Schaltet die geteilte Live-Vorschau ein oder aus (`Strg+P`).
    VorschauToggle,
    /// Springt im Markdown-Editor an eine bestimmte Quelltextzeile (aus der Vorschau geklickt).
    SpringeZuZeile { zeile: usize, spalte: usize },
    /// Erstellt ein neues leeres Kapitel im aktiven Teil.
    NeuesKapitel,
    /// Erstellt einen neuen Buchteil im Projekt.
    NeuerTeil,
    /// Setzt das Wortziel des aktuellen Kapitels.
    ZielSetzen(usize),
    /// Ändert den redaktionellen Status (Entwurf, In Arbeit, Überarbeitet, Fertig).
    StatusAendern(KapitelStatus),
    /// Startet die Generierung der Verlagsausgaben.
    BauenStarten,
    /// Startet die Qualitätsprüfung für Sprache und Code.
    PrueflaufStarten,
    /// Schließt Dialoge, Verlauf und Eingaben (`Esc`).
    DialogSchliessen,
    /// Führt den Knopf eines Bestätigungsdialogs aus.
    DialogAktion(usize),
    /// Texteingabe im Eingabedialog.
    EingabeText(String),
    /// Bestätigt den Eingabedialog.
    EingabeOk,
    /// Fragt nach einem neuen Titel für das offene Kapitel (`F2`).
    KapitelUmbenennenDialog,
    /// Fragt nach einem neuen Titel für den aktiven Teil.
    TeilUmbenennenDialog,
    /// Fragt vor dem Löschen des offenen Kapitels.
    KapitelLoeschenFrage,
    /// Löscht das offene Kapitel.
    KapitelLoeschen,
    /// Verschiebt das offene Kapitel (`Alt+Umschalt+↑/↓`).
    KapitelVerschieben(i32),
    /// Verschiebt den aktiven Teil.
    TeilVerschieben(i32),
    /// Fragt vor dem Entfernen des aktiven Teils.
    TeilEntfernenFrage,
    /// Entfernt den aktiven Teil (Datei kommt in den Papierkorb).
    TeilEntfernen,
    /// Öffnet den Verlauf des offenen Kapitels (`Strg+H`).
    VerlaufOeffnen,
    /// Öffnet die Zeitleiste direkt im Papierkorb.
    PapierkorbOeffnen,
    /// Wählt einen Stand der Zeitleiste.
    ZeitAuswahl(usize),
    /// Klappt eine Schreibsitzung auf oder zu.
    ZeitSitzung(usize),
    ZeitAnsicht(zeitleiste::Ansicht),
    ZeitReiter(zeitleiste::Reiter),
    /// Namensfeld der Zeitleiste.
    ZeitName(String),
    ZeitBenennen,
    /// Holt das ganze Kapitel aus dem gewählten Stand zurück.
    ZeitWiederherstellen,
    /// Holt nur die n-te Änderung aus dem gewählten Stand zurück.
    ZeitPassage(usize),
    /// Legt den gewählten Stand als neues Kapitel daneben.
    ZeitKopie,
    ZeitKopieren,
    ZeitExport,
    ZeitExportZiel(Option<PathBuf>),
    ZeitAufraeumenPlan,
    ZeitAufraeumenAbbruch,
    ZeitAufraeumenJa,
    /// Zeigt den Sicherungsordner im Dateimanager.
    ZeitOrdner,
    KorbAuswahl(usize),
    KorbZurueck,
    KorbLoeschen,
    /// Das Fenster soll geschlossen werden.
    FensterSchliessen(iced::window::Id),
    /// Speichern und beenden.
    BeendenSpeichern,
    /// Ohne Speichern beenden.
    BeendenVerwerfen,
    /// Schaltet das automatische Speichern ein oder aus.
    AutospeichernToggle(bool),
    /// Stellt das Tagespensum ein.
    PensumSetzen(usize),
    /// Startet den Code-Prüfer für alle Rust- und Dart/Flutter-Beispiele.
    CodePruefungStarten,
    /// Ergebnis für ein Beispiel (Index, Ergebnis).
    CodeErgebnis(usize, Ergebnis),
    /// Der Code-Prüfer ist fertig.
    CodeEnde,
    /// Startet die Sprachprüfung über das ganze Buch.
    BuchSpracheStarten,
    /// Ergebnis für ein Kapitel (Kennung, Titel, Zahl der Hinweise).
    BuchSpracheKapitel { id: String, titel: String, ergebnis: Result<usize, String> },
    /// Die Buch-Sprachprüfung ist fertig.
    BuchSpracheEnde,
    /// Wechselt den Reiter des Inspektors.
    InspektorTab(u8),
    /// Beginnt oder beendet das Ziehen am Rand der rechten Detailspalte.
    InspektorZiehen(bool),
    /// Mausposition (x) während des Ziehens.
    InspektorMaus(f32),
    /// Sucht einen Begriff im Kapitel (aus dem Stil-Reiter).
    SucheNach(String),
    /// Prüft, ob Pandoc und Ghostscript installiert sind.
    WerkzeugePruefen,
    /// Ergebnis der Werkzeugprüfung.
    WerkzeugeGeprueft(werkzeuge::WerkzeugStatus),
    /// Zeigt oder verbirgt die Installationshinweise (1 = Pandoc, 2 = Ghostscript).
    WerkzeugInfoToggle(u8),
    /// Setzt die Satzvorschau des offenen Kapitels neu.
    VorschauAktualisieren,
    /// Ergebnis der Satzvorschau (Versionsnummer, Seiten).
    VorschauFertig(u64, Result<Arc<werkbank_satz::VorschauErgebnis>, String>),
    /// Zwischen Satzvorschau und schneller Textvorschau umschalten (`true` = Text).
    VorschauText(bool),
    /// PDF/X-3 mit Ghostscript erzeugen.
    SatzPdfx(bool),
    SatzStilOffen,
    SatzStilVorlage(String),
    SatzStilElement(String),
    SatzStilFeld(String, String),
    SatzStilZuruecksetzen(bool),
    SatzStilMusterFarbig(bool),
    SatzMusterFertig(u64, Result<Arc<Vec<werkbank_satz::Seitenbild>>, String>),
    /// Installiert ein Werkzeug unter Windows mit winget (Pandoc, Ghostscript, Python, Git).
    WerkzeugInstallieren(&'static str),
    /// Öffnet eine Webseite im Standardbrowser.
    LinkOeffnen(String),
    /// Setzt das Buch: Druck-PDFs, EPUB, Verlagsordner (ein Ablauf für alle Bücher).
    SatzStarten,
    /// Ergebnis des Satzes.
    SatzFertig(Result<werkbank_satz::buchbau::Bericht, String>),
    /// Startet einen Bauschritt (Index in der Befehlsliste).
    BauStarten(usize),
    /// Nächster Arbeitsschritt des Satzes (für die Fortschrittsanzeige).
    SatzSchritt(String),
    /// Takt der Arbeitsanzeige, solange gesetzt oder gebaut wird.
    Puls,
    /// Eine Zeile Ausgabe des Bauprozesses.
    BauZeile(String),
    /// Der Bauprozess ist beendet (Exitcode, `None` = abgebrochen).
    BauFertig(Option<i32>),
    /// Bricht den laufenden Bauprozess ab.
    BauAbbrechen,
    /// Öffnet eine Datei mit dem Standardprogramm.
    DateiOeffnen(PathBuf),
    /// Schreibt die Portalregeln als bearbeitbare Datei in den Band.
    PortaleAnlegen,
    /// Öffnet ein Kapitel und springt zu einer Zeile (ab 0).
    KapitelZeileOeffnen { kapitel_id: String, zeile: usize },
    /// Öffnet ein Kapitel und prüft es mit LanguageTool.
    KapitelPruefenOeffnen(String),
    /// Ändert ein Feld der Cover-Vorlage.
    CoverFeld(ui::cover::CoverFeld, String),
    /// Barcode-Freifläche ein/aus.
    CoverBarcode(bool),
    /// ISBN der Druckausgabe direkt im Cover-Bereich ändern (wird in den Buchdaten gespeichert).
    CoverIsbn(String),
    /// Farben aus dem Cover-Bild ableiten und übernehmen.
    CoverFarbenVorschlagen,
    /// Letzten Farbvorschlag zurücknehmen.
    CoverFarbenZurueck,
    /// Schrift der Rückseite automatisch verkleinern, wenn der Klappentext nicht passt.
    CoverSchriftAuto(bool),
    /// Regler der Cover-Vorlage (Prozent).
    CoverRegler(ui::cover::CoverRegler, u32),
    /// Auswahlfeld der Cover-Vorlage (Bildbereich, Ausschnitt).
    CoverAuswahl(ui::cover::CoverAuswahl, String),
    /// Graustufen-Vorschau mit 16 E-Ink-Stufen.
    CoverEink(bool),
    /// Gesamtumschlag statt Vorderseite zeigen.
    CoverUmschlag(bool),
    /// Portal für Umschlag und Prüfung wählen.
    CoverPortal(String),
    /// Öffnet den Dateidialog für das Hintergrundbild.
    CoverBildWaehlen,
    /// Ergebnis des Dateidialogs (`None` = entfernen/abbrechen).
    CoverBildGewaehlt(Option<PathBuf>),
    /// Neue Vorschau (Versionsnummer, Bild).
    CoverVorschau(u64, Result<(ui::cover::Bild, Option<werkbank_satz::buchbau::umschlag::Bericht>), String>),
    /// Exportiert alle Cover-Formate.
    CoverExportieren,
    /// Export abgeschlossen.
    CoverExportiert(Result<Vec<PathBuf>, String>),
    /// Nimmt ein neues Kürzel für einen Befehl auf (nächster Tastendruck).
    KuerzelAufnehmen(String),
    /// Stellt das Standardkürzel eines Befehls wieder her.
    KuerzelZuruecksetzen(String),
    /// Hebt die Belegung eines Befehls auf.
    KuerzelLeeren(String),
    /// Öffnet das Handbuch (`F1`).
    HandbuchOeffnen,
    /// Öffnet „Über die Werkbank“.
    UeberOeffnen,
    /// Öffnet die Spendenseite (Buy me a coffee) im Browser.
    SpendenOeffnen,
    /// Tut nichts (für Bedienelemente ohne Wirkung).
    Nichts,
    /// Schriftart des Editors (`serif`, `sans`, `mono`).
    SchriftartSetzen(String),
    /// Zeilenabstand des Editors.
    ZeilenabstandSetzen(f32),
    /// Schreibmaschinen-Modus ein/aus.
    SchreibmaschineToggle(bool),
    /// Schaltet zwischen hellem und dunklem Erscheinungsbild um.
    HellToggle(bool),
    /// Stellt die Editor-Schriftgröße ein.
    SchriftSetzen(f32),
    /// Stellt die Breite der Textspalte ein.
    SpalteSetzen(f32),
    /// Öffnet den Dialog für Titel, Autor, ISBN usw.
    MetaOeffnen,
    /// Ändert ein Feld im Buchdaten-Dialog.
    MetaAendern(ui::dialog::MetaFeld, String),
    /// Speichert die Buchdaten.
    MetaSpeichern,
    /// Lädt das Hunspell-Wörterbuch (neu).
    WoerterbuchLaden,
    /// Ergebnis des Ladens.
    WoerterbuchGeladen(Result<Arc<rechtschreibung::Woerterbuch>, String>),
    /// Rechtschreibmarkierung ein/aus.
    RechtschreibungToggle(bool),
    /// Anderer Ordner für Wörterbücher.
    WoerterbuchOrdner(String),
    /// Eigener Ordner von Ghostscript.
    GsOrdner(String),
    /// Merkt ein Wort als richtig.
    WortMerken(String),
    /// Öffnet die Suche im Kapitel (`false`) oder im ganzen Buch (`true`).
    SucheOeffnen(bool),
    /// Suchbegriff geändert.
    SucheMuster(String),
    /// Ersatztext geändert.
    SucheErsatz(String),
    /// Groß-/Kleinschreibung beachten ein/aus.
    SucheGrossKlein(bool),
    SucheGanzesWort(bool),
    SucheRegex(bool),
    SucheUmfang(ui::suche::Umfang),
    SucheBestaetigungAus,
    /// Suche im ganzen Buch ein/aus.
    /// Springt zu einem Treffer und schließt das Fenster.
    SucheSpringe(usize),
    /// Nächster (`1`) oder vorheriger (`-1`) Treffer (`F3`).
    SucheNaechste(i32),
    /// Ersetzt den gewählten Treffer.
    SucheErsetzenEins,
    /// Ersetzt alle Treffer.
    SucheErsetzenAlle,
    /// Eingabe im Notizfeld.
    NotizenAktion(text_editor::Action),
    /// Fügt einen `@kap:`-Verweis an der Cursorposition ein.
    VerweisEinfuegen(String),
    /// Öffnet oder schließt das Einstellungsfenster (`Strg+,`).
    EinstellungenOeffnen,
    /// Öffnet die Einstellungen mit der LanguageTool-Installationsanleitung.
    AnleitungOeffnen,
    /// Rechtsklick auf ein Kapitel oder einen Teil: wählt es aus und öffnet das Kontextmenü.
    KontextOeffnen(ui::kontext::Ziel, iced::Point),
    /// Eintrag des Kontextmenüs: schließt das Menü und führt die Aktion aus.
    KontextAktion(Box<Message>),
    /// Schließt das Kontextmenü.
    KontextSchliessen,
    /// Klick auf einen Menütitel: öffnet oder schließt das Menü.
    MenueUmschalten(ui::menue::Menue),
    /// Maus über einem Menütitel: wechselt das Menü, wenn eines offen ist.
    MenueUeber(ui::menue::Menue),
    MenueSchliessen,
    /// Eintrag eines Menüs: schließt das Menü und führt die Aktion aus.
    MenueAktion(Box<Message>),
    /// Rechtsklick auf eine markierte Stelle im Editor: öffnet das Korrekturmenü.
    WellenMenue(text_editor::WellenKlick),
    /// Ersetzt einen Bereich einer Zeile (Korrekturvorschlag der Rechtschreibung).
    WortErsetzen { zeile: usize, von: usize, bis: usize, ersatz: String },
    /// Blendet die Markierung eines Wortes bis zum Neustart aus.
    WortIgnorieren(String),
    /// Verwirft einen LanguageTool-Hinweis an dieser Stelle.
    TrefferIgnorieren(usize),
    /// Das Fenster hat eine neue Größe.
    FensterGroesse(Size),
    /// Öffnet die Einstellungen auf einer bestimmten Seite (oder wechselt dorthin).
    EinstellungenSeite(u8),
    /// Schließt das Einstellungsfenster (`Esc`).
    EinstellungenSchliessen,
    /// Blendet die Installationsanleitung ein oder aus.
    EinstellungenInfoToggle,
    /// Wählt das Betriebssystem der Installationsanleitung.
    EinstellungenSystem(ui::einstellungen::Betriebssystem),
    /// Schaltet die Sprachprüfung ein oder aus.
    LtAktiv(bool),
    /// Schaltet die automatische Prüfung ein oder aus.
    LtAutomatisch(bool),
    /// Schaltet den automatischen Start des lokalen Servers ein oder aus.
    LtAutostart(bool),
    /// Ändert die Prüfsprache.
    LtSprache(String),
    /// Ändert die Server-Adresse.
    LtUrl(String),
    /// Ändert den Pfad zum LanguageTool-Ordner.
    LtOrdner(String),
    /// Ändert das Java-Programm.
    LtJava(String),
    /// Öffnet den Ordnerdialog für den LanguageTool-Ordner.
    LtOrdnerWaehlen,
    /// Ergebnis des Ordnerdialogs.
    LtOrdnerGewaehlt(Option<PathBuf>),
    /// Startet den lokalen LanguageTool-Server.
    LtServerStarten,
    /// Beendet den von der Werkbank gestarteten Server.
    LtServerStoppen,
    /// Testet die Verbindung zum Server.
    LtVerbindungTesten,
    /// Ergebnis eines Verbindungstests.
    LtVerbindung(Result<String, String>),
    /// Ergebnis eines Verbindungsversuchs während der Server hochfährt (Versuchsnummer).
    LtStartVersuch(u32, Result<String, String>),
    /// Prüft das offene Kapitel mit LanguageTool (`Strg+L`).
    KapitelPruefen,
    /// Ergebnis eines Prüflaufs.
    PruefErgebnis {
        kapitel_id: String,
        ergebnis: Result<Vec<Treffer>, String>,
    },
    /// Zeitgeber für die automatische Prüfung nach einer Schreibpause.
    Takt(Instant),
    /// Markiert einen Hinweis im Editor.
    TrefferAnspringen(usize),
    /// Ersetzt einen Hinweis durch einen Vorschlag (Hinweis, Vorschlag).
    TrefferErsetzen(usize, usize),
    /// Merkt sich das Wort eines Hinweises als richtig.
    TrefferWortMerken(usize),
    /// Schaltet die Regel eines Hinweises für dieses Buch aus.
    TrefferRegelAus(usize),
    /// Hebt eine ausgeschaltete Regel wieder auf.
    AusnahmeRegelEntfernen(String),
    /// Entfernt ein gemerktes Wort.
    AusnahmeWortEntfernen(String),
    /// Kopiert Text in die Zwischenablage.
    Kopieren(String),
    /// Öffnet oder schließt die Befehlspalette (`Strg+K`).
    PaletteToggle,
    /// Schließt die Befehlspalette (`Esc`).
    PaletteSchliessen,
    /// Der Suchtext der Palette hat sich geändert.
    PaletteSuche(String),
    /// Verschiebt die Auswahl in der Palette um n Zeilen.
    PaletteBewegen(i32),
    /// Führt den gewählten Palette-Treffer aus (`Enter`).
    PaletteAusfuehren,
    /// Führt den angeklickten Palette-Treffer aus.
    PaletteWaehlen(usize),
    /// Globale Tastaturereignisse (Shortcuts).
    TastaturEreignis(keyboard::Event),
}

/// Erster Teil, erstes Kapitel und dessen Text eines Projekts.
fn startauswahl(projekt: &Projekt) -> (String, String, String) {
    if let Some(teil) = projekt.teile.first() {
        let t_id = teil.id.clone();
        if let Some(k) = teil.kapitel.first() {
            (t_id, k.id.clone(), k.inhalt.clone())
        } else {
            (t_id, String::new(), "# Neues Kapitel\n\n".to_string())
        }
    } else {
        (
            String::new(),
            String::new(),
            "# Neues Kapitel\n\n".to_string(),
        )
    }
}

/// Initialisiert die Anwendung mit dem zuletzt geöffneten Ordner oder dem Demoprojekt.
fn new() -> (AppState, Task<Message>) {
    let (baende, band, projekt, meldung) = letzten_ordner_lesen()
        .and_then(|o| ordner_einlesen(&o).ok())
        .and_then(|(baende, band)| {
            let projekt = band.laden().ok()?;
            let meldung = format!("„{}“ geöffnet", band);
            Some((baende, band, projekt, meldung))
        })
        .unwrap_or_else(|| {
            let demo = Band::demo();
            (
                vec![demo.clone()],
                demo,
                Projekt::beispiel_projekt(),
                "Gespeichert".to_string(),
            )
        });
    let (erst_teil_id, erst_kapitel_id, initialer_text) = startauswahl(&projekt);
    let einstellungen = Einstellungen::laden();
    einstellungen.darstellung_anwenden();
    werkzeuge::gs_ordner_setzen(&einstellungen.gs_ordner);
    let (statistik, ausnahmen) = dateien::band_daten_laden(&band);

    let state = AppState {
        projekt,
        band,
        baende,
        projekt_cache: HashMap::new(),
        aktiver_bereich: Arbeitsbereich::Schreiben,
        aktiver_teil_id: erst_teil_id,
        aktives_kapitel_id: erst_kapitel_id,
        editor_content: text_editor::Content::with_text(&initialer_text),
        ist_geaendert: false,
        speicher_zeitstempel: meldung,
        fokus_modus: false,
        seitenleiste_sichtbar: true,
        inspektor_sichtbar: true,
        vorschau_sichtbar: false,
        palette: None,
        kontext: None,
        menue: None,
        ignoriert: HashSet::new(),
        fenster_groesse: Size::new(1380.0, 880.0),
        inspektor_tab: 0,
        inspektor_zieht: false,
        inspektor_maus_x: None,
        stil_cache: std::cell::RefCell::new(None),
        werkzeuge: Default::default(),
        satz_vorschau: Default::default(),
        vorschau_faellig: None,
        cover: Default::default(),
        meta_entwurf: None,
        portale: Portalregeln::default(),
        code: Default::default(),
        buch_sprache: Default::default(),
        bau: Default::default(),
        suche: None,
        suche_offen: false,
        notizen_content: text_editor::Content::new(),
        dirty: HashSet::new(),
        struktur_dirty: false,
        meta_dirty: false,
        snapshot_zwang: false,
        letzte_aenderung: None,
        eingabe: None,
        dialog: None,
        verlauf: None,
        statistik,
        statistik_dirty: false,
        letzte_eingabe: None,
        wortzahl_stand: initialer_text.split_whitespace().count(),
        letzte_sicherung: None,
        satz_seiten: None,
        ueber_offen: false,
        handbuch_offen: false,
        kuerzel_aufnahme: None,
        einstellungen,
        einstellungen_fenster: None,
        woerterbuch: None,
        woerterbuch_status: "Wörterbuch wird geladen …".to_string(),
        hl_cache: std::cell::RefCell::new(None),
        server_status: ServerStatus::Unbekannt,
        server: None,
        treffer: Vec::new(),
        pruef_status: PruefStatus::Leer,
        ausnahmen,
        pruef_faellig: None,
        edit_tick: 0,
        kapitel_fortlaufend: 4,
    };

    let wb_start = Task::batch([Task::done(Message::WoerterbuchLaden), Task::done(Message::WerkzeugePruefen)]);
    let start = if state.einstellungen.server_autostart
        && !state.einstellungen.lt_ordner.trim().is_empty()
    {
        Task::done(Message::LtServerStarten)
    } else if state.einstellungen.sprachpruefung {
        Task::done(Message::LtVerbindungTesten)
    } else {
        Task::none()
    };

    let mut state = state;
    band_umfeld_laden(&mut state);
    notizen_laden(&mut state);
    (state, Task::batch([start, wb_start]))
}

fn ausnahmen_speichern(state: &AppState) {
    if let Some(ordner) = &state.band.ordner {
        let _ = state.ausnahmen.speichern(ordner);
    }
}

/// Läuft die automatische Sprachprüfung (eingeschaltet und Server erreichbar)?
fn auto_pruefung(state: &AppState) -> bool {
    state.einstellungen.sprachpruefung
        && state.einstellungen.automatisch
        && matches!(state.server_status, ServerStatus::Erreichbar(_))
}

/// Setzt die Hinweise zurück, wenn der Text gewechselt wurde, und plant ggf. eine Prüfung ein.
fn pruefung_zuruecksetzen(state: &mut AppState) {
    state.treffer.clear();
    state.pruef_status = PruefStatus::Leer;
    state.pruef_faellig = auto_pruefung(state).then(Instant::now);
}

/// Wartet kurz und testet dann, ob der gerade gestartete Server schon antwortet.
fn start_versuch(url: String, versuch: u32) -> Task<Message> {
    Task::perform(
        languagetool::im_hintergrund(move || {
            std::thread::sleep(Duration::from_secs(2));
            languagetool::verbindung_testen(&url)
        }),
        move |ergebnis| Message::LtStartVersuch(versuch, ergebnis),
    )
}

/// Gültigen Hinweis holen: Der Text an der Stelle muss noch dem Fund entsprechen.
fn treffer_holen(state: &mut AppState, index: usize) -> Option<Treffer> {
    let t = state.treffer.get(index)?.clone();
    let text = state.editor_content.text();
    if text.get(t.von..t.bis) == Some(t.fundstelle.as_str()) {
        Some(t)
    } else {
        state.treffer.clear();
        state.pruef_status = PruefStatus::Leer;
        state.speicher_zeitstempel = "Der Text hat sich geändert – bitte neu prüfen".to_string();
        None
    }
}

/// Macht einen Band zum aktuellen. Der verlassene Band wird samt Editor-Stand zwischengelagert.
fn band_aktivieren(state: &mut AppState, band: Band) -> Result<(), String> {
    if band == state.band {
        return Ok(());
    }

    // Erst den aktuellen Band sichern; schlägt das fehl, bleibt alles wie es ist.
    if dateien::hat_ungespeichertes(state) {
        dateien::speichern(state, dateien::SNAPSHOT_MANUELL)
            .map_err(|e| format!("Der aktuelle Band ließ sich nicht speichern: {e}"))?;
    }

    let neu = match state.projekt_cache.remove(&band) {
        Some(p) => p,
        None => band.laden()?,
    };

    dateien::editor_in_modell(state);
    if let Some(ordner) = state.band.ordner.clone() {
        dateien::statistik_speichern(state, &ordner);
    }
    let alt = std::mem::replace(&mut state.projekt, neu);
    let alter_band = std::mem::replace(&mut state.band, band);
    // Nur das Demo-Buch (ohne Ordner) lebt allein im Speicher und wird zwischengelagert.
    if alter_band.ordner.is_none() {
        state.projekt_cache.insert(alter_band, alt);
    }

    let (teil_id, kapitel_id, text) = startauswahl(&state.projekt);
    state.aktiver_teil_id = teil_id;
    state.aktives_kapitel_id = kapitel_id;
    state.editor_content = text_editor::Content::with_text(&text);
    state.wortzahl_stand = text.split_whitespace().count();
    state.ist_geaendert = false;
    state.edit_tick = state.edit_tick.wrapping_add(1);
    state.speicher_zeitstempel = format!("„{}“ geöffnet", state.band);
    let (statistik, ausnahmen) = dateien::band_daten_laden(&state.band);
    state.statistik = statistik;
    state.statistik_dirty = false;
    state.ausnahmen = ausnahmen;
    state.letzte_eingabe = None;
    band_umfeld_laden(state);
    notizen_laden(state);
    pruefung_zuruecksetzen(state);
    Ok(())
}

/// Lädt die Notizen des offenen Kapitels ins Notizfeld.
fn band_umfeld_laden(state: &mut AppState) {
    state.code = Default::default();
    state.buch_sprache = Default::default();
    state.bau = Default::default();
    state.cover = Default::default();
    state.cover.daten = state
        .band
        .ordner
        .as_deref()
        .and_then(werkbank_kern::cover::CoverDaten::laden)
        .unwrap_or_else(|| werkbank_kern::cover::CoverDaten {
            titel: state.projekt.metadaten.titel.clone(),
            untertitel: state.projekt.metadaten.untertitel.clone(),
            bandzeile: state.projekt.metadaten.band_bezeichnung.clone(),
            autor: state.projekt.metadaten.autor.clone(),
            ..Default::default()
        });
    state.cover.bild_masse = (!state.cover.daten.bild.is_empty()).then(|| werkbank_kern::cover::bildmasse(&state.cover.daten.bild)).flatten();
    match state.band.ordner.clone() {
        Some(o) => {
            state.portale = Portalregeln::laden(&o);
            state.bau.befehle = bau::erkennen(&o);
            // Gehört der Band zu einem Buch mit buchbau.yaml? (einmal beim Laden, nicht beim Zeichnen)
            state.bau.buchbau = werkbank_satz::buchbau::profil::Profil::suchen(&o)
                .and_then(|(buch, p)| p.band_zu_ordner(&buch, &o).map(|b| (buch.clone(), b.nummer, b.untertitel.clone())));
            state.bau.stil = werkbank_satz::gestaltung::Satzstil::laden(&o);
            state.bau.muster.clear();
        }
        None => state.portale = Portalregeln::default(),
    }
}

/// Rendert die Cover-Vorschau im Hintergrund; nur die neueste Version wird übernommen.
fn cover_vorschau(state: &mut AppState) -> Task<Message> {
    state.cover.version += 1;
    let version = state.cover.version;
    let daten = state.cover.daten.clone();
    let portal = state.portale.portale.get(state.cover.portal).cloned();
    let seiten = if daten.seiten > 0 { daten.seiten } else { state.satz_seiten.unwrap_or(state.projekt.buchseiten_schaetzung() as u32) };
    let umschlag = state.cover.umschlag;
    let ordner = state.band.ordner.clone();
    let isbn = state.projekt.metadaten.isbn_print.clone();
    // Den Bericht über den Klappentext liefert der Umschlagsatz mit der Vorschau
    state.cover.klappentext = None;
    Task::perform(
        languagetool::im_hintergrund(move || {
            // Gesamtumschlag: derselbe Satz wie beim Bauen (Typst); Text aus klappentext.md, Farben und Bild aus dem Designer
            if umschlag && let Some(p) = &portal {
                let r = cover::umschlag_setzen(&daten, p, seiten, ordner.as_deref(), isbn.as_deref(), false, Some(1100))?;
                let b = r.bild.ok_or("kein Umschlagbild")?;
                return Ok(((b.breite, b.hoehe, Arc::new(b.rgba)), Some(r.bericht)));
            }
            // Hat der Band ein fertiges cover.png und wurde im Cover-Designer kein Hintergrundbild gewählt, zeigt die Vorschau dieses Bild
            if !umschlag
                && daten.bild.is_empty()
                && let Some(o) = ordner.as_deref()
                && let Ok(daten) = std::fs::read(o.join("cover.png"))
                && let Ok(voll) = resvg::tiny_skia::Pixmap::decode_png(&daten)
            {
                let faktor = 520.0 / voll.width() as f32;
                let (w, h) = (520u32, ((voll.height() as f32 * faktor).round() as u32).max(1));
                if let Some(mut klein) = resvg::tiny_skia::Pixmap::new(w, h) {
                    klein.draw_pixmap(
                        0,
                        0,
                        voll.as_ref(),
                        &resvg::tiny_skia::PixmapPaint { quality: resvg::tiny_skia::FilterQuality::Bilinear, ..Default::default() },
                        resvg::tiny_skia::Transform::from_scale(faktor, faktor),
                        None,
                    );
                    return Ok(((w, h, Arc::new(klein.take())), None));
                }
            }
            let (b, h) = portal.as_ref().map_or((1600, 2560), |p| p.cover_ebook_px);
            let svg = werkbank_kern::cover::svg_ebook(&daten, b as f32, h as f32, portal.as_ref().map(|p| (p, seiten)));
            cover::rendern(&svg, 520).map(|(b, h, px)| ((b, h, Arc::new(px)), None))
        }),
        move |ergebnis| Message::CoverVorschau(version, ergebnis),
    )
}

/// Erzeugt die Graustufen-Fassung der Vorschau (für E-Reader) aus den Farbpixeln.
fn cover_grau_neu(state: &mut AppState) {
    if let Some((b, h, px)) = &state.cover.pixel {
        let stufen = state.cover.eink.then_some(16);
        let grau = cover::graustufen(px, stufen);
        state.cover.vorschau_grau = Some(iced::widget::image::Handle::from_rgba(*b, *h, grau));
    }
}

fn cover_speichern(state: &AppState) {
    if let Some(o) = &state.band.ordner {
        let _ = state.cover.daten.speichern(o);
    }
}

fn notizen_laden(state: &mut AppState) {
    let text = state
        .projekt
        .kapitel_finden(&state.aktives_kapitel_id)
        .map(|k| k.notizen.clone())
        .unwrap_or_default();
    state.notizen_content = text_editor::Content::with_text(&text);
}

/// Aktualisiert die Trefferliste der Suche.
fn suche_aktualisieren(state: &mut AppState) {
    dateien::editor_in_modell(state);
    let aktiv = state.aktives_kapitel_id.clone();
    let andere_baende: Vec<(String, PathBuf)> = state
        .baende
        .iter()
        .filter(|b| b.ordner.is_some() && b.ordner != state.band.ordner)
        .filter_map(|b| Some((b.name.clone(), b.ordner.clone()?)))
        .collect();
    if let Some(s) = state.suche.as_mut() {
        let nur = (s.umfang == ui::suche::Umfang::Kapitel).then_some(aktiv.as_str());
        s.bestaetigen = false;
        s.fehler = None;
        s.andere.clear();
        match werkbank_kern::suche::finden(&state.projekt, &s.muster, &s.ersatz, s.optionen(), nur) {
            Ok(f) => s.funde = f,
            Err(e) => {
                s.funde.clear();
                s.fehler = Some(e);
            }
        }
        if s.umfang == ui::suche::Umfang::Buch && s.fehler.is_none() {
            match werkbank_kern::suche::in_baenden_finden(&andere_baende, &s.muster, &s.ersatz, s.optionen()) {
                Ok(f) => s.andere = f,
                Err(e) => s.fehler = Some(e),
            }
        }
        s.auswahl = s.auswahl.min(s.anzahl().saturating_sub(1));
    }
}

/// Markiert einen Fund im Editor (öffnet vorher sein Kapitel) und fokussiert den Editor.
fn zu_fund_springen(state: &mut AppState, index: usize) -> Task<Message> {
    // Treffer in einem anderen Band: Band öffnen (speichert den aktuellen vorher) und dort suchen
    let fremd = state.suche.as_ref().and_then(|s| s.andere.get(index.checked_sub(s.funde.len())?)).cloned();
    if let Some(b) = fremd {
        let Some(band) = state.baende.iter().find(|x| x.ordner.as_ref() == Some(&b.ordner)).cloned() else {
            return Task::none();
        };
        if let Err(f) = band_aktivieren(state, band.clone()) {
            state.speicher_zeitstempel = format!("„{band}“ ließ sich nicht öffnen: {f}");
            return Task::none();
        }
        suche_aktualisieren(state);
        let ziel = state.suche.as_ref().and_then(|s| s.funde.iter().position(|f| f.kapitel_id == b.fund.kapitel_id && f.byte == b.fund.byte));
        if let Some(neu) = ziel {
            if let Some(s) = state.suche.as_mut() {
                s.auswahl = neu;
            }
            return zu_fund_springen(state, neu);
        }
        return Task::none();
    }
    let Some(fund) = state.suche.as_ref().and_then(|s| s.funde.get(index)).cloned() else {
        return Task::none();
    };
    if fund.kapitel_id != state.aktives_kapitel_id {
        state.aktiver_bereich = Arbeitsbereich::Schreiben;
        kapitel_oeffnen(state, &fund.kapitel_id);
    }
    state.editor_content.move_to(text_editor::Cursor {
        position: text_editor::Position { line: fund.zeile, column: fund.spalte + fund.laenge },
        selection: Some(text_editor::Position { line: fund.zeile, column: fund.spalte }),
    });
    iced::widget::operation::focus(ui::schreiben::editor_id())
}

/// Öffnet die Zeitleiste für das offene Kapitel.
fn zeitleiste_oeffnen(state: &mut AppState, reiter: zeitleiste::Reiter) {
    dateien::editor_in_modell(state);
    let id = state.aktives_kapitel_id.clone();
    let z = Zeitleiste {
        titel: state.projekt.kapitel_finden(&id).map_or_else(|| "Kapitel".to_string(), |k| k.titel.clone()),
        kapitel_id: id,
        staende: Vec::new(),
        aktuell: String::new(),
        ungespeichert: false,
        reiter,
        ansicht: zeitleiste::Ansicht::Aenderungen,
        auswahl: None,
        offen: HashSet::new(),
        alter_text: None,
        vergleich: None,
        name: String::new(),
        aufraeumen: None,
        uebersicht: (0, 0),
        korb: Vec::new(),
        korb_auswahl: None,
        korb_text: String::new(),
        korb_sicher: false,
    };
    state.verlauf = Some(Box::new(z));
    zeitleiste_aktualisieren(state, None);
}

/// Liest Stände, Papierkorb und Vergleich neu ein; `behalte` ist der Stand, der gewählt bleiben soll.
fn zeitleiste_aktualisieren(state: &mut AppState, behalte: Option<PathBuf>) {
    let Some(mut z) = state.verlauf.take() else { return };
    dateien::editor_in_modell(state);
    z.aktuell = state.editor_content.text();
    z.ungespeichert = dateien::hat_ungespeichertes(state);
    z.staende = state.projekt.staende_fuer_kapitel(&z.kapitel_id);
    z.uebersicht = state.projekt.speicher_uebersicht();
    z.korb = state.projekt.papierkorb_liste();
    z.auswahl = behalte
        .and_then(|p| z.staende.iter().position(|s| s.pfad == p))
        .or(if z.staende.is_empty() { None } else { Some(0) });
    z.offen.clear();
    if let Some(i) = z.auswahl
        && let Some((start, _)) = z.gruppen().into_iter().find(|(a, e)| (*a..*e).contains(&i))
    {
        z.offen.insert(start);
    }
    zeitleiste_auswahl(&state.projekt, &mut z);
    z.korb_auswahl = match z.korb_auswahl {
        Some(i) if i < z.korb.len() => Some(i),
        _ if z.korb.is_empty() => None,
        _ => Some(0),
    };
    z.korb_sicher = false;
    korb_text_laden(&mut z);
    state.verlauf = Some(z);
}

/// Lädt Text, Vergleich und Namensfeld des gewählten Standes.
fn zeitleiste_auswahl(projekt: &Projekt, z: &mut Zeitleiste) {
    let stand = z.auswahl.and_then(|i| z.staende.get(i));
    let alt = stand.and_then(|s| projekt.kapitel_text_im_stand(s, &z.kapitel_id));
    z.name = stand.and_then(|s| s.name.clone()).unwrap_or_default();
    z.vergleich = alt.as_ref().map(|a| werkbank_kern::diff::Vergleich::neu(a, &z.aktuell));
    z.alter_text = alt;
}

fn korb_text_laden(z: &mut Zeitleiste) {
    z.korb_text = z
        .korb_auswahl
        .and_then(|i| z.korb.get(i))
        .and_then(|k| std::fs::read_to_string(&k.pfad).ok())
        .unwrap_or_default();
}

/// Ersetzt den Text des offenen Kapitels und sichert dabei beide Seiten: erst der bisherige Stand,
/// dann (beim nächsten Speichern) wandert der Text davor in die Zeitleiste. Nichts geht verloren.
fn kapitel_text_setzen(state: &mut AppState, neu: String) -> Result<(), String> {
    let id = state.aktives_kapitel_id.clone();
    dateien::speichern_nummeriert(state)?;
    if let Some(k) = state.projekt.kapitel_finden_mut(&id) {
        k.inhalt = neu;
    }
    dateien::teil_vormerken(state, &id);
    dateien::editor_neu_laden(state);
    dateien::speichern_nummeriert(state)?;
    Ok(())
}

/// Öffnet ein Kapitel im Editor und wählt den zugehörigen Teil in der Bibliothek.
/// Ungesicherte Eingaben des bisherigen Kapitels werden vorher ins Modell übernommen.
fn kapitel_oeffnen(state: &mut AppState, kapitel_id: &str) {
    if state.ist_geaendert
        && let Some(alt) = state.projekt.kapitel_finden_mut(&state.aktives_kapitel_id)
    {
        alt.inhalt = state.editor_content.text();
    }

    let Some(inhalt) = state
        .projekt
        .kapitel_finden(kapitel_id)
        .map(|k| k.inhalt.clone())
    else {
        return;
    };

    if let Some(teil) = state
        .projekt
        .teile
        .iter()
        .find(|t| t.kapitel.iter().any(|k| k.id == kapitel_id))
    {
        state.aktiver_teil_id = teil.id.clone();
    }
    state.aktives_kapitel_id = kapitel_id.to_string();
    state.wortzahl_stand = inhalt.split_whitespace().count();
    state.editor_content = text_editor::Content::with_text(&inhalt);
    state.ist_geaendert = false;
    state.edit_tick = state.edit_tick.wrapping_add(1);
    notizen_laden(state);
    pruefung_zuruecksetzen(state);
    if state.vorschau_sichtbar {
        state.vorschau_faellig = Some(Instant::now() - Duration::from_secs(5));
    }
}

/// Alle Kapitel des Buches in Lesereihenfolge.
fn kapitel_ids(projekt: &Projekt) -> Vec<String> {
    projekt
        .teile
        .iter()
        .flat_map(|t| t.kapitel.iter().map(|k| k.id.clone()))
        .collect()
}

/// Behandelt Zustandsänderungen nach The Elm Architecture.
fn update(state: &mut AppState, message: Message) -> Task<Message> {
        match message {
        Message::BereichWechseln(bereich) => {
            state.aktiver_bereich = bereich;
            if bereich == Arbeitsbereich::Cover {
                return cover_vorschau(state);
            }
        }

        Message::ProjektWechseln(band) => {
            if let Err(fehler) = band_aktivieren(state, band.clone()) {
                state.speicher_zeitstempel =
                    format!("„{}“ ließ sich nicht öffnen: {}", band, fehler);
            }
        }

        Message::OrdnerDialog => {
            return Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Ordner mit Buchprojekt öffnen")
                        .pick_folder()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::OrdnerGewaehlt,
            );
        }

        Message::OrdnerGewaehlt(None) => {}

        Message::OrdnerGewaehlt(Some(ordner)) => match ordner_einlesen(&ordner) {
            Ok((baende, band)) => {
                state.baende = baende;
                // Den Anzeigenamen des Zielbandes ggf. gleich halten, damit der Cache greift
                match band_aktivieren(state, band) {
                    Ok(()) => letzten_ordner_merken(&ordner),
                    Err(fehler) => state.speicher_zeitstempel = fehler,
                }
            }
            Err(fehler) => state.speicher_zeitstempel = fehler,
        },

        Message::TeilAusgewaehlt(teil_id) => {
            state.aktiver_teil_id = teil_id.clone();
            // Das erste Kapitel des neuen Teils wird direkt geöffnet.
            let erstes = state
                .projekt
                .teile
                .iter()
                .find(|t| t.id == teil_id)
                .and_then(|t| t.kapitel.first())
                .map(|k| k.id.clone());
            if let Some(k_id) = erstes {
                kapitel_oeffnen(state, &k_id);
            }
        }

        Message::KapitelAusgewaehlt(kapitel_id) => {
            kapitel_oeffnen(state, &kapitel_id);
        }

        Message::KapitelNachbar(schritt) => {
            let ids = kapitel_ids(&state.projekt);
            if let Some(pos) = ids.iter().position(|i| *i == state.aktives_kapitel_id) {
                let ziel = (pos as i32 + schritt).clamp(0, ids.len() as i32 - 1) as usize;
                kapitel_oeffnen(state, &ids[ziel]);
            }
        }

        Message::QuerverweisOeffnen(kapitel_id) => {
            if state.projekt.kapitel_finden(&kapitel_id).is_some() {
                kapitel_oeffnen(state, &kapitel_id);
            } else {
                state.speicher_zeitstempel =
                    format!("Verweisziel „{}“ existiert nicht", kapitel_id);
            }
        }

        Message::EditorAktion(action) => {
            let war_edit = action.is_edit();
            state.editor_content.perform(action);

            if war_edit {
                state.ist_geaendert = true;
                state.edit_tick = state.edit_tick.wrapping_add(1);
                // Text live im Datenmodell synchronisieren für Wort- und Zeichenzähler
                dateien::editor_in_modell(state);
                let id = state.aktives_kapitel_id.clone();
                state.projekt.titel_aus_ueberschrift(&id);
                dateien::teil_vormerken(state, &id);
                statistik_erfassen(state);
                if auto_pruefung(state) {
                    state.pruef_faellig = Some(Instant::now());
                }
                if state.vorschau_sichtbar {
                    state.vorschau_faellig = Some(Instant::now());
                }
            }
            return vorschau_folgen(state);
        }

        Message::Speichern => {
            state.speicher_zeitstempel = match dateien::speichern_nummeriert(state) {
                Ok(0) if state.band.ordner.is_none() => "Demo-Buch: wird nicht auf die Platte geschrieben".to_string(),
                Ok(0) => "Keine Änderungen – alles ist schon gespeichert".to_string(),
                Ok(n) => format!(
                    "Gespeichert um {} ({} Datei{}){}",
                    Local::now().format("%H:%M:%S"),
                    n,
                    if n == 1 { "" } else { "en" },
                    state.letzte_sicherung.map(|_| " · alter Stand gesichert".to_string()).unwrap_or_default()
                ),
                Err(e) => format!("Speichern fehlgeschlagen: {e}"),
            };
        }

        Message::FokusToggle => {
            state.fokus_modus = !state.fokus_modus;
        }

        Message::SeitenleisteToggle => {
            state.seitenleiste_sichtbar = !state.seitenleiste_sichtbar;
        }

        Message::InspektorToggle => {
            state.inspektor_sichtbar = !state.inspektor_sichtbar;
        }

        Message::VorschauToggle => {
            state.vorschau_sichtbar = !state.vorschau_sichtbar;
            if state.vorschau_sichtbar {
                state.satz_vorschau.text_modus = false;
                state.vorschau_faellig = Some(Instant::now() - Duration::from_secs(5));
            }
        }

        Message::InspektorZiehen(an) => {
            state.inspektor_zieht = an;
            state.inspektor_maus_x = None;
            if !an {
                state.einstellungen.speichern();
            }
        }

        Message::InspektorMaus(x) => {
            if let Some(vorher) = state.inspektor_maus_x {
                // Nach links ziehen macht die Spalte breiter
                let neu = (state.einstellungen.inspektor_breite + (vorher - x)).clamp(240.0, 720.0);
                state.einstellungen.inspektor_breite = neu;
                thema::inspektor_breite_setzen(neu);
            }
            state.inspektor_maus_x = Some(x);
        }

        Message::InspektorTab(nr) => {
            state.inspektor_tab = nr.min(3);
            state.inspektor_sichtbar = true;
        }

        Message::SucheNach(begriff) => {
            let s = state.suche.get_or_insert_with(Suche::default);
            s.muster = begriff;
            s.umfang = ui::suche::Umfang::Kapitel;
            state.suche_offen = true;
            suche_aktualisieren(state);
            return iced::widget::operation::focus(ui::suche::feld_id());
        }

        Message::WerkzeugePruefen => {
            state.werkzeuge.geprueft = false;
            return Task::perform(languagetool::im_hintergrund(|| Ok(werkzeuge::pruefen())), |e| {
                Message::WerkzeugeGeprueft(e.unwrap_or_default())
            });
        }

        Message::WerkzeugeGeprueft(status) => {
            state.werkzeuge = status;
        }

        Message::WerkzeugInfoToggle(nr) => {
            if let Some(f) = state.einstellungen_fenster.as_mut() {
                f.werkzeug_info = if f.werkzeug_info == nr { 0 } else { nr };
            }
        }

        Message::VorschauText(an) => {
            state.satz_vorschau.text_modus = an;
            return vorschau_folgen(state);
        }

        Message::SatzStilOffen => {
            state.bau.stil_offen = !state.bau.stil_offen;
            if state.bau.stil_offen {
                return muster_setzen(state);
            }
        }

        Message::SatzStilVorlage(name) => {
            if let Some(v) = werkbank_satz::gestaltung::vorlagen().into_iter().find(|v| v.name == name) {
                state.bau.stil = v.stil;
                stil_sichern(state);
                return muster_setzen(state);
            }
        }

        Message::SatzStilElement(k) => state.bau.stil_element = k,

        Message::SatzStilFeld(feld, wert) => {
            let k = if state.bau.stil_element.is_empty() { "text".to_string() } else { state.bau.stil_element.clone() };
            state.bau.stil.setze(&k, &feld, wert);
            stil_sichern(state);
            return muster_setzen(state);
        }

        Message::SatzStilZuruecksetzen(alles) => {
            if alles {
                state.bau.stil = Default::default();
            } else {
                let k = if state.bau.stil_element.is_empty() { "text".to_string() } else { state.bau.stil_element.clone() };
                state.bau.stil.formate.remove(&k);
            }
            stil_sichern(state);
            return muster_setzen(state);
        }

        Message::SatzStilMusterFarbig(an) => {
            state.bau.muster_farbig = an;
            return muster_setzen(state);
        }

        Message::SatzMusterFertig(version, ergebnis) => {
            if version == state.bau.muster_version {
                match ergebnis {
                    Ok(seiten) => {
                        state.bau.muster = seiten.iter().map(|s| iced::widget::image::Handle::from_rgba(s.breite, s.hoehe, s.rgba.clone())).collect();
                        state.bau.muster_meldung.clear();
                    }
                    Err(f) => {
                        state.bau.muster.clear();
                        state.bau.muster_meldung = format!("Muster nicht setzbar: {f}");
                    }
                }
            }
        }

        Message::LinkOeffnen(url) => {
            if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("mailto:") {
                bau::oeffnen(std::path::Path::new(&url));
            }
        }

        Message::WerkzeugInstallieren(id) => {
            werkzeuge::winget_installieren(id);
        }

        Message::SatzPdfx(an) => {
            state.einstellungen.pdfx = an;
            state.einstellungen.speichern();
        }

        Message::VorschauAktualisieren => {
            if state.satz_vorschau.laeuft {
                return Task::none();
            }
            state.vorschau_faellig = None;
            dateien::editor_in_modell(state);
            let Some(k) = state.projekt.kapitel_finden(&state.aktives_kapitel_id) else { return Task::none() };
            let md = k.inhalt.clone();
            let nummern = werkbank_satz::markdown::kapitelnummern(&state.projekt);
            let teile = werkbank_satz::markdown::teil_nummern(&state.projekt);
            let wurzel = state.projekt.wurzelverzeichnis.clone();
            let pandoc = state.werkzeuge.pandoc_brauchbar();
            state.satz_vorschau.version += 1;
            state.satz_vorschau.laeuft = true;
            let version = state.satz_vorschau.version;
            return Task::perform(
                languagetool::im_hintergrund(move || {
                    werkbank_satz::vorschau(&md, &wurzel, &nummern, &teile, pandoc, 24, 1.3).map(Arc::new)
                }),
                move |e| Message::VorschauFertig(version, e),
            );
        }

        Message::VorschauFertig(version, ergebnis) => {
            let v = &mut state.satz_vorschau;
            v.laeuft = false;
            if version == v.version {
                match ergebnis {
                    Ok(e) => {
                        v.seiten = e
                            .seiten
                            .iter()
                            .map(|s| iced::widget::image::Handle::from_rgba(s.breite, s.hoehe, s.rgba.clone()))
                            .collect();
                        v.seitenzahl = e.seitenzahl;
                        v.quelle = e.quelle.to_string();
                        v.hinweise = e.hinweise.clone();
                        v.fehler = None;
                    }
                    Err(f) => v.fehler = Some(f),
                }
            }
            // Wurde währenddessen weitergeschrieben, setzt der nächste Takt neu.
        }

        Message::SpringeZuZeile { zeile, spalte } => {
            state.editor_content.move_to(text_editor::Cursor {
                position: text_editor::Position { line: zeile, column: spalte },
                selection: None,
            });
            return Task::batch([iced::widget::operation::focus(ui::schreiben::editor_id()), vorschau_folgen(state)]);
        }

        Message::NeuesKapitel => {
            state.palette = None;
            state.eingabe = Some(Eingabe { zweck: EingabeZweck::NeuesKapitel, text: String::new() });
            return iced::widget::operation::focus(ui::dialog::eingabe_id());
        }

        Message::NeuerTeil => {
            state.palette = None;
            state.eingabe = Some(Eingabe { zweck: EingabeZweck::NeuerTeil, text: String::new() });
            return iced::widget::operation::focus(ui::dialog::eingabe_id());
        }

        Message::KapitelUmbenennenDialog => {
            if let Some(k) = state.projekt.kapitel_finden(&state.aktives_kapitel_id) {
                let text = k.titel.clone();
                state.palette = None;
                state.eingabe = Some(Eingabe { zweck: EingabeZweck::KapitelUmbenennen, text });
                return iced::widget::operation::focus(ui::dialog::eingabe_id());
            }
        }

        Message::TeilUmbenennenDialog => {
            if let Some(t) = state.projekt.teile.iter().find(|t| t.id == state.aktiver_teil_id) {
                let text = t.titel.clone();
                state.palette = None;
                state.eingabe = Some(Eingabe { zweck: EingabeZweck::TeilUmbenennen, text });
                return iced::widget::operation::focus(ui::dialog::eingabe_id());
            }
        }

        Message::EingabeText(text) => {
            if let Some(e) = state.eingabe.as_mut() {
                e.text = text;
            }
        }

        Message::EingabeOk => {
            let Some(e) = state.eingabe.take() else { return Task::none() };
            dateien::editor_in_modell(state);
            let fehler = match e.zweck {
                EingabeZweck::NeuesKapitel => {
                    let teil = state
                        .projekt
                        .teil_von_kapitel(&state.aktives_kapitel_id)
                        .map(str::to_string)
                        .unwrap_or_else(|| state.aktiver_teil_id.clone());
                    let nach = state.aktives_kapitel_id.clone();
                    let nach = state.projekt.teile.iter().find(|t| t.id == state.aktiver_teil_id).and_then(|t| t.kapitel.iter().find(|k| k.id == nach)).map(|k| k.id.clone());
                    match state.projekt.neues_kapitel(&teil, &e.text, nach.as_deref()) {
                        Ok(id) => {
                            state.dirty.insert(teil);
                            state.letzte_aenderung = Some(Instant::now());
                            state.meta_dirty = true;
                            state.aktiver_bereich = Arbeitsbereich::Schreiben;
                            kapitel_oeffnen(state, &id);
                            None
                        }
                        Err(f) => Some(f),
                    }
                }
                EingabeZweck::NeuerTeil => match state.projekt.neuer_teil(&e.text) {
                    Ok(id) => {
                        state.dirty.insert(id.clone());
                        state.struktur_dirty = true;
                        state.letzte_aenderung = Some(Instant::now());
                        state.aktiver_teil_id = id;
                        state.aktiver_bereich = Arbeitsbereich::Schreiben;
                        // Ein Teil ohne Kapitel ist leer: gleich nach dem ersten Kapitel fragen.
                        state.eingabe = Some(Eingabe { zweck: EingabeZweck::NeuesKapitel, text: String::new() });
                        return iced::widget::operation::focus(ui::dialog::eingabe_id());
                    }
                    Err(f) => Some(f),
                },
                EingabeZweck::KapitelUmbenennen => {
                    let id = state.aktives_kapitel_id.clone();
                    match state.projekt.kapitel_umbenennen(&id, &e.text) {
                        Ok(()) => {
                            dateien::teil_vormerken(state, &id);
                            dateien::editor_neu_laden(state);
                            None
                        }
                        Err(f) => Some(f),
                    }
                }
                EingabeZweck::TeilUmbenennen => {
                    let id = state.aktiver_teil_id.clone();
                    match state.projekt.teil_umbenennen(&id, &e.text) {
                        Ok(()) => {
                            state.dirty.insert(id);
                            state.letzte_aenderung = Some(Instant::now());
                            None
                        }
                        Err(f) => Some(f),
                    }
                }
            };
            if let Some(f) = fehler {
                state.speicher_zeitstempel = f;
            }
        }

        Message::KapitelLoeschenFrage => {
            let Some(k) = state.projekt.kapitel_finden(&state.aktives_kapitel_id) else { return Task::none() };
            let letztes = state
                .projekt
                .teile
                .iter()
                .find(|t| t.kapitel.iter().any(|x| x.id == k.id))
                .is_some_and(|t| t.kapitel.len() <= 1);
            state.palette = None;
            state.dialog = Some(if letztes {
                Dialog {
                    titel: "Letztes Kapitel des Teils".to_string(),
                    text: "Das letzte Kapitel eines Teils lässt sich nicht löschen. Entferne stattdessen den ganzen Teil.".to_string(),
                    knoepfe: vec![("OK".to_string(), Message::DialogSchliessen, true)],
                }
            } else {
                Dialog {
                    titel: "Kapitel löschen?".to_string(),
                    text: format!(
                        "„{}“ wird aus der Datei entfernt. Der bisherige Text bleibt in der Zeitleiste erhalten.",
                        k.titel
                    ),
                    knoepfe: vec![
                        ("Abbrechen".to_string(), Message::DialogSchliessen, false),
                        ("Löschen".to_string(), Message::KapitelLoeschen, true),
                    ],
                }
            });
        }

        Message::KapitelLoeschen => {
            let id = state.aktives_kapitel_id.clone();
            let ids = kapitel_ids(&state.projekt);
            let pos = ids.iter().position(|i| *i == id).unwrap_or(0);
            let teil = state.projekt.teil_von_kapitel(&id).map(str::to_string);
            match state.projekt.kapitel_loeschen(&id) {
                Ok(k) => {
                    state.dirty.extend(teil);
                    state.snapshot_zwang = true;
                    state.meta_dirty = true;
                    state.letzte_aenderung = Some(Instant::now());
                    state.speicher_zeitstempel = format!("„{}“ gelöscht (ungespeichert, siehe Zeitleiste)", k.titel);
                    let rest = kapitel_ids(&state.projekt);
                    let nachbar = rest.get(pos.min(rest.len().saturating_sub(1))).cloned();
                    state.aktives_kapitel_id.clear();
                    state.ist_geaendert = false;
                    if let Some(n) = nachbar {
                        kapitel_oeffnen(state, &n);
                    }
                }
                Err(f) => state.speicher_zeitstempel = f,
            }
        }

        Message::KapitelVerschieben(schritt) => {
            dateien::editor_in_modell(state);
            let id = state.aktives_kapitel_id.clone();
            let Some(kapitel) = state.projekt.kapitel_finden(&id) else { return Task::none() };
            if !kapitel.inhalt.starts_with("## ") {
                state.speicher_zeitstempel = "Dieses Kapitel ist eine ganze Datei und lässt sich nicht verschieben.".to_string();
                return Task::none();
            }
            let vorher = state.projekt.teil_von_kapitel(&id).map(str::to_string);
            if state.projekt.kapitel_verschieben(&id, schritt).is_ok() {
                let nachher = state.projekt.teil_von_kapitel(&id).map(str::to_string);
                if vorher != nachher {
                    state.snapshot_zwang = true;
                }
                state.dirty.extend(vorher);
                if let Some(n) = nachher {
                    state.aktiver_teil_id = n.clone();
                    state.dirty.insert(n);
                }
                state.letzte_aenderung = Some(Instant::now());
            }
        }

        Message::TeilVerschieben(schritt) => {
            let id = state.aktiver_teil_id.clone();
            state.projekt.teil_verschieben(&id, schritt);
            state.struktur_dirty = true;
            state.letzte_aenderung = Some(Instant::now());
        }

        Message::TeilEntfernenFrage => {
            let Some(t) = state.projekt.teile.iter().find(|t| t.id == state.aktiver_teil_id) else { return Task::none() };
            state.palette = None;
            state.dialog = Some(Dialog {
                titel: "Teil entfernen?".to_string(),
                text: format!(
                    "„{}“ wird aus dem Buch genommen. Die Datei {} wandert in .werkbank/papierkorb/ und lässt sich von dort zurückholen.",
                    t.titel,
                    t.dateipfad.display()
                ),
                knoepfe: vec![
                    ("Abbrechen".to_string(), Message::DialogSchliessen, false),
                    ("Entfernen".to_string(), Message::TeilEntfernen, true),
                ],
            });
        }

        Message::TeilEntfernen => {
            let id = state.aktiver_teil_id.clone();
            match state.projekt.teil_entfernen(&id) {
                Ok(()) => {
                    state.dirty.remove(&id);
                    state.struktur_dirty = true;
                    state.letzte_aenderung = Some(Instant::now());
                    let (t, k, text) = startauswahl(&state.projekt);
                    state.aktiver_teil_id = t;
                    state.aktives_kapitel_id = k;
                    state.editor_content = text_editor::Content::with_text(&text);
                    state.edit_tick = state.edit_tick.wrapping_add(1);
                    notizen_laden(state);
                    state.speicher_zeitstempel = "Teil entfernt (Datei im Papierkorb)".to_string();
                }
                Err(e) => state.speicher_zeitstempel = format!("Teil ließ sich nicht entfernen: {e}"),
            }
        }

        Message::VerlaufOeffnen => {
            state.palette = None;
            state.kontext = None;
            zeitleiste_oeffnen(state, zeitleiste::Reiter::Staende);
        }

        Message::PapierkorbOeffnen => {
            state.palette = None;
            state.kontext = None;
            zeitleiste_oeffnen(state, zeitleiste::Reiter::Papierkorb);
        }

        Message::ZeitAuswahl(i) => {
            if let Some(z) = state.verlauf.as_mut()
                && i < z.staende.len()
            {
                z.auswahl = Some(i);
                z.aufraeumen = None;
                if let Some((start, _)) = z.gruppen().into_iter().find(|(a, e)| (*a..*e).contains(&i)) {
                    z.offen.insert(start);
                }
                zeitleiste_auswahl(&state.projekt, z);
            }
        }

        Message::ZeitSitzung(start) => {
            if let Some(z) = state.verlauf.as_mut()
                && !z.offen.remove(&start)
            {
                z.offen.insert(start);
            }
        }

        Message::ZeitAnsicht(a) => {
            if let Some(z) = state.verlauf.as_mut() {
                z.ansicht = a;
            }
        }

        Message::ZeitReiter(r) => {
            if let Some(z) = state.verlauf.as_mut() {
                z.reiter = r;
                z.korb_sicher = false;
                z.aufraeumen = None;
                if r == zeitleiste::Reiter::Papierkorb && z.korb_auswahl.is_none() && !z.korb.is_empty() {
                    z.korb_auswahl = Some(0);
                    korb_text_laden(z);
                }
            }
        }

        Message::ZeitName(t) => {
            if let Some(z) = state.verlauf.as_mut() {
                z.name = t;
            }
        }

        Message::ZeitBenennen => {
            let Some(z) = state.verlauf.as_ref() else { return Task::none() };
            let Some(stand) = z.auswahl.and_then(|i| z.staende.get(i)).filter(|s| s.art == werkbank_kern::verlauf::StandArt::Sicherung) else {
                return Task::none();
            };
            let (pfad, name) = (stand.pfad.clone(), z.name.trim().to_string());
            match state.projekt.stand_benennen(&pfad, &name) {
                Ok(()) => {
                    state.speicher_zeitstempel = if name.is_empty() { "Name des Standes entfernt".to_string() } else { format!("Stand „{name}“ wird beim Aufräumen nie gelöscht") };
                    zeitleiste_aktualisieren(state, Some(pfad));
                }
                Err(e) => state.speicher_zeitstempel = format!("Stand ließ sich nicht benennen: {e}"),
            }
        }

        Message::ZeitWiederherstellen => {
            let Some(z) = state.verlauf.as_ref() else { return Task::none() };
            let (Some(alt), Some(stand)) = (z.alter_text.clone(), z.auswahl.and_then(|i| z.staende.get(i))) else { return Task::none() };
            let zeit = zeitleiste::zeit_text(stand.zeit);
            match kapitel_text_setzen(state, alt) {
                Ok(()) => {
                    state.speicher_zeitstempel = format!("Stand von {zeit} wiederhergestellt – der vorherige Text steht in der Zeitleiste");
                    zeitleiste_aktualisieren(state, None);
                }
                Err(e) => state.speicher_zeitstempel = format!("Wiederherstellen fehlgeschlagen: {e}"),
            }
        }

        Message::ZeitPassage(nr) => {
            let Some(neu) = state.verlauf.as_ref().and_then(|z| z.vergleich.as_ref()).map(|v| v.eine_zurueck(nr)) else { return Task::none() };
            let behalte = state.verlauf.as_ref().and_then(|z| z.auswahl.and_then(|i| z.staende.get(i))).map(|s| s.pfad.clone());
            match kapitel_text_setzen(state, neu) {
                Ok(()) => {
                    state.speicher_zeitstempel = "Passage übernommen – der vorherige Text steht in der Zeitleiste".to_string();
                    // Die Auswahl bleibt dieselbe Datei; die neue Sicherung steht oben
                    zeitleiste_aktualisieren(state, behalte);
                }
                Err(e) => state.speicher_zeitstempel = format!("Übernehmen fehlgeschlagen: {e}"),
            }
        }

        Message::ZeitKopie => {
            let Some(z) = state.verlauf.as_ref() else { return Task::none() };
            let (Some(alt), Some(stand)) = (z.alter_text.clone(), z.auswahl.and_then(|i| z.staende.get(i))) else { return Task::none() };
            let titel = format!("{} (Stand {})", z.titel, stand.zeit.format("%d.%m. %H:%M"));
            let nach = state.aktives_kapitel_id.clone();
            let Some(teil) = state.projekt.teil_von_kapitel(&nach).map(str::to_string) else { return Task::none() };
            dateien::editor_in_modell(state);
            match state.projekt.kapitel_kopie_einfuegen(&teil, &nach, &titel, &alt) {
                Ok(id) => {
                    state.dirty.insert(teil);
                    state.letzte_aenderung = Some(Instant::now());
                    state.meta_dirty = true;
                    state.verlauf = None;
                    state.aktiver_bereich = Arbeitsbereich::Schreiben;
                    kapitel_oeffnen(state, &id);
                    state.speicher_zeitstempel = match dateien::speichern_nummeriert(state) {
                        Ok(_) => format!("„{titel}“ als neues Kapitel eingefügt"),
                        Err(e) => format!("Kopie eingefügt, Speichern fehlgeschlagen: {e}"),
                    };
                }
                Err(e) => state.speicher_zeitstempel = e,
            }
        }

        Message::ZeitKopieren => {
            if let Some(alt) = state.verlauf.as_ref().and_then(|z| z.alter_text.clone()) {
                state.speicher_zeitstempel = "Text des Standes in die Zwischenablage kopiert".to_string();
                return iced::clipboard::write(alt);
            }
        }

        Message::ZeitExport => {
            let Some(z) = state.verlauf.as_ref() else { return Task::none() };
            let Some(stand) = z.auswahl.and_then(|i| z.staende.get(i)) else { return Task::none() };
            let name = format!("{}-{}.md", werkbank_kern::speicher::kennung_aus_titel(&z.titel), stand.zeit.format("%Y%m%d-%H%M%S"));
            let start = state.band.ordner.clone();
            return Task::perform(
                async move {
                    let mut d = rfd::AsyncFileDialog::new().set_title("Stand als Datei speichern").set_file_name(name).add_filter("Markdown", &["md"]);
                    if let Some(o) = start {
                        d = d.set_directory(o);
                    }
                    d.save_file().await.map(|h| h.path().to_path_buf())
                },
                Message::ZeitExportZiel,
            );
        }

        Message::ZeitExportZiel(pfad) => {
            if let (Some(pfad), Some(alt)) = (pfad, state.verlauf.as_ref().and_then(|z| z.alter_text.clone())) {
                state.speicher_zeitstempel = match std::fs::write(&pfad, alt) {
                    Ok(()) => format!("Stand gespeichert: {}", pfad.display()),
                    Err(e) => format!("Datei ließ sich nicht schreiben: {e}"),
                };
            }
        }

        Message::ZeitAufraeumenPlan => {
            let plan = state.projekt.aufraeumen_planen();
            if let Some(z) = state.verlauf.as_mut() {
                z.aufraeumen = Some(plan);
            }
        }

        Message::ZeitAufraeumenAbbruch => {
            if let Some(z) = state.verlauf.as_mut() {
                z.aufraeumen = None;
            }
        }

        Message::ZeitAufraeumenJa => {
            if let Some(plan) = state.verlauf.as_mut().and_then(|z| z.aufraeumen.take()) {
                let behalte = state.verlauf.as_ref().and_then(|z| z.auswahl.and_then(|i| z.staende.get(i))).map(|s| s.pfad.clone());
                state.speicher_zeitstempel = match state.projekt.aufraeumen_ausfuehren(&plan) {
                    Ok(n) => format!("{n} alte Stände gelöscht"),
                    Err(e) => format!("Aufräumen fehlgeschlagen: {e}"),
                };
                zeitleiste_aktualisieren(state, behalte);
            }
        }

        Message::ZeitOrdner => {
            let ordner = state.projekt.sicherungsordner();
            if ordner.is_dir() {
                bau::oeffnen(&ordner);
            } else {
                state.speicher_zeitstempel = "Es gibt noch keine Sicherungen – der Ordner entsteht beim ersten Speichern".to_string();
            }
        }

        Message::KorbAuswahl(i) => {
            if let Some(z) = state.verlauf.as_mut()
                && i < z.korb.len()
            {
                z.korb_auswahl = Some(i);
                z.korb_sicher = false;
                korb_text_laden(z);
            }
        }

        Message::KorbZurueck => {
            let Some(eintrag) = state.verlauf.as_ref().and_then(|z| z.korb_auswahl.and_then(|i| z.korb.get(i))).cloned() else { return Task::none() };
            // Erst alles speichern, dann den Band neu einlesen
            let ergebnis = dateien::speichern_nummeriert(state).map_err(|e| e.to_string()).and_then(|_| {
                state.projekt.papierkorb_zurueckholen(&eintrag).map_err(|e| e.to_string())?;
                state.band.laden()
            });
            match ergebnis {
                Ok(neu) => {
                    state.projekt = neu;
                    dateien::editor_neu_laden(state);
                    state.speicher_zeitstempel = format!("„{}“ zurückgeholt – er steht am Ende des Buches", eintrag.titel);
                    zeitleiste_aktualisieren(state, None);
                }
                Err(e) => state.speicher_zeitstempel = format!("Zurückholen fehlgeschlagen: {e}"),
            }
        }

        Message::KorbLoeschen => {
            let Some(z) = state.verlauf.as_mut() else { return Task::none() };
            if !z.korb_sicher {
                z.korb_sicher = true;
            } else if let Some(eintrag) = z.korb_auswahl.and_then(|i| z.korb.get(i)).cloned() {
                state.speicher_zeitstempel = match state.projekt.papierkorb_loeschen(&eintrag) {
                    Ok(()) => format!("„{}“ endgültig gelöscht", eintrag.titel),
                    Err(e) => format!("Löschen fehlgeschlagen: {e}"),
                };
                zeitleiste_aktualisieren(state, None);
            }
        }

        Message::DialogSchliessen => {
            state.dialog = None;
            state.eingabe = None;
            state.verlauf = None;
            state.handbuch_offen = false;
            state.ueber_offen = false;
            state.suche_offen = false;
            state.meta_entwurf = None;
        }

        Message::DialogAktion(index) => {
            if let Some(d) = state.dialog.take()
                && let Some((_, nachricht, _)) = d.knoepfe.get(index)
            {
                return update(state, nachricht.clone());
            }
        }

        Message::FensterSchliessen(_) => {
            if state.einstellungen.autospeichern || !dateien::hat_ungespeichertes(state) {
                match dateien::speichern_nummeriert(state) {
                    Ok(_) => return beenden(state),
                    Err(e) => {
                        state.dialog = Some(Dialog {
                            titel: "Speichern fehlgeschlagen".to_string(),
                            text: format!("{e}\n\nTrotzdem beenden? Nicht gespeicherte Änderungen gehen verloren."),
                            knoepfe: vec![
                                ("Abbrechen".to_string(), Message::DialogSchliessen, true),
                                ("Trotzdem beenden".to_string(), Message::BeendenVerwerfen, false),
                            ],
                        });
                    }
                }
            } else {
                state.dialog = Some(Dialog {
                    titel: "Ungespeicherte Änderungen".to_string(),
                    text: "Soll vor dem Beenden gespeichert werden?".to_string(),
                    knoepfe: vec![
                        ("Abbrechen".to_string(), Message::DialogSchliessen, false),
                        ("Verwerfen".to_string(), Message::BeendenVerwerfen, false),
                        ("Speichern".to_string(), Message::BeendenSpeichern, true),
                    ],
                });
            }
        }

        Message::BeendenSpeichern => match dateien::speichern_nummeriert(state) {
            Ok(_) => return beenden(state),
            Err(e) => state.speicher_zeitstempel = format!("Speichern fehlgeschlagen: {e}"),
        },

        Message::BeendenVerwerfen => return beenden(state),

        Message::PensumSetzen(wert) => {
            state.einstellungen.tagespensum = wert;
            state.einstellungen.speichern();
        }

        Message::AutospeichernToggle(an) => {
            state.einstellungen.autospeichern = an;
            state.einstellungen.speichern();
        }

        Message::ZielSetzen(ziel) => {
            if let Some(k) = state.projekt.kapitel_finden_mut(&state.aktives_kapitel_id) {
                k.wortziel = ziel.max(250);
                state.meta_dirty = true;
                state.letzte_aenderung = Some(Instant::now());
            }
        }

        Message::StatusAendern(status) => {
            if let Some(k) = state.projekt.kapitel_finden_mut(&state.aktives_kapitel_id) {
                k.status = status;
                state.meta_dirty = true;
                state.letzte_aenderung = Some(Instant::now());
            }
        }

        Message::BauenStarten => return update(state, Message::BauStarten(0)),

        Message::PrueflaufStarten => {
            let mut aufgaben = vec![update(state, Message::CodePruefungStarten)];
            if state.einstellungen.sprachpruefung && matches!(state.server_status, ServerStatus::Erreichbar(_)) {
                aufgaben.push(update(state, Message::BuchSpracheStarten));
            }
            state.aktiver_bereich = Arbeitsbereich::Pruefen;
            return Task::batch(aufgaben);
        }

        Message::CodePruefungStarten => {
            if state.code.laeuft {
                return Task::none();
            }
            dateien::editor_in_modell(state);
            let beispiele = codepruefung::beispiele(&state.projekt);
            state.code.ergebnisse = vec![None; beispiele.len()];
            state.code.beispiele = beispiele.clone();
            state.code.laeuft = !beispiele.is_empty();
            return Task::run(hintergrund::code_pruefen(beispiele, state.portale.edition.clone()), |m| m);
        }

        Message::CodeErgebnis(index, ergebnis) => {
            if let Some(platz) = state.code.ergebnisse.get_mut(index) {
                *platz = Some(ergebnis);
            }
        }

        Message::CodeEnde => {
            state.code.laeuft = false;
            let fehler = state.code.ergebnisse.iter().filter(|e| matches!(e, Some(Ergebnis::Fehler(_)))).count();
            state.speicher_zeitstempel = format!("Code-Prüfer: {} Beispiele, {fehler} Fehler", state.code.beispiele.len());
        }

        Message::BuchSpracheStarten => {
            if state.buch_sprache.laeuft {
                return Task::none();
            }
            dateien::editor_in_modell(state);
            let kapitel: Vec<(String, String, String)> = state
                .projekt
                .teile
                .iter()
                .flat_map(|t| t.kapitel.iter())
                .map(|k| (k.id.clone(), k.titel.clone(), k.inhalt.clone()))
                .collect();
            state.buch_sprache = ui::pruefen::BuchSprache { gesamt: kapitel.len(), laeuft: true, ..Default::default() };
            return Task::run(
                hintergrund::buch_sprache(
                    kapitel,
                    state.einstellungen.server_url.clone(),
                    state.einstellungen.sprache.clone(),
                    state.ausnahmen.clone(),
                ),
                |m| m,
            );
        }

        Message::BuchSpracheKapitel { id, titel, ergebnis } => match ergebnis {
            Ok(n) => state.buch_sprache.zeilen.push((id, titel, n)),
            Err(f) => {
                state.server_status = ServerStatus::Fehler(f.clone());
                state.buch_sprache.fehler = Some(f);
            }
        },

        Message::BuchSpracheEnde => {
            state.buch_sprache.laeuft = false;
            let n: usize = state.buch_sprache.zeilen.iter().map(|z| z.2).sum();
            state.speicher_zeitstempel = format!("Buch-Sprachprüfung: {n} Hinweise");
        }

        Message::SatzStarten => {
            if state.bau.satz_laeuft {
                return Task::none();
            }
            let Some(ordner) = state.band.ordner.clone() else {
                state.bau.satz_meldung = "Das Demo-Buch hat keinen Ordner, in den gesetzt werden könnte.".to_string();
                return Task::none();
            };
            // Der Satz liest die Dateien von der Platte: Ungespeichertes zuerst sichern
            if dateien::hat_ungespeichertes(state)
                && let Err(e) = dateien::speichern(state, dateien::SNAPSHOT_MANUELL)
            {
                state.bau.satz_meldung = format!("Setzen abgebrochen, Speichern fehlgeschlagen: {e}");
                return Task::none();
            }
            let auftrag = match satz_auftrag(state, &ordner) {
                Ok(a) => a,
                Err(f) => {
                    state.bau.satz_meldung = format!("Setzen nicht möglich: {f}");
                    return Task::none();
                }
            };
            state.bau.satz_laeuft = true;
            let schritte = werkbank_satz::buchbau::SCHRITTE + usize::from(state.einstellungen.pdfx);
            state.bau.satz_lauf = Some(ui::bauen::Lauf::neu(schritte, "Vorbereiten"));
            state.bau.satz_meldung.clear();
            state.bau.satz_hinweise.clear();
            state.bau.satz_dateien.clear();
            state.bau.satz_ziel = auftrag.buchordner.clone();
            state.aktiver_bereich = Arbeitsbereich::Bauen;
            state.speicher_zeitstempel = "Das Buch wird gesetzt …".to_string();
            let pdfx = state.einstellungen.pdfx;
            return Task::run(
                mit_schritten(move |melde| satz_ausfuehren(auftrag, pdfx, melde), Message::SatzSchritt, Message::SatzFertig),
                |m| m,
            );
        }

        Message::SatzSchritt(text) => {
            if let Some(l) = state.bau.satz_lauf.as_mut() {
                l.weiter(text);
                state.speicher_zeitstempel = format!("Setzen {}/{}: {} …", l.schritt, l.von, l.text);
            }
        }

        Message::Puls => state.bau.puls = state.bau.puls.wrapping_add(1),

        Message::SatzFertig(ergebnis) => {
            state.bau.satz_laeuft = false;
            let dauer = state.bau.satz_lauf.take().map(|l| ui::bauen::dauer(l.start)).unwrap_or_default();
            match ergebnis {
                Ok(b) => {
                    state.bau.satz_meldung =
                        format!("Fertig nach {dauer}: {} Dateien, Druckfassung mit {} Seiten.", b.dateien.len(), b.seiten_grau);
                    state.speicher_zeitstempel = format!("Buch gesetzt ({dauer})");
                    state.satz_seiten = Some(b.seiten_grau as u32);
                    state.bau.satz_hinweise = b.hinweise;
                    state.bau.satz_dateien = b.dateien;
                }
                Err(f) => {
                    state.bau.satz_meldung = format!("Satz fehlgeschlagen:\n{f}");
                    state.speicher_zeitstempel = "Setzen fehlgeschlagen".to_string();
                }
            }
        }

        Message::BauStarten(index) => {
            if state.bau.laeuft.is_some() {
                return Task::none();
            }
            let Some(befehl) = state.bau.befehle.get(index).cloned() else { return Task::none() };
            if state.band.ordner.is_none() {
                state.speicher_zeitstempel = "Das Demo-Buch hat keine Dateien und lässt sich nicht bauen.".to_string();
                return Task::none();
            }
            // Die Skripte lesen von der Platte: erst alles speichern.
            if dateien::hat_ungespeichertes(state)
                && let Err(e) = dateien::speichern(state, dateien::SNAPSHOT_MANUELL)
            {
                state.speicher_zeitstempel = format!("Bauen abgebrochen, Speichern fehlgeschlagen: {e}");
                return Task::none();
            }
            state.bau.abbruch = Default::default();
            state.bau.log = vec![format!("$ {}   (in {})", befehl.zeile(), befehl.ordner.display())];
            state.bau.laeuft = Some(index);
            state.bau.start = Some(std::time::SystemTime::now());
            state.bau.letzter_code = None;
            state.bau.dateien.clear();
            state.aktiver_bereich = Arbeitsbereich::Bauen;
            return Task::run(bau::laufen(befehl, state.bau.abbruch.clone()), |m| m);
        }

        Message::BauZeile(zeile) => {
            state.bau.log.push(zeile);
            if state.bau.log.len() > 3000 {
                state.bau.log.drain(..1000);
            }
        }

        Message::BauFertig(code) => {
            if let Some(i) = state.bau.laeuft.take()
                && let (Some(b), Some(start)) = (state.bau.befehle.get(i), state.bau.start)
            {
                state.bau.dateien = bau::ergebnisdateien(&b.ordner, start - Duration::from_secs(2));
            }
            state.bau.letzter_code = Some(code);
            state.speicher_zeitstempel = match code {
                Some(0) => "Bauen erfolgreich beendet".to_string(),
                Some(c) => format!("Bauen mit Fehlercode {c} beendet"),
                None => "Bauen abgebrochen".to_string(),
            };
        }

        Message::BauAbbrechen => {
            state.bau.abbruch.store(true, std::sync::atomic::Ordering::Relaxed);
        }

        Message::DateiOeffnen(pfad) => bau::oeffnen(&pfad),

        Message::PortaleAnlegen => match &state.band.ordner {
            Some(o) => {
                state.speicher_zeitstempel = match state.portale.speichern(o) {
                    Ok(()) => "Portalregeln in .werkbank/portale.json geschrieben – dort anpassbar".to_string(),
                    Err(e) => format!("Portalregeln nicht schreibbar: {e}"),
                };
            }
            None => state.speicher_zeitstempel = "Das Demo-Buch hat keinen Ordner.".to_string(),
        },

        Message::KapitelZeileOeffnen { kapitel_id, zeile } => {
            state.aktiver_bereich = Arbeitsbereich::Schreiben;
            kapitel_oeffnen(state, &kapitel_id);
            return update(state, Message::SpringeZuZeile { zeile, spalte: 0 });
        }

        Message::KapitelPruefenOeffnen(kapitel_id) => {
            state.aktiver_bereich = Arbeitsbereich::Schreiben;
            kapitel_oeffnen(state, &kapitel_id);
            return update(state, Message::KapitelPruefen);
        }

        Message::WoerterbuchLaden => {
            if !state.einstellungen.rechtschreibung {
                state.woerterbuch = None;
                state.woerterbuch_status = "Rechtschreibmarkierung ist ausgeschaltet.".to_string();
                return Task::none();
            }
            state.woerterbuch_status = "Wörterbuch wird geladen …".to_string();
            let (sprache, ordner) = (state.einstellungen.sprache.clone(), state.einstellungen.woerterbuch_ordner.clone());
            return Task::perform(
                languagetool::im_hintergrund(move || {
                    rechtschreibung::Woerterbuch::laden(&sprache, &ordner).map(Arc::new)
                }),
                Message::WoerterbuchGeladen,
            );
        }

        Message::WoerterbuchGeladen(ergebnis) => match ergebnis {
            Ok(wb) => {
                state.woerterbuch_status = format!("Geladen: {} aus {}", wb.name, wb.ordner.display());
                state.woerterbuch = Some(wb);
            }
            Err(f) => {
                state.woerterbuch = None;
                state.woerterbuch_status = f;
            }
        },

        Message::RechtschreibungToggle(an) => {
            state.einstellungen.rechtschreibung = an;
            state.einstellungen.speichern();
            return update(state, Message::WoerterbuchLaden);
        }

        Message::GsOrdner(pfad) => {
            werkzeuge::gs_ordner_setzen(&pfad);
            state.einstellungen.gs_ordner = pfad;
            state.einstellungen.speichern();
            return update(state, Message::WerkzeugePruefen);
        }

        Message::WoerterbuchOrdner(pfad) => {
            state.einstellungen.woerterbuch_ordner = pfad;
            state.einstellungen.speichern();
            return update(state, Message::WoerterbuchLaden);
        }

        Message::WortMerken(wort) => {
            if !state.ausnahmen.woerter.contains(&wort) {
                state.ausnahmen.woerter.push(wort);
                ausnahmen_speichern(state);
            }
        }

        Message::CoverFeld(feld, text) => {
            use ui::cover::CoverFeld::*;
            let d = &mut state.cover.daten;
            match feld {
                Titel => d.titel = text,
                Untertitel => d.untertitel = text,
                Bandzeile => d.bandzeile = text,
                Autor => d.autor = text,
                Hintergrund => d.hintergrund = text,
                Text => d.text = text,
                Akzent => d.akzent = text,
                Seiten => d.seiten = text.trim().parse().unwrap_or(0),
                Klappentext => d.klappentext = text,
                FarbeBandzeile => d.farbe_bandzeile = text,
                FarbeTitel => d.farbe_titel = text,
                FarbeUntertitel => d.farbe_untertitel = text,
                FarbeAutor => d.farbe_autor = text,
                FarbeRueckseite => d.farbe_rueckseite = text,
                FarbeRuecken => d.farbe_ruecken = text,
                RueckenKastenFarbe => d.ruecken_kasten_farbe = text,
                RueckseiteKastenFarbe => d.rueckseite_kasten_farbe = text,
            }
            cover_speichern(state);
            return cover_vorschau(state);
        }

        Message::CoverFarbenVorschlagen => {
            let d = &state.cover.daten;
            let quelle = if d.bild.is_empty() {
                state.band.ordner.as_ref().map(|o| o.join("cover.png"))
            } else {
                Some(PathBuf::from(match &state.band.ordner {
                    Some(o) => werkbank_kern::cover::bild_aufloesen(&d.bild, o),
                    None => d.bild.clone(),
                }))
            };
            let Some(bytes) = quelle.and_then(|p| std::fs::read(p).ok()) else {
                state.cover.meldung = "Kein Bild für den Farbvorschlag: Lege ein cover.png in den Bandordner oder wähle ein Hintergrundbild.".to_string();
                return Task::none();
            };
            match werkbank_satz::buchbau::farben::aus_bild(&bytes) {
                Err(f) => state.cover.meldung = format!("Farbvorschlag nicht möglich: {f}"),
                Ok(v) => {
                    state.cover.farben_zuvor = Some(Box::new(state.cover.daten.clone()));
                    let d = &mut state.cover.daten;
                    d.hintergrund = v.hintergrund.clone();
                    d.text = v.text;
                    d.akzent = v.akzent.clone();
                    d.ruecken_kasten_farbe = v.ruecken.clone();
                    d.ruecken_kasten_deckkraft = v.ruecken_deckkraft;
                    d.rueckseite_kasten_farbe = v.rueckseite_kasten;
                    d.rueckseite_kasten_deckkraft = v.rueckseite_kasten_deckkraft;
                    // Einzelfarben der Texte würden den Vorschlag übersteuern
                    for f in [&mut d.farbe_bandzeile, &mut d.farbe_titel, &mut d.farbe_untertitel, &mut d.farbe_autor, &mut d.farbe_rueckseite, &mut d.farbe_ruecken] {
                        f.clear();
                    }
                    state.cover.meldung = format!("Farben aus dem Bild übernommen: Hintergrund {}, Akzent {}, Rücken {}. „Rückgängig“ nimmt sie zurück.", v.hintergrund, v.akzent, v.ruecken);
                    cover_speichern(state);
                    return cover_vorschau(state);
                }
            }
        }

        Message::CoverFarbenZurueck => {
            if let Some(z) = state.cover.farben_zuvor.take() {
                // Nur die Farben zurücknehmen, spätere Änderungen an Titel, Schrift usw. bleiben
                let d = &mut state.cover.daten;
                d.hintergrund = z.hintergrund;
                d.text = z.text;
                d.akzent = z.akzent;
                d.ruecken_kasten_farbe = z.ruecken_kasten_farbe;
                d.ruecken_kasten_deckkraft = z.ruecken_kasten_deckkraft;
                d.rueckseite_kasten_farbe = z.rueckseite_kasten_farbe;
                d.rueckseite_kasten_deckkraft = z.rueckseite_kasten_deckkraft;
                d.farbe_bandzeile = z.farbe_bandzeile;
                d.farbe_titel = z.farbe_titel;
                d.farbe_untertitel = z.farbe_untertitel;
                d.farbe_autor = z.farbe_autor;
                d.farbe_rueckseite = z.farbe_rueckseite;
                d.farbe_ruecken = z.farbe_ruecken;
                state.cover.meldung = "Farben zurückgesetzt.".to_string();
                cover_speichern(state);
                return cover_vorschau(state);
            }
        }

        Message::CoverIsbn(text) => {
            state.projekt.metadaten.isbn_print = (!text.trim().is_empty()).then_some(text);
            if state.band.ordner.is_some() {
                state.speicher_zeitstempel = match state.projekt.band_yaml_speichern() {
                    Ok(p) => format!("ISBN gespeichert in {}", p.display()),
                    Err(e) => format!("ISBN nicht gespeichert: {e}"),
                };
            }
            return cover_vorschau(state);
        }

        Message::CoverBarcode(an) => {
            state.cover.daten.barcode = an;
            cover_speichern(state);
            return cover_vorschau(state);
        }

        Message::CoverSchriftAuto(an) => {
            state.cover.daten.rueckseite_schrift_auto = an;
            cover_speichern(state);
            return cover_vorschau(state);
        }

        Message::CoverRegler(regler, prozent) => {
            let wert = prozent as f32 / 100.0;
            let d = &mut state.cover.daten;
            match regler {
                ui::cover::CoverRegler::Abdunkeln => d.bild_abdunkeln = wert.min(0.9),
                ui::cover::CoverRegler::RueckenDeckkraft => d.ruecken_kasten_deckkraft = wert.min(1.0),
                ui::cover::CoverRegler::RueckseiteDeckkraft => d.rueckseite_kasten_deckkraft = wert.min(1.0),
                ui::cover::CoverRegler::RueckseiteSchrift => d.rueckseite_schrift_pt = prozent as f32, // Punkt, nicht Prozent
            }
            cover_speichern(state);
            return cover_vorschau(state);
        }

        Message::CoverAuswahl(feld, wert) => {
            match feld {
                ui::cover::CoverAuswahl::BildModus => state.cover.daten.bild_modus = wert,
                ui::cover::CoverAuswahl::Ausrichtung => state.cover.daten.bild_ausrichtung = wert,
            }
            cover_speichern(state);
            return cover_vorschau(state);
        }

        Message::CoverEink(an) => {
            state.cover.eink = an;
            cover_grau_neu(state);
        }

        Message::CoverUmschlag(an) => {
            state.cover.umschlag = an;
            return cover_vorschau(state);
        }

        Message::CoverPortal(name) => {
            state.cover.portal = state.portale.portale.iter().position(|p| p.name == name).unwrap_or(0);
            return cover_vorschau(state);
        }

        Message::CoverBildWaehlen => {
            return Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("Hintergrundbild für das Cover")
                        .add_filter("Bilder", &["png", "jpg", "jpeg"])
                        .pick_file()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::CoverBildGewaehlt,
            );
        }

        Message::CoverBildGewaehlt(pfad) => {
            // „Entfernen“ schickt ebenfalls `None`; ein abgebrochener Dialog entfernt dann das Bild.
            state.cover.daten.bild = pfad.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
            state.cover.bild_masse = werkbank_kern::cover::bildmasse(&state.cover.daten.bild);
            cover_speichern(state);
            return cover_vorschau(state);
        }

        Message::CoverVorschau(version, ergebnis) => {
            if version == state.cover.version {
                match ergebnis {
                    Ok(((b, h, px), bericht)) => {
                        state.cover.klappentext = bericht;
                        state.cover.vorschau = Some(iced::widget::image::Handle::from_rgba(b, h, px.as_ref().clone()));
                        state.cover.pixel = Some((b, h, px));
                        cover_grau_neu(state);
                        state.cover.meldung.clear();
                    }
                    Err(f) => state.cover.meldung = format!("Vorschau nicht möglich: {f}"),
                }
            }
        }

        Message::CoverExportieren => {
            let Some(ordner) = state.band.ordner.clone() else {
                state.cover.meldung = "Das Demo-Buch hat keinen Ordner, in den exportiert werden könnte.".to_string();
                return Task::none();
            };
            cover_speichern(state);
            state.cover.meldung = "Exportiere …".to_string();
            let daten = state.cover.daten.clone();
            let portale = state.portale.portale.clone();
            let seiten = if daten.seiten > 0 { daten.seiten } else { state.satz_seiten.unwrap_or(state.projekt.buchseiten_schaetzung() as u32) };
            let isbn = state.projekt.metadaten.isbn_print.clone();
            return Task::perform(
                languagetool::im_hintergrund(move || cover::exportieren(&daten, &portale, seiten, &ordner, isbn.as_deref())),
                Message::CoverExportiert,
            );
        }

        Message::CoverExportiert(ergebnis) => match ergebnis {
            Ok(dateien) => {
                state.cover.meldung = format!("{} Dateien in .werkbank/cover/ geschrieben:", dateien.len());
                state.cover.dateien = dateien;
            }
            Err(f) => state.cover.meldung = format!("Export fehlgeschlagen: {f}"),
        },

        Message::KuerzelAufnehmen(id) => {
            state.kuerzel_aufnahme = Some(id);
        }

        Message::KuerzelZuruecksetzen(id) => {
            state.einstellungen.tasten.remove(&id);
            state.einstellungen.speichern();
        }

        Message::KuerzelLeeren(id) => {
            state.einstellungen.tasten.insert(id, String::new());
            state.einstellungen.speichern();
        }

        Message::Nichts => {}

        Message::UeberOeffnen => {
            state.palette = None;
            state.ueber_offen = true;
        }

        Message::SpendenOeffnen => {
            bau::oeffnen(Path::new(SPENDEN_URL));
        }

        Message::HandbuchOeffnen => {
            state.palette = None;
            state.handbuch_offen = true;
        }

        Message::SchriftartSetzen(art) => {
            state.einstellungen.schriftart = art;
            state.einstellungen.darstellung_anwenden();
            state.einstellungen.speichern();
            state.edit_tick = state.edit_tick.wrapping_add(1);
        }

        Message::ZeilenabstandSetzen(f) => {
            state.einstellungen.zeilenabstand = f;
            state.einstellungen.darstellung_anwenden();
            state.einstellungen.speichern();
        }

        Message::SchreibmaschineToggle(an) => {
            state.einstellungen.schreibmaschine = an;
            state.einstellungen.darstellung_anwenden();
            state.einstellungen.speichern();
        }

        Message::HellToggle(an) => {
            state.einstellungen.hell = an;
            state.einstellungen.darstellung_anwenden();
            state.einstellungen.speichern();
            state.edit_tick = state.edit_tick.wrapping_add(1);
        }

        Message::SchriftSetzen(pt) => {
            state.einstellungen.schriftgroesse = pt;
            state.einstellungen.darstellung_anwenden();
            state.einstellungen.speichern();
        }

        Message::SpalteSetzen(px) => {
            state.einstellungen.spaltenbreite = px;
            state.einstellungen.darstellung_anwenden();
            state.einstellungen.speichern();
        }

        Message::MetaOeffnen => {
            state.palette = None;
            state.meta_entwurf = Some(state.projekt.metadaten.clone());
        }

        Message::MetaAendern(feld, text) => {
            use ui::dialog::MetaFeld::*;
            if let Some(md) = state.meta_entwurf.as_mut() {
                match feld {
                    Titel => md.titel = text,
                    Untertitel => md.untertitel = text,
                    Band => md.band_bezeichnung = text,
                    Autor => md.autor = text,
                    Jahr => md.jahr = text,
                    Isbn => md.isbn_print = (!text.trim().is_empty()).then_some(text),
                }
            }
        }

        Message::MetaSpeichern => {
            if let Some(md) = state.meta_entwurf.take() {
                state.projekt.metadaten = md;
                state.speicher_zeitstempel = if state.band.ordner.is_none() {
                    "Buchdaten übernommen (Demo-Buch, nicht gespeichert)".to_string()
                } else {
                    match state.projekt.band_yaml_speichern() {
                        Ok(p) => format!("Buchdaten gespeichert in {}", p.display()),
                        Err(e) => format!("Buchdaten nicht gespeichert: {e}"),
                    }
                };
            }
        }

        Message::SucheOeffnen(im_buch) => {
            state.palette = None;
            let auswahl = state.editor_content.selection().filter(|t| !t.is_empty() && !t.contains('\n'));
            let s = state.suche.get_or_insert_with(Suche::default);
            s.umfang = if im_buch { ui::suche::Umfang::Buch } else { ui::suche::Umfang::Kapitel };
            if let Some(t) = auswahl {
                s.muster = t;
            }
            state.suche_offen = true;
            suche_aktualisieren(state);
            return iced::widget::operation::focus(ui::suche::feld_id());
        }

        Message::SucheMuster(text) => {
            if let Some(s) = state.suche.as_mut() {
                s.muster = text;
                s.auswahl = 0;
            }
            suche_aktualisieren(state);
        }

        Message::SucheErsatz(text) => {
            if let Some(s) = state.suche.as_mut() {
                s.ersatz = text;
            }
            suche_aktualisieren(state);
        }

        Message::SucheGrossKlein(an) => {
            if let Some(s) = state.suche.as_mut() {
                s.gross_klein = an;
            }
            suche_aktualisieren(state);
        }

        Message::SucheUmfang(u) => {
            if let Some(s) = state.suche.as_mut() {
                s.umfang = u;
            }
            suche_aktualisieren(state);
        }

        Message::SucheGanzesWort(an) => {
            if let Some(s) = state.suche.as_mut() {
                s.ganzes_wort = an;
            }
            suche_aktualisieren(state);
        }

        Message::SucheRegex(an) => {
            if let Some(s) = state.suche.as_mut() {
                s.regex = an;
            }
            suche_aktualisieren(state);
        }

        Message::SucheBestaetigungAus => {
            if let Some(s) = state.suche.as_mut() {
                s.bestaetigen = false;
            }
        }

        Message::SucheSpringe(index) => {
            if let Some(s) = state.suche.as_mut() {
                s.auswahl = index;
            }
            state.suche_offen = false;
            return zu_fund_springen(state, index);
        }

        Message::SucheNaechste(schritt) => {
            suche_aktualisieren(state);
            let Some(s) = state.suche.as_mut() else { return Task::none() };
            if s.anzahl() == 0 {
                state.speicher_zeitstempel = "Keine Treffer".to_string();
                return Task::none();
            }
            let n = s.anzahl() as i32;
            s.auswahl = ((s.auswahl as i32 + schritt).rem_euclid(n)) as usize;
            let index = s.auswahl;
            state.speicher_zeitstempel = format!("Treffer {} von {}", index + 1, n);
            return zu_fund_springen(state, index);
        }

        Message::SucheErsetzenEins => {
            suche_aktualisieren(state);
            let Some(s) = state.suche.as_ref() else { return Task::none() };
            let Some(fund) = s.funde.get(s.auswahl).cloned() else { return Task::none() };
            if let Some(teil) = werkbank_kern::suche::fund_ersetzen(&mut state.projekt, &fund) {
                state.dirty.insert(teil);
                state.letzte_aenderung = Some(Instant::now());
                if fund.kapitel_id == state.aktives_kapitel_id {
                    dateien::editor_neu_laden(state);
                }
                suche_aktualisieren(state);
            }
        }

        Message::SucheErsetzenAlle => {
            suche_aktualisieren(state);
            let Some(s) = state.suche.as_mut() else { return Task::none() };
            if s.anzahl() == 0 || s.fehler.is_some() {
                return Task::none();
            }
            // Erster Klick: nur die Anzahl nennen, zweiter Klick führt aus
            if !s.bestaetigen {
                s.bestaetigen = true;
                return Task::none();
            }
            s.bestaetigen = false;
            let (muster, ersatz, opt, umfang) = (s.muster.clone(), s.ersatz.clone(), s.optionen(), s.umfang);
            let hat_andere = !s.andere.is_empty();
            let andere_baende: Vec<(String, PathBuf)> = {
                let mut v: Vec<(String, PathBuf)> = Vec::new();
                for b in &s.andere {
                    if !v.iter().any(|(_, o)| *o == b.ordner) {
                        v.push((b.band.clone(), b.ordner.clone()));
                    }
                }
                v
            };
            let nur = (umfang == ui::suche::Umfang::Kapitel).then(|| state.aktives_kapitel_id.clone());
            let (anzahl, teile, kapitel) = match werkbank_kern::suche::alle_ersetzen(&mut state.projekt, &muster, &ersatz, opt, nur.as_deref()) {
                Ok(r) => r,
                Err(f) => {
                    state.speicher_zeitstempel = f;
                    return Task::none();
                }
            };
            state.dirty.extend(teile);
            state.snapshot_zwang = true;
            state.letzte_aenderung = Some(Instant::now());
            if kapitel.contains(&state.aktives_kapitel_id) {
                dateien::editor_neu_laden(state);
            }
            let mut meldung = format!("{anzahl} Ersetzungen im geöffneten Band (ungespeichert, die alte Fassung bleibt beim Speichern als Sicherung erhalten)");
            if umfang == ui::suche::Umfang::Buch && hat_andere {
                match werkbank_kern::suche::in_baenden_ersetzen(&andere_baende, &muster, &ersatz, opt) {
                    Ok((n, dateien_n)) => meldung = format!("{} Ersetzungen: {anzahl} im geöffneten Band (ungespeichert), {n} in {} weiteren Bänden ({dateien_n} Dateien, gespeichert mit nummerierter Sicherung)", anzahl + n, andere_baende.len()),
                    Err(f) => meldung = format!("{meldung}. Die anderen Bände ließen sich nicht ersetzen: {f}"),
                }
            }
            suche_aktualisieren(state);
            state.speicher_zeitstempel = meldung;
        }

        Message::NotizenAktion(action) => {
            let war_edit = action.is_edit();
            state.notizen_content.perform(action);
            if war_edit {
                let text = state.notizen_content.text();
                if let Some(k) = state.projekt.kapitel_finden_mut(&state.aktives_kapitel_id) {
                    k.notizen = text;
                    state.meta_dirty = true;
                    state.letzte_aenderung = Some(Instant::now());
                }
            }
        }

        Message::VerweisEinfuegen(id) => {
            let paste = text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(format!("@kap:{id}"))));
            let _ = update(state, Message::EditorAktion(paste));
            return iced::widget::operation::focus(ui::schreiben::editor_id());
        }

        Message::EinstellungenOeffnen => {
            state.palette = None;
            state.einstellungen_fenster = match state.einstellungen_fenster {
                Some(_) => None,
                None => Some(ui::einstellungen::Fenster::default()),
            };
        }

        Message::AnleitungOeffnen => {
            state.palette = None;
            state.einstellungen_fenster = Some(ui::einstellungen::Fenster {
                info_offen: true,
                seite: 2,
                ..Default::default()
            });
        }

        Message::KontextOeffnen(ziel, position) => {
            let auswahl = match &ziel {
                ui::kontext::Ziel::Kapitel(id) => update(state, Message::KapitelAusgewaehlt(id.clone())),
                ui::kontext::Ziel::Teil(id) => update(state, Message::TeilAusgewaehlt(id.clone())),
                ui::kontext::Ziel::Wort(_) => Task::none(),
            };
            state.kontext = Some(ui::kontext::Kontext { ziel, position });
            return auswahl;
        }

        Message::KontextAktion(aktion) => {
            state.kontext = None;
            return update(state, *aktion);
        }

        Message::KontextSchliessen => state.kontext = None,

        Message::MenueUmschalten(m) => {
            state.kontext = None;
            state.menue = if state.menue == Some(m) { None } else { Some(m) };
        }

        Message::MenueUeber(m) => {
            if state.menue.is_some() {
                state.menue = Some(m);
            }
        }

        Message::MenueSchliessen => state.menue = None,

        Message::MenueAktion(aktion) => {
            state.menue = None;
            return update(state, *aktion);
        }

        Message::WellenMenue(klick) => {
            let rechtschreibung = klick.art == markierung::WELLE_RECHTSCHREIBUNG;
            let (vorschlaege, treffer, meldung, tippfehler) = if rechtschreibung {
                let v = state.woerterbuch.as_ref().map(|w| w.vorschlaege(&klick.wort)).unwrap_or_default();
                (v.into_iter().take(5).collect(), None, String::new(), false)
            } else {
                let pos = (klick.zeile, klick.bereich.start);
                let Some(i) = state.treffer.iter().position(|t| {
                    (t.zeile, t.spalte) <= pos && pos < (t.ende_zeile, t.ende_spalte.max(t.spalte + 1))
                }) else {
                    return Task::none();
                };
                let t = &state.treffer[i];
                (t.ersetzungen.iter().take(5).cloned().collect(), Some(i), t.meldung.clone(), t.kategorie_id == "TYPOS")
            };
            state.kontext = Some(ui::kontext::Kontext {
                ziel: ui::kontext::Ziel::Wort(ui::kontext::WortZiel {
                    zeile: klick.zeile,
                    von: klick.bereich.start,
                    bis: klick.bereich.end,
                    wort: klick.wort,
                    rechtschreibung,
                    vorschlaege,
                    treffer,
                    meldung,
                    tippfehler,
                }),
                position: klick.position,
            });
        }

        Message::WortErsetzen { zeile, von, bis, ersatz } => {
            // Hinweise dahinter in derselben Zeile rücken um die Längenänderung nach
            let delta = ersatz.len() as isize - (bis - von) as isize;
            let schieben = |wert: usize| (wert as isize + delta).max(0) as usize;
            for t in state.treffer.iter_mut() {
                if t.zeile == zeile && t.spalte >= bis {
                    t.spalte = schieben(t.spalte);
                }
                if t.ende_zeile == zeile && t.ende_spalte >= bis {
                    t.ende_spalte = schieben(t.ende_spalte);
                }
            }
            state.editor_content.move_to(text_editor::Cursor {
                position: text_editor::Position { line: zeile, column: bis },
                selection: Some(text_editor::Position { line: zeile, column: von }),
            });
            let paste = text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(ersatz)));
            return update(state, Message::EditorAktion(paste));
        }

        Message::WortIgnorieren(wort) => {
            state.ignoriert.insert(wort.clone());
            state.speicher_zeitstempel = format!("„{wort}“ wird bis zum Neustart nicht markiert");
        }

        Message::TrefferIgnorieren(index) => {
            if index < state.treffer.len() {
                state.treffer.remove(index);
                state.pruef_status = PruefStatus::Fertig(state.treffer.len());
            }
        }

        Message::FensterGroesse(groesse) => state.fenster_groesse = groesse,

        Message::EinstellungenSeite(seite) => {
            state.palette = None;
            let f = state.einstellungen_fenster.get_or_insert_with(ui::einstellungen::Fenster::default);
            f.seite = seite;
        }

        Message::EinstellungenSchliessen => {
            state.einstellungen_fenster = None;
        }

        Message::EinstellungenInfoToggle => {
            if let Some(f) = state.einstellungen_fenster.as_mut() {
                f.info_offen = !f.info_offen;
            }
        }

        Message::EinstellungenSystem(system) => {
            if let Some(f) = state.einstellungen_fenster.as_mut() {
                f.system = system;
            }
        }

        Message::LtAktiv(an) => {
            state.einstellungen.sprachpruefung = an;
            state.einstellungen.speichern();
            if an {
                return update(state, Message::LtVerbindungTesten);
            }
            state.treffer.clear();
            state.pruef_status = PruefStatus::Leer;
            state.pruef_faellig = None;
        }

        Message::LtAutomatisch(an) => {
            state.einstellungen.automatisch = an;
            state.einstellungen.speichern();
            if !an {
                state.pruef_faellig = None;
            }
        }

        Message::LtAutostart(an) => {
            state.einstellungen.server_autostart = an;
            state.einstellungen.speichern();
        }

        Message::LtSprache(sprache) => {
            state.einstellungen.sprache = sprache;
            state.einstellungen.speichern();
            state.treffer.clear();
            state.pruef_status = PruefStatus::Leer;
            return update(state, Message::WoerterbuchLaden);
        }

        Message::LtUrl(url) => {
            state.einstellungen.server_url = url;
            state.einstellungen.speichern();
            state.server_status = ServerStatus::Unbekannt;
        }

        Message::LtOrdner(pfad) => {
            state.einstellungen.lt_ordner = pfad;
            state.einstellungen.speichern();
        }

        Message::LtJava(java) => {
            state.einstellungen.java = java;
            state.einstellungen.speichern();
        }

        Message::LtOrdnerWaehlen => {
            return Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .set_title("LanguageTool-Ordner wählen (mit languagetool-server.jar)")
                        .pick_folder()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::LtOrdnerGewaehlt,
            );
        }

        Message::LtOrdnerGewaehlt(ordner) => {
            if let Some(ordner) = ordner {
                state.einstellungen.lt_ordner = ordner.to_string_lossy().into_owned();
                state.einstellungen.speichern();
            }
        }

        Message::LtServerStarten => {
            if state.server.as_mut().is_some_and(|s| s.laeuft()) {
                return Task::none();
            }
            let ordner = PathBuf::from(state.einstellungen.lt_ordner.trim());
            match ServerProzess::starten(&state.einstellungen.java, &ordner, state.einstellungen.port()) {
                Ok(prozess) => {
                    state.server = Some(prozess);
                    state.server_status = ServerStatus::Startet;
                    return start_versuch(state.einstellungen.server_url.clone(), 0);
                }
                Err(fehler) => state.server_status = ServerStatus::Fehler(fehler),
            }
        }

        Message::LtServerStoppen => {
            state.server = None;
            state.server_status = ServerStatus::Unbekannt;
        }

        Message::LtVerbindungTesten => {
            state.server_status = ServerStatus::Pruefe;
            let url = state.einstellungen.server_url.clone();
            return Task::perform(
                languagetool::im_hintergrund(move || languagetool::verbindung_testen(&url)),
                Message::LtVerbindung,
            );
        }

        Message::LtVerbindung(ergebnis) => {
            // Ein Test, der während des Hochfahrens fehlschlägt, soll den Startstatus nicht überschreiben.
            if state.server_status == ServerStatus::Startet && ergebnis.is_err() {
                return Task::none();
            }
            state.server_status = match ergebnis {
                Ok(info) => ServerStatus::Erreichbar(info),
                Err(fehler) => ServerStatus::Fehler(fehler),
            };
            if auto_pruefung(state) && state.pruef_status == PruefStatus::Leer {
                state.pruef_faellig = Some(Instant::now());
            }
        }

        Message::LtStartVersuch(versuch, ergebnis) => {
            if state.server_status != ServerStatus::Startet {
                return Task::none();
            }
            match ergebnis {
                Ok(info) => {
                    state.server_status = ServerStatus::Erreichbar(info);
                    if auto_pruefung(state) {
                        state.pruef_faellig = Some(Instant::now());
                    }
                }
                Err(fehler) => {
                    let lebt = state.server.as_mut().is_some_and(|s| s.laeuft());
                    if !lebt {
                        state.server = None;
                        state.server_status = ServerStatus::Fehler(
                            "Der Server wurde sofort beendet – ist Java 17 oder neuer installiert?".to_string(),
                        );
                    } else if versuch >= 25 {
                        state.server_status = ServerStatus::Fehler(format!(
                            "Der Server antwortet nach 50 s noch nicht ({fehler})"
                        ));
                    } else {
                        return start_versuch(state.einstellungen.server_url.clone(), versuch + 1);
                    }
                }
            }
        }

        Message::KapitelPruefen => {
            if !state.einstellungen.sprachpruefung {
                state.speicher_zeitstempel = "Sprachprüfung ist in den Einstellungen ausgeschaltet".to_string();
                return Task::none();
            }
            if state.pruef_status == PruefStatus::Laeuft {
                return Task::none();
            }
            state.pruef_status = PruefStatus::Laeuft;
            state.pruef_faellig = None;
            let text = state.editor_content.text();
            let url = state.einstellungen.server_url.clone();
            let sprache = state.einstellungen.sprache.clone();
            let kapitel_id = state.aktives_kapitel_id.clone();
            return Task::perform(
                languagetool::im_hintergrund(move || languagetool::pruefen(&url, &sprache, &text)),
                move |ergebnis| Message::PruefErgebnis {
                    kapitel_id: kapitel_id.clone(),
                    ergebnis,
                },
            );
        }

        Message::PruefErgebnis { kapitel_id, ergebnis } => {
            if kapitel_id != state.aktives_kapitel_id {
                return Task::none();
            }
            match ergebnis {
                Ok(treffer) => {
                    state.treffer = state.ausnahmen.filtern(treffer);
                    state.pruef_status = PruefStatus::Fertig(state.treffer.len());
                    if !matches!(state.server_status, ServerStatus::Erreichbar(_)) {
                        state.server_status = ServerStatus::Erreichbar("zuletzt erfolgreich geprüft".to_string());
                    }
                    state.speicher_zeitstempel = match state.treffer.len() {
                        0 => "Sprachprüfung: keine Hinweise".to_string(),
                        n => format!("Sprachprüfung: {n} Hinweise"),
                    };
                }
                Err(fehler) => {
                    state.treffer.clear();
                    state.server_status = ServerStatus::Fehler(fehler.clone());
                    state.pruef_status = PruefStatus::Fehler(fehler);
                }
            }
        }

        Message::Takt(jetzt) => {
            if let Some(seit) = state.vorschau_faellig
                && state.vorschau_sichtbar
                && !state.satz_vorschau.text_modus
                && !state.satz_vorschau.laeuft
                && jetzt.saturating_duration_since(seit) >= Duration::from_millis(900)
            {
                return update(state, Message::VorschauAktualisieren);
            }
            // Automatisches Speichern nach einer Schreibpause
            if state.einstellungen.autospeichern
                && dateien::hat_ungespeichertes(state)
                && state.letzte_aenderung.is_some_and(|t| jetzt.saturating_duration_since(t) >= Duration::from_secs(8))
            {
                state.letzte_aenderung = Some(jetzt); // bei Fehlern nicht im Sekundentakt wiederholen
                match dateien::speichern(state, dateien::SNAPSHOT_AUTO) {
                    Ok(0) => {}
                    Ok(_) => state.speicher_zeitstempel = format!("Automatisch gespeichert um {}", Local::now().format("%H:%M:%S")),
                    Err(e) => state.speicher_zeitstempel = format!("Automatisches Speichern fehlgeschlagen: {e}"),
                }
            }
            if let Some(seit) = state.pruef_faellig
                && jetzt.saturating_duration_since(seit) >= Duration::from_millis(1500)
                && state.pruef_status != PruefStatus::Laeuft
            {
                return update(state, Message::KapitelPruefen);
            }
        }

        Message::TrefferAnspringen(index) => {
            if let Some(t) = treffer_holen(state, index) {
                state.editor_content.move_to(text_editor::Cursor {
                    position: text_editor::Position {
                        line: t.ende_zeile,
                        column: t.ende_spalte,
                    },
                    selection: Some(text_editor::Position {
                        line: t.zeile,
                        column: t.spalte,
                    }),
                });
                return iced::widget::operation::focus(ui::schreiben::editor_id());
            }
        }

        Message::TrefferErsetzen(index, vorschlag) => {
            let Some(t) = treffer_holen(state, index) else {
                return Task::none();
            };
            let Some(ersatz) = t.ersetzungen.get(vorschlag).cloned() else {
                return Task::none();
            };

            // Die übrigen Hinweise hinter der Stelle rücken um die Längenänderung nach.
            state.treffer.remove(index);
            if ersatz.contains('\n') || t.zeile != t.ende_zeile {
                state.treffer.clear();
                state.pruef_status = PruefStatus::Leer;
            } else {
                let delta = ersatz.len() as isize - (t.bis - t.von) as isize;
                let schieben = |wert: usize| (wert as isize + delta).max(0) as usize;
                for rest in state.treffer.iter_mut().filter(|r| r.von >= t.bis) {
                    rest.von = schieben(rest.von);
                    rest.bis = schieben(rest.bis);
                    if rest.zeile == t.ende_zeile {
                        rest.spalte = schieben(rest.spalte);
                    }
                    if rest.ende_zeile == t.ende_zeile {
                        rest.ende_spalte = schieben(rest.ende_spalte);
                    }
                }
                state.pruef_status = PruefStatus::Fertig(state.treffer.len());
            }

            state.editor_content.move_to(text_editor::Cursor {
                position: text_editor::Position {
                    line: t.ende_zeile,
                    column: t.ende_spalte,
                },
                selection: Some(text_editor::Position {
                    line: t.zeile,
                    column: t.spalte,
                }),
            });
            let paste = text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(ersatz)));
            return update(state, Message::EditorAktion(paste));
        }

        Message::TrefferWortMerken(index) => {
            if let Some(t) = state.treffer.get(index).cloned() {
                if !state.ausnahmen.woerter.contains(&t.fundstelle) {
                    state.ausnahmen.woerter.push(t.fundstelle.clone());
                    ausnahmen_speichern(state);
                }
                state
                    .treffer
                    .retain(|r| !(r.kategorie_id == "TYPOS" && r.fundstelle == t.fundstelle));
                state.pruef_status = PruefStatus::Fertig(state.treffer.len());
            }
        }

        Message::TrefferRegelAus(index) => {
            if let Some(t) = state.treffer.get(index).cloned() {
                if !state.ausnahmen.regeln.contains(&t.regel_id) {
                    state.ausnahmen.regeln.push(t.regel_id.clone());
                    ausnahmen_speichern(state);
                }
                state.treffer.retain(|r| r.regel_id != t.regel_id);
                state.pruef_status = PruefStatus::Fertig(state.treffer.len());
            }
        }

        Message::AusnahmeRegelEntfernen(regel) => {
            state.ausnahmen.regeln.retain(|r| *r != regel);
            ausnahmen_speichern(state);
        }

        Message::AusnahmeWortEntfernen(wort) => {
            state.ausnahmen.woerter.retain(|w| *w != wort);
            ausnahmen_speichern(state);
        }

        Message::Kopieren(text) => {
            return iced::clipboard::write(text);
        }

        Message::PaletteToggle => {
            if state.palette.is_some() {
                state.palette = None;
            } else {
                state.palette = Some(Palette::default());
                return iced::widget::operation::focus(ui::palette::feld_id());
            }
        }

        Message::PaletteSchliessen => {
            state.palette = None;
        }

        Message::PaletteSuche(text) => {
            if let Some(p) = state.palette.as_mut() {
                p.suche = text;
                p.auswahl = 0;
            }
        }

        Message::PaletteBewegen(schritt) => {
            let anzahl = palette_befehle(state).len();
            if let Some(p) = state.palette.as_mut()
                && anzahl > 0
            {
                p.auswahl = (p.auswahl as i32 + schritt).clamp(0, anzahl as i32 - 1) as usize;
            }
        }

        Message::PaletteAusfuehren => {
            let auswahl = state.palette.as_ref().map_or(0, |p| p.auswahl);
            return update(state, Message::PaletteWaehlen(auswahl));
        }

        Message::PaletteWaehlen(index) => {
            let befehl = palette_befehle(state).into_iter().nth(index);
            state.palette = None;
            if let Some(befehl) = befehl {
                return update(state, befehl.aktion);
            }
        }

        Message::TastaturEreignis(keyboard::Event::KeyPressed {
            key,
            physical_key,
            modifiers,
            ..
        }) => {
            // Aufnahme eines neuen Kürzels: der nächste Tastendruck wird zugewiesen, Esc bricht ab.
            if let Some(id) = state.kuerzel_aufnahme.clone() {
                if key == Key::Named(Named::Escape) {
                    state.kuerzel_aufnahme = None;
                } else if let Some(neu) = kuerzel::aus_ereignis(&key, &physical_key, &modifiers) {
                    if matches!(key, Key::Named(Named::Tab | Named::F6 | Named::Enter)) {
                        state.speicher_zeitstempel = "Tab, F6 und Enter sind fest belegt.".to_string();
                        return Task::none();
                    }
                    // Hatte ein anderer Befehl dieses Kürzel, verliert er es.
                    let anderer: Vec<&str> = kuerzel::BEFEHLE
                        .iter()
                        .map(|b| b.0)
                        .filter(|b| *b != id && kuerzel::kuerzel_von(&state.einstellungen, b) == neu)
                        .collect();
                    for b in &anderer {
                        state.einstellungen.tasten.insert((*b).to_string(), String::new());
                    }
                    state.einstellungen.tasten.insert(id, neu.clone());
                    state.einstellungen.speichern();
                    state.kuerzel_aufnahme = None;
                    state.speicher_zeitstempel = format!("Kürzel {} gesetzt", kuerzel::anzeige(&neu));
                }
                return Task::none();
            }

            if key == Key::Named(Named::Escape) && state.menue.is_some() {
                state.menue = None;
                return Task::none();
            }
            if key == Key::Named(Named::Escape) && state.kontext.is_some() {
                state.kontext = None;
                return Task::none();
            }
            if key == Key::Named(Named::Escape) {
                if state.dialog.is_some() || state.eingabe.is_some() || state.verlauf.is_some() || state.suche_offen || state.meta_entwurf.is_some() || state.handbuch_offen || state.ueber_offen {
                    return update(state, Message::DialogSchliessen);
                }
                if state.einstellungen_fenster.is_some() {
                    return update(state, Message::EinstellungenSchliessen);
                }
            }

            // Tab (in Fenstern) und F6 (überall) wandern über Knöpfe, Schalter, Regler und Felder.
            let fenster_offen = state.einstellungen_fenster.is_some()
                || state.dialog.is_some()
                || state.eingabe.is_some()
                || state.verlauf.is_some()
                || state.suche_offen
                || state.meta_entwurf.is_some()
                || state.handbuch_offen
                || state.ueber_offen
                || state.palette.is_some();
            if (fenster_offen && key == Key::Named(Named::Tab)) || key == Key::Named(Named::F6) {
                return if modifiers.shift() {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
            }

            if state.palette.is_some() {
                match key {
                    Key::Named(Named::Escape) => return update(state, Message::PaletteSchliessen),
                    Key::Named(Named::ArrowDown) => {
                        return update(state, Message::PaletteBewegen(1));
                    }
                    Key::Named(Named::ArrowUp) => {
                        return update(state, Message::PaletteBewegen(-1));
                    }
                    _ => {}
                }
            }

            if let Some(taste) = kuerzel::aus_ereignis(&key, &physical_key, &modifiers)
                && let Some(id) = kuerzel::belegung(&state.einstellungen).get(&taste).copied()
                && let Some(nachricht) = befehl_nachricht(id, state)
            {
                state.menue = None;
                return update(state, nachricht);
            }

            match key {
                // Esc verlässt den Fokus-Modus
                Key::Named(Named::Escape) if state.fokus_modus => {
                    return update(state, Message::FokusToggle);
                }
                _ => {}
            }
        }

        Message::TastaturEreignis(_) => {}
    }

    Task::none()
}

/// Speichert die Gestaltung im Bandordner.
fn stil_sichern(state: &mut AppState) {
    if let Some(o) = state.band.ordner.clone()
        && let Err(f) = state.bau.stil.speichern(&o)
    {
        state.bau.satz_meldung = format!("Gestaltung nicht gespeichert: {f}");
    }
}

/// Setzt die Musterseite der Gestaltung neu (im Hintergrund).
fn muster_setzen(state: &mut AppState) -> Task<Message> {
    state.bau.muster_version += 1;
    let version = state.bau.muster_version;
    let (typ, dateien) = state.bau.stil.muster(state.bau.muster_farbig);
    Task::perform(
        languagetool::im_hintergrund(move || werkbank_satz::typst_zu_seiten_mit(typ, None, dateien, 1, 1.3).map(|(s, _, _)| Arc::new(s))),
        move |e| Message::SatzMusterFertig(version, e),
    )
}

/// Alles, was der Satz eines Bandes braucht (im Hintergrund ausführbar).
struct SatzAuftrag {
    buchordner: PathBuf,
    bandordner: PathBuf,
    profil: werkbank_satz::buchbau::profil::Profil,
    band: werkbank_satz::buchbau::profil::Band,
    stil: werkbank_satz::gestaltung::Satzstil,
    ersatz_cover: Option<Vec<u8>>,
    isbn: Option<String>,
}

/// Stabile Kennung `urn:uuid:…` aus einem Text (gleiches Buch → gleiche Kennung für das E-Book).
fn uuid_aus(text: &str) -> String {
    use std::hash::{Hash, Hasher};
    let teil = |salz: u8| {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (salz, text).hash(&mut h);
        h.finish()
    };
    let (a, b) = (teil(1), teil(2));
    format!(
        "urn:uuid:{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
        a >> 32,
        (a >> 16) & 0xffff,
        a & 0xfff,
        0x8000 | ((b >> 48) & 0x3fff),
        b & 0xffff_ffff_ffff
    )
}

/// Profil und Band für den Satz: aus `buchbau.yaml`, wenn der Band dazugehört, sonst aus den
/// Angaben des Bandes (Titel, Autor, Dateien, Format des ersten Portals) – dann landet alles in `.werkbank/satz/`.
fn satz_auftrag(state: &AppState, ordner: &Path) -> Result<SatzAuftrag, String> {
    use werkbank_satz::buchbau::profil::{Band, Profil, Raender};
    let (buchordner, bandordner, profil, band) = match &state.bau.buchbau {
        Some((buch, nummer, _)) => {
            let profil = Profil::laden(buch)?;
            let band = profil.band(*nummer).cloned().ok_or_else(|| format!("Band {nummer} fehlt in buchbau.yaml"))?;
            (buch.clone(), buch.join(&band.ordner), profil, band)
        }
        None => {
            let m = &state.projekt.metadaten;
            let mut quellen: Vec<String> = Vec::new();
            for t in &state.projekt.teile {
                let d = t.dateipfad.to_string_lossy().replace('\\', "/");
                if !quellen.contains(&d) {
                    quellen.push(d);
                }
            }
            let format = state.portale.portale.first().map_or([170.0, 240.0], |p| [p.druck_format_mm.0, p.druck_format_mm.1]);
            let untertitel = [m.untertitel.as_str(), m.band_bezeichnung.as_str()].iter().find(|t| !t.trim().is_empty()).map_or(String::new(), |t| t.to_string());
            let kurzname = werkbank_kern::speicher::kennung_aus_titel(&m.titel);
            let band = Band {
                nummer: 1,
                ordner: ".".into(),
                untertitel,
                uuid: uuid_aus(&format!("{}|{}|{}", m.titel, m.band_bezeichnung, m.autor)),
                kurzname: if kurzname.is_empty() { "buch".into() } else { kurzname },
                umschlag: Default::default(),
            };
            let profil = Profil {
                titel: m.titel.clone(),
                autor: m.autor.clone(),
                sprache: "de".into(),
                verlag: m.autor.clone(),
                format_mm: format,
                raender_mm: Raender { innen: 22.0, aussen: 18.0, oben: 20.0, unten: 22.0 },
                quellen,
                rubriken: Vec::new(),
                ausgabe: ".".into(),
                webseiten: Vec::new(),
                zeilennummern: false,
                baende: vec![band.clone()],
                reihe: Default::default(),
            };
            (ordner.join(".werkbank").join("satz"), ordner.to_path_buf(), profil, band)
        }
    };
    // Ohne cover.png im Bandordner: Vorderseite aus dem Bereich „Cover“
    let ersatz_cover = if bandordner.join("cover.png").is_file() {
        None
    } else {
        let seiten = state.satz_seiten.unwrap_or(state.projekt.buchseiten_schaetzung() as u32);
        let svg = werkbank_kern::cover::svg_ebook(&state.cover.daten, 1600.0, 2560.0, state.portale.portale.first().map(|p| (p, seiten)));
        cover::rendern(&svg, 1600).ok().and_then(|(b, h, px)| {
            let pixmap = resvg::tiny_skia::Pixmap::from_vec(px, resvg::tiny_skia::IntSize::from_wh(b, h)?)?;
            pixmap.encode_png().ok()
        })
    };
    let stil = werkbank_satz::gestaltung::Satzstil::laden(ordner);
    let isbn = state.projekt.metadaten.isbn_print.clone().filter(|i| !i.trim().is_empty());
    Ok(SatzAuftrag { buchordner, bandordner, profil, band, stil, ersatz_cover, isbn })
}

/// Setzt den Band (PDFs, EPUB, Verlagsordner); auf Wunsch zusätzlich PDF/X-3 des Graudrucks.
fn satz_ausfuehren(a: SatzAuftrag, pdfx: bool, melde: &dyn Fn(String)) -> Result<werkbank_satz::buchbau::Bericht, String> {
    let auftrag = werkbank_satz::buchbau::Auftrag {
        buchordner: &a.buchordner,
        bandordner: &a.bandordner,
        profil: &a.profil,
        band: &a.band,
        stil: &a.stil,
        ersatz_cover: a.ersatz_cover.clone(),
        isbn: a.isbn.clone(),
    };
    let mut bericht = werkbank_satz::buchbau::setzen(&auftrag, &|s| melde(s.to_string()))?;
    if pdfx {
        melde("PDF/X-3 des Graudrucks (Ghostscript)".to_string());
        let ausgabe = a.buchordner.join(&a.profil.ausgabe);
        let grau = ausgabe.join(format!("{}-print-sw.pdf", a.band.kurzname));
        let ziel = ausgabe.join(format!("{}-print-sw-pdfx.pdf", a.band.kurzname));
        match werkzeuge::pdfx_erzeugen(&grau, &ziel, &a.profil.titel) {
            Ok(()) => {
                let groesse = std::fs::metadata(&ziel).map(|m| m.len()).unwrap_or(0);
                bericht.dateien.push((ziel, groesse));
            }
            Err(f) => bericht.hinweise.push(format!("PDF/X-3 nicht erzeugt: {f}")),
        }
    }
    Ok(bericht)
}

/// Homepage der Werkbank (Hilfe-Menü, „Über die Werkbank“).
pub const HOMEPAGE: &str = "https://werkbank.jurzel-apps.de";
/// Kontaktadresse für Fragen und Fehlermeldungen.
pub const KONTAKT: &str = "mailto:kontakt@jurzel.de?subject=Werkbank";

/// Spendenseite „Buy me a coffee“.
pub const SPENDEN_URL: &str = "https://buymeacoffee.com/services5";

/// Nachricht zu einer Befehl-Kennung der Kürzel-Tabelle.
fn befehl_nachricht(id: &str, state: &AppState) -> Option<Message> {
    Some(match id {
        "palette" => Message::PaletteToggle,
        "speichern" => Message::Speichern,
        "ordner" => Message::OrdnerDialog,
        "pruefen" => Message::KapitelPruefen,
        "verlauf" => Message::VerlaufOeffnen,
        "suche" => Message::SucheOeffnen(false),
        "suche_buch" => Message::SucheOeffnen(true),
        "suche_weiter" => Message::SucheNaechste(1),
        "suche_zurueck" => Message::SucheNaechste(-1),
        "einstellungen" | "einstellungen2" => Message::EinstellungenOeffnen,
        "handbuch" => Message::HandbuchOeffnen,
        "vorschau" => Message::VorschauToggle,
        "seitenleiste" => Message::SeitenleisteToggle,
        "inspektor" => Message::InspektorToggle,
        "fokus" => Message::FokusToggle,
        "schreibmaschine" => Message::SchreibmaschineToggle(!state.einstellungen.schreibmaschine),
        "neues_kapitel" => Message::NeuesKapitel,
        "umbenennen" => Message::KapitelUmbenennenDialog,
        "kapitel_hoch" => Message::KapitelVerschieben(-1),
        "kapitel_runter" => Message::KapitelVerschieben(1),
        "kapitel_zurueck" => Message::KapitelNachbar(-1),
        "kapitel_weiter" => Message::KapitelNachbar(1),
        "bereich1" => Message::BereichWechseln(Arbeitsbereich::Schreiben),
        "bereich2" => Message::BereichWechseln(Arbeitsbereich::Bauen),
        "bereich3" => Message::BereichWechseln(Arbeitsbereich::Pruefen),
        "bereich4" => Message::BereichWechseln(Arbeitsbereich::Zahlen),
        "bereich5" => Message::BereichWechseln(Arbeitsbereich::Cover),
        _ => return None,
    })
}

/// Zuordnung Palettentitel → Kürzel-Kennung, damit die Palette die gültigen Kürzel zeigt.
const TITEL_KUERZEL: &[(&str, &str)] = &[
    ("Speichern", "speichern"),
    ("Ordner mit Buchprojekt öffnen …", "ordner"),
    ("Kapitel auf Sprache prüfen", "pruefen"),
    ("Einstellungen …", "einstellungen"),
    ("Neues Kapitel", "neues_kapitel"),
    ("Im Kapitel suchen und ersetzen", "suche"),
    ("In allen Bänden suchen und ersetzen", "suche_buch"),
    ("Nächster Suchtreffer", "suche_weiter"),
    ("Vorheriger Suchtreffer", "suche_zurueck"),
    ("Kapitel umbenennen", "umbenennen"),
    ("Kapitel nach oben schieben", "kapitel_hoch"),
    ("Kapitel nach unten schieben", "kapitel_runter"),
    ("Zeitleiste: frühere Stände und Papierkorb", "verlauf"),
    ("Live-Vorschau umschalten", "vorschau"),
    ("Seitenleisten umschalten", "seitenleiste"),
    ("Details umschalten", "inspektor"),
    ("Schreibmaschinen-Modus umschalten", "schreibmaschine"),
    ("Fokus-Modus umschalten", "fokus"),
    ("Nächstes Kapitel", "kapitel_weiter"),
    ("Vorheriges Kapitel", "kapitel_zurueck"),
    ("Gehe zu Schreiben", "bereich1"),
    ("Gehe zu Bauen", "bereich2"),
    ("Gehe zu Prüfen", "bereich3"),
    ("Gehe zu Zahlen", "bereich4"),
    ("Cover gestalten", "bereich5"),
    ("Handbuch öffnen", "handbuch"),
];

/// Alle Befehle der Palette: Ansichten, Aktionen, Bücher und Kapitel zum Anspringen.
fn alle_befehle(state: &AppState) -> Vec<Befehl> {
    let befehl = |titel: &str, hinweis: &str, aktion: Message| Befehl {
        titel: titel.to_string(),
        hinweis: hinweis.to_string(),
        aktion,
    };

    let mut liste = vec![
        befehl("Speichern", "Strg+S", Message::Speichern),
        befehl("Ordner mit Buchprojekt öffnen …", "Strg+O", Message::OrdnerDialog),
        befehl("Kapitel auf Sprache prüfen", "Strg+L", Message::KapitelPruefen),
        befehl("Einstellungen …", "Strg+, oder F10", Message::EinstellungenOeffnen),
        befehl("LanguageTool installieren – Anleitung", "Linux, Windows, macOS", Message::AnleitungOeffnen),
        befehl("LanguageTool-Server starten", "Einstellungen · Prüfung", Message::LtServerStarten),
        befehl("LanguageTool-Server stoppen", "Einstellungen · Prüfung", Message::LtServerStoppen),
        befehl("Neues Kapitel", "Strg+Umschalt+N", Message::NeuesKapitel),
        befehl("Im Kapitel suchen und ersetzen", "Strg+F", Message::SucheOeffnen(false)),
        befehl("In allen Bänden suchen und ersetzen", "Strg+Umschalt+F", Message::SucheOeffnen(true)),
        befehl("Nächster Suchtreffer", "F3", Message::SucheNaechste(1)),
        befehl("Vorheriger Suchtreffer", "Umschalt+F3", Message::SucheNaechste(-1)),
        befehl("Kapitel umbenennen", "F2", Message::KapitelUmbenennenDialog),
        befehl("Handbuch öffnen", "", Message::HandbuchOeffnen),
        befehl("Werkbank unterstützen (Buy me a coffee)", "", Message::SpendenOeffnen),
        befehl("Über die Werkbank (Bausteine)", "", Message::UeberOeffnen),
        befehl("Kapitel löschen", "", Message::KapitelLoeschenFrage),
        befehl("Kapitel nach oben schieben", "Alt+Umschalt+↑", Message::KapitelVerschieben(-1)),
        befehl("Kapitel nach unten schieben", "Alt+Umschalt+↓", Message::KapitelVerschieben(1)),
        befehl("Teil umbenennen", "", Message::TeilUmbenennenDialog),
        befehl("Teil nach oben schieben", "", Message::TeilVerschieben(-1)),
        befehl("Teil nach unten schieben", "", Message::TeilVerschieben(1)),
        befehl("Teil entfernen", "", Message::TeilEntfernenFrage),
        befehl("Buchdaten bearbeiten (Titel, Autor, ISBN)", "", Message::MetaOeffnen),
        befehl("Zeitleiste: frühere Stände und Papierkorb", "Strg+H", Message::VerlaufOeffnen),
        befehl("Papierkorb: entfernte Teile zurückholen", "", Message::PapierkorbOeffnen),
        befehl("Neuer Buchteil", "", Message::NeuerTeil),
        befehl(
            "Live-Vorschau umschalten",
            "Strg+P",
            Message::VorschauToggle,
        ),
        befehl(
            "Seitenleisten umschalten",
            "Strg+B",
            Message::SeitenleisteToggle,
        ),
        befehl("Details umschalten", "Strg+I", Message::InspektorToggle),
        befehl("Schreibmaschinen-Modus umschalten", "", Message::SchreibmaschineToggle(!state.einstellungen.schreibmaschine)),
        befehl("Fokus-Modus umschalten", "F11", Message::FokusToggle),
        befehl("Nächstes Kapitel", "Alt+↓", Message::KapitelNachbar(1)),
        befehl("Vorheriges Kapitel", "Alt+↑", Message::KapitelNachbar(-1)),
        befehl(
            "Gehe zu Schreiben",
            "Strg+1",
            Message::BereichWechseln(Arbeitsbereich::Schreiben),
        ),
        befehl(
            "Gehe zu Bauen",
            "Strg+2",
            Message::BereichWechseln(Arbeitsbereich::Bauen),
        ),
        befehl(
            "Gehe zu Prüfen",
            "Strg+3",
            Message::BereichWechseln(Arbeitsbereich::Pruefen),
        ),
        befehl(
            "Gehe zu Zahlen",
            "Strg+4",
            Message::BereichWechseln(Arbeitsbereich::Zahlen),
        ),
        befehl("Cover gestalten", "Strg+5", Message::BereichWechseln(Arbeitsbereich::Cover)),
        befehl("Cover exportieren (PNG/SVG, alle Portale)", "Cover", Message::CoverExportieren),
        befehl("Alle Fassungen bauen", "Bauen", Message::BauenStarten),
        befehl("Buch setzen (Druck-PDFs, EPUB, Verlagsordner)", "Bauen", Message::SatzStarten),
        befehl("Alle Prüfungen starten (Code und Sprache)", "Prüfen", Message::PrueflaufStarten),
        befehl("Codebeispiele prüfen (Rust, Dart/Flutter)", "Prüfen", Message::CodePruefungStarten),
        befehl("Ganzes Buch auf Sprache prüfen", "Prüfen", Message::BuchSpracheStarten),
        befehl("Portalregeln als Datei anlegen", "Bauen", Message::PortaleAnlegen),
    ];
    // Jede Einstellungsseite ist direkt über die Suche erreichbar
    for (nr, (name, inhalt)) in ui::einstellungen::SEITEN.iter().enumerate() {
        liste.push(Befehl {
            titel: format!("Einstellungen: {name}"),
            hinweis: inhalt.to_string(),
            aktion: Message::EinstellungenSeite(nr as u8),
        });
    }

    for band in &state.baende {
        if *band != state.band {
            liste.push(befehl(
                &format!("Band öffnen: {band}"),
                "Band",
                Message::ProjektWechseln(band.clone()),
            ));
        }
    }

    if let Some(k) = state.projekt.kapitel_finden(&state.aktives_kapitel_id) {
        for u in werkbank_kern::markdown::ueberschriften(&k.inhalt) {
            liste.push(befehl(
                &format!("Überschrift: {}", u.text),
                &format!("Zeile {}", u.zeile + 1),
                Message::SpringeZuZeile { zeile: u.zeile, spalte: 0 },
            ));
        }
    }

    for teil in &state.projekt.teile {
        for k in &teil.kapitel {
            liste.push(befehl(
                &format!("Kapitel: {}", k.titel),
                &teil.titel,
                Message::KapitelAusgewaehlt(k.id.clone()),
            ));
        }
    }

    for k in state.projekt.teile.iter().flat_map(|t| t.kapitel.iter()) {
        liste.push(befehl(
            &format!("Verweis einfügen: {}", k.titel),
            &format!("@kap:{}", k.id),
            Message::VerweisEinfuegen(k.id.clone()),
        ));
    }

    for b in &mut liste {
        if let Some((_, id)) = TITEL_KUERZEL.iter().find(|(t, _)| *t == b.titel) {
            let k = kuerzel::kuerzel_von(&state.einstellungen, id);
            b.hinweis = if k.is_empty() { String::new() } else { kuerzel::anzeige(k) };
        }
    }
    liste
}

/// Die zum Suchtext passenden Befehle (jedes Suchwort muss im Titel oder Hinweis vorkommen).
fn palette_befehle(state: &AppState) -> Vec<Befehl> {
    let suche = state
        .palette
        .as_ref()
        .map(|p| p.suche.to_lowercase())
        .unwrap_or_default();
    let woerter: Vec<&str> = suche.split_whitespace().collect();

    alle_befehle(state)
        .into_iter()
        .filter(|b| {
            let heu = format!("{} {}", b.titel, b.hinweis).to_lowercase();
            woerter.iter().all(|w| heu.contains(w))
        })
        .take(12)
        .collect()
}

/// Erzeugt die Benutzeroberfläche passend zum gewählten Bereich.
/// Was die Statusleiste unten zeigt.
fn statusdaten(state: &AppState) -> ui::statusleiste::Daten<'_> {
    use ui::statusleiste::{Daten, Pruefung, Schreibstand};
    let schreiben = state.aktiver_bereich == Arbeitsbereich::Schreiben;
    let schreibstand = schreiben.then(|| {
        let pos = state.editor_content.cursor().position;
        Schreibstand {
            zeile: pos.line + 1,
            spalte: pos.column + 1,
            woerter: state.editor_content.text().split_whitespace().count(),
            ziel: state.projekt.kapitel_finden(&state.aktives_kapitel_id).map_or(0, |k| k.wortziel),
        }
    });
    let pruefung = if !state.einstellungen.sprachpruefung {
        Pruefung::Aus
    } else {
        match (&state.server_status, &state.pruef_status) {
            (ServerStatus::Erreichbar(_), PruefStatus::Laeuft) => Pruefung::Laeuft,
            (ServerStatus::Erreichbar(_), PruefStatus::Fertig(n)) => Pruefung::Hinweise(*n),
            (ServerStatus::Erreichbar(_), _) => Pruefung::Bereit,
            (ServerStatus::Startet | ServerStatus::Pruefe, _) => Pruefung::Laeuft,
            _ => Pruefung::Offline,
        }
    };
    Daten {
        meldung: &state.speicher_zeitstempel,
        ungespeichert: dateien::hat_ungespeichertes(state),
        demo: state.band.ordner.is_none(),
        schreibstand,
        pruefung,
    }
}

/// Die Einträge eines Menüs der Menüleiste; Kürzel stammen aus den Einstellungen des Anwenders.
fn menue_punkte(state: &AppState, menue: ui::menue::Menue) -> Vec<ui::menue::Punkt> {
    use ui::menue::{Menue, Punkt};
    let k = |id: &str| kuerzel::anzeige(kuerzel::kuerzel_von(&state.einstellungen, id));
    let bereich = |b: Arbeitsbereich, id: &str| Punkt::schalter(b.titel(), k(id), state.aktiver_bereich == b, Message::BereichWechseln(b));
    match menue {
        Menue::Datei => vec![
            Punkt::aktion("Ordner mit Buchprojekt öffnen …", k("ordner"), Message::OrdnerDialog),
            Punkt::aktion("Speichern", k("speichern"), Message::Speichern),
            Punkt::Trenner,
            Punkt::aktion("Neues Kapitel …", k("neues_kapitel"), Message::NeuesKapitel),
            Punkt::aktion("Neuer Buchteil …", "", Message::NeuerTeil),
            Punkt::Trenner,
            Punkt::aktion("Einstellungen …", k("einstellungen"), Message::EinstellungenOeffnen),
            Punkt::Trenner,
            Punkt::aktion("Beenden", "", Message::FensterSchliessen(iced::window::Id::unique())),
        ],
        Menue::Bearbeiten => vec![
            Punkt::aktion("Im Kapitel suchen und ersetzen …", k("suche"), Message::SucheOeffnen(false)),
            Punkt::aktion("In allen Bänden suchen …", k("suche_buch"), Message::SucheOeffnen(true)),
            Punkt::aktion("Nächster Treffer", k("suche_weiter"), Message::SucheNaechste(1)),
            Punkt::aktion("Vorheriger Treffer", k("suche_zurueck"), Message::SucheNaechste(-1)),
            Punkt::Trenner,
            Punkt::aktion("Kapitel auf Sprache prüfen", k("pruefen"), Message::KapitelPruefen),
            Punkt::Trenner,
            Punkt::aktion("Kapitel umbenennen …", k("umbenennen"), Message::KapitelUmbenennenDialog),
            Punkt::aktion("Kapitel nach oben schieben", k("kapitel_hoch"), Message::KapitelVerschieben(-1)),
            Punkt::aktion("Kapitel nach unten schieben", k("kapitel_runter"), Message::KapitelVerschieben(1)),
            Punkt::aktion("Teil umbenennen …", "", Message::TeilUmbenennenDialog),
            Punkt::Trenner,
            Punkt::gefahr("Kapitel löschen …", "", Message::KapitelLoeschenFrage),
            Punkt::gefahr("Teil entfernen …", "", Message::TeilEntfernenFrage),
        ],
        Menue::Ansicht => vec![
            Punkt::schalter("Bibliothek und Blattliste", k("seitenleiste"), state.seitenleiste_sichtbar && !state.fokus_modus, Message::SeitenleisteToggle),
            Punkt::schalter("Live-Vorschau", k("vorschau"), state.vorschau_sichtbar, Message::VorschauToggle),
            Punkt::schalter("Details (Inspektor)", k("inspektor"), state.inspektor_sichtbar && !state.fokus_modus, Message::InspektorToggle),
            Punkt::Trenner,
            Punkt::schalter("Fokus-Modus", k("fokus"), state.fokus_modus, Message::FokusToggle),
            Punkt::schalter("Schreibmaschinen-Modus", k("schreibmaschine"), state.einstellungen.schreibmaschine, Message::SchreibmaschineToggle(!state.einstellungen.schreibmaschine)),
            Punkt::Trenner,
            bereich(Arbeitsbereich::Schreiben, "bereich1"),
            bereich(Arbeitsbereich::Bauen, "bereich2"),
            bereich(Arbeitsbereich::Pruefen, "bereich3"),
            bereich(Arbeitsbereich::Zahlen, "bereich4"),
            bereich(Arbeitsbereich::Cover, "bereich5"),
            Punkt::Trenner,
            Punkt::aktion("Nächstes Kapitel", k("kapitel_weiter"), Message::KapitelNachbar(1)),
            Punkt::aktion("Vorheriges Kapitel", k("kapitel_zurueck"), Message::KapitelNachbar(-1)),
        ],
        Menue::Buch => vec![
            Punkt::aktion("Zeitleiste: frühere Stände …", k("verlauf"), Message::VerlaufOeffnen),
            Punkt::aktion("Papierkorb …", "", Message::PapierkorbOeffnen),
            Punkt::Trenner,
            Punkt::aktion("Buchdaten bearbeiten (Titel, Autor, ISBN) …", "", Message::MetaOeffnen),
            Punkt::Trenner,
            Punkt::aktion("Befehle und Kapitel suchen …", k("palette"), Message::PaletteToggle),
        ],
        Menue::Hilfe => vec![
            Punkt::aktion("Handbuch", k("handbuch"), Message::HandbuchOeffnen),
            Punkt::aktion("Homepage besuchen", "", Message::LinkOeffnen(HOMEPAGE.to_string())),
            Punkt::aktion("Kontakt per E-Mail …", "", Message::LinkOeffnen(KONTAKT.to_string())),
            Punkt::Trenner,
            Punkt::aktion("☕ Buy me a coffee …", "", Message::SpendenOeffnen),
            Punkt::Trenner,
            Punkt::aktion("Über die Werkbank", "", Message::UeberOeffnen),
        ],
    }
}

fn view(state: &AppState) -> Element<'_, Message> {
    let kopf = ui::navigation::kopfleiste(
        state.aktiver_bereich,
        dateien::hat_ungespeichertes(state),
        state.baende.clone(),
        state.band.clone(),
        Message::ProjektWechseln,
        Message::BereichWechseln,
        ui::navigation::Aktionen {
            speichern: Message::Speichern,
            palette: Message::PaletteToggle,
        },
    );

    let inhalt: Element<'_, Message> = match state.aktiver_bereich {
        Arbeitsbereich::Schreiben => ui::schreiben::ansicht(
            &state.projekt,
            &state.aktiver_teil_id,
            &state.aktives_kapitel_id,
            &state.editor_content,
            state.edit_tick,
            state.fokus_modus,
            state.seitenleiste_sichtbar,
            state.inspektor_sichtbar,
            state.vorschau_sichtbar,
            Message::TeilAusgewaehlt,
            Message::KapitelAusgewaehlt,
            Message::EditorAktion,
            Message::NeuesKapitel,
            Message::NeuerTeil,
            Message::ZielSetzen,
            Message::StatusAendern,
            |zeile, spalte| Message::SpringeZuZeile { zeile, spalte },
            Message::QuerverweisOeffnen,
            ui::zusatz::ansicht(state),
            state.inspektor_tab,
            Message::InspektorTab,
            Message::InspektorZiehen(true),
            hl_einstellung(state),
            vorschau_element(state),
            erklaerung(state),
            Message::WellenMenue,
            Message::KontextOeffnen,
        ),
        Arbeitsbereich::Bauen => ui::bauen::ansicht(state),
        Arbeitsbereich::Pruefen => ui::pruefen::ansicht(state),
        Arbeitsbereich::Zahlen => ui::zahlen::ansicht(state),
        Arbeitsbereich::Cover => ui::cover::ansicht(state),
    };

    let basis: Element<'_, Message> = if state.fokus_modus {
        iced::widget::column![kopf, inhalt].width(Fill).height(Fill).into()
    } else {
        iced::widget::column![ui::menue::leiste(state.menue), kopf, inhalt, ui::statusleiste::leiste(&statusdaten(state))]
            .width(Fill)
            .height(Fill)
            .into()
    };

    let overlay: Option<Element<'_, Message>> = if let Some(m) = state.menue.filter(|_| !state.fokus_modus) {
        Some(ui::menue::auswahl(m, menue_punkte(state, m)))
    } else if let Some(k) = &state.kontext {
        Some(ui::kontext::menue(k, &state.projekt.teile, state.fenster_groesse))
    } else if state.handbuch_offen {
        Some(ui::hilfe::handbuch())
    } else if state.ueber_offen {
        Some(ui::hilfe::ueber())
    } else if let Some(d) = &state.dialog {
        Some(ui::dialog::dialog_ansicht(d))
    } else if let Some(md) = &state.meta_entwurf {
        let ziel = if state.band.ordner.is_some() { "band.yaml im Bandordner" } else { "Demo-Buch: nur im Speicher" };
        Some(ui::dialog::meta_ansicht(md, ziel))
    } else if let Some(e) = &state.eingabe {
        Some(ui::dialog::eingabe_ansicht(e))
    } else if let Some(v) = &state.verlauf {
        Some(ui::zeitleiste::ansicht(v, state.fenster_groesse))
    } else if state.suche_offen && state.suche.is_some() {
        state.suche.as_ref().map(ui::suche::ansicht)
    } else if let Some(fenster) = state.einstellungen_fenster {
        Some(ui::einstellungen::ansicht(
            &state.werkzeuge,
            state.kuerzel_aufnahme.as_deref(),
            &state.woerterbuch_status,
            &state.einstellungen,
            fenster,
            &state.server_status,
            state.server.is_some(),
            &state.ausnahmen,
        ))
    } else if let Some(palette) = &state.palette {
        let treffer = palette_befehle(state)
            .into_iter()
            .map(|b| ui::palette::Zeile { titel: b.titel, hinweis: b.hinweis })
            .collect();
        Some(ui::palette::ansicht(
            &palette.suche,
            treffer,
            palette.auswahl,
            Message::PaletteSuche,
            Message::PaletteAusfuehren,
            Message::PaletteWaehlen,
            Message::PaletteSchliessen,
        ))
    } else {
        None
    };

    match overlay {
        Some(o) => stack![basis, o].into(),
        None => basis,
    }
}

/// Die Text-Vorschau folgt dem Cursor: Sie scrollt an die Stelle, die der Cursorzeile im Kapitel entspricht.
fn vorschau_folgen(state: &AppState) -> Task<Message> {
    if !(state.vorschau_sichtbar && state.satz_vorschau.text_modus) {
        return Task::none();
    }
    let zeilen = state.editor_content.line_count().max(2) - 1;
    let anteil = (state.editor_content.cursor().position.line as f32 / zeilen as f32).clamp(0.0, 1.0);
    iced::widget::operation::snap_to(
        ui::vorschau::scroll_id(),
        iced::widget::operation::RelativeOffset { x: None, y: Some(anteil) },
    )
}

/// Die Vorschau neben dem Editor: Satzbild (Standard) oder schnelle Textvorschau.
fn vorschau_element(state: &AppState) -> Element<'_, Message> {
    if state.satz_vorschau.text_modus {
        let inhalt = state.projekt.kapitel_finden(&state.aktives_kapitel_id).map_or("", |k| k.inhalt.as_str());
        let zeile = state.editor_content.cursor().position.line;
        iced::widget::column![
            iced::widget::container(
                ui::fokus::button(iced::widget::text("Satzvorschau").size(11))
                    .padding([2, 8])
                    .style(thema::sekundaer_knopf_stil)
                    .on_press(Message::VorschauText(false))
            )
            .padding([6, 12]),
            ui::vorschau::ansicht(
                inhalt,
                zeile,
                |zeile, spalte| Message::SpringeZuZeile { zeile, spalte },
                Some(state.projekt.wurzelverzeichnis.as_path()),
            ),
        ]
        .into()
    } else {
        ui::satzvorschau::ansicht(&state.satz_vorschau, state.werkzeuge.pandoc_brauchbar())
    }
}

/// Text der Hinweisblase, die beim Überfahren einer unterschlängelten Stelle erscheint.
fn erklaerung(state: &AppState) -> text_editor::Erklaerung<'_> {
    let woerterbuch = state.woerterbuch.clone();
    let treffer = &state.treffer;
    Box::new(move |zeile, wort, art, spalte| {
        if art == markierung::WELLE_RECHTSCHREIBUNG {
            let wb = woerterbuch.as_ref()?;
            let vorschlag = wb
                .erster_vorschlag(wort)
                .map(|v| format!("Vorschlag: {v}\n"))
                .unwrap_or_default();
            Some(format!(
                "Rechtschreibung: „{wort}“ steht nicht im Wörterbuch ({}).\n{vorschlag}Rechtsklick: korrigieren, merken oder ignorieren.",
                wb.name
            ))
        } else {
            let t = treffer.iter().find(|t| {
                (t.zeile, t.spalte) <= (zeile, spalte) && (zeile, spalte) < (t.ende_zeile, t.ende_spalte.max(t.spalte + 1))
            })?;
            let art = match t.kategorie_id.as_str() {
                "TYPOS" => "Tippfehler",
                "GRAMMAR" => "Grammatik",
                "PUNCTUATION" => "Zeichensetzung",
                "STYLE" | "REDUNDANCY" | "COLLOQUIALISMS" => "Stil",
                "CASING" => "Groß- und Kleinschreibung",
                "TYPOGRAPHY" => "Typografie",
                _ => "Sprachprüfung",
            };
            let ersatz = if t.ersetzungen.is_empty() {
                String::new()
            } else {
                format!("\nVorschlag: {}", t.ersetzungen.iter().take(3).cloned().collect::<Vec<_>>().join(" · "))
            };
            Some(format!("{art}: {}{ersatz}\nRechtsklick: korrigieren oder ignorieren.", t.meldung))
        }
    })
}

/// Markierungs-Einstellung für den Editor; wird nur neu gebaut, wenn sich etwas Relevantes ändert.
fn hl_einstellung(state: &AppState) -> markierung::Einstellung {
    let wb = state.einstellungen.rechtschreibung.then(|| state.woerterbuch.clone()).flatten();
    let eigene: HashSet<String> =
        state.ausnahmen.woerter.iter().map(|w| w.to_string()).chain(state.ignoriert.iter().cloned()).collect();
    let hinweise: Vec<markierung::Bereich> =
        state.treffer.iter().map(|t| (t.zeile, t.spalte, t.ende_zeile, t.ende_spalte)).collect();
    let version = markierung::Einstellung::version_berechnen(&wb, &eigene, &hinweise, state.einstellungen.hell);
    let mut cache = state.hl_cache.borrow_mut();
    if let Some(c) = cache.as_ref()
        && c.version == version
    {
        return c.clone();
    }
    let neu = markierung::Einstellung {
        version,
        woerterbuch: wb,
        eigene: Arc::new(eigene),
        hinweise: Arc::new(hinweise),
        wellen: Default::default(),
    };
    *cache = Some(neu.clone());
    neu
}

/// Beendet das Programm; ein gestarteter LanguageTool-Server wird dabei mitbeendet.
fn beenden(state: &mut AppState) -> Task<Message> {
    if let Some(ordner) = state.band.ordner.clone() {
        dateien::statistik_speichern(state, &ordner);
    }
    state.server = None;
    iced::exit()
}

/// Verbucht Netto-Wörter und Schreibzeit der letzten Eingabe in der Statistik.
fn statistik_erfassen(state: &mut AppState) {
    let heute = Local::now().date_naive();
    let jetzt = Instant::now();
    let neu = dateien::kapitel_woerter(&state.projekt, &state.aktives_kapitel_id);
    let luecke = state.letzte_eingabe.map(|t| jetzt.duration_since(t).as_secs());
    state.statistik.wortaenderung(heute, state.wortzahl_stand, neu);
    state.statistik.kapitel_erfassen(&state.aktives_kapitel_id, state.wortzahl_stand, neu, luecke);
    state.wortzahl_stand = neu;
    state.statistik.aktivitaet(heute, luecke);
    state.letzte_eingabe = Some(jetzt);
    state.statistik_dirty = true;
}

/// Führt `arbeit` auf einem eigenen Thread aus; ihre Meldungen (`melde`) kommen als `schritt`-Nachrichten,
/// das Ergebnis als `fertig`-Nachricht an.
fn mit_schritten<T: Send + 'static>(
    arbeit: impl FnOnce(&dyn Fn(String)) -> T + Send + 'static,
    schritt: fn(String) -> Message,
    fertig: fn(T) -> Message,
) -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(16, async move |mut ausgabe: iced::futures::channel::mpsc::Sender<Message>| {
        use iced::futures::{SinkExt, StreamExt};
        let (sender, mut empfaenger) = iced::futures::channel::mpsc::unbounded::<Message>();
        std::thread::spawn(move || {
            let s = sender.clone();
            let ergebnis = arbeit(&move |text| {
                let _ = s.unbounded_send(schritt(text));
            });
            let _ = sender.unbounded_send(fertig(ergebnis));
        });
        while let Some(m) = empfaenger.next().await {
            if ausgabe.send(m).await.is_err() {
                break;
            }
        }
    })
}

/// Takt für die Arbeitsanzeige (alle 100 ms), solange gesetzt oder gebaut wird.
fn puls_strom() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(1, async |mut ausgabe: iced::futures::channel::mpsc::Sender<Message>| {
        use iced::futures::{SinkExt, StreamExt};
        let (sender, mut empfaenger) = iced::futures::channel::mpsc::unbounded::<()>();
        std::thread::spawn(move || {
            while sender.unbounded_send(()).is_ok() {
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        while empfaenger.next().await.is_some() {
            if ausgabe.send(Message::Puls).await.is_err() {
                break;
            }
        }
    })
}

/// Liefert alle 500 ms einen Zeitpunkt (ein Hilfsthread, keine Laufzeit-Abhängigkeit wie tokio).
fn takt_strom() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(1, async |mut ausgabe: iced::futures::channel::mpsc::Sender<Message>| {
        use iced::futures::{SinkExt, StreamExt};
        let (sender, mut empfaenger) = iced::futures::channel::mpsc::unbounded::<()>();
        std::thread::spawn(move || {
            while sender.unbounded_send(()).is_ok() {
                std::thread::sleep(Duration::from_millis(500));
            }
        });
        while empfaenger.next().await.is_some() {
            if ausgabe.send(Message::Takt(Instant::now())).await.is_err() {
                break;
            }
        }
    })
}

/// Abonniert Tastendrücke für Shortcuts, auch solche, die ein Eingabefeld bereits verarbeitet hat
/// (z. B. `Esc` im Suchfeld der Palette).
fn subscription(state: &AppState) -> Subscription<Message> {
    let tasten = iced::event::listen_with(|event, _status, _fenster| match event {
        iced::Event::Keyboard(e @ keyboard::Event::KeyPressed { .. }) => {
            Some(Message::TastaturEreignis(e))
        }
        _ => None,
    });

    let braucht_takt = (state.vorschau_faellig.is_some()
        && state.vorschau_sichtbar
        && !state.satz_vorschau.text_modus
        && !state.satz_vorschau.laeuft)
        || (state.einstellungen.autospeichern
            && dateien::hat_ungespeichertes(state)
            && state.letzte_aenderung.is_some())
        || (state.pruef_faellig.is_some()
            && state.pruef_status != PruefStatus::Laeuft);

    let takt = if braucht_takt {
        Subscription::run(takt_strom)
    } else {
        Subscription::none()
    };
    let schliessen = iced::window::close_requests().map(Message::FensterSchliessen);
    let puls = if state.bau.satz_laeuft || state.bau.laeuft.is_some() {
        Subscription::run(puls_strom)
    } else {
        Subscription::none()
    };
    let groesse = iced::window::resize_events().map(|(_, g)| Message::FensterGroesse(g));

    // Nur während des Ziehens mitlesen, wohin die Maus wandert
    let ziehen = if state.inspektor_zieht {
        iced::event::listen_with(|event, _status, _fenster| match event {
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => Some(Message::InspektorMaus(position.x)),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(_)) => Some(Message::InspektorZiehen(false)),
            _ => None,
        })
    } else {
        Subscription::none()
    };

    Subscription::batch([tasten, takt, schliessen, ziehen, groesse, puls])
}

/// Das Iced-Thema folgt der Werkbank-Palette (hell: „Papier“, dunkel: „Dämmerung“).
fn theme(state: &AppState) -> Theme {
    let _ = state;
    thema::iced_theme()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn band_ordner(wurzel: &Path, name: &str) {
        let d = wurzel.join(name);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("teil-1.md"), "# Teil\n\n## Kapitel\n\nText\n").unwrap();
    }

    #[test]
    fn reihenordner_liefert_alle_baende_sortiert() {
        let wurzel = std::env::temp_dir().join("werkbank-test-reihe");
        let _ = std::fs::remove_dir_all(&wurzel);
        for n in ["Band 10", "Band 2", "Band 1"] {
            band_ordner(&wurzel, n);
        }
        std::fs::create_dir_all(wurzel.join("scripte")).unwrap();

        let (baende, erster) = ordner_einlesen(&wurzel).unwrap();
        let namen: Vec<_> = baende.iter().map(|b| b.name.rsplit(" · ").next().unwrap()).collect();
        assert_eq!(namen, ["Band 1", "Band 2", "Band 10"]);
        assert_eq!(erster, baende[0]);
        let _ = std::fs::remove_dir_all(&wurzel);
    }

    #[test]
    fn einzelner_bandordner_findet_geschwister() {
        let wurzel = std::env::temp_dir().join("werkbank-test-geschwister");
        let _ = std::fs::remove_dir_all(&wurzel);
        band_ordner(&wurzel, "Band_1");
        band_ordner(&wurzel, "Band_2");

        let (baende, gewaehlt) = ordner_einlesen(&wurzel.join("Band_2")).unwrap();
        assert_eq!(baende.len(), 2);
        assert_eq!(gewaehlt.ordner, Some(wurzel.join("Band_2")));
        let _ = std::fs::remove_dir_all(&wurzel);
    }

    #[test]
    fn ordner_ohne_buch_ist_fehler() {
        let wurzel = std::env::temp_dir().join("werkbank-test-leer");
        std::fs::create_dir_all(&wurzel).unwrap();
        assert!(ordner_einlesen(&wurzel).is_err());
        let _ = std::fs::remove_dir_all(&wurzel);
    }
}
