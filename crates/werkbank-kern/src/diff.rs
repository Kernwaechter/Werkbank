//! # Vergleich zweier Textstände
//!
//! Für die Zeitleiste: Ein älterer Stand wird mit dem aktuellen Text verglichen, zuerst
//! absatzweise, innerhalb geänderter Stellen wortweise. Jede zusammenhängende Änderung
//! lässt sich einzeln zurückholen („Passage übernehmen“).
//!
//! Blickrichtung: Es wird gezeigt, was beim Zurückholen des älteren Standes passiert.
//! [`Art::Alt`] steht nur im älteren Stand (käme zurück), [`Art::Neu`] steht nur im
//! aktuellen Text (ginge verloren).

/// Obergrenze der Tabellenzellen für den genauen Vergleich; darüber wird gröber verglichen.
const MAX_ZELLEN: usize = 4_000_000;

/// Herkunft eines Textstücks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Art {
    /// In beiden Ständen vorhanden.
    Gleich,
    /// Nur im älteren Stand (käme beim Zurückholen wieder).
    Alt,
    /// Nur im aktuellen Text (ginge beim Zurückholen verloren).
    Neu,
}

/// Ein Stück Text mit seiner Herkunft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stueck {
    pub art: Art,
    pub text: String,
}

/// Eine zusammenhängende Änderung (ein oder mehrere Absätze).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aenderung {
    /// Absätze im älteren Stand.
    pub alt: Vec<String>,
    /// Absätze im aktuellen Text.
    pub neu: Vec<String>,
    /// Wortgenaue Darstellung (Alt vor Neu, dazwischen Gleiches).
    pub stuecke: Vec<Stueck>,
}

/// Abschnitt des Vergleichs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    Gleich(String),
    Aenderung(Aenderung),
}

/// Ergebnis des Vergleichs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Vergleich {
    pub bloecke: Vec<Block>,
}

impl Vergleich {
    /// Vergleicht den älteren Stand `alt` mit dem aktuellen Text `neu`.
    pub fn neu(alt: &str, neu: &str) -> Self {
        let a: Vec<&str> = alt.split("\n\n").collect();
        let n: Vec<&str> = neu.split("\n\n").collect();
        let mut bloecke = Vec::new();

        let mut vorn = 0;
        while vorn < a.len() && vorn < n.len() && a[vorn] == n[vorn] {
            vorn += 1;
        }
        let mut hinten = 0;
        while hinten < a.len() - vorn && hinten < n.len() - vorn && a[a.len() - 1 - hinten] == n[n.len() - 1 - hinten] {
            hinten += 1;
        }
        for p in &a[..vorn] {
            bloecke.push(Block::Gleich((*p).to_string()));
        }
        let (ma, mn) = (&a[vorn..a.len() - hinten], &n[vorn..n.len() - hinten]);
        match ops(ma, mn) {
            Some(ops) => {
                let (mut alt_lauf, mut neu_lauf): (Vec<&str>, Vec<&str>) = (Vec::new(), Vec::new());
                for op in ops {
                    match op {
                        Op::Gleich(i) => {
                            hunk_abschliessen(&mut bloecke, &mut alt_lauf, &mut neu_lauf);
                            bloecke.push(Block::Gleich(ma[i].to_string()));
                        }
                        Op::Alt(i) => alt_lauf.push(ma[i]),
                        Op::Neu(j) => neu_lauf.push(mn[j]),
                    }
                }
                hunk_abschliessen(&mut bloecke, &mut alt_lauf, &mut neu_lauf);
            }
            None => {
                let (mut al, mut nl): (Vec<&str>, Vec<&str>) = (ma.to_vec(), mn.to_vec());
                hunk_abschliessen(&mut bloecke, &mut al, &mut nl);
            }
        }
        for p in &a[a.len() - hinten..] {
            bloecke.push(Block::Gleich((*p).to_string()));
        }
        Self { bloecke }
    }

    /// Anzahl der Änderungen.
    pub fn anzahl(&self) -> usize {
        self.bloecke.iter().filter(|b| matches!(b, Block::Aenderung(_))).count()
    }

    /// Sind beide Stände gleich?
    pub fn ist_gleich(&self) -> bool {
        self.anzahl() == 0
    }

