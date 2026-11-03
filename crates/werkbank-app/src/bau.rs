//! # Bauen: vorhandene Skripte als Hintergrundprozess
//!
//! Erkennt die Bauskripte eines Bandes (`build.sh`, `alle.py`, `build.py`,
//! `scripts/build_book.py`) oder liest eigene Befehle aus `.werkbank/bauen.json`,
//! startet sie asynchron, liefert die Ausgabe zeilenweise und kann abbrechen.

use crate::Message;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime};

/// Ein Befehl, der ein Buch oder einen Teil davon baut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BauBefehl {
    /// Anzeigename.
    pub name: String,
    /// Programm (`bash`, `python3`, …).
    pub programm: String,
    /// Argumente.
    pub args: Vec<String>,
    /// Arbeitsordner.
    pub ordner: PathBuf,
}

impl BauBefehl {
    /// Befehlszeile zum Anzeigen.
    pub fn zeile(&self) -> String {
        format!("{} {}", self.programm, self.args.join(" "))
    }
}

fn befehl(name: &str, programm: &str, args: &[&str], ordner: &Path) -> BauBefehl {
    BauBefehl {
        name: name.to_string(),
        programm: programm.to_string(),
        args: args.iter().map(|a| a.to_string()).collect(),
        ordner: ordner.to_path_buf(),
    }
}

/// Findet die Bauskripte eines Bandes.
pub fn erkennen(band: &Path) -> Vec<BauBefehl> {
    // Eigene Befehle haben Vorrang
    if let Ok(text) = std::fs::read_to_string(band.join(".werkbank").join("bauen.json"))
        && let Ok(liste) = serde_json::from_str::<Vec<BauBefehl>>(&text)
        && !liste.is_empty()
    {
        return liste;
    }

    let mut liste = Vec::new();
    let mut kandidaten = vec![band.to_path_buf(), band.join("werkstatt")];
    if let Some(eltern) = band.parent() {
        kandidaten.push(eltern.to_path_buf());
    }
    for d in kandidaten {
        let hinweis = if d == band {
            String::new()
        } else {
            format!(" ({})", d.file_name().and_then(|n| n.to_str()).unwrap_or("."))
        };
        if d.join("build.sh").is_file() {
            liste.push(befehl(&format!("Alle Fassungen{hinweis}"), "bash", &["build.sh"], &d));
            let inhalt = std::fs::read_to_string(d.join("build.sh")).unwrap_or_default();
            let py = std::fs::read_to_string(d.join("alle.py")).unwrap_or_default();
            if inhalt.contains("--nur") || py.contains("--nur") {
                liste.push(befehl(&format!("Nur Tolino{hinweis}"), "bash", &["build.sh", "--nur", "tolino"], &d));
                liste.push(befehl(&format!("Nur Amazon{hinweis}"), "bash", &["build.sh", "--nur", "amazon"], &d));
            }
        } else if d.join("alle.py").is_file() {
            liste.push(befehl(&format!("Alle Fassungen{hinweis}"), "python3", &["alle.py"], &d));
        } else if d.join("build.py").is_file() {
            liste.push(befehl(&format!("Buch bauen{hinweis}"), "python3", &["build.py"], &d));
        }
        if d.join("scripts").join("build_book.py").is_file() {
            liste.push(befehl(&format!("Buch bauen{hinweis}"), "python3", &["scripts/build_book.py"], &d));
        }
        if !liste.is_empty() {
            break;
        }
    }
    liste
}

fn pfad_mit_zusaetzen() -> std::ffi::OsString {
    let mut ordner: Vec<PathBuf> = Vec::new();
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        ordner.push(home.join(".local/bin"));
        ordner.push(home.join(".cargo/bin"));
    }
    ordner.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
    std::env::join_paths(ordner).unwrap_or_default()
}

/// Sucht ein Programm im Suchpfad (inklusive `~/.local/bin`).
pub fn programm_finden(name: &str) -> Option<PathBuf> {
    let endungen: &[&str] = if cfg!(windows) { &["", ".exe", ".cmd", ".bat"] } else { &[""] };
    std::env::split_paths(&pfad_mit_zusaetzen()).find_map(|d| {
        endungen.iter().map(|e| d.join(format!("{name}{e}"))).find(|p| p.is_file())
    })
}

/// Dateien unter `out/` (bis Tiefe 3), die seit `seit` geändert wurden.
pub fn ergebnisdateien(ordner: &Path, seit: SystemTime) -> Vec<(PathBuf, u64)> {
    fn sammeln(d: &Path, tiefe: usize, seit: SystemTime, aus: &mut Vec<(PathBuf, u64)>) {
        let Ok(eintraege) = std::fs::read_dir(d) else { return };
        for e in eintraege.flatten() {
            let p = e.path();
            if p.is_dir() && tiefe < 3 {
                sammeln(&p, tiefe + 1, seit, aus);
            } else if let Ok(m) = e.metadata()
                && m.is_file()
                && m.modified().is_ok_and(|t| t >= seit)
                && p.extension().and_then(|x| x.to_str()).is_some_and(|x| matches!(x, "pdf" | "epub" | "jpg" | "png" | "txt" | "html"))
            {
                aus.push((p, m.len()));
            }
        }
    }
    let mut liste = Vec::new();
    sammeln(&ordner.join("out"), 0, seit, &mut liste);
    liste.sort();
    liste
}

