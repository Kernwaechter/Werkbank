//! # Externe Werkzeuge: Pandoc, Ghostscript und die Compiler des Code-Prüfers
//!
//! Die Bauskripte setzen Bücher so: Markdown → **Pandoc** (`-t typst`) → **Typst** → PDF,
//! für Druckdateien danach **Ghostscript** (PDF/X-3). Die Werkbank prüft, ob die Programme
//! da sind, und nutzt sie für die Satzvorschau und die PDF/X-Ausgabe.
//! Für den Code-Prüfer zeigt sie außerdem, ob **rustc** und **Dart/Flutter** gefunden werden.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Gefundene Werkzeuge mit Version.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WerkzeugStatus {
    /// Version von Pandoc (z. B. `3.7.0.2`), `None` = nicht gefunden.
    pub pandoc: Option<String>,
    /// Selbsttest des eingebauten Typst-Satzes: `Ok(Beschreibung)` oder `Err(Fehler)`; `None` = noch nicht geprüft.
    pub typst_eingebaut: Option<Result<String, String>>,
    /// Version des Typst-Programms (nur für die Bauskripte nötig), `None` = nicht gefunden.
    pub typst: Option<String>,
    /// Version von Ghostscript, `None` = nicht gefunden.
    pub ghostscript: Option<String>,
    /// Version von rustc (Code-Prüfer für Rust).
    pub rustc: Option<String>,
    /// Version des Dart-SDK (Code-Prüfer für Dart).
    pub dart: Option<String>,
    /// Version von Flutter (Code-Prüfer für Flutter-Beispiele).
    pub flutter: Option<String>,
    /// Wurde schon geprüft?
    pub geprueft: bool,
}

impl WerkzeugStatus {
    /// Pandoc ab Version 3 kann `-t typst`.
    pub fn pandoc_brauchbar(&self) -> bool {
        self.pandoc.as_deref().and_then(werkbank_satz::hauptversion).is_some_and(|v| v >= 3)
    }
}

/// Installiert ein Werkzeug mit winget in einem sichtbaren Fenster (Fortschritt, Rückfragen, Administratorabfrage).
/// Danach in den Einstellungen „Erneut prüfen“ wählen.
pub fn winget_installieren(id: &str) {
    if !cfg!(windows) {
        return;
    }
    let _ = Command::new("cmd")
        .args(["/c", "start", "Werkbank – Installation", "cmd", "/k", "winget", "install", "-e", "--id", id, "--accept-package-agreements", "--accept-source-agreements"])
        .spawn();
}

/// Prüft alle Programme (ruft sie mit `--version` auf bzw. liest die Versionsdateien des Flutter-SDK).
pub fn pruefen() -> WerkzeugStatus {
    let (dart, flutter) = werkbank_kern::dartpruefung::versionen();
    WerkzeugStatus {
        pandoc: werkbank_satz::pandoc_version(),
        typst_eingebaut: Some(werkbank_satz::typst_selbsttest()),
        typst: werkbank_satz::programm_version("typst", &["--version"]).map(|z| z.trim_start_matches("typst").trim().to_string()),
        ghostscript: gs_programm().and_then(|gs| werkbank_satz::programm_version(&gs, &["--version"])),
        rustc: werkbank_kern::codepruefung::rustc_version(),
        dart,
        flutter,
        geprueft: true,
    }
}

/// Heimatordner (`HOME`, unter Windows `USERPROFILE`).
fn heimat() -> Option<PathBuf> {
    std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from)
}

/// Programmordner von Windows, in denen Ghostscript (`gs\gs10.x\bin\gswin64c.exe`) installiert wird.
fn windows_gs_basen() -> Vec<PathBuf> {
    if !cfg!(windows) {
        return Vec::new();
    }
    ["ProgramFiles", "ProgramW6432", "ProgramFiles(x86)", "LOCALAPPDATA"]
        .iter()
        .filter_map(|v| std::env::var_os(v).map(PathBuf::from))
        .flat_map(|p| [p.join("gs"), p.join("Programs").join("gs")])
        .collect()
}

static GS_ORDNER: std::sync::RwLock<String> = std::sync::RwLock::new(String::new());

/// Setzt den vom Benutzer angegebenen Ghostscript-Ordner (leer = suchen).
pub fn gs_ordner_setzen(ordner: &str) {
    if let Ok(mut g) = GS_ORDNER.write() {
        *g = ordner.trim().trim_matches('"').to_string();
    }
}

