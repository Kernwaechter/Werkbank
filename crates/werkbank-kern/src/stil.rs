//! # Stilprofil: lebendig oder steril?
//!
//! Misst Merkmale, an denen Leser gleichförmigen, „maschinell“ wirkenden Text erkennen:
//! gleich lange Sätze und Absätze, immer gleiche Satzanfänge, Füllformeln, viele
//! Verbindungswörter, wenig persönliche Stimme, Gedankenstriche und Dreierreihen.
//!
//! **Das ist kein KI-Detektor.** Es gibt kein verlässliches Verfahren dafür. Der Wert sagt nur,
//! wie abwechslungsreich und persönlich ein Text geschrieben ist; er hilft beim Überarbeiten.
//! Fachtexte erreichen bei „Stimme“ naturgemäß weniger als Erzählungen.

use crate::markdown;

/// Gesamturteil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urteil {
    /// Zu wenig Text für eine Einschätzung.
    ZuWenigText,
    /// Lebendig, abwechslungsreich, persönlich.
    Lebendig,
    /// Überwiegend natürlich, mit einzelnen glatten Stellen.
    Ueberwiegend,
    /// Gleichförmig, stellenweise maschinell.
    Gleichfoermig,
    /// Steril: klingt nach Maschinentext.
    Steril,
}

impl Urteil {
    /// Kurzbezeichnung für die Oberfläche.
    pub fn name(self) -> &'static str {
        match self {
            Self::ZuWenigText => "Zu wenig Text",
            Self::Lebendig => "Lebendig und menschlich",
            Self::Ueberwiegend => "Überwiegend natürlich",
            Self::Gleichfoermig => "Gleichförmig",
            Self::Steril => "Steril, klingt nach Maschine",
        }
    }
}

/// Eine einzelne Messgröße mit Einstufung (0 = auffällig, 1 = unauffällig).
#[derive(Debug, Clone, PartialEq)]
pub struct Kennzahl {
    /// Bezeichnung.
    pub name: &'static str,
    /// Gemessener Wert in lesbarer Form.
    pub wert: String,
    /// Einstufung von 0 (auffällig) bis 1 (unauffällig).
    pub einstufung: f32,
    /// Gewicht im Gesamtwert (Prozentpunkte).
    pub gewicht: f32,
    /// Rat zur Überarbeitung, wenn die Einstufung schwach ist.
    pub tipp: Option<&'static str>,
}

/// Ergebnis der Analyse.
#[derive(Debug, Clone, PartialEq)]
pub struct Stilprofil {
    /// Wörter im Fließtext.
    pub woerter: usize,
    /// Sätze im Fließtext.
    pub saetze: usize,
    /// Satzlängen in Wörtern, in Textreihenfolge (für die Rhythmusanzeige).
    pub satzlaengen: Vec<usize>,
    /// Durchschnittliche Satzlänge.
    pub satzlaenge_mittel: f32,
    /// Anteil kurzer Sätze (bis 8 Wörter).
    pub kurz_anteil: f32,
    /// Anteil langer Sätze (ab 25 Wörter).
    pub lang_anteil: f32,
    /// Häufigste Satzanfänge mit Anzahl.
    pub satzanfaenge: Vec<(String, usize)>,
    /// Gefundene Füllformeln mit Anzahl, häufigste zuerst.
    pub floskeln: Vec<(String, usize)>,
    /// Einzelne Messgrößen.
    pub kennzahlen: Vec<Kennzahl>,
    /// Gesamtwert 0 bis 100 (hoch = lebendig).
    pub punktzahl: u8,
    /// Gesamturteil.
    pub urteil: Urteil,
}

/// Füllformeln und Wendungen, die in maschinell erzeugten Texten gehäuft vorkommen (kleingeschrieben).
const FLOSKELN: &[&str] = &[
    "es ist wichtig zu beachten",
    "es ist wichtig zu betonen",
    "es ist wichtig zu verstehen",
    "es ist zu beachten",
    "zusammenfassend lässt sich sagen",
    "abschließend lässt sich",
    "es lässt sich festhalten",
    "zusammenfassend",
    "in der heutigen digitalen welt",
    "in der heutigen schnelllebigen welt",
    "in der heutigen zeit",
    "heutzutage",
    "tauchen wir ein",
    "eintauchen",
    "in die welt von",
    "spielt eine entscheidende rolle",
    "spielt eine zentrale rolle",
    "spielt eine wichtige rolle",
    "von entscheidender bedeutung",
    "von zentraler bedeutung",
    "ein wichtiger aspekt",
    "ein breites spektrum",
    "eine vielzahl von",
    "nahtlos",
    "maßgeblich",
    "unerlässlich",
    "ganzheitlich",
    "in diesem zusammenhang",
    "im folgenden",
    "im rahmen von",
    "zielt darauf ab",
    "darauf abzielt",
    "ermöglicht es",
    "bietet die möglichkeit",
    "stellt eine herausforderung dar",
    "eine reise",
    "meilenstein",
    "landschaft",
    "es bleibt abzuwarten",
    "game-changer",
    "revolutioniert",
];

