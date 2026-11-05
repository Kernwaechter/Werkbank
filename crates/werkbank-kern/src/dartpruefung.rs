//! # Dart-/Flutter-Prüfer für die Buchbeispiele
//!
//! Alle Dart-Blöcke landen als Dateien in einem gemeinsamen Analyseprojekt
//! (Flutter-Projekt, wenn Flutter installiert ist, sonst reines Dart), das
//! `dart analyze` in einem Durchgang prüft. Das ist um Größenordnungen schneller
//! als ein Analyzer-Start je Block.
//!
//! Buchbeispiele sind meist Ausschnitte. Jeder Block wird deshalb in drei Formen
//! geprüft, die beste zählt:
//! 1. so, wie er dasteht (Klassen, `main`, Funktionen),
//! 2. als Rumpf einer `async`-Funktion (Anweisungen, auch `return Widget(…)`),
//! 3. als Rumpf einer abstrakten `State`-Klasse (Methoden wie `build`, `setState`).
//!
//! Blöcke ohne eigene Importe übernehmen die Importe ihres Kapitels; hat das
//! Kapitel keine, bekommen sie die üblichen (`dart:async`, `dart:convert`,
//! `dart:math`, unter Flutter `material.dart` und `services.dart`). Pakete aus
//! `package:`-Importen werden per `pub get` eingebunden; was sich nicht auflösen
//! lässt, gilt als übersprungen, ebenso Blöcke mit relativen Importen.
//!
//! Unbekannte Namen (`undefined_identifier`, `undefined_class` …) sind in
//! Buchausschnitten meist Verweise auf Code, der woanders steht oder gar nicht
//! gezeigt wird; solche Blöcke zählen als Ausschnitt, samt Folgefehlern wie
//! `invalid_constant`. Ein Fehler bleibt es, wenn der unbekannte Name einem
//! bekannten Namen des Buches zum Verwechseln ähnlich ist (Tippfehler wie
//! `Colum` statt `Column`), oder bei echten Typ- und API-Fehlern.

use crate::codepruefung::{Beispiel, Ergebnis};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Gilt ein Codeblock als Dart?
pub fn ist_dart(sprache: &str) -> bool {
    matches!(sprache.to_ascii_lowercase().as_str(), "dart" | "flutter")
}

/// Pakete, die das SDK selbst mitbringt.
const SDK_PAKETE: &[&str] = &["flutter", "flutter_test", "flutter_localizations", "flutter_driver", "flutter_web_plugins"];

/// Name des Analyseprojekts (darf in keinem Buchimport vorkommen).
const PROJEKTNAME: &str = "werkbank_beispiele";

/// Gefundene Werkzeuge.
#[derive(Debug, Clone)]
struct Werkzeug {
    dart: PathBuf,
    flutter: Option<PathBuf>,
}

/// Ordner, in denen Flutter/Dart üblicherweise liegen, auch wenn die Desktop-Sitzung sie nicht im PATH hat.
fn suchordner() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Some(root) = std::env::var_os("FLUTTER_ROOT") {
        v.push(PathBuf::from(root).join("bin"));
    }
    v.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        for u in ["flutter/bin", "Development/flutter/bin", "development/flutter/bin", "src/flutter/bin", "snap/flutter/common/flutter/bin", "fvm/default/bin", ".local/bin"] {
            v.push(home.join(u));
        }
    }
    for f in ["/opt/flutter/bin", "/usr/lib/flutter/bin", "/usr/local/flutter/bin", "/usr/lib/dart/bin"] {
        v.push(PathBuf::from(f));
    }
    v
}

fn werkzeug_finden() -> Option<Werkzeug> {
    let endung = if cfg!(windows) { ".bat" } else { "" };
    let exe = if cfg!(windows) { ".exe" } else { "" };
    let ordner = suchordner();
    // Flutter bringt sein eigenes Dart mit; das passt immer zur Flutter-Version
    if let Some(bin) = ordner.iter().find(|d| d.join(format!("flutter{endung}")).is_file()) {
        let dart = [bin.join(format!("dart{endung}")), bin.join(format!("dart{exe}"))].into_iter().find(|p| p.is_file());
        if let Some(dart) = dart {
            return Some(Werkzeug { dart, flutter: Some(bin.join(format!("flutter{endung}"))) });
        }
    }
    ordner
        .iter()
        .map(|d| d.join(format!("dart{exe}")))
        .find(|p| p.is_file())
        .map(|dart| Werkzeug { dart, flutter: None })
}

