//! # Skript-Prüfer für die Buchbeispiele
//!
//! Prüft Shell-Beispiele (```bash, ```sh, ```zsh, ```fish, ```powershell) auf **Syntaxfehler**, ohne sie
//! auszuführen: `bash -n`, `sh -n`, `zsh -n`, `fish --no-execute` und den PowerShell-Parser. Welche Shells
//! geprüft werden, hängt davon ab, was auf dem Rechner installiert ist; fehlt eine, wird das Beispiel
//! als übersprungen gemeldet. Ist `shellcheck` da, prüft es Bash- und sh-Beispiele zusätzlich auf Fehler
//! (nur die Stufe „error“, keine Stilhinweise).

use crate::codepruefung::{Beispiel, Ergebnis};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Die unterstützten Shells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Sh,
    Zsh,
    Fish,
    PowerShell,
}

impl Shell {
    /// Anzeigename.
    pub fn name(self) -> &'static str {
        match self {
            Shell::Bash => "Bash",
            Shell::Sh => "sh",
            Shell::Zsh => "Zsh",
            Shell::Fish => "Fish",
            Shell::PowerShell => "PowerShell",
        }
    }

    fn endung(self) -> &'static str {
        match self {
            Shell::PowerShell => "ps1",
            _ => "sh",
        }
    }

    /// Mögliche Programmnamen, der erste Treffer im Suchpfad gilt.
    fn programme(self) -> &'static [&'static str] {
        match self {
            Shell::Bash => &["bash"],
            Shell::Sh => &["sh"],
            Shell::Zsh => &["zsh"],
            Shell::Fish => &["fish"],
            Shell::PowerShell => &["pwsh", "powershell"],
        }
    }
}

/// Die Shell zu einer Codeblock-Sprache (`bash`, `sh`, `shell`, `zsh`, `fish`, `powershell`, `pwsh`, `ps1`).
pub fn shell_von(sprache: &str) -> Option<Shell> {
    match sprache.to_ascii_lowercase().as_str() {
        "bash" | "shell" => Some(Shell::Bash),
        "sh" => Some(Shell::Sh),
        "zsh" => Some(Shell::Zsh),
        "fish" => Some(Shell::Fish),
        "powershell" | "pwsh" | "ps1" => Some(Shell::PowerShell),
        _ => None,
    }
}

/// Ist die Sprache ein Shell-Skript?
pub fn ist_skript(sprache: &str) -> bool {
    shell_von(sprache).is_some()
}

/// Sucht ein Programm im Suchpfad.
fn programm_suchen(name: &str) -> Option<PathBuf> {
    let endungen: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ".bat"] } else { &[""] };
    std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
        .flat_map(|d| endungen.iter().map(move |e| d.join(format!("{name}{e}"))))
        .find(|p| p.is_file())
}

/// Das Programm für eine Shell, falls installiert.
pub fn programm(shell: Shell) -> Option<PathBuf> {
    shell.programme().iter().find_map(|n| programm_suchen(n))
}

/// Erste Versionsnummer aus der Ausgabe von `--version`.
fn version_aus(text: &str) -> Option<String> {
    text.split(|c: char| c.is_whitespace() || c == ',' || c == '(')
        .find(|w| w.chars().next().is_some_and(|c| c.is_ascii_digit()) && w.contains('.'))
        .map(|w| w.trim_end_matches(|c: char| !c.is_ascii_digit()).to_string())
}

fn version_von(pfad: &Path) -> Option<String> {
    let aus = crate::prozess::ohne_fenster(Command::new(pfad).arg("--version")).output().ok()?;
    version_aus(&String::from_utf8_lossy(&aus.stdout))
}

/// Die gefundenen Shells mit Version, z. B. `Bash 5.2.21`, dazu `shellcheck`, falls vorhanden.
pub fn gefunden() -> Vec<String> {
    let mut liste: Vec<String> = [Shell::Bash, Shell::Sh, Shell::Zsh, Shell::Fish, Shell::PowerShell]
        .into_iter()
        .filter_map(|s| {
            let p = programm(s)?;
            Some(match version_von(&p) {
                Some(v) => format!("{} {v}", s.name()),
                None => s.name().to_string(),
            })
        })
        .collect();
    if let Some(p) = programm_suchen("shellcheck") {
        liste.push(match version_von(&p) {
            Some(v) => format!("ShellCheck {v}"),
            None => "ShellCheck".to_string(),
        });
    }
    liste
}

