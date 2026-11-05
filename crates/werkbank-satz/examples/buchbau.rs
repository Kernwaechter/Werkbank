//! Baut einen Band nach `buchbau.yaml`: `cargo run --release -p werkbank-satz --example buchbau -- <Buchordner> <Bandnummer>`
fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let t = std::time::Instant::now();
    match werkbank_satz::buchbau::bauen(std::path::Path::new(&a[0]), a[1].parse().unwrap(), &|s| println!("{s}")) {
        Ok(b) => {
            for h in &b.hinweise {
                println!("Hinweis: {h}");
            }
            for (p, g) in &b.dateien {
                println!("  {:<70} {:>9.1} KB", p.display(), *g as f64 / 1024.0);
            }
            println!("fertig in {:.1} s", t.elapsed().as_secs_f32());
        }
        Err(e) => {
            eprintln!("Fehler: {e}");
            std::process::exit(1);
        }
    }
}
