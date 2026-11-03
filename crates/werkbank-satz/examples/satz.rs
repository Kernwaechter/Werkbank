//! Setzt einen Band nativ: `cargo run --release -p werkbank-satz --example satz -- <Bandordner> <Ausgabeordner>`
use std::path::Path;
use werkbank_kern::Projekt;
use werkbank_satz::{SatzOptionen, epub_schreiben, pdf_setzen};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let p = Projekt::aus_verzeichnis(Path::new(&args[0])).expect("laden");
    let aus = Path::new(&args[1]);
    std::fs::create_dir_all(aus).unwrap();
    let t = std::time::Instant::now();
    let e = pdf_setzen(&p, &SatzOptionen::druck((190.0, 270.0))).unwrap_or_else(|f| panic!("Satz fehlgeschlagen:\n{f}"));
    std::fs::write(aus.join("druck.pdf"), &e.bytes).unwrap();
    println!("Druck-PDF: {} Seiten, {:.1} MB, {:.1} s, {} Warnungen", e.seiten, e.bytes.len() as f32 / 1e6, t.elapsed().as_secs_f32(), e.warnungen.len());
    for w in e.warnungen.iter().take(3) {
        println!("  Warnung: {w}");
    }
    let t = std::time::Instant::now();
    epub_schreiben(&p, None, &aus.join("buch.epub")).unwrap();
    println!("EPUB: {:.1} s", t.elapsed().as_secs_f32());
}