/// Gefundene Versionen von Dart und Flutter (`None` = nicht gefunden), ohne langsames `flutter --version`.
pub fn versionen() -> (Option<String>, Option<String>) {
    let Some(w) = werkzeug_finden() else { return (None, None) };
    // Flutter legt beide Versionen als Dateien im Cache ab
    let cache = w.flutter.as_ref().and_then(|f| f.parent()).map(|bin| bin.join("cache"));
    let flutter = cache.as_ref().and_then(|c| {
        let json = fs::read_to_string(c.join("flutter.version.json")).ok()?;
        let wert: serde_json::Value = serde_json::from_str(&json).ok()?;
        wert.get("frameworkVersion")?.as_str().map(str::to_string)
    });
    let flutter = flutter.or_else(|| w.flutter.as_ref().map(|_| "installiert".to_string()));
    let dart = cache
        .and_then(|c| fs::read_to_string(c.join("dart-sdk/version")).ok())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .or_else(|| {
            let aus = crate::prozess::ohne_fenster(Command::new(&w.dart).arg("--version")).output().ok()?;
            let text = format!("{}{}", String::from_utf8_lossy(&aus.stdout), String::from_utf8_lossy(&aus.stderr));
            text.split("version:").nth(1)?.split_whitespace().next().map(str::to_string)
        });
    (dart, flutter)
}

/// Eine Importzeile: (`package:`-Name | `dart:` | relativ).
#[derive(Debug, PartialEq)]
enum Import<'a> {
    Paket(&'a str),
    Sdk,
    Relativ,
}

fn import_art(zeile: &str) -> Option<Import<'_>> {
    let t = zeile.trim_start();
    let rest = t.strip_prefix("import ").or_else(|| t.strip_prefix("export "))?;
    let rest = rest.trim_start();
    let quote = rest.chars().next().filter(|c| *c == '\'' || *c == '"')?;
    let uri = rest[1..].split(quote).next()?;
    Some(if let Some(p) = uri.strip_prefix("package:") {
        Import::Paket(p.split('/').next().unwrap_or(p))
    } else if uri.starts_with("dart:") {
        Import::Sdk
    } else {
        Import::Relativ
    })
}

/// Ist die Zeile eine Direktive, die an den Dateianfang gehört?
fn ist_direktive(zeile: &str) -> bool {
    let t = zeile.trim_start();
    import_art(zeile).is_some() || t.starts_with("library ") || t.starts_with("part ")
}

/// Ein für die Analyse aufbereiteter Block.
struct Vorbereitet {
    /// Dateien je Form: (Dateiname, Zeilen vor dem Blockcode).
    formen: Vec<(String, usize)>,
}

/// Ist ein Import im Analyseprojekt auflösbar?
fn import_aufloesbar(zeile: &str, flutter: bool, pakete: &BTreeSet<String>) -> bool {
    match import_art(zeile) {
        Some(Import::Sdk) => true,
        Some(Import::Paket(p)) => pakete.contains(p) || flutter && SDK_PAKETE.contains(&p),
        _ => false,
    }
}

/// Importe je Kapitel: Ausschnitte ohne eigene Importe übernehmen die ihres Kapitels
/// (so bekommen Flame-Ausschnitte Flame, Flutter-Ausschnitte Material).
fn kapitel_importe(beispiele: &[&Beispiel], flutter: bool, pakete: &BTreeSet<String>) -> HashMap<String, Vec<String>> {
    let mut m: HashMap<String, BTreeSet<String>> = HashMap::new();
    for b in beispiele {
        for z in b.code.lines().filter(|z| import_art(z).is_some() && import_aufloesbar(z, flutter, pakete)) {
            m.entry(b.kapitel_id.clone()).or_default().insert(z.trim().to_string());
        }
    }
    m.into_iter().map(|(k, v)| (k, v.into_iter().collect())).collect()
}