    /// Wörter, die beim Zurückholen hinzukämen und wegfielen: `(kaemen, fielen)`.
    pub fn woerter(&self) -> (usize, usize) {
        let (mut alt, mut neu) = (0, 0);
        for b in &self.bloecke {
            if let Block::Aenderung(a) = b {
                for s in &a.stuecke {
                    let n = s.text.split_whitespace().count();
                    match s.art {
                        Art::Alt => alt += n,
                        Art::Neu => neu += n,
                        Art::Gleich => {}
                    }
                }
            }
        }
        (alt, neu)
    }

    /// Der aktuelle Text, in dem nur Änderung Nr. `nr` (ab 0) auf den älteren Stand zurückgesetzt ist.
    pub fn eine_zurueck(&self, nr: usize) -> String {
        self.zusammensetzen(|i| i == nr)
    }

    /// Der ältere Stand vollständig.
    pub fn alt_text(&self) -> String {
        self.zusammensetzen(|_| true)
    }

    /// Der aktuelle Text vollständig.
    pub fn neu_text(&self) -> String {
        self.zusammensetzen(|_| false)
    }

    fn zusammensetzen(&self, alt_nehmen: impl Fn(usize) -> bool) -> String {
        let mut absaetze: Vec<&str> = Vec::new();
        let mut nr = 0;
        for b in &self.bloecke {
            match b {
                Block::Gleich(t) => absaetze.push(t),
                Block::Aenderung(a) => {
                    let quelle = if alt_nehmen(nr) { &a.alt } else { &a.neu };
                    absaetze.extend(quelle.iter().map(String::as_str));
                    nr += 1;
                }
            }
        }
        absaetze.join("\n\n")
    }
}

fn hunk_abschliessen(bloecke: &mut Vec<Block>, alt: &mut Vec<&str>, neu: &mut Vec<&str>) {
    if alt.is_empty() && neu.is_empty() {
        return;
    }
    let alt_a: Vec<String> = alt.drain(..).map(str::to_string).collect();
    let neu_a: Vec<String> = neu.drain(..).map(str::to_string).collect();
    let stuecke = wortvergleich(&alt_a.join("\n\n"), &neu_a.join("\n\n"));
    bloecke.push(Block::Aenderung(Aenderung { alt: alt_a, neu: neu_a, stuecke }));
}

/// Wortgenauer Vergleich zweier Texte.
fn wortvergleich(alt: &str, neu: &str) -> Vec<Stueck> {
    let a = zerlegen(alt);
    let n = zerlegen(neu);
    let mut vorn = 0;
    while vorn < a.len() && vorn < n.len() && a[vorn] == n[vorn] {
        vorn += 1;
    }
    let mut hinten = 0;
    while hinten < a.len() - vorn && hinten < n.len() - vorn && a[a.len() - 1 - hinten] == n[n.len() - 1 - hinten] {
        hinten += 1;
    }
    let mut aus: Vec<Stueck> = Vec::new();
    let mut anhaengen = |art: Art, t: &str| {
        if t.is_empty() {
            return;
        }
        match aus.last_mut() {
            Some(l) if l.art == art => l.text.push_str(t),
            _ => aus.push(Stueck { art, text: t.to_string() }),
        }
    };
    for t in &a[..vorn] {
        anhaengen(Art::Gleich, t);
    }
    let (ma, mn) = (&a[vorn..a.len() - hinten], &n[vorn..n.len() - hinten]);
    match ops(ma, mn) {
        Some(ops) => {
            // Alt vor Neu innerhalb einer zusammenhängenden Änderung
            let (mut alt_lauf, mut neu_lauf) = (String::new(), String::new());
            for op in ops {
                match op {
                    Op::Gleich(i) => {
                        anhaengen(Art::Alt, &std::mem::take(&mut alt_lauf));
                        anhaengen(Art::Neu, &std::mem::take(&mut neu_lauf));
                        anhaengen(Art::Gleich, ma[i]);
                    }
                    Op::Alt(i) => alt_lauf.push_str(ma[i]),
                    Op::Neu(j) => neu_lauf.push_str(mn[j]),
                }
            }
            anhaengen(Art::Alt, &alt_lauf);
            anhaengen(Art::Neu, &neu_lauf);
        }
        None => {
            anhaengen(Art::Alt, &ma.concat());
            anhaengen(Art::Neu, &mn.concat());
        }
    }
    for t in &a[a.len() - hinten..] {
        anhaengen(Art::Gleich, t);
    }
    aus
}

