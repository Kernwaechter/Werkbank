//! # Code-Prüfer für die Buchbeispiele
//!
//! Rust und Dart/Flutter. Dart-Blöcke prüft [`crate::dartpruefung`] in einem
//! gemeinsamen Analyzer-Durchgang; dieses Modul kümmert sich um Rust:
//!
//! Zieht alle Rust-Codeblöcke aus dem Buch und lässt `rustc` sie typprüfen
//! (`--emit=metadata`, ohne Linken). Blöcke ohne `fn main` werden wie bei
//! rustdoc in eine `main` gepackt. Beispiele, die externe Crates brauchen, werden
//! als übersprungen gemeldet statt als Fehler.

use crate::{Projekt, markdown};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Ein zu prüfender Codeblock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Beispiel {
    /// Kapitel, in dem der Block steht.
    pub kapitel_id: String,
    /// Titel dieses Kapitels.
    pub kapitel_titel: String,
    /// Zeile (ab 1) des öffnenden Zauns im Kapiteltext.
    pub zeile: usize,
    /// Sprache des Blocks (`rust` oder `dart`).
    pub sprache: String,
    /// Infozeile hinter dem Zaun.
    pub info: String,
    /// Quelltext.
    pub code: String,
}

/// Ergebnis der Prüfung eines Blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ergebnis {
    /// Kompiliert.
    Ok,
    /// Kompiliert nicht (Meldung des Compilers).
    Fehler(String),
    /// Nicht prüfbar (Grund).
    Uebersprungen(String),
    /// Unvollständiger Ausschnitt (Syntaxfehler ohne Fehlercode), z. B. die erste Hälfte eines Beispiels.
    Ausschnitt(String),
}

/// Gilt ein Codeblock als Rust?
pub fn ist_rust(sprache: &str) -> bool {
    sprache.eq_ignore_ascii_case("rust")
}

