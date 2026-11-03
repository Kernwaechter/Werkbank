//! Webhook für Stripe (auch Managed Payments): nimmt bezahlte Bestellungen an, stellt den
//! Lizenzschlüssel zur E-Mail-Adresse aus, trägt ihn ins Register ein und schickt ihn dem Kunden.
//!
//! Stripe verlangt eine HTTPS-Adresse. Der Server hier spricht nur HTTP und gehört hinter einen
//! Reverse-Proxy (Caddy, nginx), der das TLS übernimmt. Jede Anfrage trägt in `Stripe-Signature`
//! (`t=<Zeit>,v1=<HMAC>`) den HMAC-SHA256 über `<Zeit>.<Rohtext>` mit dem Signing-Secret des
//! Webhook-Endpunkts (`whsec_…`); Anfragen ohne passende, frische Signatur werden abgewiesen.

use super::{Eintrag, Ergebnis, eintragen, email_pruefen, option, privat_laden, register_lesen};
use chrono::Local;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use werkbank_kern::lizenz;

/// Eine bezahlte Bestellung aus dem Webhook.
#[derive(Debug, PartialEq, Eq)]
pub struct Bestellung {
    pub ereignis: String,
    pub bestellung: String,
    pub email: String,
    pub name: String,
    pub betrag: String,
    pub bezahlt: bool,
    pub test: bool,
}

/// Prüft `Stripe-Signature` gegen den HMAC-SHA256 über `<Zeit>.<Rohtext>`. Die Zeit darf höchstens
/// fünf Minuten von `jetzt` (Unix-Sekunden) abweichen, damit abgefangene Anfragen nicht wiederholt werden können.
pub fn signatur_gueltig(geheimnis: &str, body: &[u8], kopf: &str, jetzt: i64) -> bool {
    let mut zeit = None;
    let mut unterschriften = Vec::new();
    for teil in kopf.split(',') {
        match teil.trim().split_once('=') {
            Some(("t", t)) => zeit = t.parse::<i64>().ok(),
            Some(("v1", v)) => unterschriften.extend(hex_lesen(v)),
            _ => {}
        }
    }
    let Some(zeit) = zeit else { return false };
    if (jetzt - zeit).abs() > 300 {
        return false;
    }
    unterschriften.iter().any(|erwartet| {
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(geheimnis.as_bytes()) else { return false };
        mac.update(zeit.to_string().as_bytes());
        mac.update(b".");
        mac.update(body);
        mac.verify_slice(erwartet).is_ok()
    })
}

fn hex_lesen(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 || !s.is_ascii() {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok()).collect()
}

/// Liest Ereignis und Bestelldaten aus dem Stripe-JSON.
/// `link_id`: nur Käufe über diesen Zahlungslink (`plink_…`) zulassen, leer = alle.
pub fn bestellung_lesen(body: &str, link_id: &str) -> Option<Bestellung> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let ereignis = v.get("type")?.as_str()?.to_string();
    let o = v.pointer("/data/object")?;
    let text = |p: &str| o.pointer(p).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let zahl = |p: &str| o.pointer(p).and_then(|x| x.as_i64()).unwrap_or(0);
    let test = !v.get("livemode").and_then(|x| x.as_bool()).unwrap_or(true);
    let (email, name, cent, bezahlt) = if ereignis.starts_with("charge.") {
        (text("/billing_details/email"), text("/billing_details/name"), zahl("/amount_refunded"), false)
    } else {
        let email = Some(text("/customer_details/email")).filter(|e| !e.is_empty()).unwrap_or_else(|| text("/customer_email"));
        (email, text("/customer_details/name"), zahl("/amount_total"), text("/payment_status") == "paid")
    };
    if !link_id.is_empty() && !ereignis.starts_with("charge.") && text("/payment_link") != link_id {
        return Some(Bestellung { ereignis: "fremder_zahlungslink".into(), bestellung: String::new(), email, name, betrag: String::new(), bezahlt: false, test });
    }
    Some(Bestellung {
        ereignis,
        bestellung: text("/id"),
        email,
        name,
        betrag: format!("{}.{:02} {}", cent / 100, cent % 100, text("/currency").to_uppercase()),
        bezahlt,
        test,
    })
}