const KONNEKTOREN: &[&str] = &[
    "außerdem", "zudem", "darüber hinaus", "ferner", "folglich", "somit", "dennoch", "jedoch", "daher", "deshalb",
    "demnach", "allerdings", "dabei", "gleichzeitig", "insgesamt", "letztlich", "schließlich", "zusätzlich",
    "ebenfalls", "hierbei", "infolgedessen", "nichtsdestotrotz",
];

const ANSPRACHE: &[&str] = &[
    "ich", "mich", "mir", "mein", "meine", "meinen", "meiner", "meinem", "du", "dich", "dir", "dein", "deine", "deinen",
    "deiner", "deinem", "wir", "uns", "unser", "unsere", "unseren", "unserer",
];

const ABKUERZUNGEN: &[&str] = &[
    "z", "B", "d", "h", "u", "a", "ca", "Nr", "bzw", "usw", "vgl", "Abb", "Kap", "S", "etc", "evtl", "ggf", "inkl", "max",
    "min", "Dr", "Prof", "Hr", "Fr", "engl", "bspw", "sog", "o", "ä", "v", "Tab", "Bd", "Bsp", "ff", "f", "cm", "mm", "km",
    "vs", "i", "e", "g", "ebd", "Hrsg", "Jh", "Mio", "Mrd",
];

/// Fließtext-Absätze eines Markdown-Textes: ohne Code, Überschriften, Listen, Tabellen und Auszeichnung.
fn absaetze(md: &str) -> Vec<String> {
    let mut ergebnis = Vec::new();
    let mut aktuell = String::new();
    let abschliessen = |aktuell: &mut String, ergebnis: &mut Vec<String>| {
        let t = aktuell.trim();
        if !t.is_empty() {
            ergebnis.push(t.to_string());
        }
        aktuell.clear();
    };
    for z in markdown::zeilen(md) {
        let t = z.text.trim();
        let kein_fliesstext = z.im_code
            || t.starts_with('#')
            || t.starts_with('|')
            || t.starts_with("- ")
            || t.starts_with("* ")
            || t.starts_with("+ ")
            || t.starts_with("![")
            || t.chars().next().is_some_and(|c| c.is_ascii_digit()) && t.split_once(". ").is_some_and(|(a, _)| a.chars().all(|c| c.is_ascii_digit()));
        if t.is_empty() || kein_fliesstext {
            abschliessen(&mut aktuell, &mut ergebnis);
            continue;
        }
        if !aktuell.is_empty() {
            aktuell.push(' ');
        }
        aktuell.push_str(t.trim_start_matches('>').trim());
    }
    abschliessen(&mut aktuell, &mut ergebnis);
    ergebnis.iter().map(|a| bereinigen(a)).filter(|a| !a.trim().is_empty()).collect()
}

/// Entfernt Inline-Code, Link-Adressen, `@kap:`-Verweise und Hervorhebungszeichen.
fn bereinigen(s: &str) -> String {
    let mut aus = String::with_capacity(s.len());
    let mut im_code = false;
    let mut in_adresse = false;
    let zeichen: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < zeichen.len() {
        let c = zeichen[i];
        if c == '`' {
            im_code = !im_code;
            if !im_code {
                aus.push_str("Code");
            }
        } else if im_code {
            // nichts
        } else if in_adresse {
            if c == ')' {
                in_adresse = false;
            }
        } else if c == ']' && zeichen.get(i + 1) == Some(&'(') {
            in_adresse = true;
            i += 1;
        } else if c == '@' && zeichen[i..].iter().take(5).collect::<String>() == "@kap:" {
            while i < zeichen.len() && !zeichen[i].is_whitespace() {
                i += 1;
            }
            aus.push_str("Kapitel");
            continue;
        } else if !matches!(c, '*' | '_' | '[') {
            aus.push(c);
        }
        i += 1;
    }
    aus
}

