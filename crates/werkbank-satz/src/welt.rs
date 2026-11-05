//! Typst-Welt: Schriften, Dateien und Aufruf des Compilers.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use typst::diag::{FileError, FileResult, SourceDiagnostic};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, World};
use typst_kit::fonts::{FontSlot, Fonts};

/// Ergebnis eines Satzlaufs.
#[derive(Debug, Clone)]
pub struct PdfErgebnis {
    /// PDF-Daten.
    pub bytes: Vec<u8>,
    /// Seitenzahl des Dokuments.
    pub seiten: usize,
    /// Warnungen des Compilers (gekürzt).
    pub warnungen: Vec<String>,
}

struct Welt {
    bibliothek: LazyHash<Library>,
    buch: LazyHash<FontBook>,
    schriften: Vec<FontSlot>,
    /// Mitgelieferte Schriften (Fira Sans), im Schriftbuch hinter den gefundenen.
    eigene: Vec<Font>,
    haupt: Source,
    wurzel: Option<PathBuf>,
    speicher: HashMap<String, Bytes>,
    cache: Mutex<HashMap<FileId, Bytes>>,
}

/// Fira Sans (SIL Open Font License, siehe assets/fonts/OFL.txt): die serifenlose Schrift der Vorlagen.
const MITGELIEFERTE_SCHRIFTEN: &[&[u8]] = &[
    include_bytes!("../../../assets/fonts/FiraSans-Regular.ttf"),
    include_bytes!("../../../assets/fonts/FiraSans-Italic.ttf"),
    include_bytes!("../../../assets/fonts/FiraSans-Bold.ttf"),
    include_bytes!("../../../assets/fonts/FiraSans-BoldItalic.ttf"),
];

fn schriften_suchen() -> Fonts {
    typst_kit::fonts::FontSearcher::new().include_system_fonts(true).search()
}

impl Welt {
    fn neu(text: String, wurzel: Option<PathBuf>, speicher: HashMap<String, Bytes>) -> Self {
        let f = schriften_suchen();
        let mut buch = f.book;
        let mut eigene = Vec::new();
        for daten in MITGELIEFERTE_SCHRIFTEN {
            for schrift in Font::iter(Bytes::new(*daten)) {
                buch.push(schrift.info().clone());
                eigene.push(schrift);
            }
        }
        Self {
            bibliothek: LazyHash::new(Library::default()),
            buch: LazyHash::new(buch),
            schriften: f.fonts,
            eigene,
            haupt: Source::new(FileId::new(None, VirtualPath::new("buch.typ")), text),
            wurzel,
            speicher,
            cache: Mutex::new(HashMap::new()),
        }
    }
}

impl World for Welt {
    fn library(&self) -> &LazyHash<Library> {
        &self.bibliothek
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.buch
    }

    fn main(&self) -> FileId {
        self.haupt.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.haupt.id() {
            return Ok(self.haupt.clone());
        }
        let bytes = self.file(id)?;
        let text = String::from_utf8(bytes.to_vec()).map_err(|_| FileError::InvalidUtf8)?;
        Ok(Source::new(id, text))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        let pfad = id.vpath().as_rootless_path().to_string_lossy().replace('\\', "/");
        if let Some(b) = self.speicher.get(&pfad) {
            return Ok(b.clone());
        }
        if let Some(b) = self.cache.lock().unwrap_or_else(|e| e.into_inner()).get(&id) {
            return Ok(b.clone());
        }
        let wurzel = self.wurzel.as_ref().ok_or_else(|| FileError::NotFound(pfad.clone().into()))?;
        let voll = id.vpath().resolve(wurzel).ok_or_else(|| FileError::NotFound(pfad.clone().into()))?;
        let daten = std::fs::read(&voll).map_err(|e| FileError::from_io(e, &voll))?;
        let bytes = Bytes::new(daten);
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).insert(id, bytes.clone());
        Ok(bytes)
    }

    fn font(&self, index: usize) -> Option<Font> {
        match self.schriften.get(index) {
            Some(slot) => slot.get(),
            None => self.eigene.get(index - self.schriften.len()).cloned(),
        }
    }

    fn today(&self, offset: Option<i64>) -> Option<Datetime> {
        let jetzt = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64 + offset.unwrap_or(0) * 3600;
        let tage = jetzt.div_euclid(86400);
        // Tage seit 1970-01-01 in Jahr/Monat/Tag (Zivilkalender-Algorithmus von H. Hinnant)
        let z = tage + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z.rem_euclid(146_097);
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let tag = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let monat = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let jahr = (yoe + era * 400 + i64::from(monat <= 2)) as i32;
        Datetime::from_ymd(jahr, monat, tag)
    }
}