fn kodiert_kopf(text: &str) -> String {
    use base64::{Engine, engine::general_purpose::STANDARD};
    if text.is_ascii() { text.to_string() } else { format!("=?UTF-8?B?{}?=", STANDARD.encode(text)) }
}

/// Baut die Mail mit dem Schlüssel (RFC 5322, UTF-8).
pub fn mail_text(von: &str, an: &str, name: &str, schluessel: &str, download: &str) -> String {
    let download = if download.is_empty() { String::new() } else { format!("Die Werkbank lädst du hier herunter (Linux): {download}\r\n\r\n") };
    let anrede = if name.trim().is_empty() { "Guten Tag".to_string() } else { format!("Guten Tag {}", name.replace(['\r', '\n'], " ")) };
    format!(
        "From: {von}\r\nTo: {an}\r\nSubject: {}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Transfer-Encoding: 8bit\r\n\r\n\
{anrede},\r\n\r\n\
vielen Dank für deinen Kauf der Werkbank. Dein Lizenzschlüssel gehört zu dieser E-Mail-Adresse ({an}):\r\n\r\n{schluessel}\r\n\r\n\
{download}So aktivierst du ihn:\r\n1. Starte die Werkbank.\r\n2. Öffne die Einstellungen (Zahnrad-Symbol oder Strg+,).\r\n3. Trage unter „Lizenz“ deine E-Mail-Adresse und diesen Schlüssel ein.\r\n4. Klicke auf „Lizenzschlüssel aktivieren“.\r\n\
Bitte trage genau die Adresse ein, mit der du gekauft hast. Die Prüfung läuft offline.\r\n\r\n\
Hast du den Schlüssel einmal verlegt, antworte einfach auf diese Mail.\r\n",
        kodiert_kopf("Dein Lizenzschlüssel für die Werkbank")
    )
}

fn mail_senden(befehl: &str, text: &str) -> Ergebnis<()> {
    let mut teile = befehl.split_whitespace();
    let programm = teile.next().ok_or("Kein Mailbefehl angegeben.")?;
    let mut kind = Command::new(programm).args(teile).stdin(Stdio::piped()).spawn().map_err(|f| format!("Mailbefehl „{befehl}“: {f}"))?;
    kind.stdin.take().ok_or("Mailbefehl ohne Eingang")?.write_all(text.as_bytes()).map_err(|f| format!("Mailbefehl: {f}"))?;
    let status = kind.wait().map_err(|f| format!("Mailbefehl: {f}"))?;
    if status.success() { Ok(()) } else { Err(format!("Mailbefehl endete mit {status}")) }
}

/// Verarbeitet eine geprüfte Bestellung. Gibt die Antwort für Stripe zurück.
pub fn bearbeiten(ordner: &Path, b: &Bestellung, mailbefehl: &str, von: &str, download: &str) -> Ergebnis<String> {
    let register = register_lesen(ordner);
    match b.ereignis.as_str() {
        "checkout.session.completed" | "checkout.session.async_payment_succeeded" if b.bezahlt => {
            if !b.bestellung.is_empty() && register.iter().any(|e| e.bestellung == b.bestellung && e.art != "erstattet") {
                return Ok(format!("Bestellung {} schon bearbeitet", b.bestellung));
            }
            let email = email_pruefen(&b.email)?;
            let privat = privat_laden(ordner)?;
            let schluessel = lizenz::schluessel_ausstellen(&privat, &email);
            mail_senden(mailbefehl, &mail_text(von, &email, &b.name, &schluessel, download))?;
            let weitere = register.iter().any(|e| e.email == email);
            eintragen(
                ordner,
                &Eintrag {
                    zeit: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    email: email.clone(),
                    name: b.name.clone(),
                    schluessel,
                    bestellung: b.bestellung.clone(),
                    betrag: b.betrag.clone(),
                    art: if b.test { "test".into() } else if weitere { "weitere Bestellung".into() } else { "ausgestellt".into() },
                },
            )?;
            Ok(format!("Schlüssel für {email} verschickt"))
        }
        "charge.refunded" => {
            let email = email_pruefen(&b.email)?;
            let schluessel = register.iter().find(|e| e.email == email).map(|e| e.schluessel.clone()).unwrap_or_default();
            eintragen(
                ordner,
                &Eintrag {
                    zeit: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    email,
                    name: b.name.clone(),
                    schluessel,
                    bestellung: b.bestellung.clone(),
                    betrag: b.betrag.clone(),
                    art: "erstattet".into(),
                },
            )?;
            Ok("Erstattung vermerkt".to_string())
        }
        andere => Ok(format!("Ereignis {andere} ignoriert")),
    }
}

