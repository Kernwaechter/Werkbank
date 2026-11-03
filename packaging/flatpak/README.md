# Flatpak

`./bauen.sh` im Ordner `packaging/flatpak/` baut und installiert die Werkbank als Flatpak (`flatpak run de.daehn.Werkbank`).

**Grenzen im Flatpak**

- Die **Programmdateien des Systems** fehlen in der Sandbox: Java (lokaler LanguageTool-Server per „Server starten“), `rustc` (Code-Prüfer), Python, Pandoc (Bauskripte). Ein LanguageTool-Server muss außerhalb laufen (Adresse in den Einstellungen, Netzwerk ist freigegeben). Der Code-Prüfer meldet die Beispiele dann als übersprungen. Der **native Satz** (Typst, EPUB) und das Cover funktionieren vollständig.
- Hunspell-Wörterbücher liest die Werkbank aus dem Host-System (`/run/host/usr/share/hunspell`), sofern sie dort installiert sind.
- Einstellungen liegen unter `~/.var/app/de.daehn.Werkbank/config/werkbank/`.

**Für Flathub** fehlen noch Bildschirmfotos und eine Homepage-Adresse in `packaging/de.daehn.Werkbank.metainfo.xml` sowie ein Quellarchiv statt `type: dir`.
