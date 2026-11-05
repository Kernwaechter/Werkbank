//! # Rechtschreibung offline (Hunspell-Wörterbücher)
//!
//! Lädt ein Hunspell-Wörterbuch (`de_DE.aff`/`.dic`), zerlegt Zeilen in
//! prüfbare Wörter und lässt Code, Verweise, Dateinamen und Fachwörter aus.
//! Die Prüfung ist schnell genug, um beim Tippen im Editor zu markieren.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::ops::Range;
use std::path::PathBuf;
use std::sync::Mutex;

/// Fachwörter aus dem Buch, die kein Wörterbuch kennt.
pub const FACHWOERTER: &[&str] = &[
    "rust", "cargo", "rustup", "rustc", "crate", "crates", "trait", "traits", "closure", "closures", "borrow", "checker",
    "enum", "enums", "struct", "structs", "iterator", "iteratoren", "lifetime", "lifetimes", "compiler", "typst", "pandoc",
    "tokio", "async", "await", "mutex", "slice", "slices", "refactoring", "debugging", "debugger", "github", "markdown",
    "epub", "pdf", "tolino", "kindle", "ownership", "match", "matching", "pattern", "patterns", "generics", "boxed",
    "dioxus", "wasm", "toml", "json", "yaml", "api", "cli", "gui", "ide", "bool", "string", "strings", "vec", "option",
    "result", "unwrap", "tuple", "tupel", "feature", "features", "workspace", "release", "debug", "build", "builds",
    "linux", "macos", "windows", "terminal", "shell", "bash", "flutter", "dart", "widget", "widgets", "jurzel",
];

/// Ein geladenes Hunspell-Wörterbuch.
pub struct Woerterbuch {
    dict: spellbook::Dictionary,
    cache: Mutex<HashMap<String, bool>>,
    /// Erster Korrekturvorschlag je Wort. `suggest` ist teuer (zig ms pro Wort)
    /// und der Inspektor fragt bei jedem Neuzeichnen danach.
    vorschlag_cache: Mutex<HashMap<String, Option<String>>>,
    /// Name der Wörterbuchdatei, z. B. `de_DE`.
    pub name: String,
    /// Ordner, aus dem es stammt.
    pub ordner: PathBuf,
}

impl fmt::Debug for Woerterbuch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Woerterbuch({} aus {})", self.name, self.ordner.display())
    }
}

/// Üblicher Suchpfad für Hunspell-Wörterbücher.
pub fn suchordner(eigener: &str) -> Vec<PathBuf> {
    let mut liste = Vec::new();
    if !eigener.trim().is_empty() {
        liste.push(PathBuf::from(eigener.trim()));
    }
    if let Some(k) = crate::einstellungen::konfig_ordner() {
        liste.push(k.join("woerterbuecher"));
    }
    // Mitgelieferte Wörterbücher neben dem Programm (Windows-Paket)
    if let Some(exe) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) {
        liste.push(exe.join("woerterbuecher"));
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        liste.push(home.join(".local/share/hunspell"));
        liste.push(home.join("Library/Spelling"));
        liste.push(home.join(".config/enchant"));
    }
    for fest in [
        "/usr/share/hunspell",
        "/usr/share/myspell",
        "/usr/share/myspell/dicts",
        "/usr/local/share/hunspell",
        "/Library/Spelling",
        "/app/share/hunspell",
        "/run/host/usr/share/hunspell",
        "/run/host/usr/share/myspell",
        "/opt/homebrew/share/hunspell",
        "C:\\Program Files\\LibreOffice\\share\\extensions\\dict-de",
    ] {
        liste.push(PathBuf::from(fest));
    }
    liste
}

