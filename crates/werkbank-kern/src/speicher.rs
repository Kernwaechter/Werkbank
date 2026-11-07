//! # Speichern, Snapshots und Struktur
//!
//! Schreibt geänderte Teile atomar auf die Platte (mit gedrosselten Snapshots
//! unter `.werkbank/snapshots/`), führt Kapitel- und Teilstruktur nach
//! (`reihenfolge.txt`) und speichert Kapitel-Metadaten (Status, Wortziel,
//! Notizen) getrennt vom Text in `.werkbank/kapitel.json`.

use crate::io::{parse_kapitel_kopf, teil_aus_datei_inhalt};
use crate::{BandMetadaten, Kapitel, KapitelStatus, Projekt, Teil};
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Höchstens so viele Snapshots bleiben je Datei erhalten.
const MAX_SNAPSHOTS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KapitelMeta {
    wortziel: usize,
    status: KapitelStatus,
    #[serde(default)]
    notizen: String,
}

/// Ein gesicherter früherer Stand einer Teil-Datei.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Pfad der Sicherung.
    pub pfad: PathBuf,
    /// Lesbarer Zeitpunkt, z. B. `29.10.2026 10:30:15`.
    pub zeit: String,
    /// Größe in Bytes.
    pub groesse: u64,
    /// Sortierschlüssel (neueste zuerst).
    pub sortierung: String,
}

fn io_fehler(text: impl Into<String>) -> Error {
    Error::other(text.into())
}

/// Wandelt einen Titel in eine Kennung (`Größe & Maß` -> `groesse-mass`).
pub fn kennung_aus_titel(titel: &str) -> String {
    let mut id = String::new();
    for c in titel.to_lowercase().chars() {
        match c {
            'ä' => id.push_str("ae"),
            'ö' => id.push_str("oe"),
            'ü' => id.push_str("ue"),
            'ß' => id.push_str("ss"),
            c if c.is_ascii_alphanumeric() => id.push(c),
            _ if !id.ends_with('-') && !id.is_empty() => id.push('-'),
            _ => {}
        }
    }
    let id = id.trim_end_matches('-').to_string();
    if id.is_empty() { "kapitel".to_string() } else { id }
}

impl Projekt {
    pub(crate) fn werkbank_ordner(&self) -> PathBuf {
        self.wurzelverzeichnis.join(".werkbank")
    }

    // ---------------------------------------------------------------- Metadaten

    /// Wendet gespeicherte Kapitel-Metadaten (Status, Ziel, Notizen) an.
    pub fn kapitel_meta_laden(&mut self) {
        let Ok(text) = fs::read_to_string(self.werkbank_ordner().join("kapitel.json")) else {
            return;
        };
        let Ok(karte) = serde_json::from_str::<BTreeMap<String, KapitelMeta>>(&text) else {
            return;
        };
        for k in self.teile.iter_mut().flat_map(|t| t.kapitel.iter_mut()) {
            if let Some(m) = karte.get(&k.id) {
                k.wortziel = m.wortziel;
                k.status = m.status;
                k.notizen = m.notizen.clone();
            }
        }
    }

    /// Speichert Status, Wortziel und Notizen aller Kapitel.
    pub fn kapitel_meta_speichern(&self) -> io::Result<()> {
        let karte: BTreeMap<&str, KapitelMeta> = self
            .teile
            .iter()
            .flat_map(|t| t.kapitel.iter())
            .map(|k| {
                (
                    k.id.as_str(),
                    KapitelMeta { wortziel: k.wortziel, status: k.status, notizen: k.notizen.clone() },
                )
            })
            .collect();
        let ordner = self.werkbank_ordner();
        fs::create_dir_all(&ordner)?;
        let text = serde_json::to_string_pretty(&karte).map_err(io_fehler_von)?;
        atomar_schreiben(&ordner.join("kapitel.json"), &text)
    }

    /// Liest die Buchdaten aus `band.yaml` und `.werkbank/metadaten.yaml` (Letztere gewinnt).
    /// Fremde `band.yaml`-Dateien mit anderem Aufbau werden ignoriert, nie verändert.
    pub fn metadaten_aus_band_yaml(&mut self) {
        for datei in [
            self.wurzelverzeichnis.join("band.yaml"),
            self.werkbank_ordner().join("metadaten.yaml"),
        ] {
            if let Ok(text) = fs::read_to_string(datei)
                && let Ok(m) = serde_yaml::from_str::<BandMetadaten>(&text)
            {
                self.metadaten = m;
            }
        }
    }