/// Das Programm im angegebenen Ordner: die Datei selbst, `<ordner>\bin`, `<ordner>` oder `<ordner>\<Version>\bin`.
fn gs_im_ordner(ordner: &str) -> Option<PathBuf> {
    let o = PathBuf::from(ordner);
    if o.is_file() {
        return Some(o);
    }
    let namen = ["gswin64c.exe", "gswin32c.exe", "gs"];
    let mut ordner_liste = vec![o.join("bin"), o.clone()];
    ordner_liste.extend(unterordner(&o).into_iter().map(|v| v.join("bin")));
    ordner_liste.into_iter().flat_map(|d| namen.map(|n| d.join(n))).find(|p| p.is_file())
}

/// Ghostscript: eigener Ordner, Suchpfad, Programmordner von Windows oder unter `~/.local/gs` (wie in den Bauskripten).
fn gs_programm() -> Option<String> {
    let eigener = GS_ORDNER.read().map(|g| g.clone()).unwrap_or_default();
    if !eigener.is_empty() {
        return gs_im_ordner(&eigener).map(|p| p.to_string_lossy().into_owned());
    }
    for name in ["gs", "gswin64c", "gswin32c"] {
        if werkbank_satz::programm_version(name, &["--version"]).is_some() {
            return Some(name.to_string());
        }
    }
    // Der Windows-Installer von Ghostscript trägt sich nicht in PATH ein: neueste Version zuerst
    // (ältere Installer: gs\gs10.x\bin, ab 10.6 ohne Versionsordner: gs\bin)
    let mut windows: Vec<PathBuf> = windows_gs_basen()
        .iter()
        .flat_map(|b| {
            let mut v = unterordner(b);
            v.push(b.clone());
            v
        })
        .flat_map(|v| ["gswin64c.exe", "gswin32c.exe"].map(|e| v.join("bin").join(e)))
        .filter(|p| p.is_file())
        .collect();
    windows.sort();
    if let Some(p) = windows.pop() {
        return Some(p.to_string_lossy().into_owned());
    }
    let lokal = heimat()?.join(".local/gs/bin/gs");
    lokal.is_file().then(|| lokal.to_string_lossy().into_owned())
}

/// Unterordner eines Ordners (für `…/ghostscript/<Version>/…`).
fn unterordner(basis: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(basis)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect()
}

fn gs_wurzeln() -> Vec<PathBuf> {
    let mut w: Vec<PathBuf> = ["/usr/share/ghostscript", "/usr/local/share/ghostscript", "/opt/homebrew/share/ghostscript"]
        .iter()
        .map(PathBuf::from)
        .collect();
    for zelle in ["/opt/homebrew/Cellar/ghostscript", "/usr/local/Cellar/ghostscript"] {
        for v in unterordner(Path::new(zelle)) {
            w.push(v.join("share/ghostscript"));
        }
    }
    w.extend(windows_gs_basen());
    if let Some(h) = heimat() {
        w.push(h.join(".local/gs/share/ghostscript"));
    }
    w
}

/// `PDFX_def.ps` und das Graustufen-Farbprofil suchen (wie `nach_pdfx` in `tolino-print.py`).
fn pdfx_dateien() -> Result<(PathBuf, PathBuf), String> {
    let mut def = None;
    let mut profile: Vec<PathBuf> = Vec::new();
    if let Some(d) = std::env::var_os("ICC_DIR") {
        profile.push(PathBuf::from(d));
    }
    profile.push(PathBuf::from("/usr/share/color/icc/ghostscript"));
    // Installationsordner des gefundenen Programms (…\gs\bin\gswin64c.exe → …\gs, auch ohne Versionsordner)
    let eigene: Vec<PathBuf> = gs_programm()
        .map(PathBuf::from)
        .and_then(|p| p.parent().and_then(|b| b.parent()).map(Path::to_path_buf))
        .into_iter()
        .collect();
    for wurzel in gs_wurzeln() {
        let mut kandidaten = unterordner(&wurzel);
        kandidaten.push(wurzel.clone());
        kandidaten.extend(eigene.iter().cloned());
        for v in kandidaten {
            let kandidat = v.join("lib/PDFX_def.ps");
            if def.is_none() && kandidat.is_file() {
                def = Some(kandidat);
            }
            profile.push(v.join("iccprofiles"));
        }
    }
    let def = def.ok_or("PDFX_def.ps von Ghostscript nicht gefunden")?;
    let icc = profile
        .into_iter()
        .map(|p| p.join("default_gray.icc"))
        .find(|p| p.is_file())
        .ok_or("Farbprofil default_gray.icc nicht gefunden (Ordner mit ICC_DIR=… angeben)")?;
    Ok((def, icc))
}

