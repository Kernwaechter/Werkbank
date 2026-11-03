//! # Tastenkürzel
//!
//! Alle belegbaren Befehle mit Standardkürzel; Abweichungen speichert die Werkbank in den
//! Einstellungen (`tasten`). Ein Kürzel hat die Schreibweise `Strg+Alt+Umschalt+Taste`,
//! wobei Tasten wie `S`, `,`, `F2`, `ArrowUp` heißen.

use crate::einstellungen::Einstellungen;
use iced::keyboard::{Key, Modifiers, key::{Code, Named, Physical}};
use std::collections::HashMap;

/// Befehl-Kennung, Anzeigename, Standardkürzel (leer = unbelegt).
pub const BEFEHLE: &[(&str, &str, &str)] = &[
    ("palette", "Befehlspalette", "Strg+K"),
    ("speichern", "Speichern", "Strg+S"),
    ("ordner", "Ordner mit Buchprojekt öffnen", "Strg+O"),
    ("pruefen", "Kapitel auf Sprache prüfen", "Strg+L"),
    ("verlauf", "Verlauf des Kapitels", "Strg+H"),
    ("suche", "Im Kapitel suchen und ersetzen", "Strg+F"),
    ("suche_buch", "In allen Bänden suchen und ersetzen", "Strg+Umschalt+F"),
    ("suche_weiter", "Nächster Suchtreffer", "F3"),
    ("suche_zurueck", "Vorheriger Suchtreffer", "Umschalt+F3"),
    ("einstellungen", "Einstellungen", "Strg+,"),
    ("einstellungen2", "Einstellungen (zweites Kürzel)", "F10"),
    ("handbuch", "Handbuch", "F1"),
    ("vorschau", "Live-Vorschau umschalten", "Strg+P"),
    ("seitenleiste", "Seitenleisten umschalten", "Strg+B"),
    ("inspektor", "Details umschalten", "Strg+I"),
    ("fokus", "Fokus-Modus umschalten", "F11"),
    ("schreibmaschine", "Schreibmaschinen-Modus umschalten", ""),
    ("neues_kapitel", "Neues Kapitel", "Strg+Umschalt+N"),
    ("umbenennen", "Kapitel umbenennen", "F2"),
    ("kapitel_hoch", "Kapitel nach oben schieben", "Alt+Umschalt+ArrowUp"),
    ("kapitel_runter", "Kapitel nach unten schieben", "Alt+Umschalt+ArrowDown"),
    ("kapitel_zurueck", "Vorheriges Kapitel", "Alt+ArrowUp"),
    ("kapitel_weiter", "Nächstes Kapitel", "Alt+ArrowDown"),
    ("bereich1", "Gehe zu Schreiben", "Strg+1"),
    ("bereich2", "Gehe zu Bauen", "Strg+2"),
    ("bereich3", "Gehe zu Prüfen", "Strg+3"),
    ("bereich4", "Gehe zu Zahlen", "Strg+4"),
    ("bereich5", "Gehe zu Cover", "Strg+5"),
];

/// Das wirksame Kürzel eines Befehls (Einstellung vor Standard).
pub fn kuerzel_von<'a>(einst: &'a Einstellungen, id: &str) -> &'a str {
    einst
        .tasten
        .get(id)
        .map(String::as_str)
        .or_else(|| BEFEHLE.iter().find(|b| b.0 == id).map(|b| b.2))
        .unwrap_or("")
}

/// Kürzel → Befehl für alle belegten Kürzel.
pub fn belegung(einst: &Einstellungen) -> HashMap<String, &'static str> {
    BEFEHLE
        .iter()
        .filter_map(|(id, _, _)| {
            let k = kuerzel_von(einst, id);
            (!k.is_empty()).then(|| (k.to_string(), *id))
        })
        .collect()
}

/// Lesbare Darstellung (`Alt+ArrowUp` → `Alt+↑`).
pub fn anzeige(kuerzel: &str) -> String {
    if kuerzel.is_empty() {
        return "nicht belegt".to_string();
    }
    kuerzel
        .replace("ArrowUp", "↑")
        .replace("ArrowDown", "↓")
        .replace("ArrowLeft", "←")
        .replace("ArrowRight", "→")
        .replace("Escape", "Esc")
}

/// Kürzel aus einem Tastendruck; reine Zusatztasten ergeben `None`.
pub fn aus_ereignis(key: &Key, physical: &Physical, m: &Modifiers) -> Option<String> {
    let taste = match key {
        Key::Character(c) => c.to_uppercase(),
        Key::Named(Named::Control | Named::Shift | Named::Alt | Named::AltGraph | Named::Super | Named::Meta | Named::Hyper | Named::CapsLock) => {
            return None;
        }
        Key::Named(n) => format!("{n:?}"),
        _ => return None,
    };
    let taste = if matches!(physical, Physical::Code(Code::Comma)) { ",".to_string() } else { taste };
    let mut s = String::new();
    if m.command() {
        s.push_str("Strg+");
    }
    if m.alt() {
        s.push_str("Alt+");
    }
    if m.shift() {
        s.push_str("Umschalt+");
    }
    s.push_str(&taste);
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_und_eigene_belegung() {
        let mut e = Einstellungen::default();
        assert_eq!(kuerzel_von(&e, "speichern"), "Strg+S");
        assert_eq!(belegung(&e).get("Strg+S"), Some(&"speichern"));
        e.tasten.insert("speichern".into(), "Strg+Alt+S".into());
        e.tasten.insert("fokus".into(), String::new());
        let b = belegung(&e);
        assert_eq!(b.get("Strg+Alt+S"), Some(&"speichern"));
        assert!(!b.contains_key("Strg+S") && !b.contains_key("F11"));
    }

    #[test]
    fn standardkuerzel_sind_eindeutig() {
        let mut gesehen = std::collections::HashSet::new();
        for (id, _, k) in BEFEHLE {
            assert!(k.is_empty() || gesehen.insert(*k), "doppeltes Kürzel {k} bei {id}");
        }
    }

    #[test]
    fn ereignis_wird_zum_kuerzel() {
        let strg_umschalt = Modifiers::CTRL | Modifiers::SHIFT;
        let phys = Physical::Unidentified(iced::keyboard::key::NativeCode::Unidentified);
        assert_eq!(aus_ereignis(&Key::Character("f".into()), &phys, &strg_umschalt).as_deref(), Some("Strg+Umschalt+F"));
        assert_eq!(aus_ereignis(&Key::Named(Named::F3), &phys, &Modifiers::SHIFT).as_deref(), Some("Umschalt+F3"));
        assert_eq!(aus_ereignis(&Key::Named(Named::Shift), &phys, &Modifiers::SHIFT), None);
        assert_eq!(aus_ereignis(&Key::Character(".".into()), &Physical::Code(Code::Comma), &Modifiers::CTRL).as_deref(), Some("Strg+,"));
        assert_eq!(anzeige("Alt+ArrowUp"), "Alt+↑");
    }
}
