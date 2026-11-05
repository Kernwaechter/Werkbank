//! Stylesheet des Buchs – wörtlich aus `get_css()` des bisherigen `build_book.py` übertragen.
//! Das EPUB verwendet es unverändert; die Typst-Vorlage der PDFs bildet dieselben Werte nach.

/// Farben und Werte je Fassung (farbig bzw. Schwarzweiß/Graustufen für den Druck).
pub struct Palette {
    pub akzent_primaer: &'static str,
    pub akzent_sekundaer: &'static str,
    pub code_grund: &'static str,
    pub code_rand: &'static str,
    pub code_kopf_grund: &'static str,
    pub code_kopf_schrift: &'static str,
}

pub const FARBE: Palette = Palette {
    akzent_primaer: "#1E3A8A",
    akzent_sekundaer: "#0F766E",
    code_grund: "#F8FAFC",
    code_rand: "#CBD5E1",
    code_kopf_grund: "#E2E8F0",
    code_kopf_schrift: "#1E293B",
};

pub const GRAU: Palette = Palette {
    akzent_primaer: "#000000",
    akzent_sekundaer: "#111111",
    code_grund: "#F6F6F6",
    code_rand: "#888888",
    code_kopf_grund: "#E0E0E0",
    code_kopf_schrift: "#000000",
};

const SEITE: &str = r#"
        @page {{
            size: {PAGE_WIDTH_MM}mm {PAGE_HEIGHT_MM}mm;
            margin-top: 20mm;
            margin-bottom: 22mm;
            @bottom-center {{
                content: counter(page);
                font-family: 'Noto Sans', 'Liberation Sans', sans-serif;
                font-size: 9pt;
                color: #333333;
            }}
        }}
        @page :right {{
            margin-left: 22mm;
            margin-right: 18mm;
        }}
        @page :left {{
            margin-left: 18mm;
            margin-right: 22mm;
        }}
        "#;
