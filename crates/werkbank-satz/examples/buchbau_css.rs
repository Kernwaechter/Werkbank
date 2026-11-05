//! Schreibt die drei Stylesheets des Buchbaus (Prüfstand): `<ordner>`
fn main() {
    let o = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    std::fs::create_dir_all(&o).unwrap();
    use werkbank_satz::buchbau::css::css;
    std::fs::write(o.join("color_epub.css"), css(true, true, 170.0, 240.0)).unwrap();
    std::fs::write(o.join("color_print.css"), css(true, false, 170.0, 240.0)).unwrap();
    std::fs::write(o.join("bw_print.css"), css(false, false, 170.0, 240.0)).unwrap();
}