    /// Speichert die Buchdaten. Eine vorhandene `band.yaml` wird nur überschrieben, wenn sie
    /// denselben Aufbau hat; sonst landen die Daten in `.werkbank/metadaten.yaml`.
    /// Liefert die geschriebene Datei.
    pub fn band_yaml_speichern(&self) -> io::Result<PathBuf> {
        let band_yaml = self.wurzelverzeichnis.join("band.yaml");
        let eigen = !band_yaml.is_file()
            || fs::read_to_string(&band_yaml).is_ok_and(|t| serde_yaml::from_str::<BandMetadaten>(&t).is_ok());
        let ziel = if eigen { band_yaml } else { self.werkbank_ordner().join("metadaten.yaml") };
        if ziel.is_file() {
            let ordner = self.werkbank_ordner().join("snapshots").join("metadaten");
            fs::create_dir_all(&ordner)?;
            fs::copy(&ziel, ordner.join(format!("{}-metadaten.yaml", Local::now().format("%Y%m%d-%H%M%S"))))?;
        }
        let text = serde_yaml::to_string(&self.metadaten).map_err(io_fehler_von)?;
        atomar_schreiben(&ziel, &text)?;
        Ok(ziel)
    }

    // ---------------------------------------------------------------- Speichern

    /// Speichert einen Teil, falls sich sein Inhalt von der Datei unterscheidet.
    ///
    /// Vor dem Überschreiben wird der alte Stand gesichert, sofern der letzte Snapshot
    /// älter als `snapshot_abstand` ist. Liefert `true`, wenn geschrieben wurde.
    pub fn teil_speichern(&self, teil_id: &str, snapshot_abstand: Duration) -> io::Result<bool> {
        let teil = self
            .teile
            .iter()
            .find(|t| t.id == teil_id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("Teil „{teil_id}“ nicht gefunden")))?;
        let pfad = self.wurzelverzeichnis.join(&teil.dateipfad);
        let neu = teil.rekonstruiere_datei_inhalt();