impl Woerterbuch {
    /// Lädt das Wörterbuch für einen Sprachcode wie `de-DE` (Datei `de_DE`).
    pub fn laden(sprache: &str, eigener_ordner: &str) -> Result<Self, String> {
        let name = if sprache == "auto" { "de_DE".to_string() } else { sprache.replace('-', "_") };
        for ordner in suchordner(eigener_ordner) {
            let (aff, dic) = (ordner.join(format!("{name}.aff")), ordner.join(format!("{name}.dic")));
            if aff.is_file() && dic.is_file() {
                let aff_text = std::fs::read_to_string(&aff).map_err(|e| format!("{}: {e}", aff.display()))?;
                let dic_text = std::fs::read_to_string(&dic).map_err(|e| format!("{}: {e}", dic.display()))?;
                let dict = spellbook::Dictionary::new(&aff_text, &dic_text)
                    .map_err(|e| format!("Wörterbuch {name} nicht lesbar: {e}"))?;
                return Ok(Self { dict, cache: Mutex::new(HashMap::new()), vorschlag_cache: Mutex::new(HashMap::new()), name, ordner });
            }
        }
        Err(format!("Kein Hunspell-Wörterbuch „{name}“ gefunden (Pakete hunspell-de oder LibreOffice-Wörterbücher)."))
    }

    /// Aus Texten (für Tests).
    #[cfg(test)]
    pub fn aus_texten(aff: &str, dic: &str) -> Self {
        Self {
            dict: spellbook::Dictionary::new(aff, dic).expect("Wörterbuch"),
            cache: Mutex::new(HashMap::new()),
            vorschlag_cache: Mutex::new(HashMap::new()),
            name: "test".to_string(),
            ordner: PathBuf::new(),
        }
    }

    fn wort_ok(&self, wort: &str) -> bool {
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(&r) = cache.get(wort) {
            return r;
        }
        let r = self.dict.check(wort);
        if cache.len() > 200_000 {
            cache.clear();
        }
        cache.insert(wort.to_string(), r);
        r
    }

    /// Ist das Wort richtig (oder ein bekanntes Fachwort / eigenes Wort)?
    pub fn ist_richtig(&self, wort: &str, eigene: &HashSet<String>) -> bool {
        let klein = wort.to_lowercase();
        if eigene.contains(wort) || eigene.contains(&klein) || FACHWOERTER.contains(&klein.as_str()) {
            return true;
        }
        if self.wort_ok(wort) {
            return true;
        }
        // Bindestrich-Wörter: jedes Teil einzeln prüfen
        if wort.contains('-') {
            return wort.split('-').filter(|t| t.chars().count() > 1).all(|t| self.ist_richtig(t, eigene));
        }
        false
    }

    /// Bester Korrekturvorschlag, zwischengespeichert pro Wort.
    pub fn erster_vorschlag(&self, wort: &str) -> Option<String> {
        if let Some(v) = self.vorschlag_cache.lock().unwrap_or_else(|e| e.into_inner()).get(wort) {
            return v.clone();
        }
        // Ohne Sperre rechnen: `suggest` kann dauern
        let v = self.vorschlaege(wort).into_iter().next();
        let mut cache = self.vorschlag_cache.lock().unwrap_or_else(|e| e.into_inner());
        if cache.len() > 20_000 {
            cache.clear();
        }
        cache.insert(wort.to_string(), v.clone());
        v
    }

    /// Korrekturvorschläge.
    pub fn vorschlaege(&self, wort: &str) -> Vec<String> {
        let mut aus = Vec::new();
        self.dict.suggest(wort, &mut aus);
        aus.truncate(6);
        aus
    }
}