fn vorbereiten(
    bsp: &Beispiel,
    nr: usize,
    flutter: bool,
    pakete: &BTreeSet<String>,
    kapitel: &[String],
) -> Result<(Vorbereitet, Vec<(String, String)>), Ergebnis> {
    let mut direktiven = Vec::new();
    let mut rest = Vec::new();
    for z in bsp.code.lines() {
        if ist_direktive(z) {
            match import_art(z) {
                Some(Import::Relativ) => {
                    return Err(Ergebnis::Uebersprungen("importiert andere Dateien des Projekts".to_string()));
                }
                Some(Import::Paket(p)) if !SDK_PAKETE.contains(&p) && !pakete.contains(p) => {
                    return Err(Ergebnis::Uebersprungen(format!("braucht das Paket „{p}“")));
                }
                Some(Import::Paket(p)) if SDK_PAKETE.contains(&p) && !flutter => {
                    return Err(Ergebnis::Uebersprungen("braucht Flutter (nur Dart gefunden)".to_string()));
                }
                _ => {}
            }
            // `part of` / `library` verhindern das Einbetten und sind für die Prüfung unnötig
            if import_art(z).is_some() {
                direktiven.push(z.trim().to_string());
            }
        } else {
            rest.push(z);
        }
    }
    if direktiven.is_empty() && !kapitel.is_empty() {
        direktiven.extend(kapitel.iter().cloned());
    } else if direktiven.is_empty() {
        direktiven.extend(["import 'dart:async';", "import 'dart:convert';", "import 'dart:math';"].map(String::from));
        if flutter {
            direktiven.extend(["import 'package:flutter/material.dart';", "import 'package:flutter/services.dart';"].map(String::from));
        }
    }
    let kopf = format!("// ignore_for_file: type=lint\n{}\n", direktiven.join("\n"));
    let kopf_zeilen = kopf.lines().count();
    let rumpf = rest.join("\n");

    let klasse = if flutter {
        "abstract class _WerkbankBeispiel extends State<StatefulWidget> {"
    } else {
        "abstract class _WerkbankBeispiel {"
    };
    let formen = [
        (format!("{kopf}{rumpf}\n"), kopf_zeilen),
        // `dynamic`: erlaubt `return;` ebenso wie `return Widget(…);`
        (format!("{kopf}dynamic _werkbankBeispiel() async {{\n{rumpf}\n}}\n"), kopf_zeilen + 1),
        (format!("{kopf}{klasse}\n{rumpf}\n}}\n"), kopf_zeilen + 1),
    ];
    let mut dateien = Vec::new();
    let mut v = Vorbereitet { formen: Vec::new() };
    for (i, (inhalt, versatz)) in formen.into_iter().enumerate() {
        let name = format!("b{nr:05}_{i}.dart");
        dateien.push((name.clone(), inhalt));
        v.formen.push((name, versatz));
    }
    Ok((v, dateien))
}

/// Eine Meldung des Analyzers.
#[derive(Debug, Clone)]
struct Meldung {
    syntax: bool,
    code: String,
    zeile: usize,
    text: String,
}

/// Fehlerarten, die bei Ausschnitten durch Verweise auf Code außerhalb des Blocks entstehen.
const KONTEXT_CODES: &[&str] = &[
    "undefined_identifier",
    "undefined_class",
    "undefined_function",
    "undefined_method",
    "undefined_getter",
    "undefined_setter",
    "undefined_prefixed_name",
    "undefined_annotation",
    "non_type_as_type_argument",
    "creation_with_non_type",
    "not_a_type",
];

/// Fehler, die meist nur aus einem unbekannten Namen folgen (z. B. `const Text(unbekannt)`).
const FOLGE_CODES: &[&str] = &[
    "invalid_constant",
    "const_initialized_with_non_constant_value",
    "non_constant_list_element",
    "non_constant_map_value",
    "const_with_non_constant_argument",
    "super_in_invalid_context",
    "extends_non_class",
    "implements_non_class",
    "mixin_with_non_class_superclass",
    "non_type_in_catch_clause",
    "cast_to_non_type",
    "non_abstract_class_inherits_abstract_member",
    "undefined_super_member",
    "mixin_of_non_class",
    "non_constant_type_argument",
    "ambiguous_extension_member_access",
];

