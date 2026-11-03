//! # Suchen und Ersetzen
//!
//! Durchsucht Kapitel, einen Band oder alle Bände eines Buchs und ersetzt Treffer, wahlweise
//! mit regulären Ausdrücken (`$1` im Ersatz setzt Gruppen ein) und als ganzes Wort.
//! Positionen sind Byte-Offsets im Kapiteltext, damit der Editor sie direkt nutzen kann.

use crate::Projekt;
use regex::{Regex, RegexBuilder};
use std::path::PathBuf;

/// Höchstanzahl gesammelter Treffer je Band.
const MAX_TREFFER: usize = 2000;

/// Wie gesucht wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Optionen {
    /// Groß- und Kleinschreibung unterscheiden.
    pub gross_klein: bool,
    /// Nur ganze Wörter finden.
    pub ganzes_wort: bool,
    /// Das Suchmuster ist ein regulärer Ausdruck (Zeilenanfang `^` und -ende `$` gelten je Zeile).
    pub regex: bool,
}

/// Ein Fund im Buch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fund {
    /// Kapitel, in dem der Fund liegt.
    pub kapitel_id: String,
    /// Titel dieses Kapitels.
    pub kapitel_titel: String,
    /// Datei des Kapitels (relativ zum Band).
    pub datei: String,
    /// Byte-Offset im Kapiteltext.
    pub byte: usize,
    /// Länge des Fundes in Bytes.
    pub laenge: usize,
    /// Zeile (ab 0) im Kapiteltext.
    pub zeile: usize,
    /// Spalte (Byte-Index in der Zeile).
    pub spalte: usize,
    /// Die ganze Zeile als Kontext (gekürzt).
    pub kontext: String,
    /// Dieselbe Zeile, wie sie nach dem Ersetzen aussähe (gekürzt).
    pub kontext_neu: String,
    /// Der Text, durch den dieser Fund ersetzt würde (bei regulären Ausdrücken mit eingesetzten Gruppen).
    pub ersatz: String,
}

/// Baut aus dem Suchtext den Ausdruck. Fehler sind lesbar formuliert.
pub fn ausdruck(muster: &str, opt: Optionen) -> Result<Regex, String> {
    let mut p = if opt.regex { muster.to_string() } else { regex::escape(muster) };
    if opt.ganzes_wort {
        p = format!(r"\b(?:{p})\b");
    }
    RegexBuilder::new(&p)
        .case_insensitive(!opt.gross_klein)
        .multi_line(true)
        .size_limit(5_000_000)
        .build()
        .map_err(|e| {
            let text = e.to_string();
            let letzte = text.lines().last().unwrap_or("").trim().trim_start_matches("error:").trim().to_string();
            format!("Ungültiger Ausdruck: {letzte}")
        })
}

/// Byte-Bereiche aller Vorkommen samt Ersatztext (leere Treffer zählen nicht).
fn vorkommen(text: &str, re: &Regex, ersatz: &str, opt: Optionen) -> Vec<(usize, usize, String)> {
    re.captures_iter(text)
        .filter_map(|c| {
            let m = c.get(0)?;
            if m.as_str().is_empty() {
                return None;
            }
            let neu = if opt.regex {
                let mut aus = String::new();
                c.expand(ersatz, &mut aus);
                aus
            } else {
                ersatz.to_string()
            };
            Some((m.start(), m.len(), neu))
        })
        .collect()
}

fn kuerzen(zeile: &str) -> String {
    zeile.trim().chars().take(120).collect()
}