        if pfad.is_file() {
            let alt = fs::read_to_string(&pfad)?;
            if alt == neu {
                return Ok(false);
            }
            let ordner = self.snapshot_ordner(teil);
            if snapshot_faellig(&ordner, snapshot_abstand) {
                fs::create_dir_all(&ordner)?;
                let name = format!("{}-{}.md", Local::now().format("%Y%m%d-%H%M%S"), teil.id);
                fs::write(ordner.join(name), alt)?;
                snapshots_beschneiden(&ordner);
            }
        }
        atomar_schreiben(&pfad, &neu)?;
        Ok(true)
    }

    /// Speichert mehrere Teile. Fehler bei einem Teil stoppen die anderen nicht.
    /// Liefert die tatsächlich geschriebenen Teil-Kennungen oder eine Fehlermeldung.
    pub fn teile_speichern(&self, ids: &[String], snapshot_abstand: Duration) -> Result<Vec<String>, String> {
        let mut geschrieben = Vec::new();
        let mut fehler = Vec::new();
        for id in ids {
            match self.teil_speichern(id, snapshot_abstand) {
                Ok(true) => geschrieben.push(id.clone()),
                Ok(false) => {}
                Err(e) => fehler.push(format!("{id}: {e}")),
            }
        }
        if fehler.is_empty() { Ok(geschrieben) } else { Err(fehler.join("; ")) }
    }

    /// Ordner der durchnummerierten Sicherungen eines Teils: `.werkbank/backups/<Dateiname>/`.
    pub(crate) fn backup_ordner(&self, teil: &Teil) -> PathBuf {
        self.werkbank_ordner()
            .join("backups")
            .join(teil.dateipfad.file_stem().unwrap_or_default())
    }

    /// Speichert einen Teil und legt **bei jeder Änderung** den alten Stand als durchnummerierte
    /// Sicherung ab (`<Datei>-0001.md`, `-0002.md`, …). Es wird nichts gedrosselt und nichts gelöscht.
    ///
    /// Liefert `None`, wenn sich nichts geändert hat, sonst die Nummer der neuen Sicherung
    /// (`Some(0)`, wenn die Datei neu angelegt wurde und es keinen alten Stand gab).
    pub fn teil_speichern_nummeriert(&self, teil_id: &str) -> io::Result<Option<u32>> {
        let teil = self
            .teile
            .iter()
            .find(|t| t.id == teil_id)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("Teil „{teil_id}“ nicht gefunden")))?;
        let pfad = self.wurzelverzeichnis.join(&teil.dateipfad);
        let neu = teil.rekonstruiere_datei_inhalt();

        let mut nummer = 0;
        if pfad.is_file() {
            let alt = fs::read_to_string(&pfad)?;
            if alt == neu {
                return Ok(None);
            }
            let ordner = self.backup_ordner(teil);
            fs::create_dir_all(&ordner)?;
            let stem = teil.dateipfad.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            nummer = naechste_backup_nummer(&ordner, &stem);
            // Erst sichern, dann überschreiben: schlägt die Sicherung fehl, bleibt die Datei unberührt
            let name = format!("{stem}-{nummer:04}.md");
            fs::write(ordner.join(&name), alt)?;
            // Wann der gesicherte Stand entstand: Er wurde zuletzt geschrieben, als die Datei ihre jetzige Änderungszeit bekam
            if let Ok(zeit) = fs::metadata(&pfad).and_then(|m| m.modified()) {
                crate::verlauf::zeit_merken(&ordner, &name, chrono::DateTime::<Local>::from(zeit));
            }
        }
        atomar_schreiben(&pfad, &neu)?;
        Ok(Some(nummer))
    }

    /// Wie [`Projekt::teile_speichern`], aber mit nummerierten Sicherungen. Liefert (Teil, Sicherungsnummer).
    pub fn teile_speichern_nummeriert(&self, ids: &[String]) -> Result<Vec<(String, u32)>, String> {
        let mut geschrieben = Vec::new();
        let mut fehler = Vec::new();
        for id in ids {
            match self.teil_speichern_nummeriert(id) {
                Ok(Some(n)) => geschrieben.push((id.clone(), n)),
                Ok(None) => {}
                Err(e) => fehler.push(format!("{id}: {e}")),
            }
        }
        if fehler.is_empty() { Ok(geschrieben) } else { Err(fehler.join("; ")) }
    }

    pub(crate) fn snapshot_ordner(&self, teil: &Teil) -> PathBuf {
        self.werkbank_ordner()
            .join("snapshots")
            .join(teil.dateipfad.file_stem().unwrap_or_default())
    }

    /// Alle Snapshots eines Teils, neueste zuerst.
    pub fn snapshots_fuer_teil(&self, teil_id: &str) -> Vec<Snapshot> {
        let Some(teil) = self.teile.iter().find(|t| t.id == teil_id) else {
            return Vec::new();
        };
        let mut liste: Vec<Snapshot> = Vec::new();

        // Zeitgestempelte Snapshots (Autosave und ältere Stände)
        for pfad in dateien_mit_endung(&self.snapshot_ordner(teil), "md") {
            let name = pfad.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            liste.push(Snapshot {
                zeit: stempel_lesbar(name),
                groesse: fs::metadata(&pfad).map(|m| m.len()).unwrap_or(0),
                sortierung: name.to_string(),
                pfad,
            });
        }
        // Durchnummerierte Sicherungen jedes Speicherns
        for pfad in dateien_mit_endung(&self.backup_ordner(teil), "md") {
            let name = pfad.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
            let nr = name.trim_end_matches(".md").rsplit('-').next().and_then(|n| n.parse::<u32>().ok()).unwrap_or(0);
            let gespeichert = fs::metadata(&pfad).and_then(|m| m.modified()).map(chrono::DateTime::<Local>::from);
            liste.push(Snapshot {
                zeit: format!(
                    "Sicherung Nr. {nr} · {}",
                    gespeichert.as_ref().map_or(String::new(), |t| t.format("%d.%m.%Y %H:%M:%S").to_string())
                ),
                groesse: fs::metadata(&pfad).map(|m| m.len()).unwrap_or(0),
                sortierung: gespeichert.map_or(String::new(), |t| t.format("%Y%m%d-%H%M%S").to_string()) + &format!("-{nr:08}"),
                pfad,
            });
        }
        liste.sort_by(|a, b| b.sortierung.cmp(&a.sortierung));
        liste
    }

    /// Liest den Text eines Kapitels aus einem Snapshot (erst nach Kennung, dann nach Titel).
    pub fn kapitel_aus_snapshot(&self, snapshot: &Path, kapitel_id: &str) -> Option<String> {
        let text = fs::read_to_string(snapshot).ok()?;
        let titel = self.kapitel_finden(kapitel_id).map(|k| k.titel.clone());
        let mut nr = 1;
        let teil = teil_aus_datei_inhalt("snapshot", "snapshot.md", &text, &mut nr);
        teil.kapitel
            .iter()
            .find(|k| k.id == kapitel_id)
            .or_else(|| teil.kapitel.iter().find(|k| Some(&k.titel) == titel.as_ref()))
            .map(|k| k.inhalt.clone())
    }

    // ---------------------------------------------------------------- Struktur

    fn teil_index_von_kapitel(&self, kapitel_id: &str) -> Option<usize> {
        self.teile.iter().position(|t| t.kapitel.iter().any(|k| k.id == kapitel_id))
    }

    /// Teil-Kennung, zu der ein Kapitel gehört.
    pub fn teil_von_kapitel(&self, kapitel_id: &str) -> Option<&str> {
        self.teil_index_von_kapitel(kapitel_id).map(|i| self.teile[i].id.as_str())
    }

    fn eindeutige_kennung(&self, basis: &str) -> String {
        let vorhanden = |id: &str| self.teile.iter().any(|t| t.kapitel.iter().any(|k| k.id == id));
        if !vorhanden(basis) {
            return basis.to_string();
        }
        (2..).map(|n| format!("{basis}-{n}")).find(|id| !vorhanden(id)).unwrap_or_default()
    }

    /// Legt ein Kapitel an (hinter `nach`, sonst am Ende des Teils). Liefert die neue Kennung.
    pub fn neues_kapitel(&mut self, teil_id: &str, titel: &str, nach: Option<&str>) -> Result<String, String> {
        let titel = titel.trim();
        if titel.is_empty() {
            return Err("Der Titel darf nicht leer sein.".to_string());
        }
        let id = self.eindeutige_kennung(&kennung_aus_titel(titel));
        let teil_pos = self.teile.iter().position(|t| t.id == teil_id).ok_or("Teil nicht gefunden")?;
        let teil = &mut self.teile[teil_pos];

        // Überschriftsform des Teils übernehmen: „Kapitel [id]: Titel“ oder schlicht „Titel“
        let mit_kennung = teil.kapitel.iter().any(|k| k.inhalt.starts_with("## ") && k.inhalt.lines().next().is_some_and(|z| z.contains('[') && z.contains("]:")));
        let kopf = if mit_kennung { format!("Kapitel [{id}]: {titel}") } else { titel.to_string() };
        let kapitel = Kapitel::neu(&id, titel, &teil.dateipfad, format!("## {kopf}\n\n"));

        let pos = nach
            .and_then(|n| teil.kapitel.iter().position(|k| k.id == n))
            .map_or(teil.kapitel.len(), |p| p + 1);
        teil.kapitel.insert(pos, kapitel);
        Ok(id)
    }

    /// Benennt ein Kapitel um und schreibt die Überschrift im Text entsprechend.
    pub fn kapitel_umbenennen(&mut self, kapitel_id: &str, titel: &str) -> Result<(), String> {
        let titel = titel.trim();
        if titel.is_empty() {
            return Err("Der Titel darf nicht leer sein.".to_string());
        }
        let k = self.kapitel_finden_mut(kapitel_id).ok_or("Kapitel nicht gefunden")?;
        if !k.inhalt.starts_with("## ") {
            return Err("Dieses Kapitel hat keine „## “-Überschrift, der Titel kommt aus dem Dateinamen.".to_string());
        }
        let (kopf, rest) = k.inhalt.split_once('\n').map_or((k.inhalt.as_str(), ""), |(a, b)| (a, b));
        let neuer_kopf = match kopf.find("]:") {
            Some(i) => format!("{} {}", &kopf[..i + 2], titel),
            None => format!("## {titel}"),
        };
        k.inhalt = if rest.is_empty() && !k.inhalt.contains('\n') {
            neuer_kopf
        } else {
            format!("{neuer_kopf}\n{rest}")
        };
        k.titel = titel.to_string();
        Ok(())
    }

    /// Entfernt ein Kapitel aus dem Teil. Der Text bleibt im letzten Snapshot erhalten.
    pub fn kapitel_loeschen(&mut self, kapitel_id: &str) -> Result<Kapitel, String> {
        let t = self.teil_index_von_kapitel(kapitel_id).ok_or("Kapitel nicht gefunden")?;
        let pos = self.teile[t].kapitel.iter().position(|k| k.id == kapitel_id).unwrap_or(0);
        Ok(self.teile[t].kapitel.remove(pos))
    }

    /// Verschiebt ein Kapitel um eine Position; an den Rändern wechselt es in den Nachbarteil.
    /// Liefert die Kennung des Teils, in dem das Kapitel danach steht.
    pub fn kapitel_verschieben(&mut self, kapitel_id: &str, schritt: i32) -> Result<(), String> {
        let t = self.teil_index_von_kapitel(kapitel_id).ok_or("Kapitel nicht gefunden")?;
        let pos = self.teile[t].kapitel.iter().position(|k| k.id == kapitel_id).unwrap_or(0);
        let ziel = pos as i32 + schritt.signum();

        if (0..self.teile[t].kapitel.len() as i32).contains(&ziel) {
            self.teile[t].kapitel.swap(pos, ziel as usize);
            return Ok(());
        }
        let nachbar = t as i32 + schritt.signum();
        if !(0..self.teile.len() as i32).contains(&nachbar) {
            return Ok(());
        }
        let mut k = self.teile[t].kapitel.remove(pos);
        let n = nachbar as usize;
        k.dateipfad = self.teile[n].dateipfad.clone();
        if schritt > 0 {
            self.teile[n].kapitel.insert(0, k);
        } else {
            self.teile[n].kapitel.push(k);
        }
        Ok(())
    }

    /// Legt einen neuen Teil mit eigener Datei an. Liefert die Kennung.
    pub fn neuer_teil(&mut self, titel: &str) -> Result<String, String> {
        let titel = titel.trim();
        if titel.is_empty() {
            return Err("Der Titel darf nicht leer sein.".to_string());
        }
        let n = (1..)
            .find(|n| {
                let datei = format!("teil-{n}.md");
                !self.teile.iter().any(|t| t.dateipfad == Path::new(&datei))
                    && !self.wurzelverzeichnis.join(&datei).exists()
            })
            .unwrap_or(1);
        let id = format!("teil-{n}");
        let mut teil = Teil::neu(&id, titel);
        teil.dateipfad = PathBuf::from(format!("teil-{n}.md"));
        teil.praeambel = format!("# {titel}\n\n");
        self.teile.push(teil);
        Ok(id)
    }

    /// Benennt einen Teil um (Hauptüberschrift der Datei).
    pub fn teil_umbenennen(&mut self, teil_id: &str, titel: &str) -> Result<(), String> {
        let titel = titel.trim();
        if titel.is_empty() {
            return Err("Der Titel darf nicht leer sein.".to_string());
        }
        let teil = self.teile.iter_mut().find(|t| t.id == teil_id).ok_or("Teil nicht gefunden")?;
        let h1 = crate::markdown::ueberschriften(&teil.praeambel)
            .into_iter()
            .find(|u| u.ebene == 1)
            .ok_or("Die Datei hat keine „# “-Überschrift.")?;
        let ende = teil.praeambel[h1.byte..].find('\n').map_or(teil.praeambel.len(), |i| h1.byte + i);
        let zusatz = if teil.praeambel[h1.byte..ende].contains("{-}") { " {-}" } else { "" };
        teil.praeambel.replace_range(h1.byte..ende, &format!("# {titel}{zusatz}"));
        teil.titel = titel.to_string();
        Ok(())
    }

    /// Verschiebt einen Teil in der Reihenfolge.
    pub fn teil_verschieben(&mut self, teil_id: &str, schritt: i32) {
        if let Some(pos) = self.teile.iter().position(|t| t.id == teil_id) {
            let ziel = pos as i32 + schritt.signum();
            if (0..self.teile.len() as i32).contains(&ziel) {
                self.teile.swap(pos, ziel as usize);
            }
        }
    }

    /// Entfernt einen Teil: Die Datei wandert in `.werkbank/papierkorb/`, nichts geht verloren.
    pub fn teil_entfernen(&mut self, teil_id: &str) -> io::Result<()> {
        let pos = self
            .teile
            .iter()
            .position(|t| t.id == teil_id)
            .ok_or_else(|| io_fehler("Teil nicht gefunden"))?;
        let datei = self.wurzelverzeichnis.join(&self.teile[pos].dateipfad);
        if datei.is_file() {
            let korb = self.werkbank_ordner().join("papierkorb");
            fs::create_dir_all(&korb)?;
            let name = datei.file_name().and_then(|n| n.to_str()).unwrap_or("teil.md");
            fs::copy(&datei, korb.join(format!("{}-{}", Local::now().format("%Y%m%d-%H%M%S"), name)))?;
            fs::remove_file(&datei)?;
        }
        self.teile.remove(pos);
        Ok(())
    }

    /// Schreibt die Reihenfolge der Teile nach `reihenfolge.txt` (nur wenn die Datei existiert).
    /// Kommentare am Anfang bleiben erhalten; unbekannte Einträge wandern ans Ende.
    pub fn reihenfolge_speichern(&self) -> io::Result<()> {
        let datei = self.wurzelverzeichnis.join("reihenfolge.txt");
        if !datei.is_file() {
            return Ok(());
        }
        let alt = fs::read_to_string(&datei)?;
        let mut kopf = Vec::new();
        for zeile in alt.lines() {
            let t = zeile.trim();
            if t.is_empty() || t.starts_with('#') {
                kopf.push(zeile.to_string());
            } else {
                break;
            }
        }
        let bekannt: Vec<String> = self.teile.iter().map(|t| t.dateipfad.to_string_lossy().into_owned()).collect();
        let unbekannt = alt
            .lines()
            .map(str::trim)
            .filter(|z| !z.is_empty() && !z.starts_with('#') && !bekannt.iter().any(|b| b == z))
            .map(str::to_string)
            .collect::<Vec<_>>();

        let mut neu = kopf;
        neu.extend(bekannt);
        neu.extend(unbekannt);
        let text = neu.join("\n") + "\n";
        if text == alt {
            return Ok(());
        }
        let ziel = self.werkbank_ordner().join("snapshots").join("reihenfolge");
        fs::create_dir_all(&ziel)?;
        fs::write(ziel.join(format!("{}-reihenfolge.txt", Local::now().format("%Y%m%d-%H%M%S"))), alt)?;
        atomar_schreiben(&datei, &text)
    }

    /// Aktualisiert Kopfzeilen-Titel nach einer Bearbeitung der Überschrift im Editor.
    pub fn titel_aus_ueberschrift(&mut self, kapitel_id: &str) {
        if let Some(k) = self.kapitel_finden_mut(kapitel_id)
            && k.inhalt.starts_with("## ")
            && let Some(kopf) = k.inhalt.lines().next()
        {
            let (_, titel) = parse_kapitel_kopf(kopf.trim_start_matches("## ").trim(), 0);
            if !titel.is_empty() {
                k.titel = titel;
            }
        }
    }
}

