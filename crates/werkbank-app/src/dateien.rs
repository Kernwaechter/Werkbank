//! # Speichern und Dateistruktur in der App
//!
//! Verbindet den Kern (atomares Speichern, Snapshots, Struktur) mit dem
//! Zustand der Oberfläche: Änderungsverfolgung, Autosave und Statistik.

use crate::{AppState, Band};
use crate::editor as text_editor;
use std::time::{Duration, Instant};
use werkbank_kern::Projekt;

/// Mindestabstand zwischen zwei Snapshots bei Autosave.
pub const SNAPSHOT_AUTO: Duration = Duration::from_secs(10 * 60);
/// Mindestabstand bei manuellem Speichern.
pub const SNAPSHOT_MANUELL: Duration = Duration::from_secs(60);

/// Gibt es Änderungen, die noch nicht auf der Platte stehen?
pub fn hat_ungespeichertes(state: &AppState) -> bool {
    !state.dirty.is_empty() || state.struktur_dirty || state.meta_dirty
}

/// Übernimmt den Editortext ins Modell.
pub fn editor_in_modell(state: &mut AppState) {
    let text = state.editor_content.text();
    if let Some(k) = state.projekt.kapitel_finden_mut(&state.aktives_kapitel_id) {
        k.inhalt = text;
    }
}

/// Merkt den Teil des offenen Kapitels zum Speichern vor.
pub fn teil_vormerken(state: &mut AppState, kapitel_id: &str) {
    if let Some(teil) = state.projekt.teil_von_kapitel(kapitel_id) {
        state.dirty.insert(teil.to_string());
    }
    state.letzte_aenderung = Some(Instant::now());
}

/// Lädt den Editor neu aus dem Modell (nach Umbenennen, Ersetzen, Wiederherstellen).
pub fn editor_neu_laden(state: &mut AppState) {
    let text = state
        .projekt
        .kapitel_finden(&state.aktives_kapitel_id)
        .map(|k| k.inhalt.clone())
        .unwrap_or_default();
    state.editor_content = text_editor::Content::with_text(&text);
    state.edit_tick = state.edit_tick.wrapping_add(1);
    state.wortzahl_stand = text.split_whitespace().count();
}

/// Schreibt alle ungespeicherten Änderungen. Liefert die Zahl geschriebener Dateien.
///
/// Beim Demo-Buch geschieht nichts auf der Platte. Nach Fehlern bleibt alles vorgemerkt.
pub fn speichern(state: &mut AppState, abstand: Duration) -> Result<usize, String> {
    speichern_intern(state, abstand, false)
}

/// Wie [`speichern`], legt aber bei jeder geänderten Datei den alten Stand als durchnummerierte
/// Sicherung ab (`.werkbank/backups/<Datei>/<Datei>-0001.md`, …). Für jedes bewusste Speichern.
pub fn speichern_nummeriert(state: &mut AppState) -> Result<usize, String> {
    speichern_intern(state, Duration::ZERO, true)
}

fn speichern_intern(state: &mut AppState, abstand: Duration, nummeriert: bool) -> Result<usize, String> {
    editor_in_modell(state);
    state.letzte_sicherung = None;

    let Some(ordner) = state.band.ordner.clone() else {
        state.dirty.clear();
        state.struktur_dirty = false;
        state.meta_dirty = false;
        state.snapshot_zwang = false;
        state.ist_geaendert = false;
        return Ok(0);
    };

    let abstand = if state.snapshot_zwang { Duration::ZERO } else { abstand };
    let ids: Vec<String> = state.dirty.iter().cloned().collect();
    let mut fehler = Vec::new();
    let mut geschrieben = 0;

    if nummeriert {
        match state.projekt.teile_speichern_nummeriert(&ids) {
            Ok(g) => {
                geschrieben = g.len();
                state.letzte_sicherung = g.iter().map(|(_, n)| *n).filter(|n| *n > 0).max();
            }
            Err(e) => fehler.push(e),
        }
    } else {
        match state.projekt.teile_speichern(&ids, abstand) {
            Ok(g) => geschrieben = g.len(),
            Err(e) => fehler.push(e),
        }
    }
    if fehler.is_empty() && state.struktur_dirty
        && let Err(e) = state.projekt.reihenfolge_speichern() {
            fehler.push(format!("reihenfolge.txt: {e}"));
        }
    if fehler.is_empty() && state.meta_dirty
        && let Err(e) = state.projekt.kapitel_meta_speichern() {
            fehler.push(format!("kapitel.json: {e}"));
        }
    statistik_speichern(state, &ordner);

    if fehler.is_empty() {
        state.dirty.clear();
        state.struktur_dirty = false;
        state.meta_dirty = false;
        state.snapshot_zwang = false;
        state.ist_geaendert = false;
        Ok(geschrieben)
    } else {
        Err(fehler.join("; "))
    }
}

pub fn statistik_speichern(state: &mut AppState, ordner: &std::path::Path) {
    if state.statistik_dirty && state.statistik.speichern(ordner).is_ok() {
        state.statistik_dirty = false;
    }
}

/// Lädt Statistik und Ausnahmen eines Bandes.
pub fn band_daten_laden(band: &Band) -> (werkbank_kern::statistik::Statistik, werkbank_kern::sprache::Ausnahmen) {
    match &band.ordner {
        Some(o) => (
            werkbank_kern::statistik::Statistik::laden(o),
            werkbank_kern::sprache::Ausnahmen::laden(o),
        ),
        None => Default::default(),
    }
}

/// Wörter des offenen Kapitels laut Modell.
pub fn kapitel_woerter(projekt: &Projekt, id: &str) -> usize {
    projekt.kapitel_finden(id).map_or(0, |k| k.wortzahl())
}