fn fehlertext(welt: &Welt, d: &SourceDiagnostic) -> String {
    let ort = welt
        .source(d.span.id().unwrap_or(welt.haupt.id()))
        .ok()
        .and_then(|q| {
            let bereich = q.range(d.span)?;
            let zeile = q.byte_to_line(bereich.start)?;
            Some(format!(" (Zeile {})", zeile + 1))
        })
        .unwrap_or_default();
    format!("{}{ort}", d.message)
}

/// Selbsttest des eingebauten Satzes: setzt eine kleine Seite mit Text, Überschrift und Code.
/// Gibt bei Erfolg die Größe des erzeugten PDFs als Text zurück.
pub fn typst_selbsttest() -> Result<String, String> {
    let quelle = "#set page(width: 80mm, height: 40mm)\n= Test\nSatztest `code` und Text.\n".to_string();
    let pdf = typst_zu_pdf(quelle, None, HashMap::new())?;
    if pdf.bytes.starts_with(b"%PDF") { Ok(format!("{} Seite, {} Bytes PDF", pdf.seiten, pdf.bytes.len())) } else { Err("kein gültiges PDF erzeugt".into()) }
}

/// Setzt Typst-Quelltext zu PDF. `wurzel`: Ordner für relative Dateien (Bilder).
pub fn typst_zu_pdf(quelltext: String, wurzel: Option<PathBuf>, dateien: HashMap<String, Vec<u8>>) -> Result<PdfErgebnis, String> {
    let speicher = dateien.into_iter().map(|(k, v)| (k, Bytes::new(v))).collect();
    let welt = Welt::neu(quelltext, wurzel, speicher);
    let ergebnis = typst::compile::<PagedDocument>(&welt);
    let warnungen: Vec<String> = ergebnis.warnings.iter().take(10).map(|w| fehlertext(&welt, w)).collect();
    // Seitenzahlen (Inhaltsverzeichnis, Fußzeilen) sind nur dann sicher richtig, wenn der Satz stabil ist.
    if let Some(w) = ergebnis.warnings.iter().find(|w| w.message.contains("converge") || w.message.contains("did not stabilize")) {
        return Err(format!(
            "Der Satz wurde nicht stabil, die Seitenzahlen im Inhaltsverzeichnis wären nicht verlässlich: {}",
            fehlertext(&welt, w)
        ));
    }
    let dokument = ergebnis.output.map_err(|fehler| {
        fehler.iter().take(8).map(|d| fehlertext(&welt, d)).collect::<Vec<_>>().join("\n")
    })?;
    let bytes = typst_pdf::pdf(&dokument, &typst_pdf::PdfOptions::default()).map_err(|fehler| {
        fehler.iter().take(8).map(|d| fehlertext(&welt, d)).collect::<Vec<_>>().join("\n")
    })?;
    Ok(PdfErgebnis { bytes, seiten: dokument.pages.len(), warnungen })
}