/// Levenshtein-Abstand, bricht ab, sobald er `grenze` überschreitet.
fn abstand(a: &str, b: &str, grenze: usize) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > grenze {
        return grenze + 1;
    }
    let mut vorher: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut jetzt = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            jetzt[j + 1] = (vorher[j] + usize::from(ca != cb)).min(vorher[j + 1] + 1).min(jetzt[j] + 1);
        }
        if jetzt.iter().min().copied().unwrap_or(0) > grenze {
            return grenze + 1;
        }
        vorher = jetzt;
    }
    vorher[b.len()]
}

/// Ein bekannter Name aus dem Buch, der einem unbekannten zum Verwechseln ähnlich ist.
fn aehnlicher_name(name: &str, index: &HashMap<String, usize>) -> Option<String> {
    if name.chars().count() < 5 {
        return None;
    }
    // Varianten desselben Namens (`_wert`/`wert`, `World`/`world`) sind keine Tippfehler
    let grund = |s: &str| s.trim_start_matches('_').to_lowercase();
    let grenze = 1;
    index
        .keys()
        .filter(|k| k.as_str() != name && k.chars().count() >= 5 && grund(k) != grund(name))
        .map(|k| (abstand(name, k, grenze), k))
        .filter(|(d, _)| *d <= grenze)
        .min_by_key(|(d, k)| (*d, std::cmp::Reverse(index[*k]), (*k).clone()))
        .map(|(_, k)| k.clone())
}

/// Der erste Name in Hochkommas einer Analyzer-Meldung, z. B. `Undefined name 'zaehler'.` → `zaehler`.
fn genannter_name(text: &str) -> Option<&str> {
    let start = text.find('\'')? + 1;
    let laenge = text[start..].find('\'')?;
    Some(&text[start..start + laenge])
}

