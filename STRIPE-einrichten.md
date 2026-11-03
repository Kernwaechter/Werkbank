# Stripe einrichten (Managed Payments)

Stand: Sandbox. Zahlungslink: `assets/kauf-url.txt` (Testlink `…/test_…`).

## 1. Im Dashboard
- Produkt „JZ Werkbank“, 49,00 EUR, einmalig. Texte und Bild: [STRIPE-produkttext.md](STRIPE-produkttext.md).
- Zahlungslink für das Produkt, E-Mail-Adresse als Pflichtfeld. Den Link `plink_…` notieren (Zahlungslinks → Link anklicken → Adresse in der Browserzeile oder Details).
- Kontoauszug-Bezeichnung: in den Kontoeinstellungen (zum Beispiel `JZ WERKBANK`).
- Steuern und Managed Payments: Firmendaten (Jurzel), Auszahlungskonto, Steuerregistrierung nach den Anweisungen im Dashboard.

## 2. Webhook-Endpunkt (Dashboard → Entwickler → Webhooks → Endpunkt hinzufügen)
- Endpunkt-URL: `https://<deine-domain>/stripe` (HTTPS Pflicht)
- Ereignisse: `checkout.session.completed`, `checkout.session.async_payment_succeeded`, `charge.refunded`
- Danach das **Signing-Secret** (`whsec_…`) kopieren und auf dem Server speichern:
  `~/.local/share/werkbank-lizenzgeber/stripe-geheimnis.txt` oder Umgebungsvariable `STRIPE_GEHEIMNIS`.
- Sandbox und Live haben getrennte Endpunkte, Geheimnisse und Zahlungslinks.

## 3. Server
1. `cargo build --release -p werkbank-lizenz` und die Datei auf den Server kopieren.
2. Den Datenordner `~/.local/share/werkbank-lizenzgeber/` mit **demselben** `privat.key` bereitstellen, dessen öffentlicher Teil in der Werkbank steckt.
3. Mailversand: `msmtp` oder `postfix`, sodass `sendmail -t` Mails verschickt. Absender deine Domain mit SPF und DKIM, sonst landen die Schlüsselmails im Spam.
4. Starten (zum Beispiel als systemd-Dienst):
   `werkbank-lizenz webhook --port 8787 --von kontakt@jurzel.de --link-id plink_…`
   `--link-id` lässt nur Käufe über diesen Zahlungslink zu.
5. HTTPS davor, zum Beispiel mit Caddy:
   ```
   deine-domain.de {
       reverse_proxy /stripe 127.0.0.1:8787
   }
   ```

## 4. Testen (Sandbox)
1. In der Werkbank „Kaufen“ klicken (Über die Werkbank oder Sperrbildschirm). Der Testlink öffnet sich mit vorausgefüllter E-Mail-Adresse.
2. Testkarte `4242 4242 4242 4242`, beliebiges Ablaufdatum in der Zukunft, beliebige Prüfziffer.
3. Dashboard → Webhooks → Endpunkt → Ereignisse: Antwort „200“. Die Mail mit dem Schlüssel kommt an.
4. In der Werkbank Adresse und Schlüssel eintragen: „Lizenziert für …“. `werkbank-lizenz liste` zeigt die Bestellung als Art „test“.
5. Lokal ohne Server testen geht mit der Stripe-CLI: `stripe listen --forward-to localhost:8787` (die CLI gibt ein eigenes `whsec_…` aus).

## 5. Livegang
- Live-Zahlungslink in `assets/kauf-url.txt` und in `werkstatt-web/index.html` (`data-checkout`) eintragen, Live-Webhook mit eigenem Geheimnis anlegen, Werkbank neu bauen.
- Die Paket-Skripte brechen ab, solange `assets/kauf-url.txt` einen Testlink (`/test_`) enthält. Für Testpakete `ALLOW_TESTLINK=1` setzen.

## Hinweise
- Das Webhook-Format (Header `Stripe-Signature`, Ereignisse, Felder `customer_details.email`, `payment_status`) entspricht der Stripe-Dokumentation für Checkout-Sitzungen. Ob Managed Payments dieselben Ereignisse sendet, habe ich nicht gesehen. Prüfe es beim Test unter „Ereignisse“ im Dashboard.
- Ein ausgestellter Schlüssel bleibt offline gültig. Bei Erstattungen (`charge.refunded`) vermerkt der Webhook sie nur im Register.
- Impressum, Datenschutzerklärung und Widerrufsinformationen gehören auf die Webseite.