/// Öffnet eine Datei mit dem Standardprogramm des Systems.
pub fn oeffnen(pfad: &Path) {
    let (programm, args): (&str, Vec<&std::ffi::OsStr>) = if cfg!(target_os = "macos") {
        ("open", vec![pfad.as_os_str()])
    } else if cfg!(windows) {
        ("explorer", vec![pfad.as_os_str()])
    } else {
        ("xdg-open", vec![pfad.as_os_str()])
    };
    let _ = Command::new(programm).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

enum Ereignis {
    Zeile(String),
    Ende(Option<i32>),
}

fn zeilen_lesen(quelle: impl std::io::Read + Send + 'static, sender: iced::futures::channel::mpsc::UnboundedSender<Ereignis>) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut leser = BufReader::new(quelle);
        let mut puffer = Vec::new();
        while leser.read_until(b'\n', &mut puffer).is_ok_and(|n| n > 0) {
            let zeile = String::from_utf8_lossy(&puffer).trim_end().to_string();
            puffer.clear();
            if sender.unbounded_send(Ereignis::Zeile(zeile)).is_err() {
                break;
            }
        }
    })
}

/// Startet einen Befehl und meldet Ausgabezeilen (`Message::BauZeile`) und das Ende (`Message::BauFertig`).
pub fn laufen(cmd: BauBefehl, abbruch: Arc<AtomicBool>) -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(128, async move |mut aus: iced::futures::channel::mpsc::Sender<Message>| {
        use iced::futures::{SinkExt, StreamExt};
        let (sender, mut empfaenger) = iced::futures::channel::mpsc::unbounded::<Ereignis>();

        std::thread::spawn(move || {
            let kind = Command::new(&cmd.programm)
                .args(&cmd.args)
                .current_dir(&cmd.ordner)
                .env("PATH", pfad_mit_zusaetzen())
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn();
            let mut kind = match kind {
                Ok(k) => k,
                Err(e) => {
                    let _ = sender.unbounded_send(Ereignis::Zeile(format!("Start fehlgeschlagen: {} ({e})", cmd.programm)));
                    let _ = sender.unbounded_send(Ereignis::Ende(None));
                    return;
                }
            };
            let leser: Vec<_> = [
                kind.stdout.take().map(|o| zeilen_lesen(o, sender.clone())),
                kind.stderr.take().map(|e| zeilen_lesen(e, sender.clone())),
            ]
            .into_iter()
            .flatten()
            .collect();

            let code = loop {
                if abbruch.load(Ordering::Relaxed) {
                    let _ = kind.kill();
                    let _ = kind.wait();
                    let _ = sender.unbounded_send(Ereignis::Zeile("— abgebrochen —".to_string()));
                    break None;
                }
                match kind.try_wait() {
                    Ok(Some(status)) => break status.code(),
                    Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                    Err(_) => break None,
                }
            };
            for l in leser {
                let _ = l.join();
            }
            let _ = sender.unbounded_send(Ereignis::Ende(code));
        });

        while let Some(e) = empfaenger.next().await {
            let (nachricht, ende) = match e {
                Ereignis::Zeile(z) => (Message::BauZeile(z), false),
                Ereignis::Ende(c) => (Message::BauFertig(c), true),
            };
            if aus.send(nachricht).await.is_err() || ende {
                break;
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erkennt_skripte_in_unterordner_werkstatt() {
        let o = std::env::temp_dir().join("werkbank-test-bau");
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(o.join("werkstatt")).unwrap();
        std::fs::write(o.join("werkstatt/build.sh"), "python3 alle.py \"$@\" # --nur").unwrap();
        let b = erkennen(&o);
        assert_eq!(b.len(), 3);
        assert_eq!(b[1].args, ["build.sh", "--nur", "tolino"]);
        assert!(b[0].name.contains("werkstatt"));
    }

    #[test]
    fn eigene_befehle_haben_vorrang() {
        let o = std::env::temp_dir().join("werkbank-test-bau2");
        let _ = std::fs::remove_dir_all(&o);
        std::fs::create_dir_all(o.join(".werkbank")).unwrap();
        let eigener = vec![befehl("Mein Bau", "make", &["buch"], &o)];
        std::fs::write(o.join(".werkbank/bauen.json"), serde_json::to_string(&eigener).unwrap()).unwrap();
        assert_eq!(erkennen(&o), eigener);
    }
}
