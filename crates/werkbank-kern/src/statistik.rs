//! # Schreibstatistik
//!
//! Tageswerte (neu geschriebene und gestrichene Wörter, Schreibzeit, Sitzungen),
//! gespeichert in `.werkbank/statistik.json`, und daraus abgeleitete Kennzahlen:
//! Netto-Wörter, Tempo, Streak, Heatmap und Prognose.

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

/// Pause, ab der eine neue Sitzung beginnt.
pub const SITZUNGSPAUSE_SEKUNDEN: u64 = 5 * 60;

/// Werte eines Kalendertags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    /// Neu geschriebene Wörter.
    pub hinzu: u64,
    /// Gestrichene Wörter.
    pub entfernt: u64,
    /// Aktive Schreibzeit in Sekunden.
    pub sekunden: u64,
    /// Anzahl der Sitzungen.
    pub sitzungen: u32,
}

impl Tag {
    /// Netto-Wörter: neu minus gestrichen (beim Umschreiben die ehrliche Zahl).
    pub fn netto(&self) -> i64 {
        self.hinzu as i64 - self.entfernt as i64
    }
}

/// Schreibzeit und Wörter, die in einem Kapitel angefallen sind.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KapitelZeit {
    /// Aktive Schreibzeit in Sekunden.
    pub sekunden: u64,
    /// Neu geschriebene Wörter.
    pub hinzu: u64,
    /// Gestrichene Wörter.
    pub entfernt: u64,
}

impl KapitelZeit {
    /// Netto-Wörter pro Stunde (`None`, solange weniger als eine Minute erfasst ist).
    pub fn woerter_pro_stunde(&self) -> Option<f32> {
        (self.sekunden >= 60).then(|| self.hinzu.saturating_sub(self.entfernt) as f32 / (self.sekunden as f32 / 3600.0))
    }
}

/// Alle Tageswerte eines Bandes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Statistik {
    /// Schlüssel im Format `JJJJ-MM-TT`.
    pub tage: BTreeMap<String, Tag>,
    /// Schreibzeit und Wörter je Kapitel (Schlüssel: Kapitelkennung).
    #[serde(default)]
    pub kapitel: BTreeMap<String, KapitelZeit>,
}

fn schluessel(datum: NaiveDate) -> String {
    datum.format("%Y-%m-%d").to_string()
}

