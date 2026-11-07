//! Übernimmt die Umschlagtexte aus `buchbau.yaml` (rueckseite_*, merkmale, autor_bio, webseiten_zeile …) in die
//! `klappentext.md` jedes Bandes, die künftig die einzige Quelle der Rückseite ist.
//!
//! Der bisherige `klappentext.md` (meist der lange Shop-Text) wird nicht überschrieben, sondern als
//! `klappentext-shop.md` daneben gelegt (eine vorhandene Sicherung bleibt unangetastet).
//!
//! Aufruf: `cargo run -p werkbank-satz --example klappentext_uebernehmen -- <Buchordner> [--machen]`
//! Ohne `--machen` wird nur angezeigt, was geschehen würde.

use std::path::Path;
use werkbank_satz::buchbau::{klappentext, profil::Profil};

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let buch = args.next().ok_or("Aufruf: klappentext_uebernehmen <Buchordner> [--machen]")?;
    let machen = args.any(|a| a == "--machen");
    let buch = Path::new(&buch);
    let profil = Profil::laden(buch)?;
    for band in &profil.baende {
        let ordner = buch.join(&band.ordner);
        let ziel = ordner.join("klappentext.md");
        let sicherung = ordner.join("klappentext-shop.md");
        if band.umschlag.rueckseite_text.trim().is_empty() && band.umschlag.rueckseite_titel.trim().is_empty() {
            println!("{}: keine Umschlagtexte in buchbau.yaml, übersprungen", band.ordner);
            continue;
        }
        let neu = klappentext::aus_umschlag(&profil, band);
        println!("== {} → {}{}", band.ordner, ziel.display(), if machen { "" } else { " (Probelauf)" });
        if ziel.is_file() && !sicherung.exists() {
            println!("   alter Text wird nach {} gesichert", sicherung.display());
            if machen {
                std::fs::copy(&ziel, &sicherung).map_err(|e| format!("{}: {e}", sicherung.display()))?;
            }
        }
        if machen {
            std::fs::write(&ziel, &neu).map_err(|e| format!("{}: {e}", ziel.display()))?;
        } else {
            println!("{neu}");
        }
    }
    Ok(())
}
