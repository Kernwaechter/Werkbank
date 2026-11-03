kopiere den kompletten Ordner

~/.local/share/werkbank-lizenzgeber/
├── privat.key         # Der private Ed25519-Signierschlüssel
├── oeffentlich.key    # Der zugehörige öffentliche Schlüssel
├── register.jsonl     # Dein Kundenregister mit allen bisher ausgestellten Lizenzen
└── sicherung/         # Die fortlaufenden Backups des Registers


Setze wieder die Rechte

chmod 700 ~/.local/share/werkbank-lizenzgeber
chmod 600 ~/.local/share/werkbank-lizenzgeber/privat.key