/// Bezeichner eines Codeblocks (ohne Wiederholungen).
fn bezeichner(code: &str) -> std::collections::HashSet<&str> {
    code.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
        .filter(|w| w.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$'))
        .collect()
}

/// Zählt, in wie vielen Blöcken jeder Bezeichner vorkommt.
fn bezeichner_index(beispiele: &[&Beispiel]) -> HashMap<String, usize> {
    let mut index: HashMap<String, usize> = HashMap::new();
    for b in beispiele {
        for w in bezeichner(&b.code) {
            *index.entry(w.to_string()).or_default() += 1;
        }
    }
    index
}

/// Liest `dart analyze --format=machine`: `SCHWERE|ART|CODE|DATEI|ZEILE|SPALTE|LÄNGE|TEXT`.
fn meldungen_lesen(ausgabe: &str) -> HashMap<String, Vec<Meldung>> {
    let mut m: HashMap<String, Vec<Meldung>> = HashMap::new();
    for z in ausgabe.lines() {
        let teile: Vec<&str> = z.splitn(8, '|').collect();
        if teile.len() < 8 || teile[0] != "ERROR" {
            continue;
        }
        let datei = Path::new(teile[3]).file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
        m.entry(datei).or_default().push(Meldung {
            syntax: teile[1] == "SYNTACTIC_ERROR",
            code: teile[2].to_ascii_lowercase(),
            zeile: teile[4].parse().unwrap_or(0),
            text: teile[7].replace("\\|", "|").replace("\\\\", "\\"),
        });
    }
    m
}

fn bewerten(bsp: &Beispiel, v: &Vorbereitet, meldungen: &HashMap<String, Vec<Meldung>>, index: &HashMap<String, usize>) -> Ergebnis {
    let info = bsp.info.to_lowercase();
    let erwartet_fehler = info.contains("compile_fail");
    let leer = Vec::new();
    let formen: Vec<(&Vec<Meldung>, usize)> = v.formen.iter().map(|(n, versatz)| (meldungen.get(n).unwrap_or(&leer), *versatz)).collect();

    if formen.iter().any(|(m, _)| m.is_empty()) {
        return if erwartet_fehler {
            Ergebnis::Fehler("Sollte nicht kompilieren (compile_fail), ist aber fehlerfrei".to_string())
        } else {
            Ergebnis::Ok
        };
    }
    if erwartet_fehler {
        return Ergebnis::Ok;
    }
    // Beste Form: ohne Syntaxfehler, dann möglichst wenige Fehler; bei Gleichstand die frühere Form
    let (beste, versatz) = formen
        .iter()
        .min_by_key(|(m, _)| (m.iter().any(|x| x.syntax), m.len()))
        .copied()
        .unwrap_or((&leer, 0));
    // Unbekannte Namen: Verweis auf Code außerhalb des Blocks, außer sie sehen nach Tippfehler aus
    let eigene = bezeichner(&bsp.code);
    let tippfehler = |m: &Meldung| -> Option<String> {
        if !KONTEXT_CODES.contains(&m.code.as_str()) {
            return None;
        }
        let n = genannter_name(&m.text)?;
        let n = n.split('.').next_back().unwrap_or(n);
        if index.get(n).copied().unwrap_or(0) > usize::from(eigene.contains(n)) {
            return None; // steht in einem anderen Block
        }
        aehnlicher_name(n, index)
    };
    let kontext = |m: &Meldung| KONTEXT_CODES.contains(&m.code.as_str()) && tippfehler(m).is_none();
    let hat_kontext = beste.iter().any(kontext);
    // Folgefehler fehlender Namen (z. B. `const` mit unbekanntem Wert) zählen nur mit, wenn es solche gibt
    // … ebenso Fehler in einer Zeile, die selbst einen unbekannten Namen enthält
    let kontext_zeilen: std::collections::HashSet<usize> = beste.iter().filter(|m| kontext(m)).map(|m| m.zeile).collect();
    let erklaert = |m: &Meldung| {
        kontext(m) || hat_kontext && FOLGE_CODES.contains(&m.code.as_str()) || kontext_zeilen.contains(&m.zeile)
    };
    let syntax = beste.iter().any(|m| m.syntax);
    // Echte Fehler zuerst zeigen
    let mut sortiert: Vec<&Meldung> = beste.iter().collect();
    sortiert.sort_by_key(|m| erklaert(m));
    let text = sortiert
        .iter()
        .take(3)
        .map(|m| {
            let zeile = m.zeile.saturating_sub(versatz);
            let hinweis = tippfehler(m).map(|v| format!(" Tippfehler? Gemeint ist vielleicht „{v}“.")).unwrap_or_default();
            format!("Zeile {zeile}: {}{hinweis} [{}]", m.text, m.code)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let text = if beste.len() > 3 { format!("{text}\n… und {} weitere", beste.len() - 3) } else { text };
    if syntax || beste.iter().all(erklaert) {
        Ergebnis::Ausschnitt(text)
    } else {
        Ergebnis::Fehler(text)
    }
}

/// Paketnamen aus allen `package:`-Importen der Blöcke (ohne SDK-Pakete).
fn gebrauchte_pakete(beispiele: &[&Beispiel]) -> BTreeSet<String> {
    beispiele
        .iter()
        .flat_map(|b| b.code.lines())
        .filter_map(import_art)
        .filter_map(|i| match i {
            Import::Paket(p) if !SDK_PAKETE.contains(&p) && p != PROJEKTNAME => Some(p.to_string()),
            _ => None,
        })
        .filter(|p| p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'))
        .collect()
}

/// Pakete, die schon im lokalen Pub-Cache liegen (für `pub get --offline`).
fn im_pub_cache(pakete: &BTreeSet<String>) -> BTreeSet<String> {
    let cache = std::env::var_os("PUB_CACHE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".pub-cache")));
    let Some(ordner) = cache.map(|c| c.join("hosted/pub.dev")) else { return BTreeSet::new() };
    let vorhanden: BTreeSet<String> = fs::read_dir(ordner)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.rsplit_once('-')).map(|(n, _)| n.to_string()))
        .collect();
    pakete.intersection(&vorhanden).cloned().collect()
}

fn pubspec(flutter: bool, pakete: &BTreeSet<String>) -> String {
    let mut s = format!("name: {PROJEKTNAME}\npublish_to: none\nenvironment:\n  sdk: \">=3.0.0 <4.0.0\"\ndependencies:\n");
    if flutter {
        s.push_str("  flutter:\n    sdk: flutter\n  flutter_test:\n    sdk: flutter\n");
    }
    for p in pakete {
        s.push_str(&format!("  {p}: any\n"));
    }
    s
}

/// Führt einen Befehl mit Zeitlimit aus; liefert (Erfolg, stdout+stderr).
fn ausfuehren(mut befehl: Command, limit: Duration) -> Result<(bool, String), String> {
    let mut kind = befehl
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    // Ausgaben in Threads lesen, damit volle Puffer den Prozess nicht blockieren
    let mut out = kind.stdout.take();
    let mut err = kind.stderr.take();
    let t_out = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(o) = out.as_mut() {
            let _ = o.read_to_string(&mut s);
        }
        s
    });
    let t_err = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(e) = err.as_mut() {
            let _ = e.read_to_string(&mut s);
        }
        s
    });
    let start = Instant::now();
    let status = loop {
        match kind.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() > limit => {
                let _ = kind.kill();
                let _ = kind.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break None,
        }
    };
    let text = format!("{}{}", t_out.join().unwrap_or_default(), t_err.join().unwrap_or_default());
    match status {
        Some(s) => Ok((s.success(), text)),
        None => Err(format!("Zeitüberschreitung nach {} s", limit.as_secs())),
    }
}

