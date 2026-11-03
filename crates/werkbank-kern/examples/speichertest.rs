//! Prüft auf einer Kopie eines Bandes: Unverändertes wird nie geschrieben, eine Änderung berührt genau eine Datei.
use std::path::Path;
use werkbank_kern::Projekt;

fn main() {
    let ordner = std::env::args().nth(1).expect("Kopie eines Bandordners angeben");
    let mut p = Projekt::aus_verzeichnis(Path::new(&ordner)).unwrap();
    let alle: Vec<String> = p.teile.iter().map(|t| t.id.clone()).collect();
    let g = p.teile_speichern_nummeriert(&alle).unwrap();
    println!("unverändert gespeichert: {} Dateien geschrieben (soll 0 sein)", g.len());

    let kap = p.teile[1].kapitel[0].id.clone();
    p.kapitel_finden_mut(&kap).unwrap().inhalt.push_str("\nEin neuer Absatz.\n");
    let g = p.teile_speichern_nummeriert(&alle).unwrap();
    println!("nach Änderung: geschrieben {:?}", g);
    p.kapitel_finden_mut(&kap).unwrap().inhalt.push_str("Noch ein Absatz.\n");
    println!("zweites Speichern: {:?}", p.teile_speichern_nummeriert(&alle).unwrap());
    println!("Snapshots für {}: {}", p.teile[1].id, p.snapshots_fuer_teil(&p.teile[1].id).len());
    let neu = Projekt::aus_verzeichnis(Path::new(&ordner)).unwrap();
    println!("neu geladen enthält Änderung: {}", neu.kapitel_finden(&kap).unwrap().inhalt.ends_with("Ein neuer Absatz.\n"));
}
