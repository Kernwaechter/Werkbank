//! # Lizenzprüfung (Ed25519, offline)
//!
//! Eine Lizenz ist eine kleine JSON-Datei, deren Inhalt mit dem privaten Schlüssel des
//! Verkäufers signiert wurde. Die Werkbank enthält nur den öffentlichen Schlüssel und prüft
//! die Signatur ohne Netzwerk. Wer die Datei ändert, macht die Signatur ungültig.

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use chrono::NaiveDate;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

/// Platzhalter in `assets/lizenz-oeffentlich.key`, solange noch kein Schlüssel eingerichtet ist.
pub const NICHT_EINGERICHTET: &str = "NICHT-EINGERICHTET";

/// Inhalt einer Lizenzdatei.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lizenz {
    /// Name des Lizenznehmers.
    pub name: String,
    /// E-Mail-Adresse des Lizenznehmers.
    pub email: String,
    /// Ausstellungsdatum (`JJJJ-MM-TT`).
    pub ausgestellt: String,
    /// Letzter gültiger Tag (`JJJJ-MM-TT`), leer = unbefristet.
    #[serde(default)]
    pub gueltig_bis: String,
    /// Bezeichnung der Edition (z. B. „Einzelplatz“).
    #[serde(default)]
    pub edition: String,
    /// Signatur über [`Lizenz::nutzlast`], Base64.
    #[serde(default)]
    pub signatur: String,
}

/// Ergebnis der Prüfung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LizenzStatus {
    /// Die Werkbank enthält noch keinen öffentlichen Schlüssel.
    NichtEingerichtet,
    /// Es liegt keine Lizenzdatei vor.
    KeineLizenz,
    /// Die vierwöchige Testzeit läuft noch.
    Test { tage_uebrig: i64 },
    /// Die Testzeit ist vorbei, ein Lizenzschlüssel ist nötig.
    TestAbgelaufen,
    /// Gültig für diesen Namen.
    Gueltig { name: String, edition: String, bis: Option<NaiveDate> },
    /// Die Frist ist abgelaufen.
    Abgelaufen(NaiveDate),
    /// Signatur oder Datei fehlerhaft.
    Ungueltig(String),
}

impl Lizenz {
    /// Die signierten Daten in fester Reihenfolge.
    pub fn nutzlast(&self) -> String {
        format!("werkbank-lizenz-v1\n{}\n{}\n{}\n{}\n{}", self.name, self.email, self.ausgestellt, self.gueltig_bis, self.edition)
    }

    /// Stellt eine Lizenz aus und signiert sie.
    pub fn ausstellen(schluessel: &SigningKey, name: &str, email: &str, ausgestellt: NaiveDate, bis: Option<NaiveDate>, edition: &str) -> Self {
        let mut l = Self {
            name: name.to_string(),
            email: email.to_string(),
            ausgestellt: ausgestellt.format("%Y-%m-%d").to_string(),
            gueltig_bis: bis.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
            edition: edition.to_string(),
            signatur: String::new(),
        };
        l.signatur = B64.encode(schluessel.sign(l.nutzlast().as_bytes()).to_bytes());
        l
    }

    /// Prüft Signatur und Frist gegen den öffentlichen Schlüssel (Base64, 32 Bytes).
    pub fn pruefen(&self, oeffentlich_b64: &str, heute: NaiveDate) -> LizenzStatus {
        let oeffentlich = oeffentlich_b64.trim();
        if oeffentlich.is_empty() || oeffentlich == NICHT_EINGERICHTET {
            return LizenzStatus::NichtEingerichtet;
        }
        let schluessel = match B64
            .decode(oeffentlich)
            .ok()
            .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
            .and_then(|b| VerifyingKey::from_bytes(&b).ok())
        {
            Some(k) => k,
            None => return LizenzStatus::Ungueltig("Der eingebaute öffentliche Schlüssel ist fehlerhaft.".to_string()),
        };
        let signatur = match B64
            .decode(&self.signatur)
            .ok()
            .and_then(|b| <[u8; 64]>::try_from(b.as_slice()).ok())
            .map(|b| Signature::from_bytes(&b))
        {
            Some(s) => s,
            None => return LizenzStatus::Ungueltig("Die Signatur fehlt oder ist fehlerhaft.".to_string()),
        };
        if schluessel.verify(self.nutzlast().as_bytes(), &signatur).is_err() {
            return LizenzStatus::Ungueltig("Die Signatur passt nicht zu den Lizenzdaten.".to_string());
        }
        let bis = if self.gueltig_bis.is_empty() {
            None
        } else {
            match NaiveDate::parse_from_str(&self.gueltig_bis, "%Y-%m-%d") {
                Ok(d) => Some(d),
                Err(_) => return LizenzStatus::Ungueltig("Das Ablaufdatum ist nicht lesbar.".to_string()),
            }
        };
        match bis {
            Some(d) if d < heute => LizenzStatus::Abgelaufen(d),
            _ => LizenzStatus::Gueltig { name: self.name.clone(), edition: self.edition.clone(), bis },
        }
    }
}

