//! Farbvorschlag für das Cover aus einem vorhandenen Bild: Hintergrund, Akzent, Textfarbe und Rücken
//! passend zum Bild (statt Schwarz), mit lesbarem Kontrast (WCAG).

/// Vorgeschlagene Farben (`#rrggbb`).
#[derive(Debug, Clone, PartialEq)]
pub struct Vorschlag {
    pub hintergrund: String,
    pub text: String,
    pub akzent: String,
    /// Farbe und Deckkraft des Kastens auf dem Rücken.
    pub ruecken: String,
    pub ruecken_deckkraft: f32,
    /// Farbe und Deckkraft des Kastens hinter dem Rückseitentext.
    pub rueckseite_kasten: String,
    pub rueckseite_kasten_deckkraft: f32,
}

/// Farbton (0–360), Sättigung und Helligkeit (0–1).
#[derive(Debug, Clone, Copy)]
struct Hsl(f32, f32, f32);

fn zu_hsl(r: f32, g: f32, b: f32) -> Hsl {
    let (r, g, b) = (r / 255.0, g / 255.0, b / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    if (max - min).abs() < 1e-6 {
        return Hsl(0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if (max - r).abs() < 1e-6 {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-6 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    Hsl(h * 60.0, s, l)
}

fn zu_rgb(Hsl(h, s, l): Hsl) -> [f32; 3] {
    let (s, l) = (s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let h6 = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let (r, g, b) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    [(r + m) * 255.0, (g + m) * 255.0, (b + m) * 255.0]
}

fn hex(h: Hsl) -> String {
    let [r, g, b] = zu_rgb(h);
    format!("#{:02x}{:02x}{:02x}", r.round().clamp(0.0, 255.0) as u8, g.round().clamp(0.0, 255.0) as u8, b.round().clamp(0.0, 255.0) as u8)
}

/// Relative Leuchtdichte nach WCAG.
fn leuchtdichte(h: Hsl) -> f32 {
    let lin = |c: f32| {
        let c = c.clamp(0.0, 255.0) / 255.0;
        if c <= 0.03928 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    };
    let [r, g, b] = zu_rgb(h);
    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

/// Kontrastverhältnis zweier Farben (1 bis 21).
pub fn kontrast(a: &str, b: &str) -> f32 {
    let lies = |s: &str| {
        let h = s.trim_start_matches('#');
        let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f32;
        zu_hsl(v(0), v(2), v(4))
    };
    let (la, lb) = (leuchtdichte(lies(a)), leuchtdichte(lies(b)));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Schiebt die Helligkeit von `farbe` in `richtung` (+1 heller, −1 dunkler), bis der Kontrast zu `grund` reicht.
fn kontrast_herstellen(mut farbe: Hsl, grund: Hsl, mindestens: f32, richtung: f32) -> Hsl {
    let k = |f: Hsl| {
        let (a, b) = (leuchtdichte(f), leuchtdichte(grund));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    };
    let mut n = 0;
    while k(farbe) < mindestens && n < 60 {
        farbe.2 = (farbe.2 + 0.02 * richtung).clamp(0.0, 1.0);
        n += 1;
    }
    farbe
}

fn farbton_abstand(a: f32, b: f32) -> f32 {
    let d = (a - b).abs() % 360.0;
    d.min(360.0 - d)
}

/// Schlägt Farben passend zum Bild vor (PNG oder JPEG).
///
/// * Hintergrund: die häufigste Farbe des Bildes; der linke Rand zählt dreifach, weil dort der Rücken ansetzt.
/// * Akzent: die leuchtendste gut sichtbare Farbe des Bildes, auch wenn sie nur klein vorkommt (sonst derselbe Farbton, heller).
/// * Text: fast weiß bzw. fast schwarz im Farbton des Hintergrunds.
/// * Rücken: derselbe Farbton wie der Hintergrund, etwas heller (bei sehr dunklem Grund) oder dunkler, nicht schwarz.
pub fn aus_bild(daten: &[u8]) -> Result<Vorschlag, String> {
    let bild = image::load_from_memory(daten).map_err(|e| format!("Bild nicht lesbar: {e}"))?;
    let klein = bild.thumbnail(400, 400).to_rgba8();
    let (b, h) = klein.dimensions();
    if b == 0 || h == 0 {
        return Err("Bild ist leer".to_string());
    }
    // Farbeimer: 4 Bit je Kanal
    let mut eimer: std::collections::HashMap<(u8, u8, u8), ([f64; 3], f64)> = std::collections::HashMap::new();
    let mut gesamt = 0.0f64;
    for (x, _, p) in klein.enumerate_pixels() {
        if p[3] < 128 {
            continue;
        }
        let gewicht = if (x as f32) < b as f32 * 0.12 { 3.0 } else { 1.0 };
        let e = eimer.entry((p[0] >> 4, p[1] >> 4, p[2] >> 4)).or_insert(([0.0; 3], 0.0));
        for i in 0..3 {
            e.0[i] += p[i] as f64 * gewicht;
        }
        e.1 += gewicht;
        gesamt += gewicht;
    }
    if gesamt == 0.0 {
        return Err("Bild ist durchsichtig".to_string());
    }
    let farben: Vec<(Hsl, f64)> = eimer.values().map(|(s, n)| (zu_hsl((s[0] / n) as f32, (s[1] / n) as f32, (s[2] / n) as f32), n / gesamt)).collect();
    let grund = farben.iter().max_by(|a, b| a.1.total_cmp(&b.1)).map(|f| f.0).ok_or("keine Farben")?;
    let dunkel = leuchtdichte(grund) < 0.3;

    // Akzent: sättigt, ausreichend häufig, gut lesbar auf dem Grund
    let richtung = if dunkel { 1.0 } else { -1.0 };
    let akzent = farben
        .iter()
        .filter(|(f, anteil)| f.1 >= 0.35 && *anteil >= 0.0004 && farbton_abstand(f.0, grund.0) > 12.0 || (f.1 >= 0.35 && *anteil >= 0.02))
        .filter(|(f, _)| (0.25..=0.8).contains(&f.2))
        .map(|(f, anteil)| {
            let helligkeit = 1.0 - (f.2 - if dunkel { 0.62 } else { 0.38 }).abs();
            (*f, f.1 * f.1 * helligkeit * (*anteil as f32).powf(0.3))
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(f, _)| f)
        .unwrap_or(Hsl(grund.0, 0.6, if dunkel { 0.62 } else { 0.35 }));
    let akzent = kontrast_herstellen(Hsl(akzent.0, akzent.1.max(0.35), akzent.2), grund, 4.5, richtung);

    let text = if dunkel { Hsl(grund.0, 0.15f32.min(grund.1), 0.95) } else { Hsl(grund.0, 0.2f32.min(grund.1), 0.1) };
    let text = kontrast_herstellen(text, grund, 7.0, richtung);

    // Rücken: im Farbton des Grundes, nicht schwarz
    let ruecken_l = if grund.2 < 0.18 { grund.2 + 0.05 } else if grund.2 > 0.6 { grund.2 - 0.08 } else { grund.2 - 0.07 };
    let ruecken = Hsl(grund.0, grund.1.max(0.2), ruecken_l.clamp(0.04, 0.96));
    let kasten_l = if dunkel { grund.2 * 0.55 } else { grund.2 + (1.0 - grund.2) * 0.5 };
    let kasten = Hsl(grund.0, grund.1, kasten_l);
    Ok(Vorschlag {
        hintergrund: hex(grund),
        text: hex(text),
        akzent: hex(akzent),
        ruecken: hex(ruecken),
        ruecken_deckkraft: 0.9,
        rueckseite_kasten: hex(kasten),
        rueckseite_kasten_deckkraft: 0.5,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(f: impl Fn(u32, u32) -> [u8; 3]) -> Vec<u8> {
        let mut b = image::RgbImage::new(200, 300);
        for (x, y, p) in b.enumerate_pixels_mut() {
            *p = image::Rgb(f(x, y));
        }
        let mut v = Vec::new();
        image::DynamicImage::ImageRgb8(b).write_to(&mut std::io::Cursor::new(&mut v), image::ImageFormat::Png).unwrap();
        v
    }

    fn hsl_von(h: &str) -> Hsl {
        let v = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap() as f32;
        zu_hsl(v(1), v(3), v(5))
    }

    #[test]
    fn dunkelblaues_cover_bekommt_blauen_grund_blauen_ruecken_und_lesbare_schrift() {
        // Dunkelblauer Grund, große cyanfarbene Fläche, kleiner oranger Streifen
        let v = aus_bild(&png(|x, y| if y < 6 { [245, 158, 11] } else if (60..160).contains(&x) && (100..200).contains(&y) { [56, 189, 248] } else { [11, 26, 51] })).unwrap();
        let (grund, ruecken, akzent) = (hsl_von(&v.hintergrund), hsl_von(&v.ruecken), hsl_von(&v.akzent));
        assert!((200.0..235.0).contains(&grund.0) && grund.2 < 0.15, "dunkles Blau: {v:?}");
        assert!(farbton_abstand(ruecken.0, grund.0) < 8.0 && ruecken.1 > 0.3 && ruecken.2 > 0.06, "Rücken blau statt schwarz: {v:?}");
        assert!(farbton_abstand(akzent.0, 198.0) < 25.0, "Akzent ist das Cyan der großen Fläche: {v:?}");
        assert!(kontrast(&v.text, &v.hintergrund) >= 7.0, "{v:?}");
        assert!(kontrast(&v.akzent, &v.hintergrund) >= 4.5, "{v:?}");
    }

    #[test]
    fn helles_cover_bekommt_dunkle_schrift() {
        let v = aus_bild(&png(|_, _| [244, 236, 216])).unwrap();
        assert!(hsl_von(&v.text).2 < 0.3, "dunkle Schrift auf hellem Grund: {v:?}");
        assert!(kontrast(&v.text, &v.hintergrund) >= 7.0, "{v:?}");
        assert!(kontrast(&v.akzent, &v.hintergrund) >= 4.5, "{v:?}");
    }

    #[test]
    fn grauer_grund_ohne_farbe_liefert_trotzdem_brauchbare_farben() {
        let v = aus_bild(&png(|_, _| [30, 30, 30])).unwrap();
        assert!(kontrast(&v.text, &v.hintergrund) >= 7.0 && kontrast(&v.akzent, &v.hintergrund) >= 4.5, "{v:?}");
    }

    #[test]
    fn rand_links_bestimmt_den_grund_des_rueckens() {
        // Linkes Drittel grün, Rest rot: der Rücken setzt links an (dort zählt der Rand dreifach), also gewinnt Grün
        let v = aus_bild(&png(|x, _| if x < 80 { [20, 90, 40] } else { [140, 30, 30] })).unwrap();
        // Ohne diese Gewichtung läge Rot mit 60 % vorn; mit schmalem Streifen bleibt Rot vorn
        let rot = aus_bild(&png(|x, _| if x < 20 { [20, 90, 40] } else { [140, 30, 30] })).unwrap();
        assert!(hsl_von(&rot.hintergrund).0 < 20.0, "rötlich: {rot:?}");
        let g = hsl_von(&v.hintergrund);
        assert!((90.0..150.0).contains(&g.0), "grünlich: {v:?}");
    }

    #[test]
    fn kein_bild_ist_ein_fehler() {
        assert!(aus_bild(b"kein bild").is_err());
    }
}