fn antworten(strom: &mut TcpStream, code: u16, text: &str) {
    let status = match code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        _ => "Internal Server Error",
    };
    let _ = write!(strom, "HTTP/1.1 {code} {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
}

/// Liest eine HTTP-Anfrage: (Methode, Kopfzeilen klein geschrieben, Rohtext).
fn anfrage_lesen(strom: &mut TcpStream) -> Result<(String, Vec<(String, String)>, Vec<u8>), u16> {
    const GRENZE: usize = 256 * 1024;
    let mut puffer = Vec::new();
    let mut stuecke = [0u8; 4096];
    let kopf_ende = loop {
        if let Some(i) = puffer.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        if puffer.len() > GRENZE {
            return Err(413);
        }
        match strom.read(&mut stuecke) {
            Ok(0) | Err(_) => return Err(400),
            Ok(n) => puffer.extend_from_slice(&stuecke[..n]),
        }
    };
    let kopf = String::from_utf8_lossy(&puffer[..kopf_ende]).into_owned();
    let mut zeilen = kopf.lines();
    let methode = zeilen.next().and_then(|z| z.split_whitespace().next()).unwrap_or("").to_string();
    let kopfzeilen: Vec<(String, String)> =
        zeilen.filter_map(|z| z.split_once(':').map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string()))).collect();
    let laenge: usize = kopfzeilen.iter().find(|(k, _)| k == "content-length").and_then(|(_, v)| v.parse().ok()).unwrap_or(0);
    if laenge > GRENZE {
        return Err(413);
    }
    let mut body = puffer[kopf_ende + 4..].to_vec();
    while body.len() < laenge {
        match strom.read(&mut stuecke) {
            Ok(0) | Err(_) => return Err(400),
            Ok(n) => body.extend_from_slice(&stuecke[..n]),
        }
    }
    body.truncate(laenge);
    Ok((methode, kopfzeilen, body))
}

