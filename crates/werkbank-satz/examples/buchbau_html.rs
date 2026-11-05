//! Schreibt das HTML des Buchbau-Übersetzers je Quelldatei: `cargo run -p werkbank-satz --example buchbau_html -- <Bandordner> <Ausgabeordner> <datei.md>...`
//! (Prüfstand gegen das bisherige build_book.py.)
use werkbank_satz::buchbau::{code_html, md};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let band = std::path::Path::new(&a[0]);
    std::fs::create_dir_all(&a[1]).unwrap();
    let rubriken: Vec<(String, String)> = [
        ("Ran an die Tastatur", "werkbank"),
        ("Was passiert hier?", "hinschauen"),
        ("Bughunter", "bughunter"),
        ("Zeig was Du kannst", "aufgabe"),
        ("Klartext", "klartext"),
        ("In die Toolbox", "toolbox"),
    ]
    .iter()
    .map(|(a, b)| (a.to_string(), b.to_string()))
    .collect();
    for (idx, datei) in a[2..].iter().enumerate() {
        let text = std::fs::read_to_string(band.join(datei)).unwrap();
        let abschnitt = md::zerlegen(&text, &format!("s{}_", idx + 1), &rubriken);
        let html = md::html(&abschnitt, &|c, s| code_html(c, s));
        std::fs::write(std::path::Path::new(&a[1]).join(format!("{datei}.html")), html).unwrap();
        std::fs::write(std::path::Path::new(&a[1]).join(format!("{datei}.toc")), format!("{:?}", abschnitt.toc)).unwrap();
    }
}