/// Liest eine Lizenz aus JSON.
pub fn aus_json(text: &str) -> Result<Lizenz, String> {
    serde_json::from_str(text).map_err(|e| format!("Lizenzdatei nicht lesbar: {e}"))
}

/// Base64 eines öffentlichen Schlüssels.
pub fn oeffentlich_b64(schluessel: &SigningKey) -> String {
    B64.encode(schluessel.verifying_key().to_bytes())
}

/// Privaten Schlüssel aus Base64 (32 Bytes).
pub fn privat_aus_b64(text: &str) -> Result<SigningKey, String> {
    let bytes = B64.decode(text.trim()).map_err(|e| format!("Schlüssel nicht lesbar: {e}"))?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| "Der Schlüssel muss 32 Bytes lang sein.".to_string())?;
    Ok(SigningKey::from_bytes(&bytes))
}

/// Base64 eines privaten Schlüssels.
pub fn privat_b64(schluessel: &SigningKey) -> String {
    B64.encode(schluessel.to_bytes())
}

// ------------------------------------------------------------------ Testzeit

/// Dauer der Testzeit in Tagen (vier Wochen).
pub const TESTTAGE: i64 = 28;

/// Berechnet die Testzeit aus dem ersten Start, dem zuletzt gesehenen Datum und dem heutigen Datum.
/// Wird die Systemuhr zurückgestellt, zählt das zuletzt gesehene Datum. Gibt (Beginn, zuletzt, Status) zurück.
pub fn testzeit(erststart: Option<NaiveDate>, zuletzt: Option<NaiveDate>, heute: NaiveDate) -> (NaiveDate, NaiveDate, LizenzStatus) {
    let jetzt = heute.max(zuletzt.unwrap_or(heute));
    let beginn = erststart.unwrap_or(jetzt).min(jetzt);
    let uebrig = TESTTAGE - (jetzt - beginn).num_days();
    let status = if uebrig > 0 { LizenzStatus::Test { tage_uebrig: uebrig } } else { LizenzStatus::TestAbgelaufen };
    (beginn, jetzt, status)
}

// ------------------------------------------------------------------ Lizenzschlüssel zur E-Mail-Adresse
//
// Ein Lizenzschlüssel ist die Ed25519-Signatur über die (kleingeschriebene) E-Mail-Adresse, in Base32
// geschrieben und in Fünfergruppen geteilt. Die Signatur ist deterministisch: zur selben Adresse gehört
// immer derselbe Schlüssel, er lässt sich also jederzeit neu ausstellen. Die Werkbank prüft ihn offline
// mit dem eingebauten öffentlichen Schlüssel; ohne den privaten Schlüssel lässt er sich nicht fälschen.

/// Kennung am Anfang jedes Schlüssels (Version 1).
pub const SCHLUESSEL_KENNUNG: &str = "WB1";
const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

/// Adresse in der Form, in der sie signiert wird (ohne Leerzeichen am Rand, klein geschrieben).
pub fn email_normalisieren(email: &str) -> String {
    email.trim().to_lowercase()
}

fn schluessel_nutzlast(email: &str) -> String {
    format!("werkbank-schluessel-v1\n{}", email_normalisieren(email))
}

