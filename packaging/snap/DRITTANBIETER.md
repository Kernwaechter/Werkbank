# Mitgelieferte Programme von Drittanbietern (Snap)

Die Werkbank selbst steht unter der Lizenzvereinbarung (`LIZENZVEREINBARUNG.md`). Im Snap liegen außerdem
eigenständige Programme, die die Werkbank als getrennte Prozesse aufruft. Sie haben ihre eigenen Lizenzen.

| Programm | Lizenz | Quelle |
|---|---|---|
| LanguageTool 6.6 | LGPL-2.1 (siehe `/opt/languagetool/COPYING.txt`) | https://languagetool.org, https://github.com/languagetool-org/languagetool |
| OpenJDK 21 (Java) | GPL-2.0 mit Classpath-Ausnahme | Ubuntu-Paket `openjdk-21-jre-headless` |
| Pandoc | GPL-2.0-oder-später | Ubuntu-Paket `pandoc`, https://github.com/jgm/pandoc |
| Typst | Apache-2.0 (`share/doc/typst/`) | https://github.com/typst/typst |
| rustc / Rust-Standardbibliothek | MIT oder Apache-2.0 | https://github.com/rust-lang/rust |
| Python 3 | PSF-Lizenz | Ubuntu-Paket `python3` |
| xdg-utils | MIT | Ubuntu-Paket `xdg-utils` |
| Hunspell-Wörterbücher (de, en, fr, es) | je nach Wörterbuch GPL, LGPL, MPL | Ubuntu-Pakete `hunspell-*` |

**Quellcode der GPL-Programme (Java, Pandoc, Hunspell-Wörterbücher):** Die Binärpakete stammen unverändert aus
dem Ubuntu-Archiv (Ubuntu 24.04). Den zugehörigen Quellcode liefert Ubuntu unter
https://archive.ubuntu.com/ubuntu/pool/ bzw. mit `apt source <Paket>`; auf Anfrage an volker@daehn.de schicke ich ihn zu.

**Nicht mitgeliefert:** Ghostscript (AGPL-3.0), Dart-SDK und Flutter.
