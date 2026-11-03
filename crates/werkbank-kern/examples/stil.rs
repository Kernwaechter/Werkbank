//! Stilprofil aller Kapitel eines Bandes: `cargo run --release -p werkbank-kern --example stil -- <Bandordner>`
use std::path::Path;
use werkbank_kern::{Projekt, stil};

fn main() {
    let p = Projekt::aus_verzeichnis(Path::new(&std::env::args().nth(1).expect("Bandordner"))).expect("laden");
    let mut werte = Vec::new();
    for k in p.teile.iter().flat_map(|t| t.kapitel.iter()) {
        let s = stil::analysieren(&k.inhalt);
        if s.urteil != stil::Urteil::ZuWenigText {
            werte.push(s.punktzahl);
            if werte.len() <= 12 || s.punktzahl < 45 {
                println!("{:3}  {:28} {}", s.punktzahl, s.urteil.name(), k.titel);
            }
        }
    }
    werte.sort();
    println!("{} Kapitel, Median {}, min {}, max {}", werte.len(), werte[werte.len() / 2], werte[0], werte[werte.len() - 1]);
}
