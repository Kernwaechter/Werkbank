//! Syntaxhervorhebung ohne Pygments.
//!
//! Für das EPUB entsteht HTML mit den Klassennamen von Pygments (`k`, `s1`, `c1`, `nc`, `mi`, `o`, `p`),
//! damit das bisherige Stylesheet unverändert passt. Für die PDFs setzt Typst den Code selbst; dafür
//! liefert [`typst_thema`] ein TextMate-Thema mit denselben Farben (bzw. nur fett/kursiv im Graudruck).
//! Die Sprachdefinitionen sind dieselben, die Typst verwendet (`two-face`, u. a. Dart, Rust, YAML, Bash).

/// Maskierung wie Pygments' HtmlFormatter (auch ' und ").
fn maskieren(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}
use std::sync::LazyLock;
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

static SPRACHEN: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_no_newlines);

/// Pygments-Klasse für einen Bereich (innerster passender Bereich gewinnt).
fn klasse(stapel: &ScopeStack) -> Option<&'static str> {
    for scope in stapel.as_slice().iter().rev() {
        let s = scope.build_string();
        let k = if s.starts_with("comment") {
            "c1"
        } else if s.starts_with("string") {
            "s1"
        } else if s.starts_with("constant.numeric") {
            "mi"
        } else if s.starts_with("keyword.operator") {
            "o"
        } else if s.starts_with("keyword") || s.starts_with("storage.type") || s.starts_with("storage.modifier") {
            "k"
        } else if s.starts_with("entity.name.function") || s.starts_with("support.function") {
            "nf"
        } else if s.starts_with("entity.name") || s.starts_with("support.class") || s.starts_with("support.type") {
            "nc"
        } else if s.starts_with("punctuation") {
            "p"
        } else {
            continue;
        };
        return Some(k);
    }
    None
}

/// Hebt Code als HTML-Spans hervor (wie Pygments mit `nowrap=True`, inklusive abschließendem Zeilenumbruch).
pub fn html_spans(code: &str, sprache: &str) -> String {
    let syntax = SPRACHEN
        .find_syntax_by_token(sprache.trim())
        .or_else(|| SPRACHEN.find_syntax_by_token(&sprache.trim().to_lowercase()));
    let Some(syntax) = syntax else {
        return format!("{}\n", maskieren(code));
    };
    let mut zustand = ParseState::new(syntax);
    let mut stapel = ScopeStack::new();
    let mut aus = String::new();
    for zeile in code.split('\n') {
        let ops = zustand.parse_line(zeile, &SPRACHEN).unwrap_or_default();
        let mut pos = 0;
        let ausgeben = |aus: &mut String, text: &str, stapel: &ScopeStack| {
            if text.is_empty() {
                return;
            }
            match klasse(stapel) {
                Some(k) => aus.push_str(&format!("<span class=\"{k}\">{}</span>", maskieren(text))),
                None => aus.push_str(&maskieren(text)),
            }
        };
        for (stelle, op) in ops {
            ausgeben(&mut aus, &zeile[pos..stelle], &stapel);
            pos = stelle;
            let _ = stapel.apply(&op);
        }
        ausgeben(&mut aus, &zeile[pos..], &stapel);
        aus.push('\n');
    }
    aus
}

/// Gibt es eine Sprachdefinition für diesen Namen?
pub fn kennt(sprache: &str) -> bool {
    SPRACHEN.find_syntax_by_token(sprache.trim()).is_some()
}

/// TextMate-Thema für Typst-Codeblöcke in den Farben des bisherigen Buchs (farbig) bzw. nur mit
/// Fett/Kursiv für den Graudruck (ohne Farbe, damit alles reines Grau bleibt).
pub fn typst_thema(farbig: bool) -> String {
    let eintrag = |name: &str, scope: &str, farbe: Option<&str>, stil: &str| {
        let farbe = farbe.map(|f| format!("<key>foreground</key><string>{f}</string>")).unwrap_or_default();
        let stil = if stil.is_empty() { String::new() } else { format!("<key>fontStyle</key><string>{stil}</string>") };
        format!("<dict><key>name</key><string>{name}</string><key>scope</key><string>{scope}</string><key>settings</key><dict>{farbe}{stil}</dict></dict>")
    };
    let f = |c: &'static str| farbig.then_some(c);
    let regeln = [
        eintrag("Kommentar", "comment", f("#57606A"), "italic"),
        eintrag("Zeichenkette", "string", f("#0A3069"), ""),
        eintrag("Zahl", "constant.numeric", f("#0550AE"), if farbig { "" } else { "bold" }),
        eintrag("Operator", "keyword.operator, punctuation", f("#24292F"), ""),
        eintrag("Schlüsselwort", "keyword, storage.type, storage.modifier", f("#0550AE"), "bold"),
        eintrag("Name", "entity.name, support.class, support.type, support.function", f("#8250DF"), "bold"),
    ]
    .join("");
    let grund = if farbig { "<key>foreground</key><string>#24292F</string>" } else { "" };
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>name</key><string>Werkstatt</string><key>settings</key><array>
<dict><key>settings</key><dict>{grund}</dict></dict>{regeln}</array></dict></plist>"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dart_wird_hervorgehoben() {
        let h = html_spans("void main() {\n  // Hallo\n  print('x');\n}", "dart");
        assert!(h.contains("<span class=\"c1\">"), "{h}");
        assert!(h.contains("<span class=\"s1\">"), "{h}");
        assert!(h.ends_with("}\n") || h.ends_with("</span>\n"), "{h}");
        assert!(kennt("dart") && kennt("rust") && kennt("yaml"));
    }

    #[test]
    fn unbekannte_sprache_bleibt_text() {
        assert_eq!(html_spans("a < 'b'", "gibtsnicht"), "a &lt; &#39;b&#39;\n");
    }

    #[test]
    fn grauthema_ohne_farben() {
        assert!(!typst_thema(false).contains("foreground"));
        assert!(typst_thema(true).contains("#0550AE"));
    }
}