/// Setzt Typst-Quelltext und liefert die ersten Seiten als Bilder (für die Vorschau).
pub fn typst_zu_seiten(
    quelltext: String,
    wurzel: Option<PathBuf>,
    max_seiten: usize,
    pixel_pro_pt: f32,
) -> Result<(Vec<crate::vorschau::Seitenbild>, usize, Vec<String>), String> {
    let welt = Welt::neu(quelltext, wurzel, HashMap::new());
    let ergebnis = typst::compile::<PagedDocument>(&welt);
    let warnungen: Vec<String> = ergebnis.warnings.iter().take(5).map(|w| fehlertext(&welt, w)).collect();
    let dokument = ergebnis
        .output
        .map_err(|fehler| fehler.iter().take(6).map(|d| fehlertext(&welt, d)).collect::<Vec<_>>().join("\n"))?;
    let seiten = dokument
        .pages
        .iter()
        .take(max_seiten)
        .map(|seite| {
            let pixel = typst_render::render(seite, pixel_pro_pt);
            crate::vorschau::Seitenbild { breite: pixel.width(), hoehe: pixel.height(), rgba: pixel.take() }
        })
        .collect();
    Ok((seiten, dokument.pages.len(), warnungen))
}

/// Setzt ein Buchprojekt zu PDF.
pub fn pdf_setzen(projekt: &werkbank_kern::Projekt, opt: &crate::SatzOptionen) -> Result<PdfErgebnis, String> {
    let wurzel = Some(projekt.wurzelverzeichnis.clone());
    let erster = typst_zu_pdf(crate::buch_zu_typst(projekt, opt), wurzel.clone(), HashMap::new())?;
    // Druckereien verlangen eine gerade Seitenzahl: bei ungerader folgt eine Leerseite ohne Zahl.
    // Sie steht hinter dem Text und verschiebt nichts, deshalb genügt ein zweiter Lauf.
    if opt.gerade_seitenzahl && opt.teile_rechts && erster.seiten % 2 == 1 {
        return typst_zu_pdf(crate::buch_zu_typst_mit_ende(projekt, opt, true), wurzel, HashMap::new());
    }
    Ok(erster)
}

/// Gibt ein SVG (z. B. das Cover) als einseitiges PDF in exakt dieser Größe aus.
pub fn svg_als_pdf(svg: &str, breite_mm: f32, hoehe_mm: f32) -> Result<Vec<u8>, String> {
    let quelle = format!(
        "#set page(width: {breite_mm}mm, height: {hoehe_mm}mm, margin: 0pt)\n#image(\"cover.svg\", width: 100%, height: 100%)\n"
    );
    let dateien = HashMap::from([("cover.svg".to_string(), svg.as_bytes().to_vec())]);
    typst_zu_pdf(quelle, None, dateien).map(|e| e.bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use werkbank_kern::{Kapitel, Projekt, Teil};

    fn mini_projekt() -> Projekt {
        let mut p = Projekt::neu(std::env::temp_dir().join("werkbank-satz-test"));
        p.metadaten.titel = "Testbuch".into();
        p.metadaten.autor = "A. Autor".into();
        let mut t = Teil::neu("t1", "Teil 1: Anfang");
        t.praeambel = "# Teil 1: Anfang {-}\n\nEinleitung mit `Code` und @kap:zwei.\n\n".into();
        t.kapitel_hinzufuegen(Kapitel::neu("eins", "Eins", "t.md", "## Kapitel [eins]: Erstes\n\nText mit *Betonung*.\n\n```rust\nfn main() { println!(\"hi\"); }\n```\n\n- a\n- b\n\n"));
        t.kapitel_hinzufuegen(Kapitel::neu("zwei", "Zwei", "t.md", "## Kapitel [zwei]: Zweites\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n> Zitat\n\n### Unterpunkt\n\nEnde. Größe, Übung, Straße.\n"));
        p.teile.push(t);
        p
    }

    #[test]
    fn setzt_buch_zu_pdf() {
        let p = mini_projekt();
        for opt in [crate::SatzOptionen::druck((190.0, 270.0)), crate::SatzOptionen::lesen()] {
            let e = pdf_setzen(&p, &opt).unwrap_or_else(|f| panic!("Satz fehlgeschlagen:\n{f}\n\n{}", crate::buch_zu_typst(&p, &opt)));
            assert!(e.bytes.starts_with(b"%PDF"), "kein PDF");
            assert!(e.seiten >= 5, "Titel, Impressum, Inhalt, Teil, Kapitel: {} Seiten", e.seiten);
            assert!(e.bytes.len() > 5_000);
        }
    }

    #[test]
    fn svg_wird_pdf_in_umschlaggroesse() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="60" viewBox="0 0 100 60"><rect width="100" height="60" fill="#0b1810"/><text x="50" y="35" font-size="12" fill="#ffffff" text-anchor="middle" font-family="DejaVu Serif">Cover</text></svg>"##;
        let pdf = svg_als_pdf(svg, 100.0, 60.0).unwrap();
        assert!(pdf.starts_with(b"%PDF"));
        // MediaBox 100 mm × 60 mm = 283.46 × 170.08 pt
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.contains("283.46") || text.contains("283.465"), "MediaBox nicht gefunden");
    }
}


