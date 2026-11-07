//! # Programmeinstellungen
//!
//! Persönliche Einstellungen der Werkbank (derzeit: LanguageTool), gespeichert
//! als JSON im Konfigurationsordner des Benutzers.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Sprachen, zwischen denen die Prüfung umgeschaltet werden kann.
pub const SPRACHEN: [&str; 6] = ["de-DE", "de-AT", "de-CH", "en-US", "en-GB", "auto"];

/// Einstellungen der Werkbank.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Einstellungen {
    /// Eigene Tastenkürzel: Befehl-Kennung → Kürzel (leer = unbelegt).
    pub tasten: BTreeMap<String, String>,
    /// Schreibmaschinen-Modus: Cursorzeile bleibt in der Fenstermitte.
    pub schreibmaschine: bool,
    /// Helles statt dunkles Erscheinungsbild.
    pub hell: bool,
    /// Schriftgröße im Editor.
    pub schriftgroesse: f32,
    /// Schriftart des Editors: `serif`, `sans` oder `mono`.
    pub schriftart: String,
    /// Zeilenabstand des Editors (Vielfaches der Schriftgröße).
    pub zeilenabstand: f32,
    /// Maximale Breite der Textspalte.
    pub spaltenbreite: f32,
    /// Breite der rechten Detailspalte (per Ziehen veränderbar).
    pub inspektor_breite: f32,
    /// Aus dem Druck-PDF mit Ghostscript eine PDF/X-3-Datei erzeugen.
    pub pdfx: bool,
    /// Seitenzahl im Satz: `aussen` (abwechselnd außen) oder `mitte`.
    pub seitenzahl: String,
    /// Zählweise im Satz: `roemisch` (Titelei römisch, Haupttext ab 1) oder `durchgehend`.
    pub zaehlung: String,
    /// Teile beginnen im Druck auf einer rechten Seite.
    pub teile_rechts: bool,
    /// Auch Kapitel beginnen im Druck auf einer rechten Seite.
    pub kapitel_rechts: bool,
    /// Seitenzahlen auch auf Schmutztitel, Titelblatt, Impressum und Teiltiteln.
    pub titelei_zahlen: bool,
    /// Jeder Teil bekommt eine eigene Titelseite mit leerer Rückseite.
    pub teil_titelseiten: bool,
    /// Das Buch endet auf einer geraden Seite (Leerseite am Ende nötigenfalls).
    pub gerade_seitenzahl: bool,
    /// Wörter pro Tag, die als Tagespensum gelten.
    pub tagespensum: usize,
    /// Änderungen nach einer Schreibpause automatisch speichern.
    pub autospeichern: bool,
    /// Rechtschreibfehler im Text mit dem Hunspell-Wörterbuch markieren.
    pub rechtschreibung: bool,
    /// Eigener Ordner mit Hunspell-Wörterbüchern (leer = übliche Orte).
    pub woerterbuch_ordner: String,
    /// Eigener Ordner von Ghostscript (der Ordner mit `bin`, oder `bin` selbst; leer = suchen).
    pub gs_ordner: String,
    /// Sprachprüfung mit LanguageTool überhaupt verwenden.
    pub sprachpruefung: bool,
    /// Nach einer kurzen Schreibpause automatisch prüfen.
    pub automatisch: bool,
    /// Adresse des LanguageTool-Servers.
    pub server_url: String,
    /// Ordner mit `languagetool-server.jar` für den von der Werkbank gestarteten Server.
    pub lt_ordner: String,
    /// Java-Programm zum Starten des Servers.
    pub java: String,
    /// Sprachcode für die Prüfung (z. B. `de-DE`).
    pub sprache: String,
    /// Lokalen Server beim Programmstart mitstarten.
    pub server_autostart: bool,
}

impl Default for Einstellungen {
    fn default() -> Self {
        Self {
            tasten: BTreeMap::new(),
            schreibmaschine: false,
            hell: false,
            schriftart: "serif".to_string(),
            zeilenabstand: 1.55,
            schriftgroesse: 17.0,
            spaltenbreite: 700.0,
            inspektor_breite: 316.0,
            pdfx: true,
            seitenzahl: "aussen".to_string(),
            zaehlung: "roemisch".to_string(),
            teile_rechts: true,
            kapitel_rechts: false,
            titelei_zahlen: false,
            teil_titelseiten: true,
            gerade_seitenzahl: true,
            tagespensum: 2000,
            autospeichern: true,
            rechtschreibung: true,
            woerterbuch_ordner: String::new(),
            gs_ordner: String::new(),
            sprachpruefung: true,
            automatisch: true,
            server_url: "http://localhost:8081".to_string(),
            // Snap: LanguageTool liegt im Paket (WERKBANK_LT_ORDNER setzt snapcraft.yaml)
            lt_ordner: std::env::var("WERKBANK_LT_ORDNER").unwrap_or_default(),
            java: "java".to_string(),
            sprache: "de-DE".to_string(),
            server_autostart: std::env::var_os("WERKBANK_LT_ORDNER").is_some(),
        }
    }
}

/// Konfigurationsordner der Werkbank (`~/.config/werkbank` bzw. `%APPDATA%\werkbank`).
pub fn konfig_ordner() -> Option<PathBuf> {
    let basis = std::env::var_os("XDG_CONFIG_HOME")
        .or_else(|| std::env::var_os("APPDATA"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(basis.join("werkbank"))
}

impl Einstellungen {
    fn datei() -> Option<PathBuf> {
        Some(konfig_ordner()?.join("einstellungen.json"))
    }

    /// Lädt die gespeicherten Einstellungen; fehlende oder defekte Dateien ergeben Standardwerte.
    pub fn laden() -> Self {
        Self::datei()
            .and_then(|d| std::fs::read_to_string(d).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Speichert die Einstellungen (Fehler werden bewusst ignoriert, das Programm läuft weiter).
    pub fn speichern(&self) {
        let Some(datei) = Self::datei() else { return };
        if let Some(eltern) = datei.parent() {
            let _ = std::fs::create_dir_all(eltern);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(datei, text);
        }
    }

    /// Überträgt Erscheinungsbild und Editor-Maße in die Darstellung.
    pub fn darstellung_anwenden(&self) {
        crate::thema::hell_setzen(self.hell);
        crate::thema::schriftart_setzen(match self.schriftart.as_str() { "sans" => 0, "mono" => 2, _ => 1 });
        crate::thema::zeilenabstand_setzen(self.zeilenabstand.clamp(1.2, 2.0));
        crate::thema::schreibmaschine_setzen(self.schreibmaschine);
        crate::thema::inspektor_breite_setzen(self.inspektor_breite);
        crate::thema::darstellung_setzen(self.schriftgroesse.clamp(10.0, 32.0), self.spaltenbreite.clamp(480.0, 1600.0));
    }

    /// Port des Servers aus der Adresse (Standard: 8081).
    pub fn port(&self) -> u16 {
        self.server_url
            .trim_end_matches('/')
            .rsplit(':')
            .next()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8081)
    }
}