/// Wandelt ein PDF mit Ghostscript nach PDF/X-3 in Graustufen um (Weg von `tolino-print.py`).
pub fn pdfx_erzeugen(quelle: &Path, ziel: &Path, titel: &str) -> Result<(), String> {
    let gs = gs_programm().ok_or("Ghostscript (gs) nicht gefunden")?;
    let (def, icc) = pdfx_dateien()?;
    let arbeit = std::env::temp_dir().join(format!("werkbank-pdfx-{}", std::process::id()));
    std::fs::create_dir_all(&arbeit).map_err(|e| e.to_string())?;

    let titel_ps = titel.replace(['(', ')', '\\'], " ");
    let vorlage = std::fs::read(&def).map_err(|e| e.to_string())?;
    let vorlage: String = vorlage.iter().map(|&b| b as char).collect(); // Latin-1
    let vorlage = vorlage
        .replace("/Title (Title)", &format!("/Title ({titel_ps})"))
        .replace("/ICCProfile (ISO Coated sb.icc) def", &format!("/ICCProfile ({}) def", icc.to_string_lossy().replace('\\', "/")))
        .replace(
            "/OutputCondition (Commercial and specialty printing)",
            "/OutputCondition (Offset printing, uncoated paper, black and white)",
        );
    let def_neu = arbeit.join("pdfx-def.ps");
    std::fs::write(&def_neu, vorlage.chars().map(|c| c as u32 as u8).collect::<Vec<u8>>()).map_err(|e| e.to_string())?;

    let aus = werkbank_kern::prozess::ohne_fenster(&mut Command::new(gs))
        .args(["-dBATCH", "-dNOPAUSE", "-dQUIET"])
        .arg(format!("--permit-file-read={}", icc.to_string_lossy().replace('\\', "/")))
        .args([
            "-dPDFX",
            "-sColorConversionStrategy=Gray",
            "-sProcessColorModel=DeviceGray",
            "-sDEVICE=pdfwrite",
            "-dPDFSETTINGS=/prepress",
            "-dEmbedAllFonts=true",
        ])
        .arg(format!("-sOutputFile={}", ziel.display()))
        .arg(&def_neu)
        .arg(quelle)
        .output()
        .map_err(|e| format!("Ghostscript nicht startbar: {e}"))?;
    let meldung = format!("{}{}", String::from_utf8_lossy(&aus.stdout), String::from_utf8_lossy(&aus.stderr));
    if !aus.status.success() {
        return Err(format!("Ghostscript-Fehler: {}", meldung.chars().rev().take(500).collect::<String>().chars().rev().collect::<String>()));
    }
    if meldung.contains("reverting to normal PDF") {
        return Err(format!("Ghostscript konnte kein PDF/X erzeugen: {}", meldung.chars().take(400).collect::<String>()));
    }
    let daten = std::fs::read(ziel).map_err(|e| e.to_string())?;
    if !daten.windows(15).any(|w| w == b"GTS_PDFXVersion") {
        return Err("In der Ausgabe fehlt die PDF/X-Kennung".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erkennung_und_versionspruefung() {
        let s = WerkzeugStatus { pandoc: Some("3.7.0.2".into()), ghostscript: None, geprueft: true, ..Default::default() };
        assert!(s.pandoc_brauchbar());
        assert!(!WerkzeugStatus { pandoc: Some("2.9.2.1".into()), ..Default::default() }.pandoc_brauchbar());
        assert!(!WerkzeugStatus::default().pandoc_brauchbar());
    }

    #[test]
    fn pdfx_aus_einem_typst_pdf() {
        if gs_programm().is_none() || pdfx_dateien().is_err() {
            return;
        }
        let quelle = werkbank_satz::typst_zu_pdf("Hallo *Welt*, ein Absatz Text.".to_string(), None, Default::default()).unwrap();
        let o = std::env::temp_dir().join("werkbank-test-pdfx");
        std::fs::create_dir_all(&o).unwrap();
        std::fs::write(o.join("q.pdf"), quelle.bytes).unwrap();
        pdfx_erzeugen(&o.join("q.pdf"), &o.join("x.pdf"), "Testbuch").unwrap();
        let x = std::fs::read(o.join("x.pdf")).unwrap();
        assert!(x.starts_with(b"%PDF"));
        assert!(x.windows(15).any(|w| w == b"GTS_PDFXVersion"));
    }
}