fn ist_abkuerzung(wort: &str) -> bool {
    let w = wort.trim_matches(|c: char| !c.is_alphanumeric());
    w.chars().count() == 1 || ABKUERZUNGEN.contains(&w)
}

/// Teilt einen Absatz in Sätze.
fn saetze(absatz: &str) -> Vec<String> {
    let zeichen: Vec<char> = absatz.chars().collect();
    let mut ergebnis = Vec::new();
    let mut start = 0;
    for i in 0..zeichen.len() {
        if !matches!(zeichen[i], '.' | '!' | '?' | '…') {
            continue;
        }
        // Satzzeichen-Folgen („?!“, „...“) zusammen behandeln
        if zeichen.get(i + 1).is_some_and(|c| matches!(c, '.' | '!' | '?' | '…' | '“' | '"' | '«' | ')')) {
            continue;
        }
        let ende_ok = match zeichen.get(i + 1) {
            None => true,
            Some(c) if c.is_whitespace() => zeichen[i + 1..]
                .iter()
                .find(|c| !c.is_whitespace())
                .is_none_or(|n| n.is_uppercase() || matches!(n, '„' | '»' | '"' | '(' | '“') || n.is_ascii_digit()),
            _ => false,
        };
        if !ende_ok {
            continue;
        }
        if zeichen[i] == '.' {
            let davor: String = zeichen[start..i].iter().rev().take_while(|c| !c.is_whitespace()).collect::<Vec<_>>().into_iter().rev().collect();
            if ist_abkuerzung(&davor) {
                continue;
            }
        }
        let satz: String = zeichen[start..=i].iter().collect();
        if !satz.trim().is_empty() {
            ergebnis.push(satz.trim().to_string());
        }
        start = i + 1;
    }
    let rest: String = zeichen[start..].iter().collect();
    if rest.trim().chars().any(char::is_alphanumeric) {
        ergebnis.push(rest.trim().to_string());
    }
    ergebnis
}

fn woerter(s: &str) -> Vec<String> {
    s.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '’' || c == '\''))
        .map(|w| w.trim_matches(|c| c == '-' || c == '\'' || c == '’'))
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

fn mittel(werte: &[f32]) -> f32 {
    if werte.is_empty() { 0.0 } else { werte.iter().sum::<f32>() / werte.len() as f32 }
}

/// Variationskoeffizient (Standardabweichung durch Mittelwert).
fn variation(werte: &[usize]) -> f32 {
    let w: Vec<f32> = werte.iter().map(|&x| x as f32).collect();
    let m = mittel(&w);
    if m == 0.0 {
        return 0.0;
    }
    (w.iter().map(|x| (x - m).powi(2)).sum::<f32>() / w.len() as f32).sqrt() / m
}

/// Wortschatzvielfalt: mittlere Typ-Token-Rate über gleitende 50-Wort-Fenster (längenunabhängig).
fn wortschatzvielfalt(w: &[String]) -> f32 {
    const FENSTER: usize = 50;
    if w.len() < FENSTER {
        let einmalig: std::collections::HashSet<_> = w.iter().collect();
        return einmalig.len() as f32 / w.len().max(1) as f32;
    }
    let mut summe = 0.0;
    let mut n = 0;
    for start in (0..=w.len() - FENSTER).step_by(10) {
        let einmalig: std::collections::HashSet<_> = w[start..start + FENSTER].iter().collect();
        summe += einmalig.len() as f32 / FENSTER as f32;
        n += 1;
    }
    summe / n as f32
}

/// Zählt Vorkommen ganzer Wörter oder Wortgruppen und ersetzt die Fundstellen durch Leerzeichen.
fn zaehle_phrasen(text: &mut String, phrasen: &[&str]) -> Vec<(String, usize)> {
    let mut liste = Vec::new();
    let mut sortiert: Vec<&&str> = phrasen.iter().collect();
    sortiert.sort_by_key(|p| std::cmp::Reverse(p.len()));
    for p in sortiert {
        let mut n = 0;
        let mut pos = 0;
        while let Some(i) = text[pos..].find(*p) {
            let von = pos + i;
            let bis = von + p.len();
            let davor = text[..von].chars().next_back();
            let danach = text[bis..].chars().next();
            let wortgrenze = !davor.is_some_and(char::is_alphanumeric) && !danach.is_some_and(char::is_alphanumeric);
            // Wortstämme wie „nahtlos“ dürfen als Anfang von „nahtlose“ zählen
            let stamm = !davor.is_some_and(char::is_alphanumeric) && p.ends_with("los");
            if wortgrenze || stamm {
                n += 1;
                let leer = " ".repeat(p.chars().count());
                text.replace_range(von..bis, &leer);
                pos = von + leer.len();
            } else {
                pos = bis;
            }
        }
        if n > 0 {
            liste.push((p.to_string(), n));
        }
    }
    liste.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    liste
}

