//! # Markierungen im Editor
//!
//! Eigener Highlighter für `text_editor`: färbt Überschriften, Code, Zitate,
//! `@kap:`-Verweise und Fettdruck und markiert Rechtschreib- und
//! LanguageTool-Hinweise farbig (Iced kann keine Wellenlinien).

use crate::rechtschreibung::{self, Woerterbuch};
use crate::thema;
use iced::advanced::text::highlighter::{self, Format};
use iced::font::Font;
use iced::Theme;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

/// Art einer markierten Stelle (Reihenfolge = Rangfolge, spätere überdecken frühere).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Art {
    Zitat,
    Ueberschrift,
    Fett,
    Verweis,
    Code,
    Rechtschreibung,
    Sprache,
}

/// Ein Hinweis der Sprachprüfung als Zeilenbereich (Zeile ab 0, Byte-Spalten).
pub type Bereich = (usize, usize, usize, usize);

/// Einstellungen des Markierers. Gleichheit gilt nur über `version`.
#[derive(Clone)]
pub struct Einstellung {
    pub version: u64,
    pub woerterbuch: Option<Arc<Woerterbuch>>,
    pub eigene: Arc<HashSet<String>>,
    /// (Zeile, Spalte, Endzeile, Endspalte) der LanguageTool-Hinweise.
    pub hinweise: Arc<Vec<Bereich>>,
    /// Wellenlinien, die der Markierer beim Hervorheben füllt und der Editor zeichnet.
    pub wellen: crate::editor::Wellen,
}

impl PartialEq for Einstellung {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

impl Einstellung {
    /// Prüfsumme über alles, was die Markierung beeinflusst.
    pub fn version_berechnen(
        woerterbuch: &Option<Arc<Woerterbuch>>,
        eigene: &HashSet<String>,
        hinweise: &[Bereich],
        hell: bool,
    ) -> u64 {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        woerterbuch.as_ref().map(|w| (&w.name, w.ordner.clone())).hash(&mut h);
        let mut woerter: Vec<_> = eigene.iter().collect();
        woerter.sort();
        woerter.hash(&mut h);
        hinweise.hash(&mut h);
        hell.hash(&mut h);
        h.finish()
    }
}

/// Der Highlighter selbst.
pub struct Markierer {
    einstellung: Einstellung,
    zeile: usize,
    /// Zaun, der am Anfang der jeweiligen Zeile offen ist (Zeichen, Länge).
    zaeune: Vec<Option<(char, usize)>>,
    offen: Option<(char, usize)>,
}

impl Markierer {
    fn wellen_setzen(&self, nr: usize, liste: Vec<(Range<usize>, iced::Color)>) {
        if let Ok(mut w) = self.einstellung.wellen.lock() {
            if w.len() <= nr {
                w.resize(nr + 1, Vec::new());
            }
            w[nr] = liste;
        }
    }
}

fn zaun(zeile: &str) -> Option<(char, usize, &str)> {
    let ohne = zeile.trim_start_matches(' ');
    if zeile.len() - ohne.len() > 3 {
        return None;
    }
    let z = ohne.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let l = ohne.chars().take_while(|c| *c == z).count();
    (l >= 3).then(|| (z, l, &ohne[l..]))
}

impl highlighter::Highlighter for Markierer {
    type Settings = Einstellung;
    type Highlight = Art;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Art)>;

    fn new(settings: &Self::Settings) -> Self {
        Self { einstellung: settings.clone(), zeile: 0, zaeune: Vec::new(), offen: None }
    }

    fn update(&mut self, new_settings: &Self::Settings) {
        self.einstellung = new_settings.clone();
        self.change_line(0);
    }

    fn change_line(&mut self, line: usize) {
        // Zaunzustand der ersten geänderten Zeile wiederherstellen
        self.offen = if line == 0 { None } else { self.zaeune.get(line).copied().flatten() };
        self.zaeune.truncate(line);
        self.zeile = line;
        if let Ok(mut w) = self.einstellung.wellen.lock() {
            w.truncate(line);
        }
    }

    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        if self.zaeune.len() == self.zeile {
            self.zaeune.push(self.offen);
        }
        let nr = self.zeile;
        self.zeile += 1;
        let ergebnis = self.analysieren(nr, line);
        let wellen = ergebnis
            .iter()
            .filter_map(|(r, a)| match a {
                Art::Rechtschreibung => Some((r.clone(), thema::fehler_rot())),
                Art::Sprache => Some((r.clone(), thema::warnung_orange())),
                _ => None,
            })
            .collect();
        self.wellen_setzen(nr, wellen);
        ergebnis.into_iter()
    }

    fn current_line(&self) -> usize {
        self.zeile
    }
}