/// Entfernt Eingabeaufforderungen (`$ `, `PS> `) vor den Befehlen. `None` = der Block mischt Befehle
/// und Ausgabe und lässt sich nicht als Skript prüfen.
fn ohne_prompt(code: &str, shell: Shell) -> Option<String> {
    let praefix = |z: &str| -> Option<usize> {
        let t = z.trim_start();
        let eingerueckt = z.len() - t.len();
        if shell == Shell::PowerShell {
            if let Some(r) = t.strip_prefix("PS> ") {
                return Some(z.len() - r.len());
            }
            if t.starts_with("PS ")
                && let Some(i) = t.find("> ")
            {
                return Some(eingerueckt + i + 2);
            }
            None
        } else {
            t.strip_prefix("$ ").map(|r| z.len() - r.len())
        }
    };
    let zeilen: Vec<&str> = code.lines().collect();
    let mit_prompt = zeilen.iter().filter(|z| praefix(z).is_some()).count();
    if mit_prompt == 0 {
        return Some(code.to_string());
    }
    let sonstige = zeilen.iter().filter(|z| !z.trim().is_empty() && praefix(z).is_none()).count();
    if sonstige > 0 {
        return None;
    }
    Some(zeilen.iter().map(|z| praefix(z).map_or(*z, |n| &z[n..])).collect::<Vec<_>>().join("\n"))
}

/// Führt einen Prüfbefehl mit Zeitlimit aus. `None` = Zeitüberschreitung; sonst (Erfolg, stderr + stdout).
fn ausfuehren(befehl: &mut Command, zeitlimit: Duration) -> Result<Option<(bool, String)>, String> {
    let mut kind = crate::prozess::ohne_fenster(befehl)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    let status = loop {
        match kind.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() > zeitlimit => {
                let _ = kind.kill();
                let _ = kind.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => break None,
        }
    };
    let Some(status) = status else { return Ok(None) };
    let mut text = String::new();
    if let Some(mut err) = kind.stderr.take() {
        let _ = err.read_to_string(&mut text);
    }
    if let Some(mut out) = kind.stdout.take() {
        let _ = out.read_to_string(&mut text);
    }
    Ok(Some((status.success(), text)))
}

/// Kürzt die Meldung auf die ersten Zeilen und ersetzt den Dateipfad.
fn meldung_kuerzen(text: &str, datei: &Path) -> String {
    let name = datei.to_string_lossy();
    text.replace(name.as_ref(), "Beispiel").lines().filter(|z| !z.trim().is_empty()).take(6).collect::<Vec<_>>().join("\n").chars().take(600).collect()
}

/// Sieht der Fehler nach einem unvollständigen Ausschnitt aus (Block oder Quote nicht geschlossen)?
fn ist_ausschnitt(meldung: &str) -> bool {
    let m = meldung.to_lowercase();
    ["unexpected end of file", "unexpected eof", "missing end", "missing closing", "missing terminator", "unterminated", "looking for matching"]
        .iter()
        .any(|k| m.contains(k))
}

