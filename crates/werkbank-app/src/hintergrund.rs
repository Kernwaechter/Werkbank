//! # Lange Prüfläufe im Hintergrund
//!
//! Code-Prüfer und Buch-Sprachprüfung laufen auf eigenen Threads und melden
//! jedes Ergebnis einzeln, damit die Oberfläche Fortschritt zeigen kann.

use crate::{Message, languagetool};
use iced::futures::channel::mpsc::{self, Sender};
use iced::futures::{SinkExt, Stream, StreamExt};
use std::path::PathBuf;
use werkbank_kern::codepruefung::{self, Beispiel, Ergebnis};
use werkbank_kern::sprache::Ausnahmen;

/// Prüft alle Beispiele: Rust mit `rustc` (vier Threads), Dart/Flutter parallel mit `dart analyze`.
pub fn code_pruefen(beispiele: Vec<Beispiel>, edition: String) -> impl Stream<Item = Message> {
    iced::stream::channel(64, async move |mut aus: Sender<Message>| {
        let (sender, mut empfaenger) = mpsc::unbounded::<Option<(usize, Ergebnis)>>();
        std::thread::spawn(move || {
            let ordner: PathBuf = std::env::temp_dir().join("werkbank-code");
            let tx = sender.clone();
            codepruefung::alle_pruefen(&beispiele, &edition, &ordner, 4, move |i, e| {
                let _ = tx.unbounded_send(Some((i, e)));
            });
            let _ = sender.unbounded_send(None);
        });
        while let Some(e) = empfaenger.next().await {
            let nachricht = match e {
                Some((i, ergebnis)) => Message::CodeErgebnis(i, ergebnis),
                None => Message::CodeEnde,
            };
            let ende = matches!(nachricht, Message::CodeEnde);
            if aus.send(nachricht).await.is_err() || ende {
                break;
            }
        }
    })
}

/// Prüft nacheinander jedes Kapitel mit LanguageTool; stoppt beim ersten Verbindungsfehler.
pub fn buch_sprache(
    kapitel: Vec<(String, String, String)>,
    url: String,
    sprache: String,
    ausnahmen: Ausnahmen,
) -> impl Stream<Item = Message> {
    iced::stream::channel(32, async move |mut aus: Sender<Message>| {
        let (sender, mut empfaenger) = mpsc::unbounded::<Option<Message>>();
        std::thread::spawn(move || {
            for (id, titel, text) in kapitel {
                let ergebnis = languagetool::pruefen(&url, &sprache, &text).map(|t| ausnahmen.filtern(t).len());
                let fehler = ergebnis.is_err();
                let _ = sender.unbounded_send(Some(Message::BuchSpracheKapitel { id, titel, ergebnis }));
                if fehler {
                    break;
                }
            }
            let _ = sender.unbounded_send(None);
        });
        while let Some(e) = empfaenger.next().await {
            let (nachricht, ende) = match e {
                Some(m) => (m, false),
                None => (Message::BuchSpracheEnde, true),
            };
            if aus.send(nachricht).await.is_err() || ende {
                break;
            }
        }
    })
}