pub fn lauf(ordner: &Path, args: &[String]) -> Ergebnis<()> {
    let port: u16 = option(args, "--port").map(|p| p.parse().map_err(|_| "--port muss eine Zahl sein.".to_string())).transpose()?.unwrap_or(8787);
    let datei = ordner.join("stripe-geheimnis.txt");
    let geheimnis = match std::env::var("STRIPE_GEHEIMNIS") {
        Ok(g) if !g.trim().is_empty() => g.trim().to_string(),
        _ => std::fs::read_to_string(&datei)
            .map_err(|_| format!("Signing-Secret fehlt: Umgebungsvariable STRIPE_GEHEIMNIS oder Datei {}", datei.display()))?
            .trim()
            .to_string(),
    };
    if !geheimnis.starts_with("whsec_") {
        return Err("Das Signing-Secret des Webhook-Endpunkts beginnt mit „whsec_“ (Stripe Dashboard → Webhooks → Endpunkt → Signing-Secret).".to_string());
    }
    let link_id = option(args, "--link-id").unwrap_or_default();
    privat_laden(ordner)?;
    let mailbefehl = option(args, "--mail-befehl").unwrap_or_else(|| "sendmail -t".to_string());
    let download = option(args, "--download-url").unwrap_or_default();
    let von = option(args, "--von").unwrap_or_else(|| "kontakt@jurzel.de".to_string());
    let adresse = option(args, "--adresse").unwrap_or_else(|| "127.0.0.1".to_string());
    let hoerer = TcpListener::bind((adresse.as_str(), port)).map_err(|f| format!("Port {port}: {f}"))?;
    println!("Stripe-Webhook läuft auf http://{adresse}:{port}/ (hinter einen HTTPS-Proxy setzen). Mailbefehl: {mailbefehl}");
    for strom in hoerer.incoming().flatten() {
        let mut strom = strom;
        let _ = strom.set_read_timeout(Some(Duration::from_secs(10)));
        let (methode, kopf, body) = match anfrage_lesen(&mut strom) {
            Ok(a) => a,
            Err(code) => {
                antworten(&mut strom, code, "ungültige Anfrage");
                continue;
            }
        };
        if methode != "POST" {
            antworten(&mut strom, 405, "nur POST");
            continue;
        }
        let kopf_wert = |name: &str| kopf.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str()).unwrap_or("");
        let jetzt = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        if !signatur_gueltig(&geheimnis, &body, kopf_wert("stripe-signature"), jetzt) {
            eprintln!("{} Signatur ungültig, abgewiesen", Local::now().format("%F %T"));
            antworten(&mut strom, 401, "Signatur ungültig");
            continue;
        }
        let Some(bestellung) = bestellung_lesen(&String::from_utf8_lossy(&body), &link_id) else {
            antworten(&mut strom, 400, "Inhalt nicht lesbar");
            continue;
        };
        match bearbeiten(ordner, &bestellung, &mailbefehl, &von, &download) {
            Ok(meldung) => {
                println!("{} {meldung}", Local::now().format("%F %T"));
                antworten(&mut strom, 200, &meldung);
            }
            Err(f) => {
                eprintln!("{} FEHLER {f}", Local::now().format("%F %T"));
                antworten(&mut strom, 500, "Fehler, bitte später erneut senden");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signiere(geheimnis: &str, zeit: i64, body: &[u8]) -> String {
        let mut mac = Hmac::<Sha256>::new_from_slice(geheimnis.as_bytes()).unwrap();
        mac.update(format!("{zeit}.").as_bytes());
        mac.update(body);
        let hex: String = mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect();
        format!("t={zeit},v1={hex}")
    }

    #[test]
    fn signatur_wird_geprueft() {
        let body = br#"{"a":1}"#;
        let kopf = signiere("whsec_x", 1000, body);
        assert!(signatur_gueltig("whsec_x", body, &kopf, 1100));
        assert!(!signatur_gueltig("whsec_y", body, &kopf, 1100));
        assert!(!signatur_gueltig("whsec_x", br#"{"a":2}"#, &kopf, 1100));
        // zu alt (Wiederholung) und kaputte Köpfe
        assert!(!signatur_gueltig("whsec_x", body, &kopf, 1400));
        assert!(!signatur_gueltig("whsec_x", body, "", 1000));
        assert!(!signatur_gueltig("whsec_x", body, "t=1000,v1=zz", 1000));
        // mehrere v1 (Schlüsselwechsel): eine passende genügt
        let mit_alter = format!("{kopf},v1=00ff");
        assert!(signatur_gueltig("whsec_x", body, &mit_alter, 1000));
    }

    const SITZUNG: &str = r#"{"id":"evt_1","type":"checkout.session.completed","livemode":false,"data":{"object":{"id":"cs_test_a1","payment_status":"paid","amount_total":4900,"currency":"eur","payment_link":"plink_1","customer_details":{"email":"Erika@Example.org","name":"Erika Muster"}}}}"#;

    #[test]
    fn bestellung_aus_stripe_json() {
        let b = bestellung_lesen(SITZUNG, "").unwrap();
        assert_eq!(b, Bestellung {
            ereignis: "checkout.session.completed".into(), bestellung: "cs_test_a1".into(), email: "Erika@Example.org".into(),
            name: "Erika Muster".into(), betrag: "49.00 EUR".into(), bezahlt: true, test: true,
        });
        // anderer Zahlungslink wird aussortiert
        assert_eq!(bestellung_lesen(SITZUNG, "plink_andere").unwrap().ereignis, "fremder_zahlungslink");
        assert!(bestellung_lesen(SITZUNG, "plink_1").unwrap().bezahlt);
        // Erstattung
        let erst = r#"{"type":"charge.refunded","livemode":true,"data":{"object":{"id":"ch_1","amount_refunded":4900,"currency":"eur","billing_details":{"email":"e@x.de","name":"E"}}}}"#;
        let r = bestellung_lesen(erst, "plink_1").unwrap();
        assert_eq!((r.ereignis.as_str(), r.email.as_str(), r.test), ("charge.refunded", "e@x.de", false));
    }

    #[test]
    fn mail_hat_keine_header_einschleusung() {
        let m = mail_text("kontakt@jurzel.de", "a@b.de", "Erika\r\nBcc: x@y.de", "WB1-AAAAA", "https://example.org/download");
        assert!(!m.contains("\r\nBcc:"));
        assert!(m.contains("https://example.org/download") && m.contains("WB1-AAAAA") && m.contains("Subject: =?UTF-8?B?"));
    }
}