impl Markierer {
    fn analysieren(&mut self, nr: usize, zeile: &str) -> Vec<(Range<usize>, Art)> {
        if zeile.is_empty() {
            return Vec::new();
        }
        // Codezäune und Codeblöcke
        if let Some((z, l, rest)) = zaun(zeile) {
            match self.offen {
                None if !(z == '`' && rest.contains('`')) => {
                    self.offen = Some((z, l));
                    return vec![(0..zeile.len(), Art::Code)];
                }
                Some((oz, ol)) if oz == z && l >= ol && rest.trim().is_empty() => {
                    self.offen = None;
                    return vec![(0..zeile.len(), Art::Code)];
                }
                _ => {}
            }
        }
        if self.offen.is_some() {
            return vec![(0..zeile.len(), Art::Code)];
        }

        let mut art: Vec<Option<Art>> = vec![None; zeile.len()];
        let mut setze = |bereich: Range<usize>, a: Art| {
            for b in &mut art[bereich] {
                if b.is_none_or(|alt| a > alt) {
                    *b = Some(a);
                }
            }
        };

        // Überschriften und Zitate
        let hashes = zeile.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) && zeile[hashes..].starts_with(' ') {
            setze(0..zeile.len(), Art::Ueberschrift);
        } else if zeile.trim_start().starts_with('>') {
            setze(0..zeile.len(), Art::Zitat);
        }

        // Inline-Code, Fettdruck, Verweise
        let mut start = None;
        for (i, c) in zeile.char_indices() {
            if c == '`' {
                match start.take() {
                    Some(s) => setze(s..i + 1, Art::Code),
                    None => start = Some(i),
                }
            }
        }
        let mut s = 0;
        while let Some(p) = zeile[s..].find("**") {
            let von = s + p;
            match zeile[von + 2..].find("**") {
                Some(q) => {
                    setze(von..von + 2 + q + 2, Art::Fett);
                    s = von + 2 + q + 2;
                }
                None => break,
            }
        }
        let mut s = 0;
        while let Some(p) = zeile[s..].find("@kap:") {
            let von = s + p;
            let bis = zeile[von..].find(|c: char| c.is_whitespace() || ",;)".contains(c)).map_or(zeile.len(), |e| von + e);
            setze(von..bis, Art::Verweis);
            s = bis.max(von + 5);
        }

        // Rechtschreibung
        if let Some(wb) = self.einstellung.woerterbuch.clone() {
            for r in rechtschreibung::woerter_in_zeile(zeile) {
                if !wb.ist_richtig(&zeile[r.clone()], &self.einstellung.eigene) {
                    setze(r, Art::Rechtschreibung);
                }
            }
        }

        // LanguageTool-Hinweise
        for &(z0, s0, z1, s1) in self.einstellung.hinweise.iter() {
            if nr < z0 || nr > z1 {
                continue;
            }
            let von = if nr == z0 { s0 } else { 0 };
            let bis = if nr == z1 { s1 } else { zeile.len() };
            if von < bis && bis <= zeile.len() && zeile.is_char_boundary(von) && zeile.is_char_boundary(bis) {
                setze(von..bis, Art::Sprache);
            }
        }

        // Läufe gleicher Art zu Bereichen zusammenfassen
        let mut ergebnis = Vec::new();
        let mut i = 0;
        while i < art.len() {
            match art[i] {
                None => i += 1,
                Some(a) => {
                    let mut j = i;
                    while j < art.len() && art[j] == Some(a) {
                        j += 1;
                    }
                    ergebnis.push((i..j, a));
                    i = j;
                }
            }
        }
        ergebnis
    }
}

