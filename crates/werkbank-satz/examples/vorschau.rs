//! Setzt ein Kapitel eines Bandes als Vorschau: `cargo run --release -p werkbank-satz --example vorschau -- <Bandordner> <Kapitelnummer> <Ausgabeordner> [pandoc|nativ]`
use std::path::Path;
use werkbank_kern::Projekt;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let p = Projekt::aus_verzeichnis(Path::new(&a[0])).unwrap();
    let nr: usize = a[1].parse().unwrap();
    let k = p.teile.iter().flat_map(|t| t.kapitel.iter()).nth(nr).unwrap();
    let nummern = werkbank_satz::markdown::kapitelnummern(&p);
    let teile = werkbank_satz::markdown::teil_nummern(&p);
    let t = std::time::Instant::now();
    let e = werkbank_satz::vorschau(&k.inhalt, &p.wurzelverzeichnis, &nummern, &teile, a.get(3).is_none_or(|m| m == "pandoc"), 3, 1.3).unwrap();
    println!("{}: {} Seiten gesetzt in {:.1} s, Hinweise: {:?}", e.quelle, e.seitenzahl, t.elapsed().as_secs_f32(), e.hinweise);
    std::fs::create_dir_all(&a[2]).unwrap();
    for (i, s) in e.seiten.iter().enumerate() {
        std::fs::write(Path::new(&a[2]).join(format!("s{i}_{}x{}.rgba", s.breite, s.hoehe)), &s.rgba).unwrap();
    }
}
