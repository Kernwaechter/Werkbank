//! # Zeitleiste: frühere Stände, Aufräumen und Papierkorb
//!
//! Baut auf den Sicherungen unter `.werkbank/backups/<Datei>/` (jedes bewusste Speichern)
//! und `.werkbank/snapshots/<Datei>/` (Autosave) auf. Neben den Dateien liegt je Ordner eine
//! `index.json` mit dem Entstehungszeitpunkt jedes Standes und frei vergebenen Namen.

use crate::{Projekt, Teil};
use chrono::{DateTime, Datelike, Local};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Alles aus den letzten Tagen bleibt beim Aufräumen unangetastet.
pub const BEHALTEN_TAGE: i64 = 7;
/// Bis zu diesem Alter bleibt je Tag ein Stand, danach je Woche.
pub const TAGESSTAND_BIS_TAGE: i64 = 90;

/// Woher ein Stand stammt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandArt {
    /// Durchnummerierte Sicherung beim Speichern.
    Sicherung,
    /// Zeitgestempelter Stand des Autosaves.
    Auto,
}

/// Ein früherer Stand einer Teil-Datei.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stand {
    pub pfad: PathBuf,
    pub art: StandArt,
    /// Laufende Nummer (nur bei Sicherungen, sonst 0).
    pub nummer: u32,
    /// Wann dieser Text zuletzt gespeichert wurde.
    pub zeit: DateTime<Local>,
    pub groesse: u64,
    /// Wörter des gewählten Kapitels in diesem Stand (`None`, wenn es dort fehlt).
    pub woerter: Option<usize>,
    /// Vom Autor vergebener Name.
    pub name: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Index {
    #[serde(default)]
    namen: BTreeMap<String, String>,
    #[serde(default)]
    zeiten: BTreeMap<String, String>,
}

fn index_lesen(ordner: &Path) -> Index {
    fs::read_to_string(ordner.join("index.json"))
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn index_schreiben(ordner: &Path, index: &Index) -> io::Result<()> {
    let text = serde_json::to_string_pretty(index).map_err(io::Error::other)?;
    crate::speicher::atomar_schreiben(&ordner.join("index.json"), &text)
}

/// Hält fest, wann der in `datei` gesicherte Stand entstand.
pub(crate) fn zeit_merken(ordner: &Path, datei: &str, zeit: DateTime<Local>) {
    let mut index = index_lesen(ordner);
    index.zeiten.insert(datei.to_string(), zeit.to_rfc3339());
    let _ = index_schreiben(ordner, &index);
}

fn zeit_lesen(text: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(text).ok().map(|z| z.with_timezone(&Local))
}

fn datei_zeit(pfad: &Path) -> Option<DateTime<Local>> {
    fs::metadata(pfad).and_then(|m| m.modified()).ok().map(DateTime::<Local>::from)
}

fn datei_name(pfad: &Path) -> String {
    pfad.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string()
}

/// `20261029-103015-teil-1.md` -> Zeitpunkt
fn zeit_aus_stempel(name: &str) -> Option<DateTime<Local>> {
    use chrono::{NaiveDateTime, TimeZone};
    let stempel = name.get(..15)?;
    let naiv = NaiveDateTime::parse_from_str(stempel, "%Y%m%d-%H%M%S").ok()?;
    Local.from_local_datetime(&naiv).earliest()
}

/// Bearbeitungsstand eines Papierkorb-Eintrags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KorbEintrag {
    pub pfad: PathBuf,
    /// Ursprünglicher Dateiname, z. B. `teil-2.md`.
    pub datei: String,
    /// Überschrift des Teils (erste Überschrift in der Datei).
    pub titel: String,
    pub zeit: DateTime<Local>,
    pub groesse: u64,
    pub woerter: usize,
}

/// Vorschau darauf, was Aufräumen löschen würde.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AufraeumPlan {
    pub zu_loeschen: Vec<PathBuf>,
    /// Bytes, die frei würden.
    pub frei: u64,
    /// Stände, die übrig bleiben.
    pub bleiben: usize,
}

