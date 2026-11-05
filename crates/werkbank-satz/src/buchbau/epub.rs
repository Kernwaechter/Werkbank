//! EPUB 3 (mit EPUB-2-NCX) wie `build_epub` des bisherigen `build_book.py`.

use super::md::escape;
use super::{Abschnittsdaten, BandInfo};
use std::io::Write;
use std::path::Path;
use zip::write::SimpleFileOptions;

/// Medientyp eines Bildes nach Endung.
fn medientyp(endung: &str) -> &'static str {
    match endung {
        "png" => "image/png",
        "svg" => "image/svg+xml",
        _ => "image/jpeg",
    }
}

/// Schreibt das EPUB. `cover`: Pfad zu `cover.jpg|jpeg|png` (im Band- oder Buchordner).
pub fn schreiben(
    ziel: &Path,
    band: &BandInfo,
    abschnitte: &[Abschnittsdaten],
    bilderordner: Option<&Path>,
    cover: Option<&Path>,
    jetzt_iso: &str,
) -> Result<(), String> {
    let datei = std::fs::File::create(ziel).map_err(|e| format!("{}: {e}", ziel.display()))?;
    let mut zip = zip::ZipWriter::new(datei);
    let gepackt = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let roh = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let schreib = |zip: &mut zip::ZipWriter<std::fs::File>, name: &str, inhalt: &[u8], opt: SimpleFileOptions| -> Result<(), String> {
        zip.start_file(name, opt).map_err(|e| e.to_string())?;
        zip.write_all(inhalt).map_err(|e| e.to_string())
    };

    // 1. mimetype zuerst und unkomprimiert
    schreib(&mut zip, "mimetype", b"application/epub+zip", roh)?;
    // 2. container.xml
    schreib(
        &mut zip,
        "META-INF/container.xml",
        br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        gepackt,
    )?;
    // 3. Stylesheet
    schreib(&mut zip, "OEBPS/style.css", band.css_epub.as_bytes(), gepackt)?;

    let mut manifest = vec![
        r#"<item id="css" href="style.css" media-type="text/css"/>"#.to_string(),
        r#"<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>"#.to_string(),
        r#"<item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>"#.to_string(),
    ];
    let mut spine = Vec::new();
    let mut nav_links = Vec::new();
    let mut ncx_punkte = Vec::new();
    let mut reihenfolge = 1;

    let (mut cover_meta, mut guide, mut cover_spine) = (String::new(), String::new(), String::new());
    if let Some(c) = cover {
        let name = c.file_name().and_then(|n| n.to_str()).unwrap_or("cover.png");
        let endung = c.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        let mtyp = if endung == "png" { "image/png" } else { "image/jpeg" };
        let daten = std::fs::read(c).map_err(|e| format!("{}: {e}", c.display()))?;
        schreib(&mut zip, &format!("OEBPS/images/{name}"), &daten, gepackt)?;
        manifest.push(format!(r#"<item id="cover-img" href="images/{name}" media-type="{mtyp}" properties="cover-image"/>"#));
        let cover_xhtml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="de" lang="de">
<head>
  <meta charset="utf-8" />
  <title>Cover</title>
  <style>
    @page {{ margin: 0; padding: 0; }}
    html, body {{ margin: 0; padding: 0; height: 100%; text-align: center; background-color: #0B1120; }}
    img {{ max-width: 100%; max-height: 100%; height: 100%; object-fit: contain; }}
  </style>
</head>
<body>
  <div>
    <img src="images/{name}" alt="Cover" />
  </div>
</body>
</html>"#
        );
        schreib(&mut zip, "OEBPS/cover.xhtml", cover_xhtml.as_bytes(), gepackt)?;
        manifest.push(r#"<item id="cover" href="cover.xhtml" media-type="application/xhtml+xml"/>"#.to_string());
        cover_spine = "<itemref idref=\"cover\"/>\n    ".to_string();
        cover_meta = "    <meta name=\"cover\" content=\"cover-img\"/>\n".to_string();
        guide = "  <guide>\n    <reference type=\"cover\" title=\"Cover\" href=\"cover.xhtml\"/>\n  </guide>\n".to_string();
    }

    // Alle Abbildungen des Bandes
    if let Some(ordner) = bilderordner {
        let mut dateien: Vec<_> = std::fs::read_dir(ordner).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).collect();
        dateien.sort();
        for (nr, pfad) in dateien.iter().enumerate() {
            let endung = pfad.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if !matches!(endung.as_str(), "png" | "jpg" | "jpeg" | "svg") {
                continue;
            }
            let name = pfad.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            let daten = std::fs::read(pfad).map_err(|e| format!("{}: {e}", pfad.display()))?;
            schreib(&mut zip, &format!("OEBPS/images/{name}"), &daten, gepackt)?;
            manifest.push(format!(r#"<item id="fig_{:03}" href="images/{name}" media-type="{}"/>"#, nr + 1, medientyp(&endung)));
        }
    }

    for (idx, a) in abschnitte.iter().enumerate() {
        let nr = idx + 1;
        let xhtml_name = format!("sec_{nr:02}.xhtml");
        let id = format!("sec_{nr:02}");
        let titel = a.abschnitt.toc.first().map(|t| t.1.clone()).unwrap_or_else(|| format!("Abschnitt {nr}"));
        let inhalt = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="de" lang="de">
<head>
  <meta charset="utf-8" />
  <title>{}</title>
  <link rel="stylesheet" type="text/css" href="style.css" />
</head>
<body>
{}
</body>
</html>"#,
            escape(&titel),
            a.html
        );
        schreib(&mut zip, &format!("OEBPS/{xhtml_name}"), inhalt.as_bytes(), gepackt)?;
        manifest.push(format!(r#"<item id="{id}" href="{xhtml_name}" media-type="application/xhtml+xml"/>"#));
        spine.push(format!(r#"<itemref idref="{id}"/>"#));
        if !a.ist_titelei {
            for (_, titel, anker) in &a.abschnitt.toc {
                let href = format!("{xhtml_name}#{anker}");
                nav_links.push(format!(r#"<li><a href="{href}">{}</a></li>"#, escape(titel)));
                ncx_punkte.push(format!(
                    "    <navPoint id=\"navPoint-{reihenfolge}\" playOrder=\"{reihenfolge}\">\n      <navLabel><text>{}</text></navLabel>\n      <content src=\"{href}\"/>\n    </navPoint>",
                    escape(titel)
                ));
                reihenfolge += 1;
            }
        }
    }

    let nav = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" xml:lang="de" lang="de">
<head>
  <meta charset="utf-8" />
  <title>Inhaltsverzeichnis</title>
  <link rel="stylesheet" type="text/css" href="style.css" />
</head>
<body>
  <nav epub:type="toc" id="toc">
    <h1>Inhaltsverzeichnis</h1>
    <ol>
      {}
    </ol>
  </nav>
</body>
</html>"#,
        nav_links.join("")
    );
    schreib(&mut zip, "OEBPS/nav.xhtml", nav.as_bytes(), gepackt)?;

    let voller_titel = format!("{} – {}", escape(&band.titel), escape(&band.untertitel));
    let ncx = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1">
  <head>
    <meta name="dtb:uid" content="{}"/>
    <meta name="dtb:depth" content="2"/>
    <meta name="dtb:totalPageCount" content="0"/>
    <meta name="dtb:maxPageNumber" content="0"/>
  </head>
  <docTitle><text>{voller_titel}</text></docTitle>
  <navMap>
{}
  </navMap>
</ncx>"#,
        band.uuid,
        ncx_punkte.join("\n")
    );
    schreib(&mut zip, "OEBPS/toc.ncx", ncx.as_bytes(), gepackt)?;

    let opf = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="BookId" xml:lang="de">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:identifier id="BookId">{uuid}</dc:identifier>
    <dc:title>{voller_titel}</dc:title>
    <dc:creator>{autor}</dc:creator>
    <dc:language>{sprache}</dc:language>
    <dc:publisher>{verlag}</dc:publisher>
    <meta property="dcterms:modified">{jetzt_iso}</meta>
{cover_meta}  </metadata>
  <manifest>
    {manifest}
  </manifest>
  <spine toc="ncx">
    {cover_spine}<itemref idref="sec_01"/>
    <itemref idref="nav"/>
    {spine}
  </spine>
{guide}</package>"#,
        uuid = band.uuid,
        autor = escape(&band.autor),
        sprache = band.sprache,
        verlag = escape(&band.verlag),
        manifest = manifest.iter().map(|m| format!("    {m}")).collect::<Vec<_>>().join("\n"),
        spine = spine.iter().skip(1).map(|s| format!("    {s}")).collect::<Vec<_>>().join("\n"),
    );
    schreib(&mut zip, "OEBPS/content.opf", opf.as_bytes(), gepackt)?;
    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}