/// Darstellung einer Markierung (gerade/ungerade Adresse siehe `schreiben.rs`).
pub fn format(art: &Art, _theme: &Theme) -> Format<Font> {
    let fett = thema::editor_font(true);
    match art {
        Art::Zitat => Format { color: Some(thema::text_gedimmt()), font: None },
        Art::Ueberschrift => Format { color: Some(thema::akzent()), font: Some(fett) },
        Art::Fett => Format { color: None, font: Some(fett) },
        Art::Verweis => Format { color: Some(thema::akzent()), font: None },
        Art::Code => Format {
            color: Some(if thema::ist_hell() { iced::Color::from_rgb8(135, 84, 20) } else { iced::Color::from_rgb8(222, 184, 135) }),
            font: None,
        },
        Art::Rechtschreibung => Format { color: Some(thema::fehler_rot()), font: None },
        Art::Sprache => Format { color: Some(thema::warnung_orange()), font: None },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::advanced::text::Highlighter as _;

    fn markierer(wb: bool, hinweise: Vec<Bereich>) -> Markierer {
        let woerterbuch = wb.then(|| Arc::new(Woerterbuch::aus_texten("SET UTF-8\n", "4\nDas\nist\nein\nHund\n")));
        Markierer::new(&Einstellung { version: 1, woerterbuch, eigene: Arc::new(HashSet::new()), hinweise: Arc::new(hinweise), wellen: Default::default() })
    }

    fn zeile(m: &mut Markierer, z: &str) -> Vec<(String, Art)> {
        m.highlight_line(z).map(|(r, a)| (z[r].to_string(), a)).collect()
    }

    #[test]
    fn ueberschrift_code_und_zaeune_ueber_zeilen() {
        let mut m = markierer(false, vec![]);
        assert_eq!(zeile(&mut m, "## Titel"), [("## Titel".to_string(), Art::Ueberschrift)]);
        assert_eq!(zeile(&mut m, "```rust")[0].1, Art::Code);
        assert_eq!(zeile(&mut m, "## kein Titel"), [("## kein Titel".to_string(), Art::Code)]);
        assert_eq!(zeile(&mut m, "```")[0].1, Art::Code);
        assert_eq!(zeile(&mut m, "## wieder Titel")[0].1, Art::Ueberschrift);
    }

    #[test]
    fn change_line_stellt_zaunzustand_wieder_her() {
        let mut m = markierer(false, vec![]);
        for z in ["text", "```", "code", "code2", "```", "danach"] {
            m.highlight_line(z).for_each(drop);
        }
        m.change_line(3);
        assert_eq!(m.current_line(), 3);
        assert_eq!(zeile(&mut m, "code2")[0].1, Art::Code, "Zeile 3 liegt noch im Block");
        m.change_line(5);
        assert!(zeile(&mut m, "danach").is_empty(), "nach dem Block normaler Text");
    }

    #[test]
    fn inline_elemente_und_rechtschreibung() {
        let mut m = markierer(true, vec![]);
        let r = zeile(&mut m, "Das ist `x` ein **Hnud** siehe @kap:a");
        assert!(r.contains(&("`x`".to_string(), Art::Code)));
        assert!(r.contains(&("Hnud".to_string(), Art::Rechtschreibung)), "Rechtschreibung überdeckt Fett: {r:?}");
        assert!(r.contains(&("@kap:a".to_string(), Art::Verweis)));
        assert!(!r.iter().any(|(t, a)| t == "Hund" && *a == Art::Rechtschreibung));
    }

    #[test]
    fn languagetool_hinweise_ueber_mehrere_zeilen() {
        let mut m = markierer(false, vec![(0, 4, 1, 3)]);
        assert_eq!(zeile(&mut m, "Das ist schlecht"), [("ist schlecht".to_string(), Art::Sprache)]);
        assert_eq!(zeile(&mut m, "Weitermachen"), [("Wei".to_string(), Art::Sprache)]);
    }

    #[test]
    fn version_aendert_sich_mit_den_eingaben() {
        let e: HashSet<String> = HashSet::new();
        let v1 = Einstellung::version_berechnen(&None, &e, &[], false);
        assert_eq!(v1, Einstellung::version_berechnen(&None, &e, &[], false));
        assert_ne!(v1, Einstellung::version_berechnen(&None, &e, &[(0, 1, 0, 2)], false));
        assert_ne!(v1, Einstellung::version_berechnen(&None, &e, &[], true));
    }
}