impl Projekt {
    fn staende_roh(&self, teil: &Teil) -> Vec<Stand> {
        let mut liste = Vec::new();

        // Durchnummerierte Sicherungen
        let ordner = self.backup_ordner(teil);
        let index = index_lesen(&ordner);
        let mut dateien: Vec<(u32, PathBuf)> = crate::speicher::dateien_mit_endung(&ordner, "md")
            .into_iter()
            .map(|p| {
                let nr = datei_name(&p).trim_end_matches(".md").rsplit('-').next().and_then(|n| n.parse().ok()).unwrap_or(0);
                (nr, p)
            })
            .collect();
        dateien.sort_by_key(|(nr, _)| *nr);
        let mtimes: BTreeMap<u32, DateTime<Local>> = dateien.iter().filter_map(|(n, p)| Some((*n, datei_zeit(p)?))).collect();
        for (nr, pfad) in &dateien {
            let name = datei_name(pfad);
            // Ältere Sicherungen kennen ihren Zeitpunkt nicht: Der Stand entstand beim vorherigen Speichern,
            // also zur Änderungszeit der vorigen Sicherung.
            let zeit = index
                .zeiten
                .get(&name)
                .and_then(|z| zeit_lesen(z))
                .or_else(|| nr.checked_sub(1).and_then(|v| mtimes.get(&v).copied()))
                .or_else(|| mtimes.get(nr).copied());
            let Some(zeit) = zeit else { continue };
            liste.push(Stand {
                art: StandArt::Sicherung,
                nummer: *nr,
                zeit,
                groesse: fs::metadata(pfad).map(|m| m.len()).unwrap_or(0),
                woerter: None,
                name: index.namen.get(&name).cloned(),
                pfad: pfad.clone(),
            });
        }

        // Autosave
        for pfad in crate::speicher::dateien_mit_endung(&self.snapshot_ordner(teil), "md") {
            let Some(zeit) = zeit_aus_stempel(&datei_name(&pfad)).or_else(|| datei_zeit(&pfad)) else { continue };
            liste.push(Stand {
                art: StandArt::Auto,
                nummer: 0,
                zeit,
                groesse: fs::metadata(&pfad).map(|m| m.len()).unwrap_or(0),
                woerter: None,
                name: None,
                pfad,
            });
        }
        liste.sort_by(|a, b| b.zeit.cmp(&a.zeit).then(b.nummer.cmp(&a.nummer)));
        liste
    }

    /// Alle früheren Stände des Teils, zu dem das Kapitel gehört, neueste zuerst; mit den Wörtern
    /// des Kapitels im jeweiligen Stand.
    pub fn staende_fuer_kapitel(&self, kapitel_id: &str) -> Vec<Stand> {
        let Some(teil) = self.teile.iter().find(|t| t.kapitel.iter().any(|k| k.id == kapitel_id)) else {
            return Vec::new();
        };
        let mut liste = self.staende_roh(teil);
        for s in &mut liste {
            s.woerter = self.kapitel_aus_snapshot(&s.pfad, kapitel_id).map(|t| t.split_whitespace().count());
        }
        liste
    }

    /// Vergibt (oder löscht mit leerem Namen) den Namen einer Sicherung.
    pub fn stand_benennen(&self, pfad: &Path, name: &str) -> io::Result<()> {
        let Some(ordner) = pfad.parent() else { return Ok(()) };
        let datei = datei_name(pfad);
        let mut index = index_lesen(ordner);
        let name = name.trim();
        if name.is_empty() {
            index.namen.remove(&datei);
        } else {
            index.namen.insert(datei, name.chars().take(60).collect());
        }
        index_schreiben(ordner, &index)
    }

    /// Anzahl und Größe aller Stände (Sicherungen und Autosave) des Bandes.
    pub fn speicher_uebersicht(&self) -> (usize, u64) {
        self.teile
            .iter()
            .flat_map(|t| self.staende_roh(t))
            .fold((0, 0), |(n, b), s| (n + 1, b + s.groesse))
    }

    /// Stellt zusammen, was Aufräumen löschen würde. Es ändert nichts.
    ///
    /// Regel: die letzten [`BEHALTEN_TAGE`] Tage bleiben vollständig, bis [`TAGESSTAND_BIS_TAGE`] Tage
    /// bleibt je Tag der letzte Stand, danach je Woche. Benannte Stände bleiben immer.
    pub fn aufraeumen_planen(&self) -> AufraeumPlan {
        self.aufraeumen_planen_ab(Local::now())
    }

