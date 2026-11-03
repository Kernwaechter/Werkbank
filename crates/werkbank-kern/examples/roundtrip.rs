//! Prüft, ob das Einlesen und Zurückschreiben eines Bandes jede Datei unverändert lässt.
use std::path::Path;
use werkbank_kern::Projekt;

fn main() {
    let ordner = std::env::args().nth(1).expect("Bandordner angeben");
    let ordner = Path::new(&ordner);
    let p = Projekt::aus_verzeichnis(ordner).expect("laden");
    let (mut gleich, mut anders) = (0, 0);
    for teil in &p.teile {
        let original = std::fs::read_to_string(ordner.join(&teil.dateipfad)).unwrap();
        let neu = teil.rekonstruiere_datei_inhalt();
        if original == neu {
            gleich += 1;
        } else {
            anders += 1;
            let i = original.bytes().zip(neu.bytes()).position(|(a, b)| a != b).unwrap_or(original.len().min(neu.len()));
            println!("ABWEICHUNG {} (orig {} B, neu {} B, erste Abweichung bei Byte {})", teil.dateipfad.display(), original.len(), neu.len(), i);
            let von = i.saturating_sub(30);
            println!("  orig: {:?}", &original[original.floor_char_boundary(von)..original.floor_char_boundary((i + 40).min(original.len()))]);
            println!("  neu : {:?}", &neu[neu.floor_char_boundary(von)..neu.floor_char_boundary((i + 40).min(neu.len()))]);
        }
    }
    println!("{} Dateien identisch, {} weichen ab", gleich, anders);
}