fn io_fehler_von(e: impl std::fmt::Display) -> Error {
    io_fehler(e.to_string())
}

/// Schreibt über eine Temp-Datei und benennt um, damit nie eine halbe Datei entsteht.
pub(crate) fn atomar_schreiben(pfad: &Path, inhalt: &str) -> io::Result<()> {
    if let Some(eltern) = pfad.parent() {
        fs::create_dir_all(eltern)?;
    }
    let name = pfad.file_name().and_then(|n| n.to_str()).unwrap_or("datei");
    let temp = pfad.with_file_name(format!(".{name}.tmp.werkbank"));
    fs::write(&temp, inhalt)?;
    fs::rename(&temp, pfad)
}

pub(crate) fn dateien_mit_endung(ordner: &Path, endung: &str) -> Vec<PathBuf> {
    fs::read_dir(ordner)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == endung))
        .collect()
}

/// Nächste freie Nummer in `<stem>-0001.md`, `<stem>-0002.md`, …
fn naechste_backup_nummer(ordner: &Path, stem: &str) -> u32 {
    dateien_mit_endung(ordner, "md")
        .iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?;
            name.strip_prefix(&format!("{stem}-"))?.strip_suffix(".md")?.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0)
        + 1
}

fn snapshot_faellig(ordner: &Path, abstand: Duration) -> bool {
    let juengste = fs::read_dir(ordner)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.metadata().ok()?.modified().ok())
        .max();
    match juengste {
        None => true,
        Some(zeit) => SystemTime::now().duration_since(zeit).is_ok_and(|d| d >= abstand),
    }
}

