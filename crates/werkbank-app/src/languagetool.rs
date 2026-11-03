//! # Anbindung an den LanguageTool-Server
//!
//! Sendet Kapiteltext (mit Markdown-Auszeichnung als `markup`) an einen
//! LanguageTool-HTTP-Server, startet und beendet optional einen lokalen
//! Server und führt Netzwerk- und Prozessarbeit abseits der Oberfläche aus.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use werkbank_kern::sprache::{self, Treffer};

/// Regeln, die bei Markdown-Quelltext nur Lärm machen (Zeilenumbrüche, Einrückungen).
const STANDARD_AUS: &str = "WHITESPACE_RULE";

/// Zustand der Verbindung zum Server.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ServerStatus {
    /// Noch nicht geprüft.
    #[default]
    Unbekannt,
    /// Verbindung wird getestet.
    Pruefe,
    /// Lokaler Server wurde gestartet und fährt hoch.
    Startet,
    /// Server antwortet (Text: Kurzinfo).
    Erreichbar(String),
    /// Server antwortet nicht (Text: Grund).
    Fehler(String),
}

/// Ergebnis des letzten Prüflaufs für das offene Kapitel.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PruefStatus {
    /// Noch nicht geprüft.
    #[default]
    Leer,
    /// Anfrage läuft.
    Laeuft,
    /// Fertig, mit Anzahl der Hinweise.
    Fertig(usize),
    /// Fehlgeschlagen (Text: Grund).
    Fehler(String),
}

/// Ein von der Werkbank gestarteter Server; wird beim Verwerfen beendet.
pub struct ServerProzess(Child);

impl Drop for ServerProzess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

impl ServerProzess {
    /// Startet `languagetool-server.jar` aus dem angegebenen Ordner.
    pub fn starten(java: &str, ordner: &Path, port: u16) -> Result<Self, String> {
        if !ordner.join("languagetool-server.jar").is_file() {
            return Err(format!(
                "In „{}“ liegt keine languagetool-server.jar.",
                ordner.display()
            ));
        }

        let mut befehl = Command::new(java);
        befehl
            .current_dir(ordner)
            .args(["-cp", "languagetool-server.jar", "org.languagetool.server.HTTPServer", "--port"])
            .arg(port.to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            befehl.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }

        befehl
            .spawn()
            .map(Self)
            .map_err(|e| format!("Java („{java}“) ließ sich nicht starten: {e}"))
    }

    /// Ist der Prozess noch am Leben?
    pub fn laeuft(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(None))
    }
}

fn agent(sekunden: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(sekunden)))
        .build()
        .into()
}

fn fehlertext(url: &str, e: ureq::Error) -> String {
    match e {
        ureq::Error::Io(_) | ureq::Error::ConnectionFailed | ureq::Error::HostNotFound => {
            format!("Nicht erreichbar unter {url}")
        }
        ureq::Error::Timeout(_) => format!("Zeitüberschreitung bei {url}"),
        andere => andere.to_string(),
    }
}

/// Testet, ob unter der Adresse ein LanguageTool-Server antwortet; liefert eine Kurzinfo.
pub fn verbindung_testen(url: &str) -> Result<String, String> {
    let url = url.trim().trim_end_matches('/');
    let mut antwort = agent(5)
        .get(&format!("{url}/v2/languages"))
        .call()
        .map_err(|e| fehlertext(url, e))?;
    let text = antwort.body_mut().read_to_string().map_err(|e| e.to_string())?;
    let anzahl = serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|_| format!("{url} antwortet, ist aber kein LanguageTool-Server"))?
        .as_array()
        .map_or(0, Vec::len);
    Ok(format!("{anzahl} Sprachen verfügbar"))
}

/// Prüft Markdown-Text und liefert Treffer mit Positionen im Quelltext.
pub fn pruefen(url: &str, sprache: &str, markdown: &str) -> Result<Vec<Treffer>, String> {
    let url = url.trim().trim_end_matches('/');
    let stuecke = sprache::annotation_aus_markdown(markdown);
    let daten = sprache::annotation_json(&stuecke);

    let mut antwort = agent(180)
        .post(&format!("{url}/v2/check"))
        .send_form([
            ("language", sprache),
            ("data", daten.as_str()),
            ("disabledRules", STANDARD_AUS),
        ])
        .map_err(|e| fehlertext(url, e))?;
    let text = antwort
        .body_mut()
        .with_config()
        .limit(64 * 1024 * 1024)
        .read_to_string()
        .map_err(|e| e.to_string())?;

    sprache::treffer_aus_antwort(&text, markdown, &stuecke)
}

/// Führt blockierende Arbeit in einem eigenen Thread aus, damit die Oberfläche nicht stockt.
pub async fn im_hintergrund<R: Send + 'static>(
    arbeit: impl FnOnce() -> Result<R, String> + Send + 'static,
) -> Result<R, String> {
    let (sender, empfaenger) = iced::futures::channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = sender.send(arbeit());
    });
    empfaenger.await.unwrap_or_else(|_| Err("Abgebrochen".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Minimaler HTTP-Server: beantwortet genau eine Anfrage und gibt den Anfragetext zurück.
    fn schein_server(antwort: String) -> (String, std::thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut daten = Vec::new();
            let mut puffer = [0u8; 4096];
            loop {
                let n = s.read(&mut puffer).unwrap();
                daten.extend_from_slice(&puffer[..n]);
                let text = String::from_utf8_lossy(&daten).to_string();
                if let Some(ende) = text.find("\r\n\r\n") {
                    let laenge = text[..ende]
                        .to_lowercase()
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length: ").map(|v| v.trim().parse::<usize>().unwrap()))
                        .unwrap_or(0);
                    if daten.len() >= ende + 4 + laenge {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let kopf = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                antwort.len()
            );
            s.write_all(kopf.as_bytes()).unwrap();
            s.write_all(antwort.as_bytes()).unwrap();
            String::from_utf8_lossy(&daten).to_string()
        });
        (url, handle)
    }

    #[test]
    fn pruefen_sendet_annotation_und_liest_treffer() {
        let md = "Ein Beipsiel mit `code`.\n";
        let antwort = r#"{"matches":[{"message":"Tippfehler","offset":4,"length":8,
            "replacements":[{"value":"Beispiel"}],
            "context":{"text":"Ein Beipsiel mit `code`.","offset":4,"length":8},
            "rule":{"id":"GERMAN_SPELLER_RULE","category":{"id":"TYPOS"}}}]}"#;
        let (url, anfrage) = schein_server(antwort.to_string());

        let treffer = pruefen(&url, "de-DE", md).unwrap();
        assert_eq!(treffer.len(), 1);
        assert_eq!(treffer[0].fundstelle, "Beipsiel");

        let gesendet = anfrage.join().unwrap();
        assert!(gesendet.starts_with("POST /v2/check"));
        assert!(gesendet.contains("language=de-DE"));
        assert!(gesendet.contains("annotation"));
        assert!(gesendet.contains("interpretAs"));
    }

    #[test]
    fn verbindungstest_zaehlt_sprachen() {
        let (url, _h) = schein_server(r#"[{"name":"German"},{"name":"English"}]"#.to_string());
        assert_eq!(verbindung_testen(&url).unwrap(), "2 Sprachen verfügbar");
    }

    #[test]
    fn nicht_erreichbarer_server_gibt_lesbaren_fehler() {
        let fehler = verbindung_testen("http://127.0.0.1:9").unwrap_err();
        assert!(fehler.contains("Nicht erreichbar"), "{fehler}");
    }
}