#[cfg(test)]
mod seitenzahlen {
    //! Prüft mit `pdftotext` am fertigen PDF: Inhaltsverzeichnis, Zählung, Leerseiten, Stellung.
    use super::*;
    use crate::{SatzOptionen, Seitenzahl, Zaehlung};
    use std::process::Command;
    use werkbank_kern::{Kapitel, Projekt, Teil};

    fn pdftotext_da() -> bool {
        Command::new("pdftotext").arg("-v").output().is_ok()
    }

    fn fuell(n: usize) -> String {
        (0..n).map(|i| format!("Absatz {i}: Dies ist ein längerer Absatz, der etwas Platz auf der Seite braucht, damit das Kapitel mehrere Seiten füllt und die Seitenzahlen sinnvoll geprüft werden können.\n\n")).collect()
    }

    fn buch() -> Projekt {
        let mut p = Projekt::neu(std::env::temp_dir().join("werkbank-satz-zahlen"));
        p.metadaten.titel = "Zahlenbuch".into();
        p.metadaten.autor = "A. Autor".into();
        let mut vor = Teil::neu("v", "Vorwort");
        vor.praeambel = String::new();
        vor.kapitel_hinzufuegen(Kapitel::neu("vorwort", "Vorwort", "v.md", format!("## Vorwort\n\n{}", fuell(8))));
        let mut t1 = Teil::neu("t1", "Teil 1: Grundlagen");
        t1.praeambel = "# Teil 1: Grundlagen {-}\n\nEinleitung des Teils.\n\n".into();
        for (i, name) in ["Alpha", "Beta", "Gamma"].iter().enumerate() {
            t1.kapitel_hinzufuegen(Kapitel::neu(
                format!("k{i}"),
                *name,
                "t.md",
                format!("## Kapitel [k{i}]: {name}\n\n{}### Unterpunkt {name}\n\n{}", fuell(14), fuell(6)),
            ));
        }
        let mut t2 = Teil::neu("t2", "Teil 2: Vertiefung");
        t2.praeambel = "# Teil 2: Vertiefung {-}\n\nZweiter Teil.\n\n".into();
        t2.kapitel_hinzufuegen(Kapitel::neu("k3", "Delta", "t.md", format!("## Kapitel [k3]: Delta\n\n{}", fuell(9))));
        let mut anhang = Teil::neu("a", "Anhang");
        anhang.kapitel_hinzufuegen(Kapitel::neu("anh", "Glossar", "a.md", format!("## Glossar\n\n{}", fuell(5))));
        p.teile = vec![vor, t1, t2, anhang];
        p
    }