/// Sammelt alle prüfbaren Beispiele des Buches (Rust und Dart/Flutter).
pub fn beispiele(projekt: &Projekt) -> Vec<Beispiel> {
    projekt
        .teile
        .iter()
        .flat_map(|t| t.kapitel.iter())
        .flat_map(|k| {
            markdown::codebloecke(&k.inhalt)
                .into_iter()
                .filter(|b| ist_rust(&b.sprache) || crate::dartpruefung::ist_dart(&b.sprache))
                .map(|b| Beispiel {
                    kapitel_id: k.id.clone(),
                    kapitel_titel: k.titel.clone(),
                    zeile: b.start_zeile + 1,
                    sprache: b.sprache.to_ascii_lowercase(),
                    info: b.info,
                    code: b.code,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Entfernt rustdoc-Verstecktes (`# zeile`) nicht, wohl aber den Präfix, damit der Code kompiliert.
fn rustdoc_zeilen(code: &str) -> String {
    code.lines()
        .map(|z| {
            let t = z.trim_start();
            if t == "#" {
                ""
            } else if let Some(rest) = t.strip_prefix("# ") {
                // „# “ am Zeilenanfang blendet in rustdoc eine Zeile aus, ist aber Teil des Programms
                &z[z.len() - rest.len()..]
            } else {
                z
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn als_programm(code: &str) -> String {
    let code = rustdoc_zeilen(code);
    if code.contains("fn main") {
        format!("#![allow(unused)]\n{code}\n")
    } else {
        format!("#![allow(unused)]\nfn main() {{\n{code}\n}}\n")
    }
}

/// `rustc` im Suchpfad oder unter `~/.cargo/bin` (fehlt in Desktop-Sitzungen oft im PATH).
pub fn rustc_programm() -> Option<std::path::PathBuf> {
    let exe = if cfg!(windows) { "rustc.exe" } else { "rustc" };
    let mut ordner: Vec<std::path::PathBuf> = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()).collect();
    if let Some(cargo) = std::env::var_os("CARGO_HOME") {
        ordner.push(std::path::PathBuf::from(cargo).join("bin"));
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        ordner.push(std::path::PathBuf::from(home).join(".cargo/bin"));
    }
    ordner.into_iter().map(|d| d.join(exe)).find(|p| p.is_file())
}

/// Version von `rustc`, z. B. `1.99.1`; `None` = nicht gefunden.
pub fn rustc_version() -> Option<String> {
    let aus = crate::prozess::ohne_fenster(Command::new(rustc_programm()?).arg("--version")).output().ok()?;
    let text = String::from_utf8_lossy(&aus.stdout);
    text.split_whitespace().nth(1).map(str::to_string)
}

/// Prüft ein Beispiel mit `rustc`. `arbeitsordner` muss existieren und beschreibbar sein.
pub fn pruefen(bsp: &Beispiel, edition: &str, arbeitsordner: &Path, zeitlimit: Duration) -> Ergebnis {
    let info = bsp.info.to_lowercase();
    if info.contains("ignore") {
        return Ergebnis::Uebersprungen("als „ignore“ markiert".to_string());
    }
    let erwartet_fehler = info.contains("compile_fail");

    let eindeutig = format!("{}-{}-{:?}", std::process::id(), bsp.zeile, std::thread::current().id()).replace(['(', ')'], "");
    let quelle = arbeitsordner.join(format!("bsp-{eindeutig}.rs"));
    if let Err(e) = fs::write(&quelle, als_programm(&bsp.code)) {
        return Ergebnis::Uebersprungen(format!("Datei nicht schreibbar: {e}"));
    }

    let ausgabe = arbeitsordner.join(format!("bsp-{eindeutig}.rmeta"));
    let Some(rustc) = rustc_programm() else {
        return Ergebnis::Uebersprungen("rustc nicht gefunden (Rust über rustup installieren)".to_string());
    };
    let kind = crate::prozess::ohne_fenster(&mut Command::new(rustc))
        .args(["--edition", edition, "--crate-type", "bin", "--emit=metadata", "--color", "never", "-A", "warnings"])
        .arg("-o")
        .arg(&ausgabe)
        .arg(&quelle)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn();
    let mut kind = match kind {
        Ok(k) => k,
        Err(e) => return Ergebnis::Uebersprungen(format!("rustc nicht startbar: {e}")),
    };

    let start = Instant::now();
    let status = loop {
        match kind.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() > zeitlimit => {
                let _ = kind.kill();
                let _ = kind.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(_) => break None,
        }
    };

    let mut meldung = String::new();
    if let Some(mut err) = kind.stderr.take() {
        let _ = err.read_to_string(&mut meldung);
    }
    let _ = fs::remove_file(&quelle);
    let _ = fs::remove_file(&ausgabe);

    let Some(status) = status else {
        return Ergebnis::Uebersprungen(format!("Zeitüberschreitung nach {} s", zeitlimit.as_secs()));
    };
    match (status.success(), erwartet_fehler) {
        (true, false) => Ergebnis::Ok,
        (false, true) => Ergebnis::Ok,
        (true, true) => Ergebnis::Fehler("Sollte nicht kompilieren (compile_fail), kompiliert aber".to_string()),
        (false, false) => {
            if meldung.contains("can't find crate")
                || meldung.contains("unresolved import `")
                    && (meldung.contains("use of unresolved module or unlinked crate")
                        || meldung.contains("maybe a missing crate"))
                || meldung.contains("use of unresolved module or unlinked crate")
                || meldung.contains("use of undeclared crate or module")
            {
                Ergebnis::Uebersprungen("braucht externe Crates".to_string())
            } else {
                let erste = erste_fehler(&meldung);
                let kopf = erste.lines().next().unwrap_or("");
                let ohne_code = kopf.starts_with("error:");
                if ohne_code
                    && ["unclosed delimiter", "unexpected closing delimiter", "expected", "unexpected token", "unterminated"]
                        .iter()
                        .any(|m| kopf.contains(m))
                {
                    Ergebnis::Ausschnitt(erste)
                } else {
                    Ergebnis::Fehler(erste)
                }
            }
        }
    }
}

/// Der erste Fehlerblock der Compiler-Ausgabe, gekürzt.
fn erste_fehler(meldung: &str) -> String {
    let block: Vec<&str> = meldung
        .lines()
        .skip_while(|z| !z.starts_with("error"))
        .take_while(|z| !z.trim().is_empty() || z.is_empty() && false)
        .take(8)
        .collect();
    let text = if block.is_empty() { meldung.lines().take(6).collect::<Vec<_>>().join("\n") } else { block.join("\n") };
    text.chars().take(700).collect()
}

/// Prüft alle Beispiele: Rust auf mehreren Threads, Dart parallel dazu in einem Analyzer-Durchgang.
/// `melden(index, ergebnis)` wird je Block aufgerufen.
pub fn alle_pruefen(
    beispiele: &[Beispiel],
    edition: &str,
    arbeitsordner: &Path,
    threads: usize,
    melden: impl Fn(usize, Ergebnis) + Sync,
) {
    let _ = fs::create_dir_all(arbeitsordner);
    let rust: Vec<usize> = (0..beispiele.len()).filter(|&i| !crate::dartpruefung::ist_dart(&beispiele[i].sprache)).collect();
    let dart: Vec<(usize, &Beispiel)> =
        beispiele.iter().enumerate().filter(|(_, b)| crate::dartpruefung::ist_dart(&b.sprache)).collect();
    let naechster = Mutex::new(0usize);
    let melden = &melden;
    std::thread::scope(|s| {
        if !dart.is_empty() {
            s.spawn(|| crate::dartpruefung::alle_pruefen(&dart, arbeitsordner, melden));
        }
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                loop {
                    let n = {
                        let mut n = naechster.lock().unwrap_or_else(|e| e.into_inner());
                        let i = *n;
                        *n += 1;
                        i
                    };
                    let Some(&i) = rust.get(n) else { break };
                    melden(i, pruefen(&beispiele[i], edition, arbeitsordner, Duration::from_secs(60)));
                }
            });
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kapitel, Teil};

    fn bsp(code: &str, info: &str) -> Beispiel {
        Beispiel {
            kapitel_id: "k".into(),
            kapitel_titel: "K".into(),
            zeile: 1,
            sprache: "rust".into(),
            info: info.into(),
            code: code.into(),
        }
    }

    fn temp() -> std::path::PathBuf {
        let p = std::env::temp_dir().join("werkbank-test-code");
        let _ = fs::create_dir_all(&p);
        p
    }

    #[test]
    fn sammelt_rust_und_dart_bloecke() {
        let mut p = Projekt::neu("/tmp/x");
        let mut t = Teil::neu("t", "T");
        t.kapitel_hinzufuegen(Kapitel::neu("k", "K", "t.md", "## K\n\n```rust\nlet x = 1;\n```\n\n```text\nnix\n```\n\n```dart\nfinal x = 1;\n```\n"));
        p.teile.push(t);
        let b = beispiele(&p);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].zeile, 3);
        assert_eq!(b[1].sprache, "dart");
    }

    #[test]
    fn pruefen_erkennt_ok_fehler_und_compile_fail() {
        if Command::new("rustc").arg("--version").output().is_err() {
            return;
        }
        let o = temp();
        let z = Duration::from_secs(60);
        assert_eq!(pruefen(&bsp("let x: u8 = 1;\nprintln!(\"{x}\");", "rust"), "2024", &o, z), Ergebnis::Ok);
        assert_eq!(pruefen(&bsp("fn main() { println!(\"hi\"); }", "rust"), "2024", &o, z), Ergebnis::Ok);
        assert!(matches!(pruefen(&bsp("let x: u8 = \"a\";", "rust"), "2024", &o, z), Ergebnis::Fehler(m) if m.contains("mismatched")));
        assert_eq!(pruefen(&bsp("let x: u8 = \"a\";", "rust,compile_fail"), "2024", &o, z), Ergebnis::Ok);
        assert!(matches!(pruefen(&bsp("use rand::Rng;\nlet _ = rand::thread_rng();", "rust"), "2024", &o, z), Ergebnis::Uebersprungen(_)));
        assert!(matches!(pruefen(&bsp("x", "rust,ignore"), "2024", &o, z), Ergebnis::Uebersprungen(_)));
    }

    #[test]
    fn parallele_pruefung_meldet_jeden_block() {
        if Command::new("rustc").arg("--version").output().is_err() {
            return;
        }
        let liste: Vec<_> = (0..6).map(|i| bsp(&format!("let _x = {i};"), "rust")).collect();
        let ergebnisse = Mutex::new(vec![None; liste.len()]);
        alle_pruefen(&liste, "2024", &temp(), 3, |i, e| ergebnisse.lock().unwrap()[i] = Some(e));
        assert!(ergebnisse.into_inner().unwrap().iter().all(|e| e == &Some(Ergebnis::Ok)));
    }
}