/// Zerlegt in Wörter und Zwischenräume (Satzzeichen hängen am Wort).
fn zerlegen(text: &str) -> Vec<&str> {
    let mut teile = Vec::new();
    let mut start = 0;
    let mut leer: Option<bool> = None;
    for (i, c) in text.char_indices() {
        let ist_leer = c.is_whitespace();
        if leer.is_some_and(|l| l != ist_leer) {
            teile.push(&text[start..i]);
            start = i;
        }
        leer = Some(ist_leer);
    }
    if start < text.len() {
        teile.push(&text[start..]);
    }
    teile
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Gleich(usize),
    Alt(usize),
    Neu(usize),
}

/// Längste gemeinsame Teilfolge; `None`, wenn die Tabelle zu groß würde.
fn ops<T: PartialEq>(a: &[T], b: &[T]) -> Option<Vec<Op>> {
    let (n, m) = (a.len(), b.len());
    if n == 0 || m == 0 {
        return Some((0..n).map(Op::Alt).chain((0..m).map(Op::Neu)).collect());
    }
    if (n + 1).saturating_mul(m + 1) > MAX_ZELLEN {
        return None;
    }
    let breite = m + 1;
    let mut t = vec![0u32; (n + 1) * breite];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            t[i * breite + j] = if a[i] == b[j] {
                t[(i + 1) * breite + j + 1] + 1
            } else {
                t[(i + 1) * breite + j].max(t[i * breite + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut aus = Vec::with_capacity(n + m);
    while i < n && j < m {
        if a[i] == b[j] {
            aus.push(Op::Gleich(i));
            i += 1;
            j += 1;
        } else if t[(i + 1) * breite + j] >= t[i * breite + j + 1] {
            aus.push(Op::Alt(i));
            i += 1;
        } else {
            aus.push(Op::Neu(j));
            j += 1;
        }
    }
    aus.extend((i..n).map(Op::Alt));
    aus.extend((j..m).map(Op::Neu));
    Some(aus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gleiche_texte_haben_keine_aenderung() {
        let v = Vergleich::neu("a\n\nb\n\nc", "a\n\nb\n\nc");
        assert!(v.ist_gleich());
        assert_eq!(v.neu_text(), "a\n\nb\n\nc");
    }

    #[test]
    fn aenderungen_lassen_sich_einzeln_zuruecknehmen() {
        let alt = "eins\n\nzwei\n\ndrei\n\nvier\n\nfünf";
        let neu = "eins\n\nZWEI\n\ndrei\n\nvier\n\nfünf\n\nsechs";
        let v = Vergleich::neu(alt, neu);
        assert_eq!(v.anzahl(), 2);
        assert_eq!(v.alt_text(), alt);
        assert_eq!(v.neu_text(), neu);
        assert_eq!(v.eine_zurueck(0), "eins\n\nzwei\n\ndrei\n\nvier\n\nfünf\n\nsechs");
        assert_eq!(v.eine_zurueck(1), "eins\n\nZWEI\n\ndrei\n\nvier\n\nfünf");
    }

    #[test]
    fn wortgenau_innerhalb_eines_absatzes() {
        let v = Vergleich::neu("Sie sah den Mond.", "Sie sah einen Mond.");
        let Block::Aenderung(a) = &v.bloecke[0] else { panic!("{:?}", v) };
        assert!(a.stuecke.contains(&Stueck { art: Art::Alt, text: "den".into() }));
        assert!(a.stuecke.contains(&Stueck { art: Art::Neu, text: "einen".into() }));
        assert_eq!(v.woerter(), (1, 1));
    }

    #[test]
    fn einfuegen_und_loeschen_ganzer_absaetze() {
        let v = Vergleich::neu("a\n\nb", "a");
        assert_eq!(v.anzahl(), 1);
        assert_eq!(v.woerter(), (1, 0));
        assert_eq!(v.alt_text(), "a\n\nb");
        let w = Vergleich::neu("a", "a\n\nb");
        assert_eq!(w.eine_zurueck(0), "a");
    }

    #[test]
    fn leere_texte() {
        let v = Vergleich::neu("", "x");
        assert_eq!(v.alt_text(), "");
        assert_eq!(v.neu_text(), "x");
    }
}