/// Wörter einer Zeile außerhalb von Inline-Code, Verweisen, Adressen und Dateinamen.
/// Liefert Byte-Bereiche; bei `ueberspringen` liegende Bereiche werden ausgelassen.
pub fn woerter_in_zeile(zeile: &str) -> Vec<Range<usize>> {
    let zeichen: Vec<(usize, char)> = zeile.char_indices().collect();
    let mut ergebnis = Vec::new();

    // Bereiche, die nicht geprüft werden: `code`, @kap:…, [..](url)-Adressen, http(s)://…
    let mut tabu: Vec<Range<usize>> = Vec::new();
    let mut offen: Option<usize> = None;
    for &(i, c) in &zeichen {
        if c == '`' {
            match offen.take() {
                Some(s) => tabu.push(s..i + 1),
                None => offen = Some(i),
            }
        }
    }
    if let Some(s) = offen {
        tabu.push(s..zeile.len());
    }
    for muster in ["@kap:", "http://", "https://", "](", "[id]"] {
        let mut start = 0;
        while let Some(p) = zeile[start..].find(muster) {
            let von = start + p;
            let bis = zeile[von..].find(|c: char| c.is_whitespace() || c == ')').map_or(zeile.len(), |e| von + e);
            tabu.push(von..bis.max(von + muster.len()).min(zeile.len()));
            start = von + muster.len();
        }
    }
    // HTML-Tags samt Attributen: <div class="page-break">, </div>
    let mut s = 0;
    while let Some(p) = zeile[s..].find('<') {
        let von = s + p;
        let ist_tag = zeile[von + 1..].chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '/' || c == '!');
        match zeile[von..].find('>') {
            Some(q) if ist_tag => {
                tabu.push(von..von + q + 1);
                s = von + q + 1;
            }
            _ => s = von + 1,
        }
    }
    // Heading-Kennungen in eckigen Klammern: „Kapitel [werkzeug]: Titel“
    let mut s = 0;
    while let Some(p) = zeile[s..].find('[') {
        let von = s + p;
        if let Some(q) = zeile[von..].find(']') {
            tabu.push(von..von + q + 1);
            s = von + q + 1;
        } else {
            break;
        }
    }

    let ist_wort = |c: char| c.is_alphabetic() || c == '\'' || c == '’' || c == '-';
    let mut i = 0;
    while i < zeichen.len() {
        if !zeichen[i].1.is_alphabetic() {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < zeichen.len() && ist_wort(zeichen[j].1) {
            j += 1;
        }
        let von = zeichen[i].0;
        let bis = zeichen.get(j).map_or(zeile.len(), |z| z.0);
        // Ränder bereinigen (Bindestriche, Apostrophe)
        let wort = &zeile[von..bis];
        let rand_links = wort.len() - wort.trim_start_matches(['-', '\'', '’']).len();
        let rand_rechts = wort.len() - wort.trim_end_matches(['-', '\'', '’']).len();
        let (von2, bis2) = (von + rand_links, bis - rand_rechts);
        i = j.max(i + 1);
        if von2 >= bis2 {
            continue;
        }
        let w = &zeile[von2..bis2];
        let davor = zeile[..von2].chars().next_back();
        let danach = zeile[bis2..].chars().next();
        let nach_dem = zeile[bis2..].chars().nth(1);
        let technisch = davor.is_some_and(|c| c.is_ascii_digit() || c == '_' || c == ':' || c == '/' || c == '\\' || c == '#' || c == '$' || c == '%')
            || danach.is_some_and(|c| c.is_ascii_digit() || c == '_' || c == ':' || c == '/' || c == '\\' || c == '(')
            || (danach == Some('.') && nach_dem.is_some_and(char::is_alphanumeric))
            || (davor == Some('.') && zeile[..von2.saturating_sub(1)].chars().next_back().is_some_and(char::is_alphanumeric));
        let buchstaben = w.chars().filter(|c| c.is_alphabetic()).count();
        let alles_gross = w.chars().filter(|c| c.is_alphabetic()).all(char::is_uppercase);
        let mischschreibung = w.chars().skip(1).any(char::is_uppercase) && !w.contains('-');
        if technisch || buchstaben < 2 || alles_gross || mischschreibung {
            continue;
        }
        if tabu.iter().any(|t| t.start < bis2 && von2 < t.end) {
            continue;
        }
        ergebnis.push(von2..bis2);
    }
    ergebnis
}