/// Legt das Analyseprojekt an und löst die Pakete auf. Liefert die eingebundenen Pakete.
fn projekt_einrichten(w: &Werkzeug, ordner: &Path, gewuenscht: &BTreeSet<String>) -> Result<BTreeSet<String>, String> {
    fs::create_dir_all(ordner.join("lib")).map_err(|e| e.to_string())?;
    fs::write(ordner.join("analysis_options.yaml"), "analyzer:\n  exclude: [build/**, .dart_tool/**]\n").map_err(|e| e.to_string())?;
    let flutter = w.flutter.is_some();
    let offline = im_pub_cache(gewuenscht);
    let versuche: [(BTreeSet<String>, bool); 3] = [(gewuenscht.clone(), false), (offline, true), (BTreeSet::new(), true)];
    let mut letzter_fehler = String::new();
    for (pakete, ohne_netz) in versuche {
        let inhalt = pubspec(flutter, &pakete);
        let pfad = ordner.join("pubspec.yaml");
        let aufgeloest = ordner.join(".dart_tool/package_config.json");
        // Unveränderte pubspec und vorhandene Auflösung: kein erneutes `pub get`
        if fs::read_to_string(&pfad).ok().as_deref() == Some(inhalt.as_str()) && aufgeloest.is_file() {
            return Ok(pakete);
        }
        fs::write(&pfad, &inhalt).map_err(|e| e.to_string())?;
        let mut befehl = Command::new(w.flutter.as_ref().unwrap_or(&w.dart));
        crate::prozess::ohne_fenster(&mut befehl);
        befehl.current_dir(ordner).args(["pub", "get"]);
        if ohne_netz {
            befehl.arg("--offline");
        }
        match ausfuehren(befehl, Duration::from_secs(240)) {
            Ok((true, _)) => return Ok(pakete),
            Ok((false, t)) | Err(t) => letzter_fehler = t,
        }
    }
    Err(format!("pub get fehlgeschlagen: {}", letzter_fehler.lines().rev().find(|z| !z.trim().is_empty()).unwrap_or("")))
}

