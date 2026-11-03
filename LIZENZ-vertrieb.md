# Lizenzschlüssel und Vertrieb

## Wie es funktioniert
- Ein Lizenzschlüssel ist die Ed25519-Signatur über die kleingeschriebene E-Mail-Adresse des Kunden (`WB1-XXXXX-XXXXX-…`, 21 Gruppen).
- Die Werkbank enthält nur den öffentlichen Schlüssel (`assets/lizenz-oeffentlich.key`) und prüft offline. Ohne den privaten Schlüssel lässt sich kein Schlüssel fälschen.
- Zur selben Adresse gehört immer derselbe Schlüssel. Der Kunde kann ihn also jederzeit neu bekommen, auch wenn das Register verloren ging.
- Der Kunde trägt Adresse und Schlüssel in den Einstellungen unter „Lizenz“ ein (Zahnrad oder Strg+,) und klickt „Lizenzschlüssel aktivieren“.

## Dateien des Lizenzgebers
Ordner: `~/.local/share/werkbank-lizenzgeber/` (oder `$WERKBANK_LIZENZGEBER`)

| Datei | Inhalt | Sichern? |
|---|---|---|
| `privat.key` | geheimer Schlüssel | **Ja, getrennt vom Rechner. Nie weitergeben, nie ins Projekt.** |
| `oeffentlich.key` | öffentlicher Schlüssel | steckt in der Werkbank |
| `register.jsonl` | Kunden: Zeit, Adresse, Name, Schlüssel, Bestellung, Betrag | ja |
| `sicherung/register-NNNN.jsonl` | nach jedem Eintrag eine nummerierte Kopie | ja |

Das Register enthält personenbezogene Daten (E-Mail-Adressen). Aufbewahrung und Löschfristen nach DSGVO beachten.

## Befehle
```
cargo build --release -p werkbank-lizenz        # target/release/werkbank-lizenz
werkbank-lizenz ausstellen --email kunde@example.org --name "Erika Muster" --bestellung R-1001 --betrag 49
werkbank-lizenz wiederherstellen --email kunde@example.org
werkbank-lizenz suchen muster
werkbank-lizenz liste
werkbank-lizenz sichern --ziel /media/usb/werkbank   # JSONL + CSV mit Zeitstempel
werkbank-lizenz pruefen --email kunde@example.org --schluessel WB1-…
```

## Testzeit und Sperre
- Die Werkbank läuft 28 Tage mit vollem Funktionsumfang (Beginn = erster Start). Der Beginn steht an zwei Orten (Konfigurationsordner und Datenordner des Benutzers); der frühere gilt. Stellt jemand die Uhr zurück, zählt das zuletzt gesehene Datum.
- Danach erscheint ein Sperrbildschirm: Kaufen, Schlüssel eintragen oder „Speichern und beenden“. Texte bleiben unberührt.
- Grenze: Wer beide Dateien löscht, bekommt wieder 28 Tage. Eine reine Offline-Prüfung kann das nicht verhindern. Für Autorinnen und Autoren ist das meist unerheblich.
- Debug-Builds (`cargo run` ohne `--release`) sperren nie.

## Ablauf mit Stripe
1. Der Kunde kauft über die Kasse (Knopf auf der Webseite oder „Kaufen“ in der Werkbank, mit E-Mail vorbelegt).
2. Stripe schickt `checkout.session.completed` an den Webhook (`werkbank-lizenz webhook`).
3. Der Webhook prüft die Signatur (HMAC-SHA256 mit Zeitstempel), stellt den Schlüssel zur E-Mail-Adresse aus, schickt ihn per E-Mail und trägt ihn ins Register ein. Wiederholte Zustellung derselben Bestellung ändert nichts.
4. Bei `charge.refunded` wird die Erstattung im Register vermerkt. Ein Offline-Schlüssel lässt sich nicht widerrufen.

Einrichtung bei Stripe und auf dem Server: [STRIPE-einrichten.md](STRIPE-einrichten.md).

## Von Hand (ohne Webhook)
`werkbank-lizenz ausstellen --email … --name … --bestellung … --betrag 49` und den Schlüssel selbst per Mail schicken.
