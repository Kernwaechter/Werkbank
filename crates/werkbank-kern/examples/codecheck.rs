//! Prüft alle Rust- und Dart-Beispiele eines Bandes (Werkzeug für die Kommandozeile).
use std::path::Path;
use std::sync::Mutex;
use werkbank_kern::codepruefung::{self, Ergebnis};
use werkbank_kern::Projekt;

fn main() {
    let ordner = std::env::args().nth(1).expect("Bandordner angeben");
    let p = Projekt::aus_verzeichnis(Path::new(&ordner)).expect("laden");
    let b = codepruefung::beispiele(&p);
    println!("{} Beispiele", b.len());
    let start = std::time::Instant::now();
    let erg = Mutex::new(vec![None; b.len()]);
    codepruefung::alle_pruefen(&b, "2024", &std::env::temp_dir().join("werkbank-code-cli"), 4, |i, e| erg.lock().unwrap()[i] = Some(e));
    let erg = erg.into_inner().unwrap();
    let zeigen: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(5);
    let (mut ok, mut fehler, mut ausschnitt, mut skip) = (0, 0, 0, 0);
    for (bsp, e) in b.iter().zip(&erg) {
        match e {
            Some(Ergebnis::Ok) => ok += 1,
            Some(Ergebnis::Uebersprungen(m)) => {
                skip += 1;
                println!("SKIP [{}] {} Zeile {}: {}", bsp.sprache, bsp.kapitel_titel, bsp.zeile, m);
            }
            Some(Ergebnis::Ausschnitt(_)) => ausschnitt += 1,
            Some(Ergebnis::Fehler(m)) => {
                fehler += 1;
                if fehler <= zeigen {
                    println!("FEHLER [{}] {} Zeile {}:\n  {}", bsp.sprache, bsp.kapitel_titel, bsp.zeile, m.replace('\n', "\n  "));
                }
            }
            None => {}
        }
    }
    println!("ok {ok}, Fehler {fehler}, Ausschnitte {ausschnitt}, übersprungen {skip} in {:?}", start.elapsed());
}