/// Prüft alle Dart-Beispiele in einem Durchgang. `melden(index, ergebnis)` je Block, Indizes wie in `beispiele`.
pub fn alle_pruefen(beispiele: &[(usize, &Beispiel)], arbeitsordner: &Path, melden: &(impl Fn(usize, Ergebnis) + Sync)) {
    if beispiele.is_empty() {
        return;
    }
    let alle_melden = |e: Ergebnis| {
        for (i, _) in beispiele {
            melden(*i, e.clone());
        }
    };
    let Some(w) = werkzeug_finden() else {
        alle_melden(Ergebnis::Uebersprungen("Dart/Flutter nicht gefunden (flutter bzw. dart im PATH oder FLUTTER_ROOT)".to_string()));
        return;
    };
    let ordner = arbeitsordner.join("dart-projekt");
    let liste: Vec<&Beispiel> = beispiele.iter().map(|(_, b)| *b).collect();
    let pakete = match projekt_einrichten(&w, &ordner, &gebrauchte_pakete(&liste)) {
        Ok(p) => p,
        Err(e) => {
            alle_melden(Ergebnis::Uebersprungen(e));
            return;
        }
    };

    // Alte Beispieldateien weg, neue schreiben
    let lib = ordner.join("lib");
    for e in fs::read_dir(&lib).into_iter().flatten().flatten() {
        let _ = fs::remove_file(e.path());
    }
    let flutter = w.flutter.is_some();
    let importe = kapitel_importe(&liste, flutter, &pakete);
    let mut vorbereitet: Vec<(usize, &Beispiel, Option<Vorbereitet>)> = Vec::new();
    for (nr, (i, bsp)) in beispiele.iter().enumerate() {
        if bsp.info.to_lowercase().contains("ignore") {
            melden(*i, Ergebnis::Uebersprungen("als „ignore“ markiert".to_string()));
            continue;
        }
        let kapitel = importe.get(&bsp.kapitel_id).map(Vec::as_slice).unwrap_or_default();
        match vorbereiten(bsp, nr, flutter, &pakete, kapitel) {
            Ok((v, dateien)) => {
                for (name, inhalt) in dateien {
                    if let Err(e) = fs::write(lib.join(&name), inhalt) {
                        melden(*i, Ergebnis::Uebersprungen(format!("Datei nicht schreibbar: {e}")));
                    }
                }
                vorbereitet.push((*i, bsp, Some(v)));
            }
            Err(e) => melden(*i, e),
        }
    }
    if vorbereitet.is_empty() {
        return;
    }

    let mut befehl = Command::new(&w.dart);
    crate::prozess::ohne_fenster(&mut befehl);
    befehl.current_dir(&ordner).args(["analyze", "--format=machine", "lib"]);
    let ausgabe = match ausfuehren(befehl, Duration::from_secs(900)) {
        Ok((_, t)) => t,
        Err(e) => {
            for (i, _, _) in &vorbereitet {
                melden(*i, Ergebnis::Uebersprungen(format!("dart analyze: {e}")));
            }
            return;
        }
    };
    let meldungen = meldungen_lesen(&ausgabe);
    let index = bezeichner_index(&liste);
    for (i, bsp, v) in vorbereitet {
        if let Some(v) = v {
            melden(i, bewerten(bsp, &v, &meldungen, &index));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bsp(code: &str, info: &str) -> Beispiel {
        Beispiel {
            kapitel_id: "k".into(),
            kapitel_titel: "K".into(),
            zeile: 1,
            sprache: "dart".into(),
            info: info.into(),
            code: code.into(),
        }
    }

    #[test]
    fn tippfehler_erkennen() {
        let index = HashMap::from([("Column".to_string(), 9), ("zaehler".to_string(), 2), ("Text".to_string(), 9)]);
        assert_eq!(aehnlicher_name("Colum", &index), Some("Column".to_string()));
        assert_eq!(aehnlicher_name("zaeler", &index), Some("zaehler".to_string()));
        assert_eq!(aehnlicher_name("_suchController", &index), None);
        assert_eq!(aehnlicher_name("Tex", &index), None, "zu kurz");
        let index = HashMap::from([("hoverIndex".to_string(), 3), ("world".to_string(), 3)]);
        assert_eq!(aehnlicher_name("_hoverIndex", &index), None, "nur Unterstrich anders");
        assert_eq!(aehnlicher_name("World", &index), None, "nur Großschreibung anders");
    }

    #[test]
    fn namen_aus_meldungen() {
        assert_eq!(genannter_name("Undefined name 'zaehler'."), Some("zaehler"));
        assert_eq!(genannter_name("The function 'baue' isn't defined."), Some("baue"));
        assert_eq!(genannter_name("ohne"), None);
    }

    #[test]
    fn versionen_lesen() {
        let (dart, flutter) = versionen();
        if werkzeug_finden().is_some() {
            assert!(dart.as_deref().is_some_and(|v| v.chars().next().is_some_and(|c| c.is_ascii_digit())), "{dart:?}");
        }
        eprintln!("Dart {dart:?}, Flutter {flutter:?}");
    }

    #[test]
    fn importe_erkennen() {
        assert_eq!(import_art("import 'package:flame/game.dart';"), Some(Import::Paket("flame")));
        assert_eq!(import_art("import \"dart:math\" as m;"), Some(Import::Sdk));
        assert_eq!(import_art("import '../models/snack.dart';"), Some(Import::Relativ));
        assert_eq!(import_art("final x = 1;"), None);
    }

    #[test]
    fn maschinenformat_lesen() {
        let m = meldungen_lesen(
            "ERROR|SYNTACTIC_ERROR|EXPECTED_TOKEN|/t/lib/b00001_0.dart|7|3|1|Expected to find ';'.\n\
             INFO|LINT|X|/t/lib/b00001_0.dart|1|1|1|egal\n\
             ERROR|COMPILE_TIME_ERROR|UNDEFINED_IDENTIFIER|/t/lib/b00001_1.dart|5|3|1|Undefined name 'a\\|b'.",
        );
        assert_eq!(m["b00001_0.dart"].len(), 1);
        assert!(m["b00001_0.dart"][0].syntax);
        assert_eq!(m["b00001_1.dart"][0].text, "Undefined name 'a|b'.");
    }

    #[test]
    fn bewertung_waehlt_beste_form() {
        let b = bsp("print('x');", "dart");
        let (v, _) = vorbereiten(&b, 1, true, &BTreeSet::new(), &[]).ok().unwrap();
        let mut m = HashMap::new();
        m.insert(v.formen[0].0.clone(), vec![Meldung { syntax: true, code: "x".into(), zeile: 7, text: "t".into() }]);
        let leer = HashMap::new();
        assert_eq!(bewerten(&b, &v, &m, &leer), Ergebnis::Ok, "Funktionsform ist fehlerfrei");
        m.insert(v.formen[1].0.clone(), vec![Meldung { syntax: false, code: "undefined_identifier".into(), zeile: 8, text: "Undefined name 'zaehler'.".into() }]);
        m.insert(v.formen[2].0.clone(), vec![Meldung { syntax: true, code: "y".into(), zeile: 8, text: "t".into() }]);
        assert!(matches!(bewerten(&b, &v, &m, &leer), Ergebnis::Ausschnitt(_)), "Name aus ungezeigtem Code: Ausschnitt");
        let index = HashMap::from([("zaehler".to_string(), 1)]);
        assert!(matches!(bewerten(&b, &v, &m, &index), Ergebnis::Ausschnitt(_)), "Name aus anderem Block: Ausschnitt");
        let index = HashMap::from([("zaehlerX".to_string(), 3)]);
        assert!(
            matches!(bewerten(&b, &v, &m, &index), Ergebnis::Fehler(t) if t.starts_with("Zeile 1:") && t.contains("zaehlerX")),
            "ähnlicher bekannter Name: Tippfehler"
        );
    }

    #[test]
    fn fehlende_pakete_und_relative_importe_ueberspringen() {
        let leer = BTreeSet::new();
        assert!(matches!(vorbereiten(&bsp("import 'package:flame/game.dart';", "dart"), 0, true, &leer, &[]), Err(Ergebnis::Uebersprungen(_))));
        assert!(matches!(vorbereiten(&bsp("import '../a.dart';", "dart"), 0, true, &leer, &[]), Err(Ergebnis::Uebersprungen(_))));
        assert!(vorbereiten(&bsp("import 'package:flutter/material.dart';", "dart"), 0, true, &leer, &[]).is_ok());
    }

    #[test]
    fn echte_analyse() {
        if werkzeug_finden().is_none() {
            return;
        }
        let liste = [
            bsp("final zahl = 3;\nprint(zahl * 2);", "dart"),
            bsp("int x = 'text';", "dart"),
            bsp("class A {\n  void f() {}\n}", "dart"),
            bsp("int x = 'text';", "dart,compile_fail"),
            bsp("child: Text('a'),", "dart"),
        ];
        let mit_index: Vec<(usize, &Beispiel)> = liste.iter().enumerate().collect();
        let erg = std::sync::Mutex::new(vec![None; liste.len()]);
        let ordner = std::env::temp_dir().join("werkbank-test-dart");
        alle_pruefen(&mit_index, &ordner, &|i, e| erg.lock().unwrap()[i] = Some(e));
        let erg = erg.into_inner().unwrap();
        assert_eq!(erg[0], Some(Ergebnis::Ok), "{erg:?}");
        assert!(matches!(&erg[1], Some(Ergebnis::Fehler(t)) if t.contains("String")), "{erg:?}");
        assert_eq!(erg[2], Some(Ergebnis::Ok), "{erg:?}");
        assert_eq!(erg[3], Some(Ergebnis::Ok), "{erg:?}");
        assert!(matches!(&erg[4], Some(Ergebnis::Ausschnitt(_))), "{erg:?}");
    }
}
