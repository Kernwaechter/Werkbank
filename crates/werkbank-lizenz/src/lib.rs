//! Kernlogik für den Lizenzgeber der Werkbank.
//!
//! Verwaltet das Ed25519-Schlüsselpaar, stellt deterministische Lizenzschlüssel
//! zu E-Mail-Adressen aus und führt das Kundenregister inklusive Sicherungen.

pub mod gui;

use chrono::Local;
use ed25519_dalek::SigningKey;
use rand_core::OsRng;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
pub use werkbank_kern::lizenz::{self, Lizenz, LizenzStatus};

pub type Ergebnis<T> = Result<T, String>;

/// Ein Eintrag im Kundenregister (eine JSONL-Zeile).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Eintrag {
    pub zeit: String,
    pub email: String,
    pub name: String,
    pub schluessel: String,
    pub bestellung: String,
    pub betrag: String,
    pub art: String,
}

impl Eintrag {
    pub fn neu(email: String, name: String, schluessel: String, bestellung: String, betrag: String, art: String) -> Self {
        Self {
            zeit: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            email,
            name,
            schluessel,
            bestellung,
            betrag,
            art,
        }
    }

    pub fn zu_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn aus_json(zeile: &str) -> Option<Self> {
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

/// Standard-Datenordner des Lizenzgebers
pub fn datenordner() -> PathBuf {
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

pub fn privat_schluessel_pfad(ordner: &Path) -> PathBuf {
    ordner.join("privat.key")
}

pub fn oeffentlich_schluessel_pfad(ordner: &Path) -> PathBuf {
    ordner.join("oeffentlich.key")
}

pub fn privat_laden(ordner: &Path) -> Ergebnis<SigningKey> {
    let pfad = privat_schluessel_pfad(ordner);
    let text = fs::read_to_string(&pfad)
        .map_err(|_| format!("{} fehlt. Zuerst Schlüsselpaar initialisieren.", pfad.display()))?;
    lizenz::privat_aus_b64(&text)
}

pub fn oeffentlich_laden(ordner: &Path) -> Ergebnis<String> {
    let pfad = oeffentlich_schluessel_pfad(ordner);
    fs::read_to_string(&pfad).map_err(|_| format!("{} fehlt.", pfad.display()))
}

pub fn hat_schluesselpaar(ordner: &Path) -> bool {
    privat_schluessel_pfad(ordner).is_file()
}

pub fn register_pfad(ordner: &Path) -> PathBuf {
    ordner.join("register.jsonl")
}

pub fn register_lesen(ordner: &Path) -> Vec<Eintrag> {
    fs::read_to_string(register_pfad(ordner))
        .map(|t| t.lines().filter_map(Eintrag::aus_json).collect())
        .unwrap_or_default()
}

/// Nächste freie Nummer in `sicherung/register-NNNN.jsonl`.
pub fn naechste_sicherung(ordner: &Path) -> u32 {
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
pub fn eintragen(ordner: &Path, e: &Eintrag) -> Ergebnis<u32> {
    fs::create_dir_all(ordner).map_err(|f| format!("Ordner: {f}"))?;
    let pfad = register_pfad(ordner);
    let mut datei = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&pfad)
        .map_err(|f| format!("Register: {f}"))?;
    writeln!(datei, "{}", e.zu_json()).map_err(|f| format!("Register: {f}"))?;
    datei.sync_all().map_err(|f| format!("Register: {f}"))?;
    let ziel = ordner.join("sicherung");
    fs::create_dir_all(&ziel).map_err(|f| format!("Sicherung: {f}"))?;
    let nr = naechste_sicherung(ordner);
    fs::copy(&pfad, ziel.join(format!("register-{nr:04}.jsonl"))).map_err(|f| format!("Sicherung: {f}"))?;
    Ok(nr)
}

fn csv_feld(s: &str) -> String {
    let s = if s.starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.to_string()
    };
    format!("\"{}\"", s.replace('"', "\"\""))
}

pub fn csv(eintraege: &[Eintrag]) -> String {
    let mut aus = String::from("Zeit;E-Mail;Name;Schlüssel;Bestellung;Betrag;Art\n");
    for e in eintraege {
        let zeile: Vec<String> = [
            &e.zeit,
            &e.email,
            &e.name,
            &e.schluessel,
            &e.bestellung,
            &e.betrag,
            &e.art,
        ]
        .iter()
        .map(|s| csv_feld(s))
        .collect();
        aus.push_str(&zeile.join(";"));
        aus.push('\n');
    }
    aus
}

pub fn email_pruefen(email: &str) -> Ergebnis<String> {
    let e = lizenz::email_normalisieren(email);
    let (vorn, hinten) = e.split_once('@').ok_or_else(|| "Die E-Mail-Adresse benötigt ein @.".to_string())?;
    if vorn.is_empty() || !hinten.contains('.') || e.chars().any(char::is_whitespace) {
        return Err(format!("„{email}“ ist keine gültige E-Mail-Adresse."));
    }
    Ok(e)
}

pub fn pfad_privat_schuetzen(pfad: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(pfad, fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = pfad;
}

pub fn vorlage_einspielen(ordner: &Path) {
    if privat_schluessel_pfad(ordner).exists() {
        return;
    }
    let mut kandidaten: Vec<PathBuf> = std::env::var_os("WERKBANK_LIZENZGEBER_VORLAGE")
        .map(PathBuf::from)
        .into_iter()
        .collect();
    kandidaten.push(PathBuf::from("/app/share/werkbank-lizenzgeber"));
    kandidaten.push(PathBuf::from("/usr/share/werkbank-lizenzgeber"));
    let Some(quelle) = kandidaten.into_iter().find(|k| k.join("privat.key").is_file()) else { return };
    if fs::create_dir_all(ordner).is_err() {
        return;
    }
    let privat = privat_schluessel_pfad(ordner);
    if fs::copy(quelle.join("privat.key"), &privat).is_ok() {
        pfad_privat_schuetzen(&privat);
        let _ = fs::copy(quelle.join("oeffentlich.key"), oeffentlich_schluessel_pfad(ordner));
        eprintln!(
            "Schlüsselpaar aus {} nach {} übernommen.",
            quelle.display(),
            ordner.display()
        );
    }
}

/// Legt ein neues Ed25519-Schlüsselpaar im Datenordner an.
pub fn schluesselpaar_anlegen(ordner: &Path, oeffentlich_nach: Option<&Path>) -> Ergebnis<(String, String)> {
    fs::create_dir_all(ordner).map_err(|f| format!("{}: {f}", ordner.display()))?;
    let privat_pfad = privat_schluessel_pfad(ordner);
    if privat_pfad.exists() {
        return Err(format!("{} existiert bereits. Überschreiben verhindert.", privat_pfad.display()));
    }
    let k = SigningKey::generate(&mut OsRng);
    let privat_b64 = lizenz::privat_b64(&k);
    fs::write(&privat_pfad, &privat_b64).map_err(|f| format!("privat.key: {f}"))?;
    pfad_privat_schuetzen(&privat_pfad);

    let oeffentlich_b64 = lizenz::oeffentlich_b64(&k);
    let oeff_pfad = oeffentlich_schluessel_pfad(ordner);
    fs::write(&oeff_pfad, &oeffentlich_b64).map_err(|f| format!("oeffentlich.key: {f}"))?;

    if let Some(ziel) = oeffentlich_nach {
        let _ = fs::write(ziel, &oeffentlich_b64);
    }

    Ok((privat_b64, oeffentlich_b64))
}

/// Stellt einen Lizenzschlüssel aus. Gibt (Schlüssel, war_schon_vorhanden, sicherung_nr) zurück.
pub fn lizenzschluessel_ausstellen(
    ordner: &Path,
    email: &str,
    name: &str,
    bestellung: &str,
    betrag: &str,
    edition: &str,
) -> Ergebnis<(String, bool, u32)> {
    let email_sauber = email_pruefen(email)?;
    let privat = privat_laden(ordner)?;
    let schluessel = lizenz::schluessel_ausstellen(&privat, &email_sauber);

    // Prüfen, ob bereits registriert
    if let Some(_alt) = register_lesen(ordner).into_iter().find(|e| e.email == email_sauber) {
        return Ok((schluessel, true, 0));
    }

    let e = Eintrag::neu(
        email_sauber,
        name.trim().to_string(),
        schluessel.clone(),
        bestellung.trim().to_string(),
        betrag.trim().to_string(),
        if edition.trim().is_empty() {
            "ausgestellt".to_string()
        } else {
            edition.trim().to_string()
        },
    );

    let nr = eintragen(ordner, &e)?;
    Ok((schluessel, false, nr))
}

/// Prüft einen Lizenzschlüssel mit dem öffentlichen Schlüssel aus dem Datenordner.
pub fn lizenzschluessel_pruefen(ordner: &Path, email: &str, schluessel: &str) -> LizenzStatus {
    let oeffentlich = match oeffentlich_laden(ordner) {
        Ok(k) => k,
        Err(_) => return LizenzStatus::NichtEingerichtet,
    };
    lizenz::schluessel_pruefen(&oeffentlich, email, schluessel)
}

/// Sichert das Register als JSONL und CSV mit aktuellem Zeitstempel.
pub fn register_sichern(ordner: &Path, ziel_ordner: Option<&Path>) -> Ergebnis<(usize, PathBuf, PathBuf)> {
    let ziel = ziel_ordner.map(PathBuf::from).unwrap_or_else(|| ordner.join("export"));
    fs::create_dir_all(&ziel).map_err(|f| format!("{}: {f}", ziel.display()))?;
    let alle = register_lesen(ordner);
    let stempel = Local::now().format("%Y%m%d-%H%M%S");
    let jsonl = ziel.join(format!("register-{stempel}.jsonl"));
    let tabelle = ziel.join(format!("register-{stempel}.csv"));

    let jsonl_inhalt = fs::read_to_string(register_pfad(ordner)).unwrap_or_default();
    fs::write(&jsonl, jsonl_inhalt).map_err(|f| format!("{}: {f}", jsonl.display()))?;
    fs::write(&tabelle, csv(&alle)).map_err(|f| format!("{}: {f}", tabelle.display()))?;

    Ok((alle.len(), jsonl, tabelle))
}

/// Erzeugt die fertige Kunden-E-Mail zur Auslieferung des Lizenzschlüssels.
pub fn kunden_mail_text(name: &str, email: &str, schluessel: &str, edition: &str) -> String {
    let anrede = if name.trim().is_empty() {
        "Hallo,".to_string()
    } else {
        format!("Hallo {},", name.trim())
    };
    let edition_zeile = if edition.trim().is_empty() {
        "Einzelplatz".to_string()
    } else {
        edition.trim().to_string()
    };

    format!(
r#"{anrede}

vielen Dank für deinen Kauf der Werkbank!

Hier ist dein persönlicher Lizenzschlüssel zur Freischaltung:

----------------------------------------------------------------------
Lizenzschlüssel: {schluessel}
E-Mail-Adresse:  {email}
Edition:         {edition_zeile}
----------------------------------------------------------------------

So aktivierst du deine Lizenz in der Werkbank:
1. Starte die Werkbank.
2. Öffne die Einstellungen (Zahnrad-Symbol oder Tastenkürzel Strg+,).
3. Trage unter „Lizenz“ deine E-Mail-Adresse und diesen Schlüssel ein.
4. Klicke auf „Lizenzschlüssel aktivieren“.

Die Werkbank prüft deine Lizenz vollkommen offline – ohne Netzwerk und
ohne dass deine Daten irgendwohin übertragen werden.

Ich wünsche dir viel Freude und maximale Produktivität an der Werkbank!

Herzliche Grüße
Volker Dähn
"#
    )
}

// ------------------------------------------------------------------ Eintrag löschen

/// Entfernt einen Eintrag aus dem Register. Vorher wird der volle alte Stand als
/// `sicherung/register-vor-loeschung-<Zeitstempel>.jsonl` abgelegt, danach der neue Stand als nächste
/// nummerierte Sicherung. Liefert die Zahl der entfernten Einträge.
///
/// Das Löschen entfernt nur den Registereintrag. Ein bereits verschickter Schlüssel bleibt in der
/// Werkbank gültig, weil sie offline prüft und zur selben Adresse immer derselbe Schlüssel gehört.
pub fn register_loeschen(ordner: &Path, eintrag: &Eintrag) -> Ergebnis<usize> {
    let pfad = register_pfad(ordner);
    let alt = fs::read_to_string(&pfad).map_err(|f| format!("Register: {f}"))?;
    let mut entfernt = 0;
    let mut neu = String::new();
    for zeile in alt.lines() {
        if Eintrag::aus_json(zeile).as_ref() == Some(eintrag) {
            entfernt += 1;
            continue;
        }
        neu.push_str(zeile);
        neu.push('\n');
    }
    if entfernt == 0 {
        return Err("Der Eintrag ist im Register nicht mehr vorhanden.".to_string());
    }
    let sicherung = ordner.join("sicherung");
    fs::create_dir_all(&sicherung).map_err(|f| format!("Sicherung: {f}"))?;
    let stempel = Local::now().format("%Y%m%d-%H%M%S");
    fs::write(sicherung.join(format!("register-vor-loeschung-{stempel}.jsonl")), &alt).map_err(|f| format!("Sicherung: {f}"))?;
    let temp = pfad.with_extension("jsonl.neu");
    {
        let mut datei = fs::File::create(&temp).map_err(|f| format!("Register: {f}"))?;
        datei.write_all(neu.as_bytes()).map_err(|f| format!("Register: {f}"))?;
        datei.sync_all().map_err(|f| format!("Register: {f}"))?;
    }
    fs::rename(&temp, &pfad).map_err(|f| format!("Register: {f}"))?;
    let nr = naechste_sicherung(ordner);
    fs::copy(&pfad, sicherung.join(format!("register-{nr:04}.jsonl"))).map_err(|f| format!("Sicherung: {f}"))?;
    Ok(entfernt)
}

// ------------------------------------------------------------------ Vollsicherung in eine Datei

const SICHERUNG_KENNUNG: &[u8; 4] = b"WLS1";
/// Kürzestes erlaubtes Passwort für die Vollsicherung.
pub const PASSWORT_MINDESTLAENGE: usize = 8;

fn alle_dateien(wurzel: &Path, ordner: &Path, aus: &mut Vec<PathBuf>) {
    let Ok(eintraege) = fs::read_dir(ordner) else { return };
    for e in eintraege.flatten() {
        let pfad = e.path();
        if pfad.is_dir() {
            // Exportordner enthält nur abgeleitete Kopien
            if pfad.strip_prefix(wurzel).is_ok_and(|p| p == Path::new("export")) {
                continue;
            }
            alle_dateien(wurzel, &pfad, aus);
        } else if pfad.is_file() {
            aus.push(pfad);
        }
    }
}

fn schluessel_ableiten(passwort: &str, salz: &[u8]) -> Ergebnis<[u8; 32]> {
    let mut schluessel = [0u8; 32];
    argon2::Argon2::default()
        .hash_password_into(passwort.as_bytes(), salz, &mut schluessel)
        .map_err(|f| format!("Schlüsselableitung: {f}"))?;
    Ok(schluessel)
}

fn verschluesseln(klartext: &[u8], passwort: &str) -> Ergebnis<Vec<u8>> {
    use chacha20poly1305::{ChaCha20Poly1305, KeyInit, aead::Aead};
    use rand_core::RngCore;
    let (mut salz, mut nonce) = ([0u8; 16], [0u8; 12]);
    OsRng.fill_bytes(&mut salz);
    OsRng.fill_bytes(&mut nonce);
    let chiffre = ChaCha20Poly1305::new((&schluessel_ableiten(passwort, &salz)?).into());
    let verschluesselt = chiffre.encrypt((&nonce).into(), klartext).map_err(|_| "Verschlüsselung fehlgeschlagen.".to_string())?;
    let mut aus = SICHERUNG_KENNUNG.to_vec();
    aus.extend_from_slice(&salz);
    aus.extend_from_slice(&nonce);
    aus.extend_from_slice(&verschluesselt);
    Ok(aus)
}

fn entschluesseln(daten: &[u8], passwort: &str) -> Ergebnis<Vec<u8>> {
    use chacha20poly1305::{ChaCha20Poly1305, KeyInit, aead::Aead};
    if daten.len() < 4 + 16 + 12 + 16 || &daten[..4] != SICHERUNG_KENNUNG {
        return Err("Das ist keine Sicherungsdatei des Lizenzgebers.".to_string());
    }
    let (salz, rest) = daten[4..].split_at(16);
    let (nonce, chiffretext) = rest.split_at(12);
    let chiffre = ChaCha20Poly1305::new((&schluessel_ableiten(passwort, salz)?).into());
    chiffre.decrypt(nonce.into(), chiffretext).map_err(|_| "Falsches Passwort oder beschädigte Datei.".to_string())
}

/// Sichert den gesamten Datenordner (Schlüsselpaar, Register, Sicherungen, Geheimnisse) in **eine**
/// verschlüsselte Datei auf einem beliebigen Laufwerk. Liefert (Anzahl Dateien, Größe in Bytes).
pub fn vollsicherung_schreiben(ordner: &Path, ziel: &Path, passwort: &str) -> Ergebnis<(usize, u64)> {
    use base64::{Engine, engine::general_purpose::STANDARD as B64};
    if passwort.chars().count() < PASSWORT_MINDESTLAENGE {
        return Err(format!("Das Passwort braucht mindestens {PASSWORT_MINDESTLAENGE} Zeichen."));
    }
    let mut pfade = Vec::new();
    alle_dateien(ordner, ordner, &mut pfade);
    // Die Zieldatei nie in sich selbst einpacken
    let ziel_abs = fs::canonicalize(ziel).ok();
    pfade.retain(|p| ziel_abs.is_none() || fs::canonicalize(p).ok() != ziel_abs);
    pfade.sort();
    let mut dateien = Vec::new();
    for p in &pfade {
        let relativ = p.strip_prefix(ordner).map_err(|f| f.to_string())?.to_string_lossy().replace('\\', "/");
        let inhalt = fs::read(p).map_err(|f| format!("{}: {f}", p.display()))?;
        dateien.push(serde_json::json!({ "pfad": relativ, "inhalt": B64.encode(inhalt) }));
    }
    if !pfade.iter().any(|p| p.file_name().is_some_and(|n| n == "privat.key")) {
        return Err("Im Datenordner liegt kein privater Schlüssel. Es gibt nichts Vollständiges zu sichern.".to_string());
    }
    let klartext = serde_json::to_vec(&serde_json::json!({
        "format": "werkbank-lizenzgeber-sicherung",
        "version": 1,
        "erstellt": Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        "dateien": dateien,
    }))
    .map_err(|f| f.to_string())?;
    let daten = verschluesseln(&klartext, passwort)?;

    // Erst neben das Ziel schreiben, prüfen, dann umbenennen: eine halbe Datei ersetzt nie eine gute
    if let Some(eltern) = ziel.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(eltern).map_err(|f| format!("{}: {f}", eltern.display()))?;
    }
    let temp = ziel.with_extension("wls.tmp");
    {
        let mut datei = fs::File::create(&temp).map_err(|f| format!("{}: {f}", temp.display()))?;
        datei.write_all(&daten).map_err(|f| format!("{}: {f}", temp.display()))?;
        datei.sync_all().map_err(|f| format!("{}: {f}", temp.display()))?;
    }
    pfad_privat_schuetzen(&temp);
    let gelesen = fs::read(&temp).map_err(|f| format!("Kontrolllesen: {f}"))?;
    if entschluesseln(&gelesen, passwort).map_err(|f| format!("Kontrolle der Sicherung: {f}"))? != klartext {
        let _ = fs::remove_file(&temp);
        return Err("Die geschriebene Datei stimmt nicht mit den Daten überein (Datenträger defekt?).".to_string());
    }
    fs::rename(&temp, ziel).map_err(|f| format!("{}: {f}", ziel.display()))?;
    Ok((pfade.len(), daten.len() as u64))
}

/// Spielt eine Vollsicherung in einen Datenordner zurück. Der Ordner darf noch keinen
/// Schlüssel und kein Register enthalten, damit nichts überschrieben wird. Liefert die Zahl der Dateien.
pub fn vollsicherung_wiederherstellen(datei: &Path, passwort: &str, ordner: &Path) -> Ergebnis<usize> {
    use base64::{Engine, engine::general_purpose::STANDARD as B64};
    let daten = fs::read(datei).map_err(|f| format!("{}: {f}", datei.display()))?;
    let klartext = entschluesseln(&daten, passwort)?;
    let v: serde_json::Value = serde_json::from_slice(&klartext).map_err(|f| format!("Sicherung nicht lesbar: {f}"))?;
    if v.get("format").and_then(|x| x.as_str()) != Some("werkbank-lizenzgeber-sicherung") {
        return Err("Das ist keine Sicherung des Lizenzgebers.".to_string());
    }
    if privat_schluessel_pfad(ordner).exists() || register_pfad(ordner).exists() {
        return Err(format!(
            "{} enthält schon Schlüssel oder Register. Verschiebe den Ordner oder wähle einen anderen über WERKBANK_LIZENZGEBER, damit nichts überschrieben wird.",
            ordner.display()
        ));
    }
    let eintraege = v.get("dateien").and_then(|x| x.as_array()).ok_or("Die Sicherung enthält keine Dateien.")?;
    // Erst alles prüfen, dann schreiben
    let mut zu_schreiben = Vec::new();
    for e in eintraege {
        let relativ = e.get("pfad").and_then(|x| x.as_str()).ok_or("Eintrag ohne Pfad.")?;
        let pfad = Path::new(relativ);
        if pfad.is_absolute() || pfad.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
            return Err(format!("Unzulässiger Pfad in der Sicherung: {relativ}"));
        }
        let inhalt = B64.decode(e.get("inhalt").and_then(|x| x.as_str()).unwrap_or("")).map_err(|f| format!("{relativ}: {f}"))?;
        zu_schreiben.push((ordner.join(pfad), inhalt));
    }
    for (pfad, inhalt) in &zu_schreiben {
        if let Some(eltern) = pfad.parent() {
            fs::create_dir_all(eltern).map_err(|f| format!("{}: {f}", eltern.display()))?;
        }
        fs::write(pfad, inhalt).map_err(|f| format!("{}: {f}", pfad.display()))?;
        if pfad.file_name().is_some_and(|n| n == "privat.key") {
            pfad_privat_schuetzen(pfad);
        }
    }
    Ok(zu_schreiben.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schluessel_erzeugen_pruefen_und_sichern() {
        let temp = std::env::temp_dir().join(format!("werkbank-test-lizenz-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        // 1. Schlüsselpaar anlegen
        let (_privat, oeffentlich) = schluesselpaar_anlegen(&temp, None).expect("Schlüsselpaar anlegen");
        assert!(hat_schluesselpaar(&temp));
        assert_eq!(oeffentlich_laden(&temp).unwrap(), oeffentlich);

        // 2. Schlüssel ausstellen
        let (key1, war_da, nr1) = lizenzschluessel_ausstellen(
            &temp,
            "tester@beispiel.de",
            "Max Tester",
            "TEST-100",
            "49,00 €",
            "Einzelplatz",
        )
        .expect("Ausstellen 1");
        assert!(!war_da);
        assert_eq!(nr1, 1);
        assert!(key1.starts_with("WB1-"));

        // 3. Prüfen
        let status = lizenzschluessel_pruefen(&temp, "tester@beispiel.de", &key1);
        match status {
            LizenzStatus::Gueltig { name, edition, .. } => {
                assert_eq!(name, "tester@beispiel.de");
                assert_eq!(edition, "Einzelplatz");
            }
            other => panic!("Erwartet Gueltig, erhalten: {:?}", other),
        }

        // 4. Erneut ausstellen liefert denselben Schlüssel ohne neues Registerduplikat
        let (key2, war_da2, _) = lizenzschluessel_ausstellen(
            &temp,
            "tester@beispiel.de",
            "Max Tester",
            "",
            "",
            "",
        )
        .expect("Ausstellen 2");
        assert_eq!(key1, key2);
        assert!(war_da2);

        // 5. Register prüfen
        let eintraege = register_lesen(&temp);
        assert_eq!(eintraege.len(), 1);
        assert_eq!(eintraege[0].email, "tester@beispiel.de");
        assert_eq!(eintraege[0].name, "Max Tester");

        // 6. Sichern
        let (anzahl, jsonl, csv) = register_sichern(&temp, None).expect("Sichern");
        assert_eq!(anzahl, 1);
        assert!(jsonl.is_file());
        assert!(csv.is_file());

        // 7. Mail-Text enthält Schlüssel und Namen
        let mail = kunden_mail_text("Max", "tester@beispiel.de", &key1, "Einzelplatz");
        assert!(mail.contains("Hallo Max,"));
        assert!(mail.contains(&key1));

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn loeschen_und_vollsicherung() {
        let temp = std::env::temp_dir().join(format!("werkbank-test-sicherung-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        schluesselpaar_anlegen(&temp, None).unwrap();
        for (mail, name) in [("a@beispiel.de", "A"), ("b@beispiel.de", "B")] {
            lizenzschluessel_ausstellen(&temp, mail, name, "T-1", "49", "Einzelplatz").unwrap();
        }
        let alle = register_lesen(&temp);
        assert_eq!(alle.len(), 2);

        // Löschen entfernt genau einen Eintrag und legt Sicherungen an
        assert_eq!(register_loeschen(&temp, &alle[0]).unwrap(), 1);
        let rest = register_lesen(&temp);
        assert_eq!(rest.len(), 1);
        assert_eq!(rest[0].email, "b@beispiel.de");
        assert!(register_loeschen(&temp, &alle[0]).is_err(), "zweites Löschen findet nichts");
        let vorher: Vec<_> = fs::read_dir(temp.join("sicherung")).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        assert!(vorher.iter().any(|n| n.starts_with("register-vor-loeschung-")), "{vorher:?}");

        // Vollsicherung auf ein „anderes Laufwerk“ und zurück
        let ziel = std::env::temp_dir().join(format!("werkbank-test-sicherung-ziel-{}/unter/ordner/alles.wls", std::process::id()));
        assert!(vollsicherung_schreiben(&temp, &ziel, "kurz").is_err());
        let (n, bytes) = vollsicherung_schreiben(&temp, &ziel, "ein langes passwort").unwrap();
        assert!(n >= 4 && bytes > 100, "{n} {bytes}");
        let roh = fs::read(&ziel).unwrap();
        assert!(!String::from_utf8_lossy(&roh).contains("beispiel.de"), "Inhalt muss verschlüsselt sein");
        let neu = std::env::temp_dir().join(format!("werkbank-test-sicherung-neu-{}", std::process::id()));
        let _ = fs::remove_dir_all(&neu);
        assert!(vollsicherung_wiederherstellen(&ziel, "falsches passwort", &neu).is_err());
        assert_eq!(vollsicherung_wiederherstellen(&ziel, "ein langes passwort", &neu).unwrap(), n);
        assert_eq!(fs::read_to_string(neu.join("privat.key")).unwrap(), fs::read_to_string(temp.join("privat.key")).unwrap());
        assert_eq!(register_lesen(&neu), rest);
        // vorhandene Daten werden nie überschrieben
        assert!(vollsicherung_wiederherstellen(&ziel, "ein langes passwort", &neu).is_err());
        for p in [&temp, &neu, &ziel.ancestors().nth(3).unwrap().to_path_buf()] {
            let _ = fs::remove_dir_all(p);
        }
    }
}
