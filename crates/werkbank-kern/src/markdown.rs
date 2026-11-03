//! # Markdown-Hilfen
//!
//! Zeilenweise Auswertung von Markdown mit korrekter Behandlung von
//! Codezäunen (```` ``` ```` und `~~~`), damit `## ` oder `#` in Codeblöcken
//! nie als Überschrift gilt.

/// Eine Überschrift im Text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ueberschrift {
    /// Ebene 1 bis 6.
    pub ebene: usize,
    /// Text ohne `#`-Zeichen.
    pub text: String,
    /// Zeile (ab 0).
    pub zeile: usize,
    /// Byte-Offset des Zeilenanfangs.
    pub byte: usize,
}

/// Ein Codeblock im Text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Codeblock {
    /// Sprache (erstes Wort der Infozeile), z. B. `rust`.
    pub sprache: String,
    /// Komplette Infozeile hinter dem Zaun, z. B. `rust,no_run`.
    pub info: String,
    /// Inhalt ohne Zäune.
    pub code: String,
    /// Zeile (ab 0) des öffnenden Zauns.
    pub start_zeile: usize,
}

/// Zeile samt Byte-Anfang und Zustand.
#[derive(Debug, Clone, Copy)]
pub struct ZeilenInfo<'a> {
    /// Byte-Offset des Zeilenanfangs.
    pub start: usize,
    /// Zeilentext ohne Zeilenende.
    pub text: &'a str,
    /// Liegt die Zeile in einem Codeblock (Zäune eingeschlossen)?
    pub im_code: bool,
}

/// Erkennt einen Zaun: liefert (Zeichen, Länge, Rest der Zeile).
fn zaun(zeile: &str) -> Option<(char, usize, &str)> {
    let ohne = zeile.trim_start_matches(' ');
    if zeile.len() - ohne.len() > 3 {
        return None;
    }
    let zeichen = ohne.chars().next().filter(|c| *c == '`' || *c == '~')?;
    let laenge = ohne.chars().take_while(|c| *c == zeichen).count();
    if laenge < 3 {
        return None;
    }
    Some((zeichen, laenge, &ohne[laenge..]))
}

/// Zerlegt Text in Zeilen mit Codezaun-Zustand. Die Summe der Zeilenlängen plus Zeilenenden ergibt den Text.
pub fn zeilen(text: &str) -> Vec<ZeilenInfo<'_>> {
    let mut ergebnis = Vec::new();
    let mut offen: Option<(char, usize)> = None;
    let mut start = 0;
    for roh in text.split_inclusive('\n') {
        let zeile = roh.strip_suffix('\n').unwrap_or(roh);
        let zeile = zeile.strip_suffix('\r').unwrap_or(zeile);
        let mut im_code = offen.is_some();
        if let Some((z, l, rest)) = zaun(zeile) {
            match offen {
                None => {
                    // Ein Zaun mit Backticks darf in der Infozeile keine Backticks haben (sonst Inline-Code)
                    if !(z == '`' && rest.contains('`')) {
                        offen = Some((z, l));
                        im_code = true;
                    }
                }
                Some((oz, ol)) if oz == z && l >= ol && rest.trim().is_empty() => {
                    offen = None;
                    im_code = true;
                }
                _ => {}
            }
        }
        ergebnis.push(ZeilenInfo { start, text: zeile, im_code });
        start += roh.len();
    }
    ergebnis
}

/// Alle Überschriften außerhalb von Codeblöcken.
pub fn ueberschriften(text: &str) -> Vec<Ueberschrift> {
    zeilen(text)
        .into_iter()
        .enumerate()
        .filter(|(_, z)| !z.im_code)
        .filter_map(|(nr, z)| {
            let ebene = z.text.chars().take_while(|c| *c == '#').count();
            let rest = z.text[ebene..].strip_prefix(' ')?;
            (1..=6).contains(&ebene).then(|| Ueberschrift {
                ebene,
                text: rest.trim().to_string(),
                zeile: nr,
                byte: z.start,
            })
        })
        .collect()
}

/// Alle Codeblöcke des Textes.
pub fn codebloecke(text: &str) -> Vec<Codeblock> {
    let mut bloecke = Vec::new();
    let mut aktuell: Option<(Codeblock, char, usize)> = None;
    for (nr, z) in zeilen(text).into_iter().enumerate() {
        match (&mut aktuell, zaun(z.text)) {
            (None, Some((zeichen, laenge, rest))) if z.im_code => {
                let info = rest.trim().to_string();
                let sprache = info
                    .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
                    .next()
                    .unwrap_or_default()
                    .trim_matches('.')
                    .to_string();
                aktuell = Some((
                    Codeblock { sprache, info, code: String::new(), start_zeile: nr },
                    zeichen,
                    laenge,
                ));
            }
            (Some((_, zeichen, laenge)), Some((z2, l2, rest))) if *zeichen == z2 && l2 >= *laenge && rest.trim().is_empty() => {
                if let Some((block, _, _)) = aktuell.take() {
                    bloecke.push(block);
                }
            }
            (Some((block, _, _)), _) => {
                block.code.push_str(z.text);
                block.code.push('\n');
            }
            _ => {}
        }
    }
    bloecke
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "# Titel\n\nText\n\n```text\n## kein Kapitel\n# auch nicht\n```\n\n## Echt\n\n~~~rust,no_run\nfn main() {}\n~~~\n";

    #[test]
    fn ueberschriften_ignorieren_codebloecke() {
        let u = ueberschriften(TEXT);
        let texte: Vec<_> = u.iter().map(|x| x.text.as_str()).collect();
        assert_eq!(texte, ["Titel", "Echt"]);
        assert_eq!(u[1].ebene, 2);
        assert_eq!(&TEXT[u[1].byte..u[1].byte + 7], "## Echt");
    }

    #[test]
    fn codebloecke_werden_extrahiert() {
        let b = codebloecke(TEXT);
        assert_eq!(b.len(), 2);
        assert_eq!(b[0].sprache, "text");
        assert!(b[0].code.contains("## kein Kapitel"));
        assert_eq!(b[1].sprache, "rust");
        assert_eq!(b[1].info, "rust,no_run");
        assert_eq!(b[1].code, "fn main() {}\n");
    }

    #[test]
    fn zeilen_ergeben_originaltext() {
        let z = zeilen(TEXT);
        assert_eq!(z.last().unwrap().start + z.last().unwrap().text.len() + 1, TEXT.len());
    }

    #[test]
    fn inline_code_ist_kein_zaun() {
        assert!(codebloecke("```rust``` inline\ntext\n").is_empty());
    }
}
