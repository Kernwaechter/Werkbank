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

## Lizenzschlüssel (getrennt von der Codesignatur)

`werkbank-lizenz schluessel <ordner>` erzeugt das Schlüsselpaar. `oeffentlich.key` kommt nach `assets/lizenz-oeffentlich.key` und wird beim Bauen eingebaut. `privat.key` bleibt geheim und außerhalb des Repositorys. `werkbank-lizenz ausstellen …` erzeugt je Kunde eine `lizenz.json`, die der Kunde unter „Über die Werkbank“ auswählt.