fn snapshots_beschneiden(ordner: &Path) {
    let mut dateien: Vec<PathBuf> = fs::read_dir(ordner)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    if dateien.len() > MAX_SNAPSHOTS {
        dateien.sort();
        for alt in &dateien[..dateien.len() - MAX_SNAPSHOTS] {
            let _ = fs::remove_file(alt);
        }
    }
}

/// `20261029-103015-teil-1.md` -> `29.10.2026 10:30:15`
fn stempel_lesbar(name: &str) -> String {
    let b = name.as_bytes();
    if b.len() >= 15 && name[..8].bytes().all(|c| c.is_ascii_digit()) && b[8] == b'-' {
        format!(
            "{}.{}.{} {}:{}:{}",
            &name[6..8], &name[4..6], &name[..4], &name[9..11], &name[11..13], &name[13..15]
        )
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn werkstatt(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("werkbank-test-{name}"));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    const TEIL: &str = "# Teil 1: Anfang {-}\n\nEinleitung.\n\n## Kapitel [eins]: Erstes\n\nText eins.\n\n```text\n## kein Kapitel\n```\n\n## Kapitel [zwei]: Zweites\n\nText zwei.\n\n\n";

    fn projekt(ordner: &Path) -> Projekt {
        fs::write(ordner.join("teil-1.md"), TEIL).unwrap();
        fs::write(ordner.join("reihenfolge.txt"), "# Reihenfolge\nteil-1.md\n").unwrap();
        Projekt::aus_verzeichnis(ordner).unwrap()
    }

    #[test]
    fn laden_und_zusammensetzen_ist_verlustfrei() {
        let o = werkstatt("verlustfrei");
        let p = projekt(&o);
        assert_eq!(p.teile[0].kapitel.len(), 2);
        assert_eq!(p.teile[0].rekonstruiere_datei_inhalt(), TEIL);
        assert!(!p.teil_speichern("teil-1", Duration::ZERO).unwrap(), "unverändert: nichts schreiben");
    }

    #[test]
    fn speichern_legt_snapshot_an_und_schreibt_atomar() {
        let o = werkstatt("speichern");
        let mut p = projekt(&o);
        p.kapitel_finden_mut("eins").unwrap().inhalt.push_str("Neu.\n");
        assert!(p.teil_speichern("teil-1", Duration::ZERO).unwrap());
        assert!(fs::read_to_string(o.join("teil-1.md")).unwrap().contains("Neu."));
        let snaps = p.snapshots_fuer_teil("teil-1");
        assert_eq!(snaps.len(), 1);
        assert_eq!(fs::read_to_string(&snaps[0].pfad).unwrap(), TEIL);
        assert!(p.kapitel_aus_snapshot(&snaps[0].pfad, "eins").unwrap().contains("Text eins."));
        assert!(!o.join(".teil-1.md.tmp.werkbank").exists());
    }

    #[test]
    fn jedes_speichern_legt_eine_nummerierte_sicherung_an() {
        let o = werkstatt("nummeriert");
        let mut p = projekt(&o);
        // Unverändert: keine Sicherung
        assert_eq!(p.teil_speichern_nummeriert("teil-1").unwrap(), None);
        let mut staende = vec![fs::read_to_string(o.join("teil-1.md")).unwrap()];
        for i in 1..=3 {
            p.kapitel_finden_mut("eins").unwrap().inhalt.push_str(&format!("Zusatz {i}.\n"));
            assert_eq!(p.teil_speichern_nummeriert("teil-1").unwrap(), Some(i), "fortlaufend, ohne Drosselung");
            staende.push(fs::read_to_string(o.join("teil-1.md")).unwrap());
        }
        let ordner = o.join(".werkbank/backups/teil-1");
        for (i, alt) in staende.iter().take(3).enumerate() {
            let datei = ordner.join(format!("teil-1-{:04}.md", i + 1));
            assert_eq!(&fs::read_to_string(&datei).unwrap(), alt, "Sicherung {} enthält den Stand davor", i + 1);
        }
        assert_eq!(dateien_mit_endung(&ordner, "md").len(), 3);
        // Der Verlauf zeigt sie, neueste zuerst
        let verlauf = p.snapshots_fuer_teil("teil-1");
        assert_eq!(verlauf.len(), 3);
        assert!(verlauf[0].zeit.starts_with("Sicherung Nr. 3"), "{:?}", verlauf[0].zeit);
        // Mehrere Teile auf einmal
        let neu = p.neuer_teil("Zweiter").unwrap();
        assert_eq!(p.teile_speichern_nummeriert(std::slice::from_ref(&neu)).unwrap(), vec![(neu, 0)], "neue Datei: keine alte Fassung");
    }

    #[test]
    fn snapshots_werden_gedrosselt() {
        let o = werkstatt("drosselung");
        let mut p = projekt(&o);
        for i in 0..3 {
            p.kapitel_finden_mut("eins").unwrap().inhalt.push_str(&format!("{i}\n"));
            p.teil_speichern("teil-1", Duration::from_secs(600)).unwrap();
        }
        assert_eq!(p.snapshots_fuer_teil("teil-1").len(), 1);
    }

    #[test]
    fn struktur_operationen_und_reihenfolge() {
        let o = werkstatt("struktur");
        let mut p = projekt(&o);

        let id = p.neues_kapitel("teil-1", "Größe & Maß", Some("eins")).unwrap();
        assert_eq!(id, "groesse-mass");
        assert_eq!(p.teile[0].kapitel[1].id, "groesse-mass");
        assert!(p.teile[0].kapitel[1].inhalt.starts_with("## Kapitel [groesse-mass]: Größe & Maß"));

        p.kapitel_umbenennen("zwei", "Zweiter Versuch").unwrap();
        assert!(p.kapitel_finden("zwei").unwrap().inhalt.starts_with("## Kapitel [zwei]: Zweiter Versuch\n"));

        p.kapitel_verschieben("zwei", -1).unwrap();
        assert_eq!(p.teile[0].kapitel[1].id, "zwei");

        let neu = p.neuer_teil("Teil 2: Weiter").unwrap();
        p.kapitel_verschieben("zwei", 1).unwrap();
        p.kapitel_verschieben("zwei", 1).unwrap();
        assert_eq!(p.teile[1].kapitel[0].id, "zwei", "wechselt in den Nachbarteil");
        assert_eq!(p.teile[1].id, neu);

        p.teil_umbenennen("teil-1", "Teil 1: Neubeginn").unwrap();
        assert!(p.teile[0].praeambel.starts_with("# Teil 1: Neubeginn {-}"));

        p.reihenfolge_speichern().unwrap();
        assert_eq!(fs::read_to_string(o.join("reihenfolge.txt")).unwrap(), "# Reihenfolge\nteil-1.md\nteil-2.md\n");
        p.teile_speichern(&["teil-1".into(), neu.clone()], Duration::ZERO).unwrap();
        let neu_geladen = Projekt::aus_verzeichnis(&o).unwrap();
        assert_eq!(neu_geladen.teile.len(), 2);
        assert_eq!(neu_geladen.teile[1].kapitel[0].id, "zwei");
    }

    #[test]
    fn teil_entfernen_nutzt_papierkorb() {
        let o = werkstatt("papierkorb");
        let mut p = projekt(&o);
        p.neuer_teil("Zweiter").unwrap();
        p.teile_speichern(&["teil-2".into()], Duration::ZERO).unwrap();
        p.teil_entfernen("teil-2").unwrap();
        assert!(!o.join("teil-2.md").exists());
        assert_eq!(fs::read_dir(o.join(".werkbank/papierkorb")).unwrap().count(), 1);
    }

    #[test]
    fn metadaten_bleiben_erhalten() {
        let o = werkstatt("meta");
        let mut p = projekt(&o);
        {
            let k = p.kapitel_finden_mut("eins").unwrap();
            k.wortziel = 3500;
            k.status = KapitelStatus::Fertig;
            k.notizen = "Beispiel kürzen".into();
        }
        p.kapitel_meta_speichern().unwrap();
        let q = Projekt::aus_verzeichnis(&o).unwrap();
        let k = q.kapitel_finden("eins").unwrap();
        assert_eq!((k.wortziel, k.status, k.notizen.as_str()), (3500, KapitelStatus::Fertig, "Beispiel kürzen"));
    }

    #[test]
    fn band_yaml_roundtrip() {
        let o = werkstatt("yaml");
        let mut p = projekt(&o);
        p.metadaten.titel = "Neuer Titel".into();
        assert_eq!(p.band_yaml_speichern().unwrap(), o.join("band.yaml"));
        assert_eq!(Projekt::aus_verzeichnis(&o).unwrap().metadaten.titel, "Neuer Titel");
    }

    #[test]
    fn fremde_band_yaml_wird_nie_ueberschrieben() {
        let o = werkstatt("yaml-fremd");
        let mut p = projekt(&o);
        let fremd = "reihe: Rust-Werkstatt\nbandnummer: 1\ndateiname: rust-werkstatt-band1\n";
        fs::write(o.join("band.yaml"), fremd).unwrap();
        p.metadaten.titel = "Eigener Titel".into();
        let ziel = p.band_yaml_speichern().unwrap();
        assert_eq!(ziel, o.join(".werkbank/metadaten.yaml"));
        assert_eq!(fs::read_to_string(o.join("band.yaml")).unwrap(), fremd);
        assert_eq!(Projekt::aus_verzeichnis(&o).unwrap().metadaten.titel, "Eigener Titel");
    }

    #[test]
    fn kennungen() {
        assert_eq!(kennung_aus_titel("Größe & Maß!"), "groesse-mass");
        assert_eq!(kennung_aus_titel("***"), "kapitel");
    }
}
