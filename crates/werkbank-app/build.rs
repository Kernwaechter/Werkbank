//! Bettet unter Windows das Programmsymbol und die Versionsangaben in die .exe ein.

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icons/werkbank.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/icons/werkbank.ico");
        res.set("ProductName", "Werkbank");
        res.set("FileDescription", "Schreibwerkzeug für technische Bücher");
        res.set("LegalCopyright", "Volker Dähn, CC BY-NC-SA 4.0");
        if let Err(e) = res.compile() {
            println!("cargo:warning=Symbol konnte nicht eingebettet werden: {e}");
        }
    }
}