/// Zählt unbekannte Wörter eines Textes (ohne Codeblöcke), häufigste zuerst.
pub fn unbekannte_woerter(wb: &Woerterbuch, text: &str, eigene: &HashSet<String>) -> Vec<(String, usize)> {
    let mut zaehler: HashMap<String, usize> = HashMap::new();
    for z in werkbank_kern::markdown::zeilen(text) {
        if z.im_code {
            continue;
        }
        for r in woerter_in_zeile(z.text) {
            let w = &z.text[r];
            if !wb.ist_richtig(w, eigene) {
                *zaehler.entry(w.to_string()).or_default() += 1;
            }
        }
    }
    let mut liste: Vec<_> = zaehler.into_iter().collect();
    liste.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    liste
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_tags_werden_nicht_geprueft() {
        let z = "<div class=\"page-break\"></div> Hallo Welt";
        let woerter: Vec<&str> = woerter_in_zeile(z).into_iter().map(|r| &z[r]).collect();
        assert_eq!(woerter, ["Hallo", "Welt"]);
        let z = "a < b und c > d";
        assert!(woerter_in_zeile(z).iter().any(|r| &z[r.clone()] == "und"), "Vergleiche sind keine Tags");
    }

    fn wb() -> Woerterbuch {
        Woerterbuch::aus_texten("SET UTF-8\n", "5\nHund\nKatze\nist\nschön\nund\n")
    }

    fn texte(zeile: &str) -> Vec<&str> {
        woerter_in_zeile(zeile).into_iter().map(|r| &zeile[r]).collect()
    }

    #[test]
    fn code_verweise_und_dateinamen_werden_ausgelassen() {
        assert_eq!(texte("Der `HashMap` ist schön, siehe @kap:grundlagen und datei.rs oder https://x.de/abc."), ["Der", "ist", "schön", "siehe", "und", "oder"]);
        assert_eq!(texte("## Kapitel [werkzeug]: Werkzeug"), ["Kapitel", "Werkzeug"]);
        assert_eq!(texte("HashMap iPhone API std::vec snake_case 42er"), Vec::<&str>::new());
    }

    #[test]
    fn umlaute_bindestriche_und_positionen() {
        let z = "Größe Rust-Werkstatt";
        let r = woerter_in_zeile(z);
        assert_eq!(&z[r[0].clone()], "Größe");
        assert_eq!(&z[r[1].clone()], "Rust-Werkstatt");
    }

    #[test]
    fn prueft_woerter_mit_eigenen_und_fachwoertern() {
        let w = wb();
        let eigene: HashSet<String> = ["Werkbank".to_string()].into();
        assert!(w.ist_richtig("Hund", &eigene));
        assert!(!w.ist_richtig("Hnud", &eigene));
        assert!(w.ist_richtig("Werkbank", &eigene));
        assert!(w.ist_richtig("Borrow", &eigene), "Fachwort");
        assert!(w.ist_richtig("Hund-Katze", &eigene), "Bindestrich-Teile einzeln");
        let u = unbekannte_woerter(&w, "Hnud und Hund. Hnud!\n```text\nXyzzy\n```\n", &eigene);
        assert_eq!(u, vec![("Hnud".to_string(), 2)]);
    }

    #[test]
    fn echtes_deutsches_woerterbuch_falls_vorhanden() {
        let Ok(w) = Woerterbuch::laden("de-DE", "") else { return };
        let e = HashSet::new();
        assert!(w.ist_richtig("Werkstatt", &e));
        assert!(w.ist_richtig("Schreibwerkzeug", &e), "Kompositum");
        assert!(!w.ist_richtig("Beipsiel", &e));
        assert!(w.vorschlaege("Beipsiel").iter().any(|v| v == "Beispiel"));
        assert_eq!(w.erster_vorschlag("Beipsiel"), w.erster_vorschlag("Beipsiel"), "Cache liefert dasselbe");
    }
}