impl Statistik {
    /// Lädt die Statistik eines Bandordners (leer, wenn keine vorhanden).
    pub fn laden(bandordner: &Path) -> Self {
        fs::read_to_string(bandordner.join(".werkbank").join("statistik.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    /// Speichert die Statistik.
    pub fn speichern(&self, bandordner: &Path) -> io::Result<()> {
        let ordner = bandordner.join(".werkbank");
        fs::create_dir_all(&ordner)?;
        let text = serde_json::to_string_pretty(self).map_err(io::Error::other)?;
        let temp = ordner.join(".statistik.tmp");
        fs::write(&temp, text)?;
        fs::rename(temp, ordner.join("statistik.json"))
    }

    /// Veränderbarer Eintrag für einen Tag.
    pub fn tag_mut(&mut self, datum: NaiveDate) -> &mut Tag {
        self.tage.entry(schluessel(datum)).or_default()
    }

    /// Werte eines Tages (leer, wenn nichts erfasst).
    pub fn tag(&self, datum: NaiveDate) -> Tag {
        self.tage.get(&schluessel(datum)).cloned().unwrap_or_default()
    }

    /// Verbucht die Änderung der Wortzahl eines Kapitels.
    pub fn wortaenderung(&mut self, datum: NaiveDate, vorher: usize, nachher: usize) {
        let t = self.tag_mut(datum);
        if nachher >= vorher {
            t.hinzu += (nachher - vorher) as u64;
        } else {
            t.entfernt += (vorher - nachher) as u64;
        }
    }

    /// Verbucht Schreibzeit. `luecke` ist die Zeit seit der letzten Eingabe (`None` = erste Eingabe).
    pub fn aktivitaet(&mut self, datum: NaiveDate, luecke_sekunden: Option<u64>) {
        let t = self.tag_mut(datum);
        match luecke_sekunden {
            Some(s) if s < SITZUNGSPAUSE_SEKUNDEN => t.sekunden += s,
            _ => t.sitzungen += 1,
        }
    }

    /// Verbucht eine Änderung der Wortzahl und die Zeit seit der letzten Eingabe für ein Kapitel.
    pub fn kapitel_erfassen(&mut self, kapitel_id: &str, vorher: usize, nachher: usize, luecke_sekunden: Option<u64>) {
        let k = self.kapitel.entry(kapitel_id.to_string()).or_default();
        if nachher >= vorher {
            k.hinzu += (nachher - vorher) as u64;
        } else {
            k.entfernt += (vorher - nachher) as u64;
        }
        if let Some(s) = luecke_sekunden.filter(|s| *s < SITZUNGSPAUSE_SEKUNDEN) {
            k.sekunden += s;
        }
    }

    fn zeitraum(&self, heute: NaiveDate, tage: i64) -> impl Iterator<Item = Tag> + '_ {
        (0..tage).map(move |i| self.tag(heute - Duration::days(i)))
    }

    /// Netto-Wörter der letzten `tage` Tage (heute eingeschlossen).
    pub fn netto_summe(&self, heute: NaiveDate, tage: i64) -> i64 {
        self.zeitraum(heute, tage).map(|t| t.netto()).sum()
    }

    /// Schreibzeit der letzten `tage` Tage in Sekunden.
    pub fn sekunden_summe(&self, heute: NaiveDate, tage: i64) -> u64 {
        self.zeitraum(heute, tage).map(|t| t.sekunden).sum()
    }

    /// Durchschnittliche Netto-Wörter pro Tag (alle Tage zählen, auch ohne Schreiben).
    pub fn schnitt(&self, heute: NaiveDate, tage: i64) -> f32 {
        self.netto_summe(heute, tage) as f32 / tage.max(1) as f32
    }

    /// Neu geschriebene Wörter pro Stunde im Zeitraum (`None` ohne erfasste Schreibzeit).
    pub fn woerter_pro_stunde(&self, heute: NaiveDate, tage: i64) -> Option<f32> {
        let sekunden = self.sekunden_summe(heute, tage);
        let hinzu: u64 = self.zeitraum(heute, tage).map(|t| t.hinzu).sum();
        (sekunden >= 60).then(|| hinzu as f32 / (sekunden as f32 / 3600.0))
    }

    /// Aufeinanderfolgende Tage mit positivem Netto bis heute (ist heute noch leer, zählt es ab gestern).
    pub fn streak(&self, heute: NaiveDate) -> u32 {
        let mut tag = if self.tag(heute).netto() > 0 { heute } else { heute - Duration::days(1) };
        let mut n = 0;
        while self.tag(tag).netto() > 0 {
            n += 1;
            tag -= Duration::days(1);
        }
        n
    }

    /// Wochenspalten (Montag bis Sonntag) der letzten `wochen` Wochen mit dem Netto je Tag.
    pub fn heatmap(&self, heute: NaiveDate, wochen: i64) -> Vec<Vec<(NaiveDate, i64)>> {
        let montag = heute - Duration::days(heute.weekday().num_days_from_monday() as i64);
        (0..wochen)
            .rev()
            .map(|w| {
                let start = montag - Duration::weeks(w);
                (0..7)
                    .map(|d| {
                        let tag = start + Duration::days(d);
                        (tag, if tag > heute { -1 } else { self.tag(tag).netto() })
                    })
                    .collect()
            })
            .collect()
    }

    /// Voraussichtliches Fertigstellungsdatum bei dem bisherigen Tempo.
    pub fn prognose(rest_woerter: usize, schnitt_pro_tag: f32, heute: NaiveDate) -> Option<NaiveDate> {
        if rest_woerter == 0 {
            return Some(heute);
        }
        (schnitt_pro_tag >= 1.0).then(|| {
            let tage = (rest_woerter as f32 / schnitt_pro_tag).ceil() as i64;
            heute + Duration::days(tage)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(t: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, t).unwrap()
    }

    #[test]
    fn netto_streak_und_schnitt() {
        let mut s = Statistik::default();
        s.wortaenderung(d(27), 0, 500);
        s.wortaenderung(d(28), 100, 700);
        s.wortaenderung(d(28), 700, 600);
        s.wortaenderung(d(29), 0, 300);
        assert_eq!(s.tag(d(28)).netto(), 500);
        assert_eq!(s.streak(d(29)), 3);
        assert_eq!(s.streak(d(30)), 3, "heute noch leer zählt ab gestern");
        assert_eq!(s.streak(d(31)), 0);
        assert_eq!(s.netto_summe(d(29), 7), 1300);
        assert!((s.schnitt(d(29), 4) - 325.0).abs() < 0.01);
    }

    #[test]
    fn kapitel_tempo() {
        let mut s = Statistik::default();
        s.kapitel_erfassen("k1", 0, 600, Some(200));
        s.kapitel_erfassen("k1", 600, 500, Some(100));
        s.kapitel_erfassen("k1", 500, 1100, Some(4000)); // lange Pause zählt nicht als Schreibzeit
        let k = &s.kapitel["k1"];
        assert_eq!((k.hinzu, k.entfernt, k.sekunden), (1200, 100, 300));
        assert_eq!(k.woerter_pro_stunde(), Some(13200.0));
        assert_eq!(KapitelZeit::default().woerter_pro_stunde(), None);
    }

    #[test]
    fn sitzungen_und_tempo() {
        let mut s = Statistik::default();
        s.aktivitaet(d(29), None);
        s.aktivitaet(d(29), Some(30));
        s.aktivitaet(d(29), Some(30 * 60));
        s.tag_mut(d(29)).sekunden = 3600;
        s.tag_mut(d(29)).hinzu = 900;
        assert_eq!(s.tag(d(29)).sitzungen, 2);
        assert_eq!(s.woerter_pro_stunde(d(29), 1), Some(900.0));
    }

    #[test]
    fn heatmap_und_prognose() {
        let s = Statistik::default();
        let h = s.heatmap(d(29), 4);
        assert_eq!(h.len(), 4);
        assert!(h.iter().all(|w| w.len() == 7));
        assert_eq!(h[3][3].0, d(29), "29.10.2026 ist ein Donnerstag");
        assert_eq!(h[3][4].1, -1, "Zukunft");
        assert_eq!(Statistik::prognose(1000, 100.0, d(29)), Some(d(29) + Duration::days(10)));
        assert_eq!(Statistik::prognose(1000, 0.0, d(29)), None);
    }
}