    fn setzen(opt: &crate::SatzOptionen) -> Vec<String> {
        let e = pdf_setzen(&buch(), opt).unwrap_or_else(|f| panic!("Satz fehlgeschlagen: {f}"));
        // Jeder Aufruf bekommt eine eigene Datei, sonst überschreiben sich parallel laufende Tests
        static ZAEHLER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let datei = std::env::temp_dir().join(format!("werkbank-satz-zahlen-{}-{}.pdf", std::process::id(), ZAEHLER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        std::fs::write(&datei, e.bytes).unwrap();
        let aus = Command::new("pdftotext").arg("-layout").arg(&datei).arg("-").output().unwrap();
        String::from_utf8_lossy(&aus.stdout).split('\x0c').map(str::to_string).filter(|s| !s.is_empty() || true).collect::<Vec<_>>()
    }

    /// Letzte nichtleere Zeile einer Seite, bereinigt (Fußzeile).
    fn fuss(seite: &str) -> String {
        seite.lines().rev().map(|z| z.replace(['\u{2060}', '\u{200b}'], "")).map(|z| z.trim().to_string()).find(|z| !z.is_empty()).unwrap_or_default()
    }

    /// Gedruckte Zahl einer Seite; trägt sie keine (Teiltitel im Haupttext), zählt sie ab der letzten Zahl weiter.
    fn logische_zahl(seiten: &[String], j: usize) -> String {
        let f = fuss(&seiten[j]);
        if ist_zahl(&f) {
            return f;
        }
        (0..j)
            .rev()
            .find_map(|k| fuss(&seiten[k]).parse::<usize>().ok().map(|z| (z + (j - k)).to_string()))
            .or_else(|| {
                // Erste Seite des Haupttextes ohne Zahl: von der nächsten Zahl zurückrechnen
                (j + 1..seiten.len()).find_map(|k| fuss(&seiten[k]).parse::<usize>().ok().map(|z| (z - (k - j)).to_string()))
            })
            .unwrap_or_default()
    }

    fn ist_zahl(s: &str) -> bool {
        !s.is_empty() && (s.chars().all(|c| c.is_ascii_digit()) || s.chars().all(|c| "ivxlcdm".contains(c)))
    }

    /// Einträge des Inhaltsverzeichnisses: (Titel, Seitenangabe).
    fn verzeichnis(seiten: &[String]) -> (Vec<(String, String)>, Vec<usize>) {
        let mut eintraege = Vec::new();
        let mut inhalt_seiten = Vec::new();
        for (i, s) in seiten.iter().enumerate() {
            if !s.lines().any(|z| z.trim() == "Inhalt") {
                continue;
            }
            inhalt_seiten.push(i);
            for z in s.lines() {
                let z = z.replace(['\u{2060}', '\u{200b}'], "");
                if let Some(p) = z.find(" . ") {
                    let titel = z[..p].trim().to_string();
                    let zahl = z.rsplit(['.', ' ']).next().unwrap_or("").trim().to_string();
                    if !titel.is_empty() && ist_zahl(&zahl) {
                        eintraege.push((titel, zahl));
                    }
                }
            }
        }
        (eintraege, inhalt_seiten)
    }

    fn pruefe_verzeichnis(seiten: &[String]) {
        let (eintraege, inhalt_seiten) = verzeichnis(seiten);
        assert!(eintraege.len() >= 8, "zu wenige Einträge gefunden: {eintraege:?}");
        for (titel, zahl) in &eintraege {
            let seite = seiten
                .iter()
                .enumerate()
                .filter(|(i, _)| !inhalt_seiten.contains(i))
                .find(|(_, s)| s.lines().any(|z| z.trim() == titel.as_str()))
                .unwrap_or_else(|| panic!("Überschrift „{titel}“ steht auf keiner Seite"));
            assert_eq!(&logische_zahl(seiten, seite.0), zahl, "Inhaltsverzeichnis sagt {zahl} für „{titel}“, die Seite trägt {}", fuss(seite.1));
        }
    }

    #[test]
    fn verzeichnis_stimmt_bei_roemisch_und_durchgehend() {
        if !pdftotext_da() {
            return;
        }
        for z in [Zaehlung::VorspannRoemisch, Zaehlung::Durchgehend] {
            for rechts in [true, false] {
                let opt = SatzOptionen::druck((150.0, 210.0)).mit_zaehlung(z).mit_rechten_seiten(rechts, false);
                let seiten = setzen(&opt);
                pruefe_verzeichnis(&seiten);
            }
        }
        // Auch bei rechten Kapitelanfängen
        let opt = SatzOptionen::druck((150.0, 210.0)).mit_rechten_seiten(true, true);
        pruefe_verzeichnis(&setzen(&opt));
    }

    #[test]
    fn haupttext_beginnt_rechts_mit_eins_und_leerseiten_sind_leer() {
        if !pdftotext_da() {
            return;
        }
        let seiten = setzen(&SatzOptionen::druck((150.0, 210.0)));
        // Seite 2 ist die Leerseite hinter dem Schmutztitel
        assert!(seiten[1].trim().is_empty(), "Leerseite trägt Text: {:?}", seiten[1]);
        // Der Teiltitel ist die erste Seite des Haupttextes (Seite 1, ohne gedruckte Zahl), die Rückseite ist leer,
        // der Text beginnt mit der gedruckten Zahl 3.
        let erste = seiten.iter().position(|s| s.trim() == "Teil 1: Grundlagen").expect("Teiltitel");
        assert_eq!((erste + 1) % 2, 1, "Seite 1 des Haupttextes muss eine rechte (ungerade) Seite sein, ist Blatt {}", erste + 1);
        assert_eq!(fuss(&seiten[erste + 2]), "3", "erste gedruckte Zahl des Haupttextes");
        // Titelei römisch: das Inhaltsverzeichnis trägt eine römische Zahl
        let (_, inhalt) = verzeichnis(&seiten);
        assert!(fuss(&seiten[inhalt[0]]).chars().all(|c| "ivxlcdm".contains(c)), "{:?}", fuss(&seiten[inhalt[0]]));
        // Jede Seite hinter dem Impressum, die nicht leer ist, trägt eine Zahl; Leerseiten tragen keine
        for (i, s) in seiten.iter().enumerate().skip(4) {
            // Leerseiten und Teiltitel (nur die Überschrift) tragen absichtlich keine Zahl
            if s.trim().is_empty() || (s.trim().starts_with("Teil ") && s.trim().lines().count() == 1) {
                continue;
            }
            assert!(ist_zahl(&fuss(s)), "Blatt {} ohne Seitenzahl, Fußzeile: {:?}", i + 1, fuss(s));
        }
    }

    #[test]
    fn durchgehend_zaehlt_ab_dem_ersten_blatt() {
        if !pdftotext_da() {
            return;
        }
        let seiten = setzen(&SatzOptionen::druck((150.0, 210.0)).mit_zaehlung(Zaehlung::Durchgehend).mit_titelei_zahlen(true));
        assert_eq!(fuss(&seiten[0]), "1", "Schmutztitel ist Seite 1");
        let teil = seiten.iter().position(|s| s.lines().any(|z| z.trim() == "Teil 1: Grundlagen") && ist_zahl(&fuss(s))).unwrap();
        assert_eq!(fuss(&seiten[teil]), (teil + 1).to_string(), "Zahl entspricht dem Blatt");
    }

    #[test]
    fn lesen_hat_keine_leerseiten() {
        if !pdftotext_da() {
            return;
        }
        let seiten = setzen(&SatzOptionen::lesen());
        for (i, s) in seiten.iter().enumerate().take(seiten.len().saturating_sub(1)) {
            assert!(!s.trim().is_empty(), "Blatt {} ist leer", i + 1);
        }
    }

    /// Lage der Seitenzahl aus `pdftotext -bbox`: (Seitenbreite, xMin, xMax) je Seite mit Fußzahl.
    fn fusslagen(pdf: &std::path::Path) -> Vec<(f32, f32, f32)> {
        let aus = Command::new("pdftotext").arg("-bbox").arg(pdf).arg("-").output().unwrap();
        let text = String::from_utf8_lossy(&aus.stdout).to_string();
        let attr = |z: &str, n: &str| -> Option<f32> {
            let p = z.find(&format!("{n}=\""))? + n.len() + 2;
            z[p..].split('"').next()?.parse().ok()
        };
        let mut seiten = Vec::new();
        let (mut breite, mut hoehe, mut lage): (f32, f32, Option<(f32, f32)>) = (0.0, 0.0, None);
        for z in text.lines() {
            let z = z.trim();
            if z.starts_with("<page ") {
                breite = attr(z, "width").unwrap_or(0.0);
                hoehe = attr(z, "height").unwrap_or(0.0);
                lage = None;
            } else if z.starts_with("<word ") {
                let wort = z.split('>').nth(1).and_then(|w| w.split('<').next()).unwrap_or("").trim();
                if let (Some(y), Some(x0), Some(x1)) = (attr(z, "yMin"), attr(z, "xMin"), attr(z, "xMax"))
                    && y > hoehe * 0.88
                    && ist_zahl(&wort.replace('\u{2060}', ""))
                {
                    lage = Some((x0, x1));
                }
            } else if z == "</page>" {
                if let Some((x0, x1)) = lage {
                    seiten.push((breite, x0, x1));
                }
            } else if z == "</page>"
                && let Some((x0, x1)) = lage
            {
                seiten.push((breite, x0, x1));
            }
        }
        seiten
    }

    fn pdf_von(opt: &SatzOptionen) -> std::path::PathBuf {
        let e = pdf_setzen(&buch(), opt).unwrap();
        let datei = std::env::temp_dir().join(format!("werkbank-satz-lage-{}-{}.pdf", std::process::id(), opt.seitenzahl as u8));
        std::fs::write(&datei, e.bytes).unwrap();
        datei
    }

    #[test]
    fn stellung_mitte_und_aussen() {
        if !pdftotext_da() {
            return;
        }
        let mitte = fusslagen(&pdf_von(&SatzOptionen::druck((150.0, 210.0)).mit_seitenzahl(Seitenzahl::Mitte)));
        assert!(mitte.len() > 10);
        for (b, x0, x1) in &mitte {
            assert!(((x0 + x1) / 2.0 - b / 2.0).abs() < b * 0.06, "nicht mittig: {x0}..{x1} bei Breite {b}");
        }
        let aussen = fusslagen(&pdf_von(&SatzOptionen::druck((150.0, 210.0)).mit_seitenzahl(Seitenzahl::Aussen)));
        assert!(aussen.len() > 10);
        let (mut rechts, mut links) = (0, 0);
        for (b, x0, x1) in &aussen {
            if *x0 > b * 0.5 {
                assert!(*x1 > b * 0.8, "rechte Zahl nicht außen: {x0}..{x1}");
                rechts += 1;
            } else {
                assert!(*x0 < b * 0.2, "linke Zahl nicht außen: {x0}..{x1}");
                links += 1;
            }
        }
        assert!(rechts > 3 && links > 3, "es muss abwechselnd links und rechts stehen: {links} links, {rechts} rechts");
    }

    fn seitenanzahl(seiten: &[String]) -> usize {
        seiten.len() - 1 // hinter dem letzten Seitenumbruch steht ein leeres Stück
    }

    #[test]
    fn titelei_folge_schmutztitel_leerseite_titel_impressum() {
        if !pdftotext_da() {
            return;
        }
        let seiten = setzen(&SatzOptionen::druck((150.0, 210.0)));
        // 1 rechts: Schmutztitel (nur Kurztitel, ohne Zahl)
        assert!(seiten[0].contains("Zahlenbuch") && !seiten[0].contains("A. Autor"), "Schmutztitel: {:?}", seiten[0]);
        assert!(!ist_zahl(&fuss(&seiten[0])), "Schmutztitel trägt keine Zahl");
        // 2 links: Leerseite
        assert!(seiten[1].trim().is_empty(), "Rückseite des Schmutztitels muss leer sein");
        // 3 rechts: Titelblatt mit Autor, ohne Zahl
        assert!(seiten[2].contains("Zahlenbuch") && seiten[2].contains("A. Autor"));
        assert!(!ist_zahl(&fuss(&seiten[2])));
        // 4 links: Impressum auf der Rückseite des Titelblatts
        assert!(seiten[3].contains('©'), "Impressum: {:?}", seiten[3]);
        assert!(!ist_zahl(&fuss(&seiten[3])));
        // 5 rechts: Inhaltsverzeichnis, römisch nummeriert
        assert!(seiten[4].lines().any(|z| z.trim() == "Inhalt"));
        assert_eq!(fuss(&seiten[4]), "v");
    }

    #[test]
    fn teiltitelseite_hat_leere_rueckseite_und_keine_zahl() {
        if !pdftotext_da() {
            return;
        }
        let seiten = setzen(&SatzOptionen::druck((150.0, 210.0)));
        for titel in ["Teil 1: Grundlagen", "Teil 2: Vertiefung"] {
            let j = seiten.iter().position(|s| s.trim() == titel).unwrap_or_else(|| panic!("Titelseite „{titel}“ fehlt"));
            assert_eq!((j + 1) % 2, 1, "Teiltitel muss auf einer rechten Seite stehen (Blatt {})", j + 1);
            assert!(seiten[j + 1].trim().is_empty(), "Rückseite des Teiltitels muss leer sein: {:?}", seiten[j + 1]);
            assert!(!seiten[j + 2].trim().is_empty(), "danach muss Text folgen");
        }
        let eins = seiten.iter().position(|s| s.trim() == "Teil 1: Grundlagen").unwrap();
        assert!(seiten[eins + 2].contains("Einleitung des Teils."), "Einleitung steht hinter der Leerseite");
        // Ohne Teiltitelseiten steht die Überschrift mit Text auf einer Seite
        let ohne = setzen(&SatzOptionen::druck((150.0, 210.0)).mit_teiltitelseiten(false));
        assert!(ohne.iter().all(|s| s.trim() != "Teil 1: Grundlagen"));
        assert!(ohne.iter().any(|s| s.contains("Teil 1: Grundlagen") && s.contains("Einleitung des Teils.")));
        // Auf Wunsch trägt auch der Teiltitel eine Zahl
        let mit = setzen(&SatzOptionen::druck((150.0, 210.0)).mit_titelei_zahlen(true));
        let j = mit.iter().position(|s| s.contains("Teil 1: Grundlagen") && !s.contains("Einleitung")).unwrap();
        assert!(ist_zahl(&fuss(&mit[j])), "Teiltitel soll Zahl tragen: {:?}", fuss(&mit[j]));
    }

    #[test]
    fn buch_endet_auf_gerader_seite_mit_leerer_letzter_seite() {
        if !pdftotext_da() {
            return;
        }
        for (kapitel, stellung) in [(false, Seitenzahl::Aussen), (true, Seitenzahl::Mitte), (false, Seitenzahl::Mitte)] {
            let seiten = setzen(&SatzOptionen::druck((150.0, 210.0)).mit_rechten_seiten(true, kapitel).mit_seitenzahl(stellung));
            assert_eq!(seitenanzahl(&seiten) % 2, 0, "Seitenzahl muss gerade sein, ist {}", seitenanzahl(&seiten));
        }
        // Mit Füllstoff so, dass der Text auf einer ungeraden Seite endet, kommt eine unnummerierte Leerseite
        let mut p = buch();
        let mut gerade_gefunden = false;
        let mut leer_gefunden = false;
        for n in 1..7 {
            p.teile[3].kapitel[0].inhalt = format!("## Glossar\n\n{}", fuell(n));
            let e = pdf_setzen(&p, &SatzOptionen::druck((150.0, 210.0))).unwrap();
            let datei = std::env::temp_dir().join(format!("werkbank-satz-ende-{n}.pdf"));
            std::fs::write(&datei, e.bytes).unwrap();
            let t = String::from_utf8_lossy(&Command::new("pdftotext").arg("-layout").arg(&datei).arg("-").output().unwrap().stdout).to_string();
            let s: Vec<String> = t.split('\x0c').map(str::to_string).collect();
            assert_eq!(seitenanzahl(&s) % 2, 0);
            gerade_gefunden = true;
            if s[seitenanzahl(&s) - 1].trim().is_empty() {
                leer_gefunden = true;
                assert!(!s[seitenanzahl(&s) - 2].trim().is_empty(), "davor muss Text stehen");
            }
        }
        assert!(gerade_gefunden && leer_gefunden, "bei mindestens einer Länge muss eine Endleerseite entstehen");
        // Das Lese-PDF bekommt keine
        let lesen = setzen(&SatzOptionen::lesen());
        assert!(!lesen[seitenanzahl(&lesen) - 1].trim().is_empty());
    }
}