    pub(crate) fn aufraeumen_planen_ab(&self, jetzt: DateTime<Local>) -> AufraeumPlan {
        let mut plan = AufraeumPlan::default();
        for teil in &self.teile {
            // Neueste zuerst: Der erste Stand je Tag oder Woche bleibt
            let mut gesehen: BTreeSet<(u8, i64)> = BTreeSet::new();
            for s in self.staende_roh(teil) {
                let alter = (jetzt.date_naive() - s.zeit.date_naive()).num_days();
                let schluessel = if alter < BEHALTEN_TAGE {
                    None
                } else if alter < TAGESSTAND_BIS_TAGE {
                    Some((0, i64::from(s.zeit.num_days_from_ce())))
                } else {
                    Some((1, i64::from(s.zeit.num_days_from_ce()).div_euclid(7)))
                };
                let behalten = s.name.is_some()
                    || match schluessel {
                        None => true,
                        Some(k) => gesehen.insert(k),
                    };
                if behalten {
                    plan.bleiben += 1;
                } else {
                    plan.frei += s.groesse;
                    plan.zu_loeschen.push(s.pfad);
                }
            }
        }
        plan
    }

    /// Führt einen Plan aus. Die Zeitpunkte der verbleibenden Sicherungen werden vorher festgeschrieben,
    /// damit sie nicht von gelöschten Nachbarn abhängen. Liefert die Zahl gelöschter Dateien.
    pub fn aufraeumen_ausfuehren(&self, plan: &AufraeumPlan) -> io::Result<usize> {
        for teil in &self.teile {
            let ordner = self.backup_ordner(teil);
            let mut index = index_lesen(&ordner);
            let mut geaendert = false;
            for s in self.staende_roh(teil).into_iter().filter(|s| s.art == StandArt::Sicherung) {
                let name = datei_name(&s.pfad);
                if plan.zu_loeschen.contains(&s.pfad) {
                    geaendert |= index.zeiten.remove(&name).is_some() | index.namen.remove(&name).is_some();
                } else if !index.zeiten.contains_key(&name) {
                    index.zeiten.insert(name, s.zeit.to_rfc3339());
                    geaendert = true;
                }
            }
            if geaendert {
                fs::create_dir_all(&ordner)?;
                index_schreiben(&ordner, &index)?;
            }
        }
        let mut n = 0;
        for pfad in &plan.zu_loeschen {
            if fs::remove_file(pfad).is_ok() {
                n += 1;
            }
        }
        Ok(n)
    }

    /// Legt aus altem Kapiteltext ein neues Kapitel hinter `nach` an (Kopie eines früheren Standes).
    /// Die Überschrift bekommt den Titel `titel`; Liefert die neue Kennung.
    pub fn kapitel_kopie_einfuegen(&mut self, teil_id: &str, nach: &str, titel: &str, alter_text: &str) -> Result<String, String> {
        let id = self.neues_kapitel(teil_id, titel, Some(nach))?;
        let k = self.kapitel_finden_mut(&id).ok_or("Kapitel nicht gefunden")?;
        let kopf = k.inhalt.lines().next().unwrap_or_default().to_string();
        let rest = alter_text.split_once('\n').map_or("", |(_, r)| r);
        k.inhalt = format!("{kopf}\n{rest}");
        if !k.inhalt.ends_with('\n') {
            k.inhalt.push('\n');
        }
        Ok(id)
    }

    // ------------------------------------------------------------ Papierkorb