/// Prüft ein Skript-Beispiel auf Syntaxfehler. `arbeitsordner` muss existieren und beschreibbar sein.
pub fn pruefen(bsp: &Beispiel, arbeitsordner: &Path, zeitlimit: Duration) -> Ergebnis {
    let info = bsp.info.to_lowercase();
    if info.contains("ignore") {
        return Ergebnis::Uebersprungen("als „ignore“ markiert".to_string());
    }
    let erwartet_fehler = info.contains("compile_fail");
    let Some(shell) = shell_von(&bsp.sprache) else {
        return Ergebnis::Uebersprungen("keine bekannte Shell".to_string());
    };
    let Some(prog) = programm(shell) else {
        return Ergebnis::Uebersprungen(format!("{} nicht gefunden (nicht installiert)", shell.name()));
    };
    let Some(code) = ohne_prompt(&bsp.code, shell) else {
        return Ergebnis::Uebersprungen("mischt Befehle und Ausgabe".to_string());
    };

    let eindeutig = format!("{}-{}-{:?}", std::process::id(), bsp.zeile, std::thread::current().id()).replace(['(', ')'], "");
    let datei = arbeitsordner.join(format!("skript-{eindeutig}.{}", shell.endung()));
    if let Err(e) = fs::write(&datei, format!("{code}\n")) {
        return Ergebnis::Uebersprungen(format!("Datei nicht schreibbar: {e}"));
    }

    let mut befehl = Command::new(&prog);
    match shell {
        Shell::Bash | Shell::Sh | Shell::Zsh => {
            befehl.arg("-n").arg(&datei);
        }
        Shell::Fish => {
            befehl.arg("--no-execute").arg(&datei);
        }
        Shell::PowerShell => {
            befehl.env("WERKBANK_SKRIPT", &datei).args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "$e=$null;$t=$null;[void][System.Management.Automation.Language.Parser]::ParseFile($env:WERKBANK_SKRIPT,[ref]$t,[ref]$e);\
                 if($e -and $e.Count){$e|ForEach-Object{'Zeile '+$_.Extent.StartLineNumber+': '+$_.Message};exit 1}",
            ]);
        }
    }
    let ergebnis = ausfuehren(&mut befehl, zeitlimit);
    let mut ausgang = match ergebnis {
        Err(e) => Err(Ergebnis::Uebersprungen(format!("{} nicht startbar: {e}", shell.name()))),
        Ok(None) => Err(Ergebnis::Uebersprungen(format!("Zeitüberschreitung nach {} s", zeitlimit.as_secs()))),
        Ok(Some((ok, text))) => Ok((ok, meldung_kuerzen(&text, &datei))),
    };

    // ShellCheck nur, wenn die Syntax stimmt (Stufe „error“: keine Stilhinweise)
    if let (Ok((true, _)), Shell::Bash | Shell::Sh, Some(sc)) = (&ausgang, shell, programm_suchen("shellcheck")) {
        let art = if shell == Shell::Bash { "bash" } else { "sh" };
        if let Ok(Some((ok, text))) = ausfuehren(Command::new(sc).args(["-S", "error", "-s", art, "-f", "gcc"]).arg(&datei), zeitlimit)
            && !ok
            && !text.trim().is_empty()
        {
            ausgang = Ok((false, meldung_kuerzen(&text, &datei)));
        }
    }
    let _ = fs::remove_file(&datei);

    match ausgang {
        Err(e) => e,
        Ok((true, _)) if erwartet_fehler => Ergebnis::Fehler("Sollte nicht durchgehen (compile_fail), die Syntax ist aber in Ordnung".to_string()),
        Ok((true, _)) => Ergebnis::Ok,
        Ok((false, _)) if erwartet_fehler => Ergebnis::Ok,
        Ok((false, m)) if ist_ausschnitt(&m) => Ergebnis::Ausschnitt(m),
        Ok((false, m)) => Ergebnis::Fehler(m),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bsp(sprache: &str, code: &str, info: &str) -> Beispiel {
        Beispiel {
            kapitel_id: "k".into(),
            kapitel_titel: "K".into(),
            zeile: 1,
            sprache: sprache.into(),
            info: info.into(),
            code: code.into(),
        }
    }

    fn temp() -> PathBuf {
        let p = std::env::temp_dir().join("werkbank-test-skript");
        let _ = fs::create_dir_all(&p);
        p
    }

    #[test]
    fn erkennt_sprachen() {
        assert_eq!(shell_von("Bash"), Some(Shell::Bash));
        assert_eq!(shell_von("ps1"), Some(Shell::PowerShell));
        assert!(!ist_skript("rust"));
        assert!(!ist_skript("console"));
    }

    #[test]
    fn prompts_werden_entfernt_oder_block_uebersprungen() {
        assert_eq!(ohne_prompt("$ ls\n$ pwd", Shell::Bash).as_deref(), Some("ls\npwd"));
        assert_eq!(ohne_prompt("ls\npwd", Shell::Bash).as_deref(), Some("ls\npwd"));
        assert_eq!(ohne_prompt("$ ls\ndatei.txt", Shell::Bash), None);
        assert_eq!(ohne_prompt("PS> Get-Date", Shell::PowerShell).as_deref(), Some("Get-Date"));
    }

    #[test]
    fn versionen_werden_gelesen() {
        assert_eq!(version_aus("GNU bash, version 5.2.21(1)-release (x86_64)").as_deref(), Some("5.2.21"));
        assert_eq!(version_aus("zsh 5.9 (x86_64-ubuntu-linux-gnu)").as_deref(), Some("5.9"));
        assert_eq!(version_aus("PowerShell 7.4.1").as_deref(), Some("7.4.1"));
    }

    #[test]
    fn bash_syntax_ok_fehler_ausschnitt() {
        if programm(Shell::Bash).is_none() {
            return;
        }
        let o = temp();
        let z = Duration::from_secs(20);
        assert_eq!(pruefen(&bsp("bash", "echo hallo\nfor i in 1 2; do echo $i; done", "bash"), &o, z), Ergebnis::Ok);
        assert!(matches!(pruefen(&bsp("bash", "echo hallo )", "bash"), &o, z), Ergebnis::Fehler(_)));
        assert!(matches!(pruefen(&bsp("bash", "if true; then\n  echo x", "bash"), &o, z), Ergebnis::Ausschnitt(_)));
        assert_eq!(pruefen(&bsp("bash", "echo )", "bash,compile_fail"), &o, z), Ergebnis::Ok);
        assert!(matches!(pruefen(&bsp("bash", "echo x", "bash,ignore"), &o, z), Ergebnis::Uebersprungen(_)));
        assert!(matches!(pruefen(&bsp("bash", "$ ls\nausgabe", "bash"), &o, z), Ergebnis::Uebersprungen(_)));
    }
}