const PYGMENTS_FARBE: &str = r#"
        .highlight .k, .highlight .kd, .highlight .kn, .highlight .kp, .highlight .kr, .highlight .kt { color: #0550AE; font-weight: bold; }
        .highlight .s, .highlight .s1, .highlight .s2, .highlight .sb, .highlight .sc, .highlight .sd, .highlight .sh, .highlight .sx { color: #0A3069; }
        .highlight .c, .highlight .c1, .highlight .cm, .highlight .cp, .highlight .cs { color: #57606A; font-style: italic; }
        .highlight .nc, .highlight .nf, .highlight .nn { color: #8250DF; font-weight: bold; }
        .highlight .m, .highlight .mi, .highlight .mf, .highlight .mh, .highlight .mo { color: #0550AE; }
        .highlight .o, .highlight .p { color: #24292F; }
        "#;
const RUBRIK_FARBE: &str = r#"
        .rubrik-heading {
            padding: 6px 12px;
            border-left: 4px solid #1E3A8A;
            background-color: #EFF6FF;
            color: #1E3A8A;
            border-radius: 0 4px 4px 0;
        }
        .rubrik-bughunter { border-left-color: #B91C1C; background-color: #FEF2F2; color: #991B1B; }
        .rubrik-aufgabe   { border-left-color: #0F766E; background-color: #F0FDFA; color: #115E59; }
        .rubrik-klartext  { border-left-color: #B45309; background-color: #FFFBEB; color: #92400E; }
        .rubrik-toolbox   { border-left-color: #334155; background-color: #F1F5F9; color: #0F172A; }
        "#;
const PYGMENTS_GRAU: &str = r#"
        .highlight { color: #000000; }
        .highlight .k, .highlight .kd, .highlight .kn, .highlight .kp, .highlight .kr, .highlight .kt { color: #000000; font-weight: bold; }
        .highlight .nc, .highlight .nf, .highlight .nn { color: #000000; font-weight: bold; }
        .highlight .s, .highlight .s1, .highlight .s2, .highlight .sb, .highlight .sc, .highlight .sd, .highlight .sh, .highlight .sx { color: #111111; }
        .highlight .c, .highlight .c1, .highlight .cm, .highlight .cp, .highlight .cs { color: #444444; font-style: italic; }
        .highlight .m, .highlight .mi, .highlight .mf, .highlight .mh, .highlight .mo { color: #000000; font-weight: bold; }
        .highlight .o, .highlight .p { color: #000000; }
        "#;
const RUBRIK_GRAU: &str = r#"
        .rubrik-heading {
            padding: 6px 12px;
            border-left: 4px solid #000000;
            border-top: 1px solid #BBBBBB;
            border-bottom: 1px solid #BBBBBB;
            background-color: #F2F2F2;
            color: #000000;
        }
        "#;
const HAUPT: &str = r#"
    {page_rule}
    body {{
        font-family: 'Noto Serif', 'Liberation Serif', Georgia, serif;
        font-size: 10.5pt;
        line-height: 1.55;
        color: #111111;
        margin: 0;
        padding: 0;
    }}
    h1, h2, h3, h4, h5 {{
        font-family: 'Noto Sans', 'Liberation Sans', Arial, sans-serif;
        color: {accent_primary};
        page-break-after: avoid;
        break-after: avoid;
    }}
    h1 {{
        font-size: 20pt;
        border-bottom: 2px solid {accent_primary};
        padding-bottom: 6px;
        margin-top: 28pt;
        page-break-before: always;
        break-before: page;
    }}
    h2 {{
        font-size: 15pt;
        color: {accent_secondary};
        margin-top: 22pt;
        page-break-before: always;
        break-before: page;
    }}
    h3 {{
        font-size: 12pt;
        margin-top: 16pt;
    }}
    h4, h5 {{
        font-size: 10.5pt;
        margin-top: 12pt;
    }}
    p {{
        margin: 0 0 8pt 0;
        text-align: justify;
        hyphens: auto;
        orphans: 3;
        widows: 3;
    }}
    ul, ol {{
        margin: 0 0 10pt 0;
        padding-left: 20pt;
    }}
    li {{
        margin-bottom: 4pt;
    }}
    code {{
        font-family: 'JetBrains Mono', 'DejaVu Sans Mono', 'Liberation Mono', monospace;
        font-size: 9pt;
        background-color: {code_bg};
        padding: 1px 4px;
        border: 1px solid {code_border};
        border-radius: 3px;
    }}
    .code-block {{
        margin: 10pt 0 12pt 0;
        border: 1px solid {code_border};
        border-radius: 4px;
        background-color: {code_bg};
        page-break-inside: avoid;
        break-inside: avoid;
        overflow: hidden;
    }}
    .code-header {{
        font-family: 'Noto Sans', 'Liberation Sans', sans-serif;
        font-size: 7.5pt;
        font-weight: bold;
        letter-spacing: 0.05em;
        background-color: {code_header_bg};
        color: {code_header_fg};
        padding: 3px 10px;
        border-bottom: 1px solid {code_border};
    }}
    pre.highlight {{
        margin: 0;
        padding: 8px 10px;
        font-family: 'JetBrains Mono', 'DejaVu Sans Mono', 'Liberation Mono', monospace;
        font-size: 8.8pt;
        line-height: 1.42;
        white-space: pre-wrap;
        word-wrap: break-word;
    }}
    pre.highlight code {{
        background: none;
        border: none;
        padding: 0;
        font-size: inherit;
    }}
    .book-table {{
        width: 100%;
        border-collapse: collapse;
        margin: 12pt 0;
        font-size: 9.5pt;
        page-break-inside: avoid;
    }}
    .book-table th, .book-table td {{
        border: 1px solid {code_border};
        padding: 6px 8px;
        text-align: left;
        vertical-align: top;
    }}
    .book-table th {{
        background-color: {code_header_bg};
        font-family: 'Noto Sans', 'Liberation Sans', sans-serif;
        font-weight: bold;
    }}
    .book-figure {{
        margin: 14pt 0;
        text-align: center;
        page-break-inside: avoid;
        break-inside: avoid;
    }}
    .book-figure img {{
        max-width: 92%;
        max-height: 115mm;
        height: auto;
        border: 1px solid {code_border};
        border-radius: 6px;
    }}
    .book-figure figcaption {{
        font-family: 'Noto Sans', 'Liberation Sans', sans-serif;
        font-size: 8.5pt;
        color: #444444;
        margin-top: 5pt;
        font-style: italic;
    }}
    .half-title h1, .title-page h1, .title-page h2 {{
        page-break-before: avoid;
        break-before: avoid;
        border-bottom: none;
        text-align: center;
    }}
    .half-title {{
        margin-top: 60mm;
        text-align: center;
    }}
    .title-page {{
        margin-top: 45mm;
        text-align: center;
    }}
    .imprint {{
        margin-top: 20mm;
        font-size: 9pt;
        line-height: 1.45;
    }}
    .page-break {{
        page-break-after: always;
        break-after: page;
    }}
    .toc-box {{
        page-break-before: always;
        break-before: page;
        page-break-after: always;
        break-after: page;
    }}
    .toc-box h1 {{
        page-break-before: avoid;
        break-before: avoid;
    }}
    .toc-list {{
        list-style: none;
        padding-left: 0;
    }}
    .toc-list li.toc-l1 {{
        font-family: 'Noto Sans', 'Liberation Sans', sans-serif;
        font-weight: bold;
        margin-top: 8pt;
    }}
    .toc-list li.toc-l2 {{
        padding-left: 16pt;
        margin-top: 3pt;
    }}
    .toc-list a {{
        text-decoration: none;
        color: #111111;
    }}
    hr {{
        border: none;
        border-top: 1px solid {code_border};
        margin: 18pt 0;
    }}
    {rubrik_css}
    {pygments_css}
    "#;

/// Ersetzt `{name}`-Platzhalter und macht aus `{{`/`}}` einfache Klammern (wie ein Python-f-String).
fn einsetzen(vorlage: &str, werte: &[(&str, String)]) -> String {
    let mut aus = String::with_capacity(vorlage.len());
    let z: Vec<char> = vorlage.chars().collect();
    let mut i = 0;
    while i < z.len() {
        match z[i] {
            '{' if z.get(i + 1) == Some(&'{') => { aus.push('{'); i += 2; }
            '}' if z.get(i + 1) == Some(&'}') => { aus.push('}'); i += 2; }
            '{' => {
                let ende = (i..z.len()).find(|&k| z[k] == '}').unwrap_or(z.len() - 1);
                let name: String = z[i + 1..ende].iter().collect();
                if let Some((_, w)) = werte.iter().find(|(n, _)| *n == name) { aus.push_str(w); }
                i = ende + 1;
            }
            c => { aus.push(c); i += 1; }
        }
    }
    aus
}

/// Das Stylesheet wie `get_css(mode, is_epub)`; `breite_mm`/`hoehe_mm` gelten für die Seitenregel.
pub fn css(farbig: bool, epub: bool, breite_mm: f32, hoehe_mm: f32) -> String {
    let p = if farbig { &FARBE } else { &GRAU };
    let seite = if epub {
        String::new()
    } else {
        einsetzen(SEITE, &[("PAGE_WIDTH_MM", format!("{}", breite_mm)), ("PAGE_HEIGHT_MM", format!("{}", hoehe_mm))])
    };
    let (pyg, rub) = if farbig { (PYGMENTS_FARBE, RUBRIK_FARBE) } else { (PYGMENTS_GRAU, RUBRIK_GRAU) };
    einsetzen(
        HAUPT,
        &[
            ("page_rule", seite),
            ("accent_primary", p.akzent_primaer.to_string()),
            ("accent_secondary", p.akzent_sekundaer.to_string()),
            ("code_bg", p.code_grund.to_string()),
            ("code_border", p.code_rand.to_string()),
            ("code_header_bg", p.code_kopf_grund.to_string()),
            ("code_header_fg", p.code_kopf_schrift.to_string()),
            ("rubrik_css", rub.to_string()),
            ("pygments_css", pyg.to_string()),
        ],
    )
}

/// Ergänzung für das EPUB: Zitate und Kästen (kannte das bisherige Skript nicht).
pub const ZUSATZ_EPUB: &str = "
blockquote { margin: 1em 0; padding: 0.4em 0.9em; border-left: 3px solid #1E3A8A; background: #F8FAFC; }
.kasten { margin: 1em 0; padding: 0.6em 0.9em; border-left: 3px solid #334155; background: #F1F5F9; }
.kasten-titel { font-weight: bold; margin: 0 0 0.3em 0; }
.kasten-info, .kasten-merke { border-color: #1E3A8A; background: #EFF6FF; }
.kasten-tipp { border-color: #0F766E; background: #F0FDFA; }
.kasten-wichtig { border-color: #B45309; background: #FFFBEB; }
.kasten-warnung { border-color: #B91C1C; background: #FEF2F2; }
.kasten-beispiel { border-color: #6D28D9; background: #F5F3FF; }
";