    /// Entfernte Teile, neueste zuerst.
    pub fn papierkorb_liste(&self) -> Vec<KorbEintrag> {
        let mut liste: Vec<KorbEintrag> = crate::speicher::dateien_mit_endung(&self.werkbank_ordner().join("papierkorb"), "md")
            .into_iter()
            .filter_map(|pfad| {
                let name = datei_name(&pfad);
                let zeit = zeit_aus_stempel(&name).or_else(|| datei_zeit(&pfad))?;
                let datei = name.get(16..).filter(|_| zeit_aus_stempel(&name).is_some()).unwrap_or(&name).to_string();
                let text = fs::read_to_string(&pfad).unwrap_or_default();
                let titel = text
                    .lines()
                    .find(|z| z.starts_with('#'))
                    .map(|z| z.trim_start_matches('#').split('{').next().unwrap_or_default().trim().to_string())
                    .filter(|t| !t.is_empty())
                    .unwrap_or_else(|| datei.clone());
                Some(KorbEintrag {
                    groesse: fs::metadata(&pfad).map(|m| m.len()).unwrap_or(0),
                    woerter: text.split_whitespace().count(),
                    pfad,
                    datei,
                    titel,
                    zeit,
                })
            })
            .collect();
        liste.sort_by(|a, b| b.zeit.cmp(&a.zeit));
        liste
    }

