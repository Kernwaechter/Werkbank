# Signieren und Prüfen von Paketen

**Stand: vorbereitet, nie ausgeführt.** Ein Zertifikat und Apple-Zugangsdaten kann ich nicht beschaffen; sie sind Sache des Verkäufers.

## Was nötig ist

| System | Du brauchst | Kosten/Aufwand |
|---|---|---|
| Windows | Code-Signing-Zertifikat (OV oder EV) einer Zertifizierungsstelle, als `.pfx` | jährliche Gebühr; EV-Zertifikate umgehen SmartScreen-Warnungen sofort |
| macOS | Apple-Developer-Mitgliedschaft, Zertifikat „Developer ID Application“, App-spezifisches Passwort | 99 USD/Jahr |
| Linux | ein GPG-Schlüssel (für `SHA256SUMS.asc`) | kostenlos |

## Einrichtung im GitHub-Repository (Settings → Secrets)

- Windows: `WIN_PFX_BASE64` (`base64 -w0 zertifikat.pfx`), `WIN_PFX_PASSWORD`
- macOS: `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORT`, `APPLE_ZERTIFIKAT_BASE64` (`base64 -i zertifikat.p12`), `APPLE_ZERTIFIKAT_PASSWORT`, `APPLE_IDENTITAET` (z. B. `Developer ID Application: Name (TEAMID)`)
- Linux/Prüfsummen: `GPG_PRIVATE_KEY` (`gpg --armor --export-secret-keys KENNUNG`)

Fehlen die Secrets, überspringt der Workflow die Signierschritte; die Pakete entstehen dann unsigniert.

## Prüfen durch Anwender

- Windows: Rechtsklick auf die `.exe` → Eigenschaften → Digitale Signaturen
- macOS: `codesign --verify --deep --strict Werkbank.app` und `spctl --assess --type execute Werkbank.app`
- Alle Systeme: `sha256sum -c SHA256SUMS` und `gpg --verify SHA256SUMS.asc`

## RPM signieren (Linux)

`cargo generate-rpm` signiert mit einer **exportierten PGP-Schlüsseldatei** (RSA oder Ed25519). Ein Schlüssel mit Passwort wird nicht unterstützt, deshalb legst du einen eigenen Schlüssel nur für die Paketsignatur ohne Passwort an.

```bash
# 1. Schlüssel erzeugen (RSA 4096, 3 Jahre gültig, ohne Passwort)
gpg --batch --passphrase '' --quick-generate-key "Volker Dähn (Werkbank Pakete) <volker@daehn.de>" rsa4096 sign 3y

# 2. Kennung (Fingerabdruck) ansehen
gpg --list-keys --keyid-format long volker@daehn.de

# 3. Exportieren: privat (bleibt geheim, nicht ins Repository!) und öffentlich (kommt auf die Webseite)
gpg --armor --export-secret-keys volker@daehn.de > ~/werkbank-rpm-signatur.secret.asc
chmod 600 ~/werkbank-rpm-signatur.secret.asc
gpg --armor --export volker@daehn.de > werkstatt-web/downloads/werkbank-rpm-signatur.asc

# 4. RPM signieren (Release-Binary muss gebaut sein)
packaging/rpm-signieren.sh ~/werkbank-rpm-signatur.secret.asc
```

Anwender prüfen so: `sudo rpm --import werkbank-rpm-signatur.asc` und `rpm --checksig werkbank-1.4.0-1.x86_64.rpm` (Ausgabe: `… digests signatures OK`). Den Fingerabdruck aus Schritt 2 solltest du zusätzlich auf der Webseite nennen, damit man den Schlüssel gegenprüfen kann. Die private Datei außerhalb des Repositorys sichern (z. B. Passwortmanager oder verschlüsselter Datenträger); ohne sie lassen sich spätere Versionen nicht mit demselben Schlüssel signieren.
