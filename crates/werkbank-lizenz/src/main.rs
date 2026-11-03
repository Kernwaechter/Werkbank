//! Lizenzgeber der Werkbank: Schlüsselpaar anlegen, Lizenzschlüssel zu E-Mail-Adressen ausstellen,
//! das Register der Kunden führen und sichern.
//!
//! Der **private Schlüssel gehört nicht ins Repository und nicht auf Kundenrechner.** Er liegt im
//! Datenordner des Lizenzgebers (`~/.local/share/werkbank-lizenzgeber/`, unter Windows `%APPDATA%`),
//! zusammen mit dem Kundenregister `register.jsonl` und dessen nummerierten Sicherungen.
//! Der öffentliche Schlüssel kommt nach `assets/lizenz-oeffentlich.key` und wird in die Werkbank eingebaut.

mod webhook;

use chrono::{Local, NaiveDate};
use rand_core::OsRng;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use werkbank_kern::lizenz::{self, Lizenz, LizenzStatus};

const HILFE: &str = "Lizenzgeber der Werkbank

  werkbank-lizenz gui
      startet die grafische Iced-Benutzeroberfläche (auch ohne Argumente im Desktop)
  werkbank-lizenz init [--oeffentlich-nach <datei>]
      legt Schlüsselpaar und Register im Datenordner an (überschreibt nie)
  werkbank-lizenz ausstellen --email <adresse> [--name <Name>] [--bestellung <Nr>] [--betrag <EUR>]
      stellt den Schlüssel zur Adresse aus und trägt ihn ins Register ein
  werkbank-lizenz wiederherstellen --email <adresse>
      berechnet den Schlüssel erneut (geht auch, wenn das Register verloren ging)
  werkbank-lizenz suchen <Teil der Adresse oder des Namens>
  werkbank-lizenz liste
  werkbank-lizenz sichern [--ziel <ordner>]
      kopiert Register (JSONL und CSV) mit Zeitstempel, zum Beispiel auf einen USB-Stick
  werkbank-lizenz webhook [--port 8787] [--adresse 127.0.0.1] [--mail-befehl \"sendmail -t\"] [--von <Adresse>] [--link-id plink_…] [--download-url <Adresse>]
      nimmt Bestellungen von Stripe an, stellt Schlüssel aus und verschickt sie per E-Mail
  werkbank-lizenz loeschen --email <adresse>
      entfernt den Registereintrag (der alte Stand bleibt als Sicherung); der Schlüssel bleibt offline gültig
  werkbank-lizenz vollsicherung --datei <Pfad.wls>      (Passwort in WERKBANK_SICHERUNG_PASSWORT)
      sichert alles in eine verschlüsselte Datei auf beliebigem Laufwerk
  werkbank-lizenz rueckspielen --datei <Pfad.wls>       (nur in einen leeren Datenordner)
  werkbank-lizenz pruefen --email <adresse> --schluessel <Schlüssel> [--oeffentlich <datei>]
  werkbank-lizenz datei --name <Name> --email <adresse> [--bis JJJJ-MM-TT] [--edition <Text>] [--ausgabe lizenz.json]
      stellt eine Lizenzdatei (JSON) aus, für Sonderfälle mit Ablaufdatum

Datenordner: $WERKBANK_LIZENZGEBER oder ~/.local/share/werkbank-lizenzgeber
";

type Ergebnis<T> = Result<T, String>;

fn option(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn datenordner() -> PathBuf {
    if let Some(p) = std::env::var_os("WERKBANK_LIZENZGEBER") {
        return PathBuf::from(p);
    }
    let basis = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    basis.join("werkbank-lizenzgeber")
}

fn privat_laden(ordner: &Path) -> Ergebnis<ed25519_dalek::SigningKey> {
    let pfad = ordner.join("privat.key");
    let text = fs::read_to_string(&pfad).map_err(|_| format!("{} fehlt. Zuerst `init` ausführen.", pfad.display()))?;
    lizenz::privat_aus_b64(&text)
}

/// Ein Eintrag im Register (eine JSON-Zeile).
#[derive(Debug, Clone)]
struct Eintrag {
    zeit: String,
    email: String,
    name: String,
    schluessel: String,
    bestellung: String,
    betrag: String,
    art: String,
}

impl Eintrag {
    fn zu_json(&self) -> String {
        serde_json::json!({
            "zeit": self.zeit, "email": self.email, "name": self.name, "schluessel": self.schluessel,
            "bestellung": self.bestellung, "betrag": self.betrag, "art": self.art,
        })
        .to_string()
    }

    fn aus_json(zeile: &str) -> Option<Self> {
        let v: serde_json::Value = serde_json::from_str(zeile).ok()?;
        let f = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
        Some(Self {
            zeit: f("zeit"),
            email: f("email"),
            name: f("name"),
            schluessel: f("schluessel"),
            bestellung: f("bestellung"),
            betrag: f("betrag"),
            art: f("art"),
        })
    }
}

fn register_lesen(ordner: &Path) -> Vec<Eintrag> {
    fs::read_to_string(ordner.join("register.jsonl"))
        .map(|t| t.lines().filter_map(Eintrag::aus_json).collect())
        .unwrap_or_default()
}

/// Nächste freie Nummer in `sicherung/register-NNNN.jsonl`.
fn naechste_sicherung(ordner: &Path) -> u32 {
    let hoechste = fs::read_dir(ordner.join("sicherung"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().into_owned();
            n.strip_prefix("register-")?.strip_suffix(".jsonl")?.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0);
    hoechste + 1
}

/// Hängt einen Eintrag an das Register an und legt danach eine nummerierte Sicherungskopie ab.
fn eintragen(ordner: &Path, e: &Eintrag) -> Ergebnis<u32> {
    let pfad = ordner.join("register.jsonl");
    let mut datei = fs::OpenOptions::new().create(true).append(true).open(&pfad).map_err(|f| format!("Register: {f}"))?;
    writeln!(datei, "{}", e.zu_json()).map_err(|f| format!("Register: {f}"))?;
    datei.sync_all().map_err(|f| format!("Register: {f}"))?;
    let ziel = ordner.join("sicherung");
    fs::create_dir_all(&ziel).map_err(|f| format!("Sicherung: {f}"))?;
    let nr = naechste_sicherung(ordner);
    fs::copy(&pfad, ziel.join(format!("register-{nr:04}.jsonl"))).map_err(|f| format!("Sicherung: {f}"))?;
    Ok(nr)
}

fn csv_feld(s: &str) -> String {
    // Tabellenprogramme führen Zellen, die mit = + - @ beginnen, als Formel aus
    let s = if s.starts_with(['=', '+', '-', '@']) { format!("'{s}") } else { s.to_string() };
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn csv(eintraege: &[Eintrag]) -> String {
    let mut aus = String::from("Zeit;E-Mail;Name;Schlüssel;Bestellung;Betrag;Art\n");
    for e in eintraege {
        let zeile: Vec<String> = [&e.zeit, &e.email, &e.name, &e.schluessel, &e.bestellung, &e.betrag, &e.art].iter().map(|s| csv_feld(s)).collect();
        aus.push_str(&zeile.join(";"));
        aus.push('\n');
    }
    aus
}

fn email_pruefen(email: &str) -> Ergebnis<String> {
    let e = lizenz::email_normalisieren(email);
    let (vorn, hinten) = e.split_once('@').ok_or("Die E-Mail-Adresse braucht ein @.")?;
    if vorn.is_empty() || !hinten.contains('.') || e.chars().any(char::is_whitespace) {
        return Err(format!("„{email}“ sieht nicht wie eine E-Mail-Adresse aus."));
    }
    Ok(e)
}

fn pfad_privat_schuetzen(pfad: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(pfad, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = pfad;
}

/// Paketinstallationen (deb, rpm, Flatpak) bringen das Schlüsselpaar des Lizenzgebers mit, damit auf jedem
/// Rechner dieselben Schlüssel entstehen. Beim ersten Aufruf wird es in den Datenordner übernommen
/// (nur wenn dort noch kein `privat.key` liegt).
fn vorlage_einspielen(ordner: &Path) {
    if ordner.join("privat.key").exists() {
        return;
    }
    let mut kandidaten: Vec<PathBuf> = std::env::var_os("WERKBANK_LIZENZGEBER_VORLAGE").map(PathBuf::from).into_iter().collect();
    kandidaten.push(PathBuf::from("/app/share/werkbank-lizenzgeber"));
    kandidaten.push(PathBuf::from("/usr/share/werkbank-lizenzgeber"));
    let Some(quelle) = kandidaten.into_iter().find(|k| k.join("privat.key").is_file()) else { return };
    if fs::create_dir_all(ordner).is_err() {
        return;
    }
    let privat = ordner.join("privat.key");
    if fs::copy(quelle.join("privat.key"), &privat).is_ok() {
        pfad_privat_schuetzen(&privat);
        let _ = fs::copy(quelle.join("oeffentlich.key"), ordner.join("oeffentlich.key"));
        eprintln!("Schlüsselpaar aus {} nach {} übernommen.", quelle.display(), ordner.display());
    }
}

fn lauf(args: &[String]) -> Ergebnis<()> {
    let ordner = datenordner();
    vorlage_einspielen(&ordner);
    match args.first().map(String::as_str) {
        Some("init") => {
            fs::create_dir_all(&ordner).map_err(|f| format!("{}: {f}", ordner.display()))?;
            let privat = ordner.join("privat.key");
            if privat.exists() {
                return Err(format!("{} existiert schon, nichts überschrieben.", privat.display()));
            }
            let k = ed25519_dalek::SigningKey::generate(&mut OsRng);
            fs::write(&privat, lizenz::privat_b64(&k)).map_err(|f| format!("privat.key: {f}"))?;
            pfad_privat_schuetzen(&privat);
            let oeffentlich = lizenz::oeffentlich_b64(&k);
            fs::write(ordner.join("oeffentlich.key"), &oeffentlich).map_err(|f| format!("oeffentlich.key: {f}"))?;
            if let Some(ziel) = option(args, "--oeffentlich-nach") {
                fs::write(&ziel, &oeffentlich).map_err(|f| format!("{ziel}: {f}"))?;
                println!("Öffentlicher Schlüssel nach {ziel} geschrieben. Werkbank neu bauen, dann gilt er.");
            }
            println!("Angelegt in {}", ordner.display());
            println!("WICHTIG: Sichere privat.key getrennt vom Rechner (Passwortmanager, USB-Stick im Safe). Geht er verloren,");
            println!("lassen sich keine Schlüssel mehr ausstellen und alte nicht neu berechnen. Wer ihn bekommt, kann Lizenzen fälschen.");
            Ok(())
        }
        Some("ausstellen") => {
            let email = email_pruefen(&option(args, "--email").ok_or("--email fehlt.")?)?;
            let privat = privat_laden(&ordner)?;
            if let Some(alt) = register_lesen(&ordner).into_iter().find(|e| e.email == email) {
                println!("Für {email} wurde am {} schon ein Schlüssel ausgestellt (nichts neu eingetragen):\n\n{}", alt.zeit, alt.schluessel);
                return Ok(());
            }
            let schluessel = lizenz::schluessel_ausstellen(&privat, &email);
            let e = Eintrag {
                zeit: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                email: email.clone(),
                name: option(args, "--name").unwrap_or_default(),
                schluessel: schluessel.clone(),
                bestellung: option(args, "--bestellung").unwrap_or_default(),
                betrag: option(args, "--betrag").unwrap_or_default(),
                art: "ausgestellt".to_string(),
            };
            let nr = eintragen(&ordner, &e)?;
            println!("Lizenzschlüssel für {email}:\n\n{schluessel}\n\nIm Register vermerkt, Sicherung Nr. {nr} abgelegt.");
            Ok(())
        }
        Some("wiederherstellen") => {
            let email = email_pruefen(&option(args, "--email").ok_or("--email fehlt.")?)?;
            let privat = privat_laden(&ordner)?;
            let schluessel = lizenz::schluessel_ausstellen(&privat, &email);
            if let Some(alt) = register_lesen(&ordner).into_iter().find(|e| e.email == email) {
                if alt.schluessel != schluessel {
                    return Err("Der Schlüssel im Register weicht vom berechneten ab. Wurde der private Schlüssel gewechselt?".to_string());
                }
                println!("Schlüssel für {email} (im Register seit {}):\n\n{schluessel}", alt.zeit);
            } else {
                let e = Eintrag {
                    zeit: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    email: email.clone(),
                    name: String::new(),
                    schluessel: schluessel.clone(),
                    bestellung: String::new(),
                    betrag: String::new(),
                    art: "wiederhergestellt".to_string(),
                };
                eintragen(&ordner, &e)?;
                println!("Schlüssel für {email} (war nicht im Register, jetzt nachgetragen):\n\n{schluessel}");
            }
            Ok(())
        }
        Some("suchen") => {
            let wort = args.get(1).ok_or("Suchbegriff fehlt.")?.to_lowercase();
            let treffer: Vec<_> = register_lesen(&ordner).into_iter().filter(|e| e.email.contains(&wort) || e.name.to_lowercase().contains(&wort)).collect();
            if treffer.is_empty() {
                println!("Keine Treffer.");
            }
            for e in treffer {
                println!("{}  {}  {}\n    {}", e.zeit, e.email, e.name, e.schluessel);
            }
            Ok(())
        }
        Some("liste") => {
            let alle = register_lesen(&ordner);
            for e in &alle {
                println!("{}  {:<36} {:<22} {}", e.zeit, e.email, e.name, e.bestellung);
            }
            println!("{} Einträge.", alle.len());
            Ok(())
        }
        Some("sichern") => {
            let ziel = PathBuf::from(option(args, "--ziel").unwrap_or_else(|| ordner.join("export").to_string_lossy().into_owned()));
            fs::create_dir_all(&ziel).map_err(|f| format!("{}: {f}", ziel.display()))?;
            let alle = register_lesen(&ordner);
            let stempel = Local::now().format("%Y%m%d-%H%M%S");
            let jsonl = ziel.join(format!("register-{stempel}.jsonl"));
            let tabelle = ziel.join(format!("register-{stempel}.csv"));
            fs::write(&jsonl, fs::read_to_string(ordner.join("register.jsonl")).unwrap_or_default()).map_err(|f| format!("{}: {f}", jsonl.display()))?;
            fs::write(&tabelle, csv(&alle)).map_err(|f| format!("{}: {f}", tabelle.display()))?;
            println!("{} Einträge gesichert:\n  {}\n  {}", alle.len(), jsonl.display(), tabelle.display());
            println!("Die Dateien enthalten Kundenadressen. Verschlüsselt oder an einem sicheren Ort aufbewahren.");
            println!("Den privaten Schlüssel ({}) gesondert sichern: Mit ihm lässt sich jeder Schlüssel neu berechnen.", ordner.join("privat.key").display());
            Ok(())
        }
        Some("pruefen") => {
            let email = option(args, "--email").ok_or("--email fehlt.")?;
            let schluessel = option(args, "--schluessel").ok_or("--schluessel fehlt.")?;
            let oeffentlich = match option(args, "--oeffentlich") {
                Some(p) => fs::read_to_string(&p).map_err(|f| format!("{p}: {f}"))?,
                None => fs::read_to_string(ordner.join("oeffentlich.key")).map_err(|_| "oeffentlich.key fehlt, `--oeffentlich <datei>` angeben.".to_string())?,
            };
            match lizenz::schluessel_pruefen(&oeffentlich, &email, &schluessel) {
                LizenzStatus::Gueltig { name, .. } => println!("GÜLTIG für {name}"),
                andere => return Err(format!("{andere:?}")),
            }
            Ok(())
        }
        Some("webhook") => webhook::lauf(&ordner, args),
        Some("loeschen") => {
            let email = email_pruefen(&option(args, "--email").ok_or("--email fehlt.")?)?;
            let treffer: Vec<werkbank_lizenz::Eintrag> =
                register_lesen(&ordner).into_iter().filter(|e| e.email == email).filter_map(|e| werkbank_lizenz::Eintrag::aus_json(&e.zu_json())).collect();
            if treffer.is_empty() {
                return Err(format!("Kein Eintrag für {email} im Register."));
            }
            let mut n = 0;
            for e in &treffer {
                n += werkbank_lizenz::register_loeschen(&ordner, e)?;
            }
            println!("{n} Eintrag/Einträge für {email} gelöscht, der alte Stand liegt in sicherung/. Der verschickte Schlüssel bleibt offline gültig.");
            Ok(())
        }
        Some("vollsicherung") => {
            let ziel = option(args, "--datei").ok_or("--datei <Pfad> fehlt.")?;
            let passwort = std::env::var("WERKBANK_SICHERUNG_PASSWORT").map_err(|_| "Passwort in der Umgebungsvariable WERKBANK_SICHERUNG_PASSWORT angeben.".to_string())?;
            let (n, bytes) = werkbank_lizenz::vollsicherung_schreiben(&ordner, Path::new(&ziel), &passwort)?;
            println!("{n} Dateien verschlüsselt gesichert nach {ziel} ({:.1} KB).", bytes as f64 / 1024.0);
            Ok(())
        }
        Some("rueckspielen") => {
            let quelle = option(args, "--datei").ok_or("--datei <Pfad> fehlt.")?;
            let passwort = std::env::var("WERKBANK_SICHERUNG_PASSWORT").map_err(|_| "Passwort in der Umgebungsvariable WERKBANK_SICHERUNG_PASSWORT angeben.".to_string())?;
            let n = werkbank_lizenz::vollsicherung_wiederherstellen(Path::new(&quelle), &passwort, &ordner)?;
            println!("{n} Dateien nach {} wiederhergestellt.", ordner.display());
            Ok(())
        }
        Some("datei") => {
            let privat = privat_laden(&ordner)?;
            let name = option(args, "--name").ok_or("--name fehlt.")?;
            let email = email_pruefen(&option(args, "--email").ok_or("--email fehlt.")?)?;
            let bis = option(args, "--bis")
                .map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").map_err(|_| "--bis als JJJJ-MM-TT angeben.".to_string()))
                .transpose()?;
            let edition = option(args, "--edition").unwrap_or_else(|| "Einzelplatz".to_string());
            let l = Lizenz::ausstellen(&privat, &name, &email, Local::now().date_naive(), bis, &edition);
            let ziel = option(args, "--ausgabe").unwrap_or_else(|| "lizenz.json".to_string());
            fs::write(&ziel, serde_json::to_string_pretty(&l).map_err(|f| f.to_string())?).map_err(|f| format!("{ziel}: {f}"))?;
            println!("Lizenzdatei für {name} nach {ziel} geschrieben.");
            Ok(())
        }
        Some("gui") => werkbank_lizenz::gui::start_gui().map_err(|e| e.to_string()),
        None if std::env::var_os("DISPLAY").is_some() || std::env::var_os("WAYLAND_DISPLAY").is_some() => {
            werkbank_lizenz::gui::start_gui().map_err(|e| e.to_string())
        }
        _ => {
            print!("{HILFE}");
            std::process::exit(2)
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(f) = lauf(&args) {
        eprintln!("Fehler: {f}");
        std::process::exit(1);
    }
}