fn base32_kodieren(bytes: &[u8]) -> String {
    let mut aus = String::new();
    let (mut puffer, mut bits) = (0u32, 0u32);
    for &b in bytes {
        puffer = (puffer << 8) | b as u32;
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            aus.push(BASE32[((puffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        aus.push(BASE32[((puffer << (5 - bits)) & 31) as usize] as char);
    }
    aus
}

fn base32_dekodieren(text: &str) -> Option<Vec<u8>> {
    let mut aus = Vec::new();
    let (mut puffer, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let wert = BASE32.iter().position(|&b| b == c)? as u32;
        puffer = (puffer << 5) | wert;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            aus.push(((puffer >> bits) & 255) as u8);
        }
    }
    Some(aus)
}

/// Stellt den Lizenzschlüssel zu einer E-Mail-Adresse aus (nur mit dem privaten Schlüssel möglich).
pub fn schluessel_ausstellen(privat: &SigningKey, email: &str) -> String {
    let signatur = privat.sign(schluessel_nutzlast(email).as_bytes()).to_bytes();
    let kodiert = base32_kodieren(&signatur);
    let gruppen: Vec<&str> = kodiert.as_bytes().chunks(5).map(|g| std::str::from_utf8(g).unwrap_or("")).collect();
    format!("{SCHLUESSEL_KENNUNG}-{}", gruppen.join("-"))
}

/// Prüft, ob Schlüssel und E-Mail-Adresse zusammengehören. Groß-/Kleinschreibung, Striche und
/// Leerzeichen im Schlüssel spielen keine Rolle.
pub fn schluessel_pruefen(oeffentlich_b64: &str, email: &str, schluessel: &str) -> LizenzStatus {
    let oeffentlich = oeffentlich_b64.trim();
    if oeffentlich.is_empty() || oeffentlich == NICHT_EINGERICHTET {
        return LizenzStatus::NichtEingerichtet;
    }
    let falsch = || LizenzStatus::Ungueltig("Schlüssel und E-Mail-Adresse passen nicht zusammen.".to_string());
    let Some(verifier) = B64
        .decode(oeffentlich)
        .ok()
        .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
        .and_then(|b| VerifyingKey::from_bytes(&b).ok())
    else {
        return LizenzStatus::Ungueltig("Der eingebaute öffentliche Schlüssel ist fehlerhaft.".to_string());
    };
    if email_normalisieren(email).is_empty() || !email.contains('@') {
        return LizenzStatus::Ungueltig("Bitte die E-Mail-Adresse eintragen, mit der du gekauft hast.".to_string());
    }
    let sauber: String = schluessel.chars().filter(|c| !c.is_whitespace() && *c != '-').collect::<String>().to_uppercase();
    let Some(rest) = sauber.strip_prefix(SCHLUESSEL_KENNUNG) else { return falsch() };
    let Some(signatur) = base32_dekodieren(rest)
        .and_then(|b| <[u8; 64]>::try_from(b.get(..64).unwrap_or(&[])).ok())
        .filter(|_| rest.len() == 103)
        .map(|b| Signature::from_bytes(&b))
    else {
        return falsch();
    };
    if verifier.verify(schluessel_nutzlast(email).as_bytes(), &signatur).is_err() {
        return falsch();
    }
    LizenzStatus::Gueltig { name: email_normalisieren(email), edition: "Einzelplatz".to_string(), bis: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schluessel() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn d(t: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, t).unwrap()
    }

    #[test]
    fn gueltige_lizenz() {
        let k = schluessel();
        let l = Lizenz::ausstellen(&k, "Erika Muster", "e@example.org", d(1), None, "Einzelplatz");
        assert_eq!(
            l.pruefen(&oeffentlich_b64(&k), d(29)),
            LizenzStatus::Gueltig { name: "Erika Muster".into(), edition: "Einzelplatz".into(), bis: None }
        );
        let json = serde_json::to_string(&l).unwrap();
        assert_eq!(aus_json(&json).unwrap(), l);
    }

    #[test]
    fn manipulation_und_fremder_schluessel_werden_erkannt() {
        let k = schluessel();
        let mut l = Lizenz::ausstellen(&k, "Erika", "e@x.de", d(1), None, "Einzelplatz");
        l.name = "Mallory".into();
        assert!(matches!(l.pruefen(&oeffentlich_b64(&k), d(29)), LizenzStatus::Ungueltig(_)));
        let echt = Lizenz::ausstellen(&k, "Erika", "e@x.de", d(1), None, "Einzelplatz");
        let fremd = SigningKey::from_bytes(&[9u8; 32]);
        assert!(matches!(echt.pruefen(&oeffentlich_b64(&fremd), d(29)), LizenzStatus::Ungueltig(_)));
        let mut leer = echt.clone();
        leer.signatur.clear();
        assert!(matches!(leer.pruefen(&oeffentlich_b64(&k), d(29)), LizenzStatus::Ungueltig(_)));
    }

    #[test]
    fn frist_und_platzhalter() {
        let k = schluessel();
        let l = Lizenz::ausstellen(&k, "E", "e@x.de", d(1), Some(d(15)), "Test");
        assert_eq!(l.pruefen(&oeffentlich_b64(&k), d(15)), LizenzStatus::Gueltig { name: "E".into(), edition: "Test".into(), bis: Some(d(15)) });
        assert_eq!(l.pruefen(&oeffentlich_b64(&k), d(16)), LizenzStatus::Abgelaufen(d(15)));
        assert_eq!(l.pruefen(NICHT_EINGERICHTET, d(16)), LizenzStatus::NichtEingerichtet);
        assert_eq!(privat_aus_b64(&privat_b64(&k)).unwrap().to_bytes(), k.to_bytes());
    }

    #[test]
    fn schluessel_gehoert_zur_adresse() {
        let k = schluessel();
        let o = oeffentlich_b64(&k);
        let schl = schluessel_ausstellen(&k, "Erika.Muster@Example.org ");
        assert!(schl.starts_with("WB1-"));
        // deterministisch: dieselbe Adresse ergibt immer denselben Schlüssel
        assert_eq!(schl, schluessel_ausstellen(&k, "erika.muster@example.org"));
        let gut = LizenzStatus::Gueltig { name: "erika.muster@example.org".into(), edition: "Einzelplatz".into(), bis: None };
        assert_eq!(schluessel_pruefen(&o, "erika.muster@example.org", &schl), gut);
        // Schreibweise von Adresse und Schlüssel ist egal
        assert_eq!(schluessel_pruefen(&o, " ERIKA.muster@example.org", &schl.to_lowercase().replace('-', " ")), gut);
        // andere Adresse, veränderter Schlüssel, fremder Aussteller
        assert!(matches!(schluessel_pruefen(&o, "mallory@example.org", &schl), LizenzStatus::Ungueltig(_)));
        let mut kaputt = schl.clone();
        kaputt.replace_range(5..6, if &schl[5..6] == "A" { "B" } else { "A" });
        assert!(matches!(schluessel_pruefen(&o, "erika.muster@example.org", &kaputt), LizenzStatus::Ungueltig(_)));
        assert!(matches!(schluessel_pruefen(&o, "erika.muster@example.org", "WB1-ABCDE"), LizenzStatus::Ungueltig(_)));
        let fremd = oeffentlich_b64(&SigningKey::from_bytes(&[9u8; 32]));
        assert!(matches!(schluessel_pruefen(&fremd, "erika.muster@example.org", &schl), LizenzStatus::Ungueltig(_)));
        assert_eq!(schluessel_pruefen(NICHT_EINGERICHTET, "e@x.de", &schl), LizenzStatus::NichtEingerichtet);
    }

    #[test]
    fn base32_rundlauf() {
        let daten: Vec<u8> = (0..64u8).map(|i| i.wrapping_mul(37)).collect();
        assert_eq!(base32_dekodieren(&base32_kodieren(&daten)).unwrap()[..64], daten[..]);
        assert_eq!(base32_kodieren(&daten).len(), 103);
    }

    #[test]
    fn vier_wochen_testzeit() {
        let start = d(1);
        let tag = |n: i64| start + chrono::Duration::days(n);
        assert_eq!(testzeit(None, None, start).2, LizenzStatus::Test { tage_uebrig: 28 });
        assert_eq!(testzeit(Some(start), Some(start), tag(27)).2, LizenzStatus::Test { tage_uebrig: 1 });
        assert_eq!(testzeit(Some(start), Some(tag(27)), tag(28)).2, LizenzStatus::TestAbgelaufen);
        // Uhr zurückgestellt: das zuletzt gesehene Datum zählt
        assert_eq!(testzeit(Some(start), Some(tag(40)), tag(3)).2, LizenzStatus::TestAbgelaufen);
        // Datum vor dem Beginn ändert den Beginn nicht ins Negative
        assert_eq!(testzeit(Some(tag(10)), None, start).0, start);
    }
}
