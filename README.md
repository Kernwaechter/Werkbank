# Werkbank – Das Schreibwerkzeug der Rust-Werkstatt

[![Rust](https://shields.io)](https://rust-lang.org)
[![Framework](https://shields.io)](https://iced.rs)
[![Lizenz](https://shields.io)](https://creativecommons.org)
[![Website](https://shields.io)](https://werkbank.jurzel-apps.de)

Die **Werkbank** ist ein freies Desktop-Werkzeug für Autoren technischer Bücher: schreiben, prüfen, Cover gestalten und integrierte Bauskripte starten.

## 🚀 Features auf einen Blick
- **Schreiben & Gliederung:** Ablenkungsfreier Editor, Dreispalten-Layout und intelligente Querverweise.
- **Verlustfreie Sicherheit:** Automatische Backups, visuelle Zeitleiste (`Strg+H`) und integrierter Papierkorb.
- **Qualität & Code-Prüfung:** Compiler-Validierung (`rustc`, `dart analyze`, ShellCheck), Offline-Rechtschreibprüfung und lokales Stilprofil.
- **Satz & Cover-Design:** Nativer Buchsatz mit Typst (PDF/X-3, EPUB 3) und Cover-Designer (`Strg+5`).

## 📂 Repository-Struktur
- **`crates/werkbank-kern`**: Kern-Logik, Parser, Backups und Statistik-Engine.
- **`crates/werkbank-app`**: Die Desktop-Anwendung (Iced-Framework).
- **`packaging/`**: Konfigurationen für native Pakete.

## 🛠 Starten & Kompilieren
```bash
cargo run -p werkbank-app --release
cargo test --workspace
```

## ⚖️ Lizenz
Dieses Projekt ist unter der **Creative Commons Namensnennung - Nicht-kommerziell - Weitergabe unter gleichen Bedingungen 4.0 International (CC BY-NC-SA 4.0)** lizenziert. 
*Kostenlose private/literarische Nutzung und Modifikation sind erlaubt; eine kommerzielle Verwertung oder der Verkauf der Software ist untersagt.*