    /// Holt einen entfernten Teil zurück in den Bandordner und trägt ihn am Ende der
    /// `reihenfolge.txt` ein. Liefert den Dateinamen. Das Projekt muss danach neu geladen werden.
    pub fn papierkorb_zurueckholen(&self, eintrag: &KorbEintrag) -> io::Result<String> {
        let (stamm, endung) = eintrag.datei.rsplit_once('.').unwrap_or((eintrag.datei.as_str(), "md"));
        let mut name = eintrag.datei.clone();
        let mut n = 1;
        while self.wurzelverzeichnis.join(&name).exists() {
            name = if n == 1 { format!("{stamm}-zurueck.{endung}") } else { format!("{stamm}-zurueck-{n}.{endung}") };
            n += 1;
        }
        fs::copy(&eintrag.pfad, self.wurzelverzeichnis.join(&name))?;
        let reihenfolge = self.wurzelverzeichnis.join("reihenfolge.txt");
        if reihenfolge.is_file() {
            let mut text = fs::read_to_string(&reihenfolge)?;
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&name);
            text.push('\n');
            crate::speicher::atomar_schreiben(&reihenfolge, &text)?;
        }
        fs::remove_file(&eintrag.pfad)
            .map(|()| name)
    }

    /// Löscht einen Papierkorb-Eintrag endgültig.
    pub fn papierkorb_loeschen(&self, eintrag: &KorbEintrag) -> io::Result<()> {
        fs::remove_file(&eintrag.pfad)
    }

    /// Der `.werkbank`-Ordner des Bandes (zum Anzeigen im Dateimanager).
    pub fn sicherungsordner(&self) -> PathBuf {
        self.werkbank_ordner()
    }

    /// Text eines Kapitels in einem früheren Stand.
    pub fn kapitel_text_im_stand(&self, stand: &Stand, kapitel_id: &str) -> Option<String> {
        self.kapitel_aus_snapshot(&stand.pfad, kapitel_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const TEIL: &str = "# Teil 1: Anfang {-}\n\nEinleitung.\n\n## Kapitel [eins]: Erstes\n\nText eins.\n\n## Kapitel [zwei]: Zweites\n\nText zwei.\n";

    fn projekt(name: &str) -> (PathBuf, Projekt) {
        let o = std::env::temp_dir().join(format!("werkbank-verlauf-{name}"));
        let _ = fs::remove_dir_all(&o);
        fs::create_dir_all(&o).unwrap();
        fs::write(o.join("teil-1.md"), TEIL).unwrap();
        fs::write(o.join("reihenfolge.txt"), "# Reihenfolge\nteil-1.md\n").unwrap();
        let p = Projekt::aus_verzeichnis(&o).unwrap();
        (o, p)
    }

    #[test]
    fn zeitleiste_kennt_zeit_woerter_und_namen() {
        let (_o, mut p) = projekt("zeit");
        for i in 1..=3 {
            p.kapitel_finden_mut("eins").unwrap().inhalt.push_str(&format!("\nZusatz {i}.\n"));
            p.teil_speichern_nummeriert("teil-1").unwrap();
        }
        let liste = p.staende_fuer_kapitel("eins");
        assert_eq!(liste.len(), 3);
        assert_eq!(liste[0].nummer, 3, "neueste zuerst");
        assert!(liste[0].woerter.unwrap() > liste[2].woerter.unwrap());
        assert!(liste.iter().all(|s| s.art == StandArt::Sicherung));
        p.stand_benennen(&liste[1].pfad, "  Vor Lektorat ").unwrap();
        assert_eq!(p.staende_fuer_kapitel("eins")[1].name.as_deref(), Some("Vor Lektorat"));
        p.stand_benennen(&liste[1].pfad, "").unwrap();
        assert_eq!(p.staende_fuer_kapitel("eins")[1].name, None);
        assert!(p.staende_fuer_kapitel("gibt-es-nicht").is_empty());
    }

    #[test]
    fn aufraeumen_haelt_die_regel_ein() {
        let (o, p) = projekt("aufraeumen");
        let ordner = o.join(".werkbank/backups/teil-1");
        fs::create_dir_all(&ordner).unwrap();
        let jetzt = Local::now();
        // Zeiten: 3 heute, 3 am gleichen Tag vor 20 Tagen, 2 am gleichen Tag vor 200 Tagen (gleiche Woche)
        let tage_und_nr = [(0, 1), (0, 2), (0, 3), (20, 4), (20, 5), (20, 6), (200, 7), (200, 8)];
        let mut index = Index::default();
        for (tage, nr) in tage_und_nr {
            let name = format!("teil-1-{nr:04}.md");
            fs::write(ordner.join(&name), format!("Stand {nr}")).unwrap();
            let zeit = jetzt - chrono::Duration::days(tage) + chrono::Duration::minutes(i64::from(nr));
            index.zeiten.insert(name, zeit.to_rfc3339());
        }
        index.namen.insert("teil-1-0004.md".into(), "Erstfassung".into());
        index_schreiben(&ordner, &index).unwrap();
        //
        let plan = p.aufraeumen_planen();
        let geloescht: Vec<String> = plan.zu_loeschen.iter().map(|x| datei_name(x)).collect();
        // heute: alle bleiben; vor 20 Tagen: neuester (6) und der benannte (4) bleiben, 5 geht; vor 200 Tagen: 8 bleibt, 7 geht
        let mut g = geloescht.clone();
        g.sort();
        assert_eq!(g, vec!["teil-1-0005.md", "teil-1-0007.md"]);
        assert_eq!(plan.bleiben, 6);
        assert_eq!(plan.frei, 2 * "Stand 5".len() as u64);
        assert_eq!(p.aufraeumen_ausfuehren(&plan).unwrap(), 2);
        assert!(!ordner.join("teil-1-0005.md").exists());
        assert!(ordner.join("teil-1-0004.md").exists());
    }

    #[test]
    fn papierkorb_zurueckholen_traegt_in_die_reihenfolge_ein() {
        let (o, mut p) = projekt("korb");
        let neu = p.neuer_teil("Zweiter").unwrap();
        p.teile_speichern(&[neu.clone()], Duration::ZERO).unwrap();
        p.teil_entfernen(&neu).unwrap();
        let korb = p.papierkorb_liste();
        assert_eq!(korb.len(), 1);
        assert_eq!(korb[0].datei, "teil-2.md");
        let name = p.papierkorb_zurueckholen(&korb[0]).unwrap();
        assert_eq!(name, "teil-2.md");
        assert!(o.join("teil-2.md").is_file());
        assert!(p.papierkorb_liste().is_empty());
        assert!(fs::read_to_string(o.join("reihenfolge.txt")).unwrap().ends_with("teil-1.md\nteil-2.md\n"));
    }

    #[test]
    fn kopie_eines_kapitels_bekommt_neue_ueberschrift() {
        let (_o, mut p) = projekt("kopie");
        let alt = p.kapitel_finden("eins").unwrap().inhalt.clone();
        let id = p.kapitel_kopie_einfuegen("teil-1", "eins", "Erstes (Stand 03.11.)", &alt).unwrap();
        let k = p.kapitel_finden(&id).unwrap();
        assert!(k.inhalt.starts_with("## Kapitel [erstes-stand-03-11]: Erstes (Stand 03.11.)\n"), "{}", k.inhalt);
        assert!(k.inhalt.contains("Text eins."));
    }
}
