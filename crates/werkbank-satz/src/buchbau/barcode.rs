//! EAN-13 (Bookland) aus einer ISBN-13: Balkenmuster und Prüfziffer.

const L: [&str; 10] = ["0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011", "0110111", "0001011"];
const PARITAET: [&str; 10] = ["LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL", "LGGLGL"];

/// Die 13 Ziffern einer gültigen ISBN-13 (Bindestriche und Leerzeichen erlaubt, Präfix 978/979, Prüfziffer stimmt).
pub fn isbn_ziffern(isbn: &str) -> Option<[u8; 13]> {
    let z: Vec<u8> = isbn.chars().filter(|c| !matches!(c, '-' | ' ')).map(|c| c.to_digit(10).map(|d| d as u8)).collect::<Option<_>>()?;
    let z: [u8; 13] = z.try_into().ok()?;
    if !(z[..3] == [9, 7, 8] || z[..3] == [9, 7, 9]) {
        return None;
    }
    let summe: u32 = z[..12].iter().enumerate().map(|(i, d)| *d as u32 * if i % 2 == 0 { 1 } else { 3 }).sum();
    ((10 - summe % 10) % 10 == z[12] as u32).then_some(z)
}

/// G-Code einer Ziffer: Spiegelbild des R-Codes (also des Komplements vom L-Code).
fn g_code(d: u8) -> Vec<bool> {
    L[d as usize].chars().rev().map(|c| c == '0').collect()
}

/// Die 95 Module des EAN-13 (`true` = Balken): Randzeichen, 6 + 6 Ziffern, Mittelzeichen.
pub fn module(z: &[u8; 13]) -> Vec<bool> {
    let bits = |s: &str| s.chars().map(|c| c == '1').collect::<Vec<_>>();
    let mut m = bits("101");
    for (i, d) in z[1..7].iter().enumerate() {
        let l = L[*d as usize];
        if PARITAET[z[0] as usize].as_bytes()[i] == b'L' {
            m.extend(bits(l));
        } else {
            m.extend(g_code(*d));
        }
    }
    m.extend(bits("01010"));
    for d in &z[7..] {
        m.extend(bits(L[*d as usize]).into_iter().map(|b| !b)); // R-Code: Komplement des L-Codes
    }
    m.extend(bits("101"));
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(m: &[bool]) -> String {
        m.iter().map(|b| if *b { '1' } else { '0' }).collect()
    }

    #[test]
    fn bekannte_isbn_ist_gueltig_und_falsche_nicht() {
        assert!(isbn_ziffern("978-3-9825412-0-4").is_some());
        assert!(isbn_ziffern("9783982541204").is_some());
        assert!(isbn_ziffern("978-3-9825412-0-5").is_none(), "falsche Prüfziffer");
        assert!(isbn_ziffern("123-4-5678-9012-3").is_none(), "falsches Präfix");
        assert!(isbn_ziffern("978-3-98").is_none());
        assert!(isbn_ziffern("").is_none());
    }

    #[test]
    fn code_tabellen_stimmen_mit_der_ean_spezifikation_ueberein() {
        // G-Codes und Paritätsmuster der Spezifikation (GS1 General Specifications), unabhängig eingetippt
        let g = ["0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001", "0001001", "0010111"];
        for (d, soll) in g.iter().enumerate() {
            assert_eq!(text(&g_code(d as u8)), *soll, "G-Code {d}");
        }
        let paritaet = ["LLLLLL", "LLGLGG", "LLGGLG", "LLGGGL", "LGLLGG", "LGGLLG", "LGGGLL", "LGLGLG", "LGLGGL", "LGGLGL"];
        assert_eq!(PARITAET, paritaet);
        // ISBN beginnt mit 9: linke Hälfte L G G L G L
        let z = isbn_ziffern("978-3-9825412-0-4").unwrap();
        let m = module(&z);
        assert_eq!(text(&m[3..10]), L[7], "7 als L");
        assert_eq!(text(&m[10..17]), text(&g_code(8)), "8 als G");
        assert_eq!(text(&m[17..24]), text(&g_code(3)), "3 als G");
        assert_eq!(text(&m[24..31]), L[9], "9 als L");
        assert_eq!(&text(&m)[45..50], "01010");
    }

    #[test]
    fn jede_ziffer_hat_sieben_module_und_zwei_balken_je_gruppe() {
        let z = isbn_ziffern("978-3-9825412-0-4").unwrap();
        let m = module(&z);
        assert_eq!(m.len(), 95);
        // linke Ziffern beginnen mit 0 und enden mit 1; rechte beginnen mit 1 und enden mit 0
        for i in 0..6 {
            let g = &m[3 + i * 7..10 + i * 7];
            assert!(!g[0] && g[6], "links {i}: {g:?}");
            let r = &m[50 + i * 7..57 + i * 7];
            assert!(r[0] && !r[6], "rechts {i}: {r:?}");
        }
    }
}
