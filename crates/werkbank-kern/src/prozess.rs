//! Hilfen für Kindprozesse.

use std::process::Command;

/// Verhindert unter Windows das kurze Aufblitzen eines Konsolenfensters, wenn die Oberfläche
/// (ohne eigene Konsole) ein Programm wie `rustc`, `pandoc` oder `gs` startet. Sonst ohne Wirkung.
pub fn ohne_fenster(befehl: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        befehl.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    befehl
}