fn funde_im_projekt(projekt: &Projekt, re: &Regex, ersatz: &str, opt: Optionen, nur_kapitel: Option<&str>, funde: &mut Vec<Fund>) {
    for k in projekt.teile.iter().flat_map(|t| t.kapitel.iter()) {
        if nur_kapitel.is_some_and(|id| id != k.id) {
            continue;
        }
        for (byte, laenge, neu) in vorkommen(&k.inhalt, re, ersatz, opt) {
            let zeilenanfang = k.inhalt[..byte].rfind('\n').map_or(0, |i| i + 1);
            let zeilenende = k.inhalt[byte..].find('\n').map_or(k.inhalt.len(), |i| byte + i);
            let zeile = &k.inhalt[zeilenanfang..zeilenende];
            let mut zeile_neu = zeile.to_string();
            let bis = (byte - zeilenanfang + laenge).min(zeile.len());
            if zeile.is_char_boundary(bis) {
                zeile_neu.replace_range(byte - zeilenanfang..bis, &neu);
            }
            funde.push(Fund {
                kapitel_id: k.id.clone(),
                kapitel_titel: k.titel.clone(),
                datei: k.dateipfad.to_string_lossy().into_owned(),
                byte,
                laenge,
                zeile: k.inhalt[..byte].matches('\n').count(),
                spalte: byte - zeilenanfang,
                kontext: kuerzen(zeile),
                kontext_neu: kuerzen(&zeile_neu),
                ersatz: neu,
            });
            if funde.len() >= MAX_TREFFER {
                return;
            }
        }
    }
}

/// Sucht im aktuellen Kapitel (`nur_kapitel`) oder im ganzen Band.
pub fn finden(projekt: &Projekt, muster: &str, ersatz: &str, opt: Optionen, nur_kapitel: Option<&str>) -> Result<Vec<Fund>, String> {
    if muster.is_empty() {
        return Ok(Vec::new());
    }
    let re = ausdruck(muster, opt)?;
    let mut funde = Vec::new();
    funde_im_projekt(projekt, &re, ersatz, opt, nur_kapitel, &mut funde);
    Ok(funde)
}

/// Ersetzt alle Vorkommen. Liefert Anzahl, betroffene Teile und Kapitel.
pub fn alle_ersetzen(
    projekt: &mut Projekt,
    muster: &str,
    ersatz: &str,
    opt: Optionen,
    nur_kapitel: Option<&str>,
) -> Result<(usize, Vec<String>, Vec<String>), String> {
    let (mut anzahl, mut teile, mut kapitel) = (0, Vec::new(), Vec::new());
    if muster.is_empty() {
        return Ok((anzahl, teile, kapitel));
    }
    let re = ausdruck(muster, opt)?;
    for t in &mut projekt.teile {
        for k in &mut t.kapitel {
            if nur_kapitel.is_some_and(|id| id != k.id) {
                continue;
            }
            let treffer = vorkommen(&k.inhalt, &re, ersatz, opt);
            if treffer.is_empty() {
                continue;
            }
            for (byte, laenge, neu) in treffer.iter().rev() {
                k.inhalt.replace_range(*byte..*byte + *laenge, neu);
            }
            anzahl += treffer.len();
            kapitel.push(k.id.clone());
            if !teile.contains(&t.id) {
                teile.push(t.id.clone());
            }
        }
    }
    Ok((anzahl, teile, kapitel))
}

/// Ersetzt genau einen Fund, sofern der Text dort noch stimmt. Liefert die Teil-Kennung.
pub fn fund_ersetzen(projekt: &mut Projekt, fund: &Fund) -> Option<String> {
    let teil = projekt.teile.iter_mut().find(|t| t.kapitel.iter().any(|k| k.id == fund.kapitel_id))?;
    let k = teil.kapitel.iter_mut().find(|k| k.id == fund.kapitel_id)?;
    k.inhalt.get(fund.byte..fund.byte + fund.laenge)?;
    k.inhalt.replace_range(fund.byte..fund.byte + fund.laenge, &fund.ersatz);
    Some(teil.id.clone())
}

// ------------------------------------------------------------------ Mehrere Bände

/// Ein Fund in einem anderen Band des Buchs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BandFund {
    /// Anzeigename des Bandes.
    pub band: String,
    /// Ordner des Bandes.
    pub ordner: PathBuf,
    pub fund: Fund,
}

/// Sucht in mehreren Bänden, die von der Festplatte gelesen werden.
pub fn in_baenden_finden(baende: &[(String, PathBuf)], muster: &str, ersatz: &str, opt: Optionen) -> Result<Vec<BandFund>, String> {
    if muster.is_empty() {
        return Ok(Vec::new());
    }
    let re = ausdruck(muster, opt)?;
    let mut aus = Vec::new();
    for (name, ordner) in baende {
        let projekt = Projekt::aus_verzeichnis(ordner).map_err(|e| format!("{name}: {e}"))?;
        let mut funde = Vec::new();
        funde_im_projekt(&projekt, &re, ersatz, opt, None, &mut funde);
        aus.extend(funde.into_iter().map(|fund| BandFund { band: name.clone(), ordner: ordner.clone(), fund }));
    }
    Ok(aus)
}

/// Ersetzt in mehreren Bänden auf der Festplatte. Jede geänderte Datei bekommt vorher eine
/// nummerierte Sicherung. Liefert (Ersetzungen, geänderte Dateien).
pub fn in_baenden_ersetzen(baende: &[(String, PathBuf)], muster: &str, ersatz: &str, opt: Optionen) -> Result<(usize, usize), String> {
    ausdruck(muster, opt)?;
    let (mut anzahl, mut dateien) = (0, 0);
    for (name, ordner) in baende {
        let mut projekt = Projekt::aus_verzeichnis(ordner).map_err(|e| format!("{name}: {e}"))?;
        let (n, teile, _) = alle_ersetzen(&mut projekt, muster, ersatz, opt, None)?;
        if n > 0 {
            let gespeichert = projekt.teile_speichern_nummeriert(&teile).map_err(|e| format!("{name}: {e}"))?;
            dateien += gespeichert.len();
            anzahl += n;
        }
    }
    Ok((anzahl, dateien))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kapitel, Teil};

    fn projekt() -> Projekt {
        let mut p = Projekt::neu("/tmp/x");
        let mut t = Teil::neu("t", "T");
        t.kapitel_hinzufuegen(Kapitel::neu("a", "A", "t.md", "## A\nDer Borrow Checker.\nborrow checker klein\n"));
        t.kapitel_hinzufuegen(Kapitel::neu("b", "B", "t.md", "## B\nNochmal Borrow Checker\n"));
        p.teile.push(t);
        p
    }

    const GK: Optionen = Optionen { gross_klein: true, ganzes_wort: false, regex: false };
    const EGAL: Optionen = Optionen { gross_klein: false, ganzes_wort: false, regex: false };

    #[test]
    fn findet_mit_und_ohne_gross_klein() {
        let p = projekt();
        assert_eq!(finden(&p, "Borrow Checker", "", GK, None).unwrap().len(), 2);
        assert_eq!(finden(&p, "Borrow Checker", "", EGAL, None).unwrap().len(), 3);
        assert_eq!(finden(&p, "Borrow Checker", "", EGAL, Some("b")).unwrap().len(), 1);
        let f = &finden(&p, "borrow", "", EGAL, Some("a")).unwrap()[0];
        assert_eq!((f.zeile, f.spalte), (1, 4));
        assert_eq!(f.kontext, "Der Borrow Checker.");
    }

    #[test]
    fn ersetzt_alle_und_einzeln() {
        let mut p = projekt();
        let (n, teile, kapitel) = alle_ersetzen(&mut p, "Borrow Checker", "Ausleihprüfer", GK, None).unwrap();
        assert_eq!((n, teile, kapitel), (2, vec!["t".to_string()], vec!["a".to_string(), "b".to_string()]));
        let f = finden(&p, "borrow checker", "X", GK, None).unwrap().remove(0);
        fund_ersetzen(&mut p, &f).unwrap();
        assert!(p.kapitel_finden("a").unwrap().inhalt.contains("\nX klein"));
    }

    #[test]
    fn ganzes_wort_und_umlaute() {
        let mut p = Projekt::neu("/tmp/x");
        let mut t = Teil::neu("t", "T");
        t.kapitel_hinzufuegen(Kapitel::neu("a", "A", "t.md", "Ein Fall, Überfall, fallen. Über und Übertrag.\n"));
        p.teile.push(t);
        let wort = Optionen { ganzes_wort: true, ..EGAL };
        // "Überfall" und "fallen" enthalten "fall", sind aber kein ganzes Wort
        assert_eq!(finden(&p, "fall", "", wort, None).unwrap().len(), 1);
        assert_eq!(finden(&p, "fall", "", Optionen { gross_klein: true, ..wort }, None).unwrap().len(), 0);
        assert_eq!(finden(&p, "Fall", "", Optionen { gross_klein: true, ..wort }, None).unwrap().len(), 1);
        assert_eq!(finden(&p, "über", "", wort, None).unwrap().len(), 1, "Ü am Wortanfang gilt als Buchstabe");
        assert_eq!(finden(&p, "über", "", EGAL, None).unwrap().len(), 3);
    }

    #[test]
    fn regulaere_ausdruecke_mit_gruppen_und_vorschau() {
        let mut p = Projekt::neu("/tmp/x");
        let mut t = Teil::neu("t", "T");
        t.kapitel_hinzufuegen(Kapitel::neu("a", "A", "t.md", "Siehe Kapitel 3 und Kapitel 12.\nzweite Zeile\n"));
        p.teile.push(t);
        let re = Optionen { regex: true, gross_klein: true, ..Default::default() };
        let f = finden(&p, r"Kapitel (\d+)", "Abschnitt $1", re, None).unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0].kontext_neu, "Siehe Abschnitt 3 und Kapitel 12.");
        let (n, _, _) = alle_ersetzen(&mut p, r"Kapitel (\d+)", "Abschnitt $1", re, None).unwrap();
        assert_eq!(n, 2);
        assert!(p.kapitel_finden("a").unwrap().inhalt.starts_with("Siehe Abschnitt 3 und Abschnitt 12.\n"));
        // Zeilenende gilt je Zeile
        let ende = Optionen { regex: true, gross_klein: true, ..Default::default() };
        assert_eq!(finden(&p, r"Zeile$", "", ende, None).unwrap().len(), 1);
        // ohne Regex wird das Dollarzeichen im Ersatz wörtlich gesetzt
        let (n, _, _) = alle_ersetzen(&mut p, "zweite", "$1", GK, None).unwrap();
        assert_eq!(n, 1);
        assert!(p.kapitel_finden("a").unwrap().inhalt.contains("$1 Zeile"));
    }

    #[test]
    fn kaputter_ausdruck_gibt_lesbaren_fehler() {
        let p = projekt();
        let re = Optionen { regex: true, ..Default::default() };
        let f = finden(&p, "(offen", "", re, None).unwrap_err();
        assert!(f.starts_with("Ungültiger Ausdruck"), "{f}");
        // Leeres Muster und leere Treffer (a*) liefern nichts
        assert!(finden(&p, "", "", re, None).unwrap().is_empty());
        assert!(finden(&p, "x*", "", re, None).unwrap().is_empty());
    }

    #[test]
    fn mehrere_baende_auf_der_festplatte() {
        let wurzel = std::env::temp_dir().join(format!("werkbank-suche-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&wurzel);
        let mut baende = Vec::new();
        for n in 1..=2 {
            let o = wurzel.join(format!("Band {n}"));
            std::fs::create_dir_all(&o).unwrap();
            std::fs::write(o.join("teil-1.md"), format!("# Teil\n\n## Kapitel\n\nDer Borrow Checker in Band {n}.\n")).unwrap();
            baende.push((format!("Band {n}"), o));
        }
        let funde = in_baenden_finden(&baende, "Borrow Checker", "Ausleihprüfer", GK).unwrap();
        assert_eq!(funde.len(), 2);
        assert_eq!(funde[1].band, "Band 2");
        assert!(funde[0].fund.kontext_neu.contains("Ausleihprüfer"));
        let (n, dateien) = in_baenden_ersetzen(&baende, "Borrow Checker", "Ausleihprüfer", GK).unwrap();
        assert_eq!((n, dateien), (2, 2));
        let text = std::fs::read_to_string(baende[1].1.join("teil-1.md")).unwrap();
        assert!(text.contains("Der Ausleihprüfer in Band 2."), "{text}");
        // die alte Fassung liegt als nummerierte Sicherung vor
        let sicherung = baende[0].1.join(".werkbank/backups/teil-1/teil-1-0001.md");
        assert!(std::fs::read_to_string(&sicherung).unwrap().contains("Borrow Checker in Band 1"));
        let _ = std::fs::remove_dir_all(&wurzel);
    }
}