fn hat_dreiklang(satz: &str) -> bool {
    // „…, B und C“: eine Kommaliste, deren letztes Glied „X und Y“ aus kurzen Teilen besteht
    let teile: Vec<&str> = satz.split(", ").collect();
    if teile.len() < 2 {
        return false;
    }
    let letzter = teile[teile.len() - 1].trim_end_matches(['.', '!', '?']);
    let (links, rechts) = match letzter.split_once(" und ").or_else(|| letzter.split_once(" oder ")) {
        Some(p) => p,
        None => return false,
    };
    links.split_whitespace().count() <= 2 && rechts.split_whitespace().count() <= 2
}

fn gewichtet(einstufung: f32) -> f32 {
    einstufung.clamp(0.0, 1.0)
}

fn linear(wert: f32, schlecht: f32, gut: f32) -> f32 {
    gewichtet((wert - schlecht) / (gut - schlecht))
}

/// Analysiert den Fließtext eines Markdown-Textes (Code, Überschriften und Listen bleiben außen vor).
pub fn analysieren(md: &str) -> Stilprofil {
    let absatzliste = absaetze(md);
    let mut satzliste: Vec<String> = Vec::new();
    let mut absatzlaengen = Vec::new();
    for a in &absatzliste {
        let s = saetze(a);
        absatzlaengen.push(woerter(a).len());
        satzliste.extend(s);
    }
    let satzliste: Vec<String> = satzliste.into_iter().filter(|s| woerter(s).len() >= 2).collect();
    let satzlaengen: Vec<usize> = satzliste.iter().map(|s| woerter(s).len()).collect();
    let alle_woerter: Vec<String> = absatzliste.iter().flat_map(|a| woerter(a)).collect();
    let n_w = alle_woerter.len();
    let n_s = satzlaengen.len();

    if n_w < 150 || n_s < 8 {
        return Stilprofil {
            woerter: n_w,
            saetze: n_s,
            satzlaengen,
            satzlaenge_mittel: 0.0,
            kurz_anteil: 0.0,
            lang_anteil: 0.0,
            satzanfaenge: Vec::new(),
            floskeln: Vec::new(),
            kennzahlen: Vec::new(),
            punktzahl: 0,
            urteil: Urteil::ZuWenigText,
        };
    }

    let je_1000 = |n: usize| n as f32 * 1000.0 / n_w as f32;
    let je_100_saetze = |n: usize| n as f32 * 100.0 / n_s as f32;
    let satz_mittel = satzlaengen.iter().sum::<usize>() as f32 / n_s as f32;
    let kurz = satzlaengen.iter().filter(|&&l| l <= 8).count() as f32 / n_s as f32;
    let lang = satzlaengen.iter().filter(|&&l| l >= 25).count() as f32 / n_s as f32;
    let cv_saetze = variation(&satzlaengen);
    let cv_absaetze = variation(&absatzlaengen);

    // Satzanfänge
    let mut anfaenge: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for s in &satzliste {
        if let Some(w) = woerter(s).into_iter().next() {
            *anfaenge.entry(w).or_default() += 1;
        }
    }
    let mut anfangsliste: Vec<(String, usize)> = anfaenge.into_iter().collect();
    anfangsliste.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let haeufigster = anfangsliste.first().map_or(0, |a| a.1) as f32 / n_s as f32;

    // Phrasen
    let mut text_klein = absatzliste.join(" \n ").to_lowercase();
    let mut floskeln = zaehle_phrasen(&mut text_klein, FLOSKELN);
    // „nicht nur … sondern auch“ innerhalb eines Satzes
    let doppel = satzliste
        .iter()
        .filter(|s| {
            let k = s.to_lowercase();
            k.find("nicht nur").zip(k.find("sondern auch")).is_some_and(|(a, b)| a < b)
        })
        .count();
    if doppel > 0 {
        floskeln.push(("nicht nur … sondern auch".to_string(), doppel));
    }
    floskeln.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let floskel_summe: usize = floskeln.iter().map(|f| f.1).sum();
    let konnektoren: usize = zaehle_phrasen(&mut text_klein.clone(), KONNEKTOREN).iter().map(|k| k.1).sum();

    let ansprache = alle_woerter.iter().filter(|w| ANSPRACHE.contains(&w.as_str())).count();
    let fragen_ausrufe = absatzliste.iter().map(|a| a.matches(['?', '!']).count()).sum::<usize>();
    let rede = absatzliste.iter().map(|a| a.matches(['„', '»']).count() + a.matches('"').count() / 2).sum::<usize>();
    let lebendigkeit = je_1000(ansprache) + je_1000(fragen_ausrufe) + je_1000(rede) * 1.5;
    let striche = absatzliste.iter().map(|a| a.matches('—').count() + a.matches(" – ").count()).sum::<usize>();
    let dreiklaenge = satzliste.iter().filter(|s| hat_dreiklang(s)).count();
    let vielfalt = wortschatzvielfalt(&alle_woerter);

    let kennzahl = |name, wert: String, einstufung: f32, gewicht, tipp: &'static str| Kennzahl {
        name,
        wert,
        einstufung,
        gewicht,
        tipp: (einstufung < 0.5).then_some(tipp),
    };
    let kennzahlen = vec![
        kennzahl(
            "Satzrhythmus",
            format!("Variation {:.2} (Ø {:.0} Wörter)", cv_saetze, satz_mittel),
            linear(cv_saetze, 0.28, 0.62),
            25.0,
            "Die Sätze sind fast gleich lang. Mische kurze Sätze (3 bis 8 Wörter) mit längeren und lass hin und wieder einen kurzen Satz allein stehen.",
        ),
        kennzahl(
            "Füllformeln",
            format!("{:.1} je 1000 Wörter", je_1000(floskel_summe)),
            1.0 - linear(je_1000(floskel_summe), 0.0, 6.0),
            20.0,
            "Typische Füllformeln stehen im Text (siehe Liste). Streiche sie oder ersetze sie durch eine konkrete Aussage.",
        ),
        kennzahl(
            "Verbindungswörter",
            format!("{:.1} je 1000 Wörter", je_1000(konnektoren)),
            1.0 - linear(je_1000(konnektoren), 8.0, 25.0),
            10.0,
            "Viele Brückenwörter (außerdem, zudem, somit, dabei …). Oft genügt ein neuer Satz ohne Brücke.",
        ),
        kennzahl(
            "Wortschatz",
            format!("Vielfalt {:.2}", vielfalt),
            linear(vielfalt, 0.62, 0.80),
            10.0,
            "Der Wortschatz wiederholt sich stark. Suche treffendere, anschaulichere Wörter.",
        ),
        kennzahl(
            "Satzanfänge",
            format!(
                "{} Sätze beginnen mit „{}“ ({:.0} %)",
                anfangsliste.first().map_or(0, |a| a.1),
                anfangsliste.first().map_or("", |a| a.0.as_str()),
                haeufigster * 100.0
            ),
            1.0 - linear(haeufigster, 0.12, 0.30),
            10.0,
            "Viele Sätze beginnen gleich. Variiere den Einstieg: Objekt, Umstand, Frage, kurzer Ausruf.",
        ),
        kennzahl(
            "Persönliche Stimme",
            format!("{:.0} Marker je 1000 Wörter (du/ich/wir, Fragen, Rede)", lebendigkeit),
            linear(lebendigkeit, 2.0, 20.0),
            10.0,
            "Wenig persönliche Stimme. Sprich den Leser an, stelle Fragen, erzähle ein eigenes Erlebnis.",
        ),
        kennzahl(
            "Absatzlängen",
            format!("Variation {:.2}", cv_absaetze),
            linear(cv_absaetze, 0.20, 0.60),
            5.0,
            "Alle Absätze sind ähnlich lang. Variiere: ein einzelner kurzer Absatz setzt einen Akzent.",
        ),
        kennzahl(
            "Gedankenstriche",
            format!("{:.1} je 1000 Wörter", je_1000(striche)),
            1.0 - linear(je_1000(striche), 2.0, 10.0),
            5.0,
            "Viele Gedankenstriche, ein häufiges Maschinenmuster. Ersetze manche durch Punkt oder Komma.",
        ),
        kennzahl(
            "Dreierreihen",
            format!("{:.1} je 100 Sätze", je_100_saetze(dreiklaenge)),
            1.0 - linear(je_100_saetze(dreiklaenge), 6.0, 20.0),
            5.0,
            "Auffallend viele Aufzählungen „A, B und C“. Wechsle zwischen Einzelnennung, Paar und Reihe.",
        ),
    ];
    let punkte: f32 = kennzahlen.iter().map(|k| k.einstufung * k.gewicht).sum();
    let punktzahl = punkte.round().clamp(0.0, 100.0) as u8;
    let urteil = match punktzahl {
        70.. => Urteil::Lebendig,
        52..=69 => Urteil::Ueberwiegend,
        36..=51 => Urteil::Gleichfoermig,
        _ => Urteil::Steril,
    };

    Stilprofil {
        woerter: n_w,
        saetze: n_s,
        satzlaengen,
        satzlaenge_mittel: satz_mittel,
        kurz_anteil: kurz,
        lang_anteil: lang,
        satzanfaenge: anfangsliste.into_iter().take(5).collect(),
        floskeln,
        kennzahlen,
        punktzahl,
        urteil,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gleichförmig, voller Füllformeln und Brücken, ohne persönliche Stimme.
    const MASCHINELL: &str = "In der heutigen digitalen Welt spielt Rust eine entscheidende Rolle bei der Entwicklung sicherer Systeme. \
Es ist wichtig zu beachten, dass die Speicherverwaltung ein zentraler Aspekt der Programmiersprache darstellt. \
Darüber hinaus ermöglicht es das Ownership-Modell, Fehler bereits zur Übersetzungszeit zu erkennen und zu vermeiden. \
Zudem bietet die Sprache eine umfassende Standardbibliothek für verschiedene Anwendungsfälle und Plattformen. \
Somit lässt sich festhalten, dass Rust eine nahtlose Integration in bestehende Entwicklungsprozesse ermöglicht. \
Außerdem trägt das Typsystem maßgeblich zur Zuverlässigkeit der entwickelten Anwendungen in der Praxis bei. \
Des Weiteren spielt der Compiler eine wichtige Rolle bei der Analyse des geschriebenen Quelltextes und der Typen. \
Dabei werden sowohl Typen als auch Lebensdauern der verwendeten Referenzen umfassend und gründlich geprüft. \
Zusammenfassend lässt sich sagen, dass Rust nicht nur sicher, sondern auch performant und ausdrucksstark ist. \
\n\nIn diesem Zusammenhang ist es wichtig zu beachten, dass die Lernkurve für Einsteiger zunächst steil erscheinen kann. \
Dennoch ermöglicht es die Sprache, komplexe Probleme auf eine strukturierte und nachvollziehbare Weise zu lösen. \
Zudem bietet die Community eine Vielzahl von Ressourcen, Werkzeugen und Bibliotheken für Entwickler jeder Erfahrungsstufe. \
Darüber hinaus spielt Cargo eine zentrale Rolle bei der Verwaltung von Abhängigkeiten, Builds und Tests. \
Somit stellt Rust eine ganzheitliche Lösung für moderne Softwareprojekte dar, die höchsten Ansprüchen genügt. \
Außerdem ermöglicht es die Sprache, nebenläufige Programme sicher, effizient und ohne Datenwettläufe zu schreiben. \
Abschließend lässt sich festhalten, dass Rust ein Meilenstein in der Landschaft der Systemprogrammierung ist. \
Es ist wichtig zu beachten, dass jede Technologie ihre Grenzen hat und sorgfältig bewertet werden sollte. \
Dabei sollten sowohl die Anforderungen als auch die Rahmenbedingungen des jeweiligen Projekts berücksichtigt werden.";

    /// Abwechslungsreich, persönlich, mit kurzen und langen Sätzen, Fragen und Ansprache.
    const MENSCHLICH: &str = "Ich erinnere mich noch genau an meinen ersten Borrow-Checker-Fehler. Drei Stunden. So lange habe ich damals gebraucht, \
um zu begreifen, was der Compiler mir sagen wollte, und am Ende lag es an einer einzigen Zeile, die ich dreimal gelesen und nie gesehen hatte. \
Kennst du das? Du starrst auf den Bildschirm, und er starrt zurück.\n\n\
Das Schöne daran: Der Fehler ist ehrlich. Rust sagt dir nicht „irgendwas stimmt nicht“, sondern zeigt auf die Stelle, nennt den Grund und schlägt oft sogar vor, \
wie du es beheben kannst. Das hat mich zuerst genervt. Dann hat es mich beeindruckt.\n\n\
Warum erzähle ich das? Weil du mit genau diesem Gefühl rechnen solltest. Am Anfang kämpfst du gegen den Compiler. Nach ein paar Wochen merkst du, \
dass er auf deiner Seite steht, und plötzlich schreibst du Code, den du Monate später noch verstehst, ohne dass dir jemand erklären muss, was du dir dabei gedacht hast.\n\n\
Probier es aus. Schreib absichtlich falschen Code! Lies die Meldung langsam, von oben nach unten, und frag dich bei jedem Satz, was er bedeutet. \
Wir gehen das gleich gemeinsam durch, Schritt für Schritt, an einem kleinen Beispiel, das du in fünf Minuten nachbauen kannst. \
Los geht's?\n\n\
Zuerst legst du ein neues Projekt an. Ein Befehl genügt. Dann öffnest du die Datei und löschst alles. Alles. Ja, wirklich.";

    #[test]
    fn unterscheidet_gleichfoermigen_von_lebendigem_text() {
        let m = analysieren(MASCHINELL);
        let h = analysieren(MENSCHLICH);
        assert!(m.punktzahl <= 40, "maschineller Text zu gut bewertet: {} ({:?})", m.punktzahl, m.kennzahlen.iter().map(|k| (k.name, k.einstufung)).collect::<Vec<_>>());
        assert!(h.punktzahl >= 70, "lebendiger Text zu schlecht bewertet: {} ({:?})", h.punktzahl, h.kennzahlen.iter().map(|k| (k.name, k.einstufung)).collect::<Vec<_>>());
        assert!(h.punktzahl > m.punktzahl + 30);
        assert!(matches!(m.urteil, Urteil::Steril | Urteil::Gleichfoermig));
        assert_eq!(h.urteil, Urteil::Lebendig);
    }

    #[test]
    fn floskeln_werden_gefunden_und_nicht_doppelt_gezaehlt() {
        let p = analysieren(MASCHINELL);
        let von = |n: &str| p.floskeln.iter().find(|f| f.0 == n).map(|f| f.1);
        assert_eq!(von("es ist wichtig zu beachten"), Some(2));
        assert_eq!(von("zusammenfassend lässt sich sagen"), Some(1));
        assert!(von("zusammenfassend").is_none(), "längere Wendung darf nicht zusätzlich als Kurzform zählen");
        assert!(von("nicht nur … sondern auch").is_some());
        assert!(analysieren(MENSCHLICH).floskeln.is_empty());
    }

    #[test]
    fn code_ueberschriften_und_listen_zaehlen_nicht() {
        let mit = format!("# Titel\n\n```rust\nfn main() {{ println!(\"Außerdem zudem somit\"); }}\n```\n\n- Außerdem zudem\n- somit\n\n{MENSCHLICH}");
        let ohne = analysieren(MENSCHLICH);
        let p = analysieren(&mit);
        assert_eq!(p.woerter, ohne.woerter);
        assert_eq!(p.punktzahl, ohne.punktzahl);
    }

    #[test]
    fn satzteilung_beachtet_abkuerzungen_und_zahlen() {
        let s = saetze("Das gilt z. B. für Rust 1.80 und ca. 20 weitere Fälle. Wirklich? Ja! Dr. Müller sagt es auch.");
        assert_eq!(s.len(), 4, "{s:?}");
        assert!(s[0].ends_with("Fälle."));
    }

    #[test]
    fn zu_wenig_text() {
        let p = analysieren("Nur ein kurzer Satz.");
        assert_eq!(p.urteil, Urteil::ZuWenigText);
        assert!(p.kennzahlen.is_empty());
    }

    #[test]
    fn dreiklang_erkennung() {
        assert!(hat_dreiklang("Wir brauchen Mut, Zeit und Geduld."));
        assert!(!hat_dreiklang("Wir brauchen Mut und Zeit."));
        assert!(!hat_dreiklang("Das ist, wie gesagt, nicht einfach."));
    }
}
