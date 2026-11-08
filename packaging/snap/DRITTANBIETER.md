# Mitgelieferte Programme von Drittanbietern (Snap)

Die Werkbank selbst steht unter der Lizenzvereinbarung (`LIZENZVEREINBARUNG.md`). Im Snap liegen außerdem
eigenständige Programme und Daten, die die Werkbank als getrennte Prozesse aufruft bzw. einliest. Sie haben ihre eigenen Lizenzen.

| Programm | Lizenz | Quelle |
|---|---|---|
| xdg-utils | MIT | Ubuntu-Paket `xdg-utils` |
| Hunspell-Wörterbücher (de, en-US, en-GB, fr, fr-classical, es) | je nach Wörterbuch GPL, LGPL, MPL | Ubuntu-Pakete `hunspell-*` |
| Abhängigkeiten der Ubuntu-Pakete (Perl, debconf, emacsen-common u. a.) | GPL, LGPL, MPL, MIT u. a. | Texte je Paket unter `usr/share/doc/<Paket>/copyright` im Snap |

**Quellcode der GPL-Teile (Hunspell-Wörterbücher und Abhängigkeiten der Ubuntu-Pakete):** Die Binärpakete stammen unverändert aus
dem Ubuntu-Archiv (Ubuntu 24.04). Den zugehörigen Quellcode liefert Ubuntu unter
https://archive.ubuntu.com/ubuntu/pool/ bzw. mit `apt source <Paket>`; auf Anfrage an kontakt@jurzel.de schicke ich ihn zu. Dieses Angebot gilt mindestens drei Jahre ab der jeweiligen Auslieferung und für jeden Empfänger.

**Nicht mitgeliefert, vom Nutzer selbst zu installieren:** LanguageTool und Java (Sprachprüfung über einen Server auf `localhost`), Pandoc, das Typst-Programm und Python. Die Rust-Standardbibliothek steckt im Programm selbst (MIT/Apache-2.0, Texte in `share/doc/rust/`). Auch Rust, Dart, Flutter und Ghostscript (AGPL-3.0) sind nicht dabei. Die Werkbank erklärt die Installation in den Einstellungen.

**Fremdbausteine der Werkbank selbst (Rust-Bibliotheken, Schriften):** Liste mit Lizenzen in `share/doc/werkbank/lizenzen.txt`, vollständige Lizenz- und Copyright-Texte in `share/doc/werkbank/lizenztexte.txt` (erzeugt mit `python3 packaging/lizenztexte.py`), Schriftlizenzen in `share/doc/werkbank/schriften-lizenzen.txt`.
