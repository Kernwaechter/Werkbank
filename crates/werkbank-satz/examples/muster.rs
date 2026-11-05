//! Setzt die Musterseite der Gestaltung eines Bandes als PNG: `cargo run --release -p werkbank-satz --example muster -- <Bandordner|vorlage:N> <farbig|grau> <ziel.png>`
use werkbank_satz::gestaltung::Satzstil;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    // „vorlage:0“ nimmt eine mitgelieferte Vorlage, sonst gilt .werkbank/satzstil.json des Bandordners
    let stil = match a[0].strip_prefix("vorlage:") {
        Some(n) => werkbank_satz::gestaltung::vorlagen().remove(n.parse().unwrap()).stil,
        None => Satzstil::laden(std::path::Path::new(&a[0])),
    };
    let (seiten, _, w) = werkbank_satz::typst_zu_seiten(stil.muster(a[1] == "farbig"), None, 1, 1.3).unwrap();
    println!("{w:?}");
    let s = &seiten[0];
    // Rohes RGBA als PPM schreiben (ohne Zusatzbibliothek)
    let mut ppm = format!("P6\n{} {}\n255\n", s.breite, s.hoehe).into_bytes();
    for p in s.rgba.chunks(4) {
        ppm.extend_from_slice(&p[..3]);
    }
    std::fs::write(&a[2], ppm).unwrap();
}