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

/// Prüft alle Programme (ruft sie mit `--version` auf bzw. liest die Versionsdateien des Flutter-SDK).
pub fn pruefen() -> WerkzeugStatus {
    let (dart, flutter) = werkbank_kern::dartpruefung::versionen();
    WerkzeugStatus {
        pandoc: werkbank_satz::pandoc_version(),
        ghostscript: gs_programm().and_then(|gs| werkbank_satz::programm_version(&gs, &["--version"])),
        rustc: werkbank_kern::codepruefung::rustc_version(),
        dart,
        flutter,
        geprueft: true,
    }
}

/// Ghostscript im Suchpfad oder unter `~/.local/gs` (wie in den Bauskripten).
fn gs_programm() -> Option<String> {
    for name in ["gs", "gswin64c", "gswin32c"] {
        if werkbank_satz::programm_version(name, &["--version"]).is_some() {
            return Some(name.to_string());
        }
    }
    let lokal = PathBuf::from(std::env::var_os("HOME")?).join(".local/gs/bin/gs");
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
    if let Some(h) = std::env::var_os("HOME") {
        w.push(PathBuf::from(h).join(".local/gs/share/ghostscript"));
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
    for wurzel in gs_wurzeln() {
        for v in unterordner(&wurzel) {
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
        .replace("/ICCProfile (ISO Coated sb.icc) def", &format!("/ICCProfile ({}) def", icc.display()))
        .replace(
            "/OutputCondition (Commercial and specialty printing)",
            "/OutputCondition (Offset printing, uncoated paper, black and white)",
        );
    let def_neu = arbeit.join("pdfx-def.ps");
    std::fs::write(&def_neu, vorlage.chars().map(|c| c as u32 as u8).collect::<Vec<u8>>()).map_err(|e| e.to_string())?;

    let aus = Command::new(gs)
        .args(["-dBATCH", "-dNOPAUSE", "-dQUIET"])
        .arg(format!("--permit-file-read={}", icc.display()))
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
