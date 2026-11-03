//! # Werkbank Styling & Farbsystem
//!
//! Stellt die visuelle Gestaltung der Werkbank bereit. Wir orientieren uns
//! auf ruhige, neutrale Flächen mit einem einzigen zurückhaltenden Akzent, damit lange
//! Schreibsitzungen die Augen nicht ermüden (Kontraste im Test `kontraste` geprüft).

#![allow(dead_code)]

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

static HELL: AtomicBool = AtomicBool::new(false);
static SCHREIBMASCHINE: AtomicBool = AtomicBool::new(false);
static SCHRIFT: AtomicU32 = AtomicU32::new(17.0f32.to_bits());
static SPALTE: AtomicU32 = AtomicU32::new(700.0f32.to_bits());
static INSPEKTOR: AtomicU32 = AtomicU32::new(316.0f32.to_bits());
static ZEILE: AtomicU32 = AtomicU32::new(1.55f32.to_bits());
static ART: AtomicU8 = AtomicU8::new(1);

/// Schaltet zwischen dunklem und hellem Erscheinungsbild um.
pub fn hell_setzen(hell: bool) {
    HELL.store(hell, Ordering::Relaxed);
}

/// Schreibmaschinen-Modus (Cursorzeile bleibt in der Mitte).
pub fn schreibmaschine_setzen(an: bool) {
    SCHREIBMASCHINE.store(an, Ordering::Relaxed);
}

/// Ist der Schreibmaschinen-Modus aktiv?
pub fn schreibmaschine() -> bool {
    SCHREIBMASCHINE.load(Ordering::Relaxed)
}

/// Ist das helle Erscheinungsbild aktiv?
pub fn ist_hell() -> bool {
    HELL.load(Ordering::Relaxed)
}

/// Breite der rechten Detailspalte in Pixeln.
pub fn inspektor_breite() -> f32 {
    f32::from_bits(INSPEKTOR.load(Ordering::Relaxed))
}

/// Setzt die Breite der rechten Detailspalte (begrenzt auf 240 bis 720 Pixel).
pub fn inspektor_breite_setzen(px: f32) {
    INSPEKTOR.store(px.clamp(240.0, 720.0).to_bits(), Ordering::Relaxed);
}

/// Stellt Schriftgröße und Spaltenbreite des Editors ein.
pub fn darstellung_setzen(schrift: f32, spalte: f32) {
    SCHRIFT.store(schrift.to_bits(), Ordering::Relaxed);
    SPALTE.store(spalte.to_bits(), Ordering::Relaxed);
}

/// Schriftart des Editors: 0 = Sans (System), 1 = Serif (Libertinus, eingebettet), 2 = Monospace.
pub fn schriftart_setzen(art: u8) {
    ART.store(art, Ordering::Relaxed);
}

/// Zeilenabstand des Editors als Vielfaches der Schriftgröße.
pub fn zeilenabstand_setzen(faktor: f32) {
    ZEILE.store(faktor.to_bits(), Ordering::Relaxed);
}

/// Zeilenabstand des Editors.
pub fn zeilenabstand() -> f32 {
    f32::from_bits(ZEILE.load(Ordering::Relaxed))
}

/// Schrift des Editors (auch für fette Auszeichnungen, damit die Familie gleich bleibt).
pub fn editor_font(fett: bool) -> iced::Font {
    let gewicht = if fett { iced::font::Weight::Bold } else { iced::font::Weight::Normal };
    let basis = match ART.load(Ordering::Relaxed) {
        0 => iced::Font::DEFAULT,
        2 => iced::Font::MONOSPACE,
        _ => iced::Font::with_name("Libertinus Serif"),
    };
    iced::Font { weight: gewicht, ..basis }
}

/// Schriftgröße des Editors in Punkt.
pub fn editor_schrift() -> f32 {
    f32::from_bits(SCHRIFT.load(Ordering::Relaxed))
}

/// Maximale Breite der Textspalte im Editor.
pub fn editor_spalte() -> f32 {
    f32::from_bits(SPALTE.load(Ordering::Relaxed))
}

fn waehle(dunkel: Color, hell: Color) -> Color {
    if ist_hell() { hell } else { dunkel }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

use iced::border::Radius;
use iced::overlay::menu;
use iced::theme::Palette;
use iced::widget::{button, container, pick_list, slider};
use iced::{Background, Border, Color, Shadow, Theme};

// ---------------------------------------------------------------- Farbsystem
//
// Ruhige Flächen (hell: neutrales Papier, dunkel: Schiefergrau; nie Grün), ein einziger zurückhaltender
// Akzent (Stahlblau) für Auswahl und Fokus, gedämpfte Signalfarben. Texte erreichen mindestens
// Kontrastverhältnis 4,5:1 (Fließtext über 10:1), geprüft im Test `kontraste`.

/// Akzent für Textmarken, Auswahlrahmen und Links.
pub fn akzent() -> Color {
    waehle(rgb(135, 174, 208), rgb(40, 88, 128))
}

/// Akzentfläche für gefüllte Knöpfe (Text darauf: [`auf_akzent`]).
pub fn akzent_fuell() -> Color {
    waehle(rgb(62, 104, 142), rgb(44, 92, 132))
}

/// Schrift auf Akzentflächen.
pub fn auf_akzent() -> Color {
    rgb(255, 255, 255)
}

/// Sehr dezent getönte Fläche in Akzentnähe (aktive Symbole, Link-Hover).
pub fn akzent_flaeche() -> Color {
    waehle(rgb(44, 58, 73), rgb(222, 230, 240))
}

/// Hintergrund des Schreibbereichs.
pub fn dunkel_hintergrund() -> Color {
    waehle(rgb(30, 35, 41), rgb(251, 250, 247))
}

/// Seitenleisten und Kopfleiste, leicht abgesetzt vom Schreibbereich.
pub fn panel_hintergrund() -> Color {
    waehle(rgb(37, 43, 51), rgb(243, 242, 238))
}

/// Karten, Eingabefelder und Infoboxen.
pub fn karte_hintergrund() -> Color {
    waehle(rgb(45, 52, 61), rgb(237, 236, 232))
}

/// Überfahrene Karte.
pub fn karte_hover() -> Color {
    waehle(rgb(54, 62, 73), rgb(229, 228, 223))
}

/// Ausgewählte Karte.
pub fn karte_aktiv() -> Color {
    waehle(rgb(47, 60, 78), rgb(223, 230, 238))
}

/// Trennlinien und Rahmen.
pub fn rahmen_farbe() -> Color {
    waehle(rgb(66, 75, 88), rgb(214, 212, 206))
}

/// Haupttext (kein reines Weiß/Schwarz, damit die Augen nicht ermüden).
pub fn text_hell() -> Color {
    waehle(rgb(226, 230, 236), rgb(34, 36, 40))
}

/// Sekundärtext, Zähler, Hinweise.
pub fn text_gedimmt() -> Color {
    waehle(rgb(170, 177, 188), rgb(92, 96, 104))
}

/// Erfolg (gespeichert, fehlerfrei).
pub fn erfolg_gruen() -> Color {
    waehle(rgb(116, 176, 136), rgb(40, 118, 76))
}

/// Warnung (ungespeichert, Hinweise).
pub fn warnung_orange() -> Color {
    waehle(rgb(217, 165, 98), rgb(160, 96, 20))
}

/// Fehler.
pub fn fehler_rot() -> Color {
    waehle(rgb(224, 112, 104), rgb(180, 50, 42))
}

/// Codeblöcke und Eingabeflächen in Dialogen.
pub fn code_hintergrund() -> Color {
    waehle(rgb(24, 28, 34), rgb(238, 237, 232))
}

/// Neutrale Hover-Fläche für Knöpfe.
pub fn hover_flaeche() -> Color {
    waehle(rgb(56, 64, 75), rgb(230, 229, 224))
}

/// Gedrückte Knopffläche.
pub fn gedrueckt_flaeche() -> Color {
    waehle(rgb(33, 38, 45), rgb(220, 219, 214))
}

/// Grundfläche sekundärer Knöpfe.
pub fn knopf_flaeche() -> Color {
    waehle(rgb(50, 58, 68), rgb(240, 239, 235))
}

/// Passendes Iced-Thema, damit Eingabefelder, Scrollleisten und Editor zur Palette passen.
pub fn iced_theme() -> Theme {
    Theme::custom(
        if ist_hell() { "Werkbank Papier" } else { "Werkbank Dämmerung" },
        Palette {
            background: dunkel_hintergrund(),
            text: text_hell(),
            primary: akzent_fuell(),
            success: erfolg_gruen(),
            warning: warnung_orange(),
            danger: fehler_rot(),
        },
    )
}

/// Stil für die obere Navigationsleiste.
pub fn kopfleiste_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(panel_hintergrund())),
        border: Border {
            color: rahmen_farbe(),
            width: 1.0,
            radius: Radius::default(),
        },
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

/// Stil für linke Seitenleisten (Bibliothek und Blattliste).
pub fn seitenleiste_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(panel_hintergrund())),
        border: Border {
            color: rahmen_farbe(),
            width: 1.0,
            radius: Radius::default(),
        },
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

/// Stil für den zentralen Editorbereich im Ulysses-Look.
pub fn editor_bereich_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(dunkel_hintergrund())),
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

/// Stil für eine inaktive Kapitelkarte in der Blattliste.
pub fn kapitel_karte_stil(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => karte_hover(),
        button::Status::Pressed => gedrueckt_flaeche(),
        _ => karte_hintergrund(),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text_hell(),
        border: Border {
            color: rahmen_farbe(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

/// Stil für die aktuell ausgewählte Kapitelkarte.
pub fn kapitel_karte_aktiv_stil(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(karte_aktiv())),
        text_color: text_hell(),
        border: Border {
            color: akzent(),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

/// Stil für primäre Aktionsknöpfe (z. B. Speichern, Neues Kapitel).
pub fn primaer_knopf_stil(_theme: &Theme, status: button::Status) -> button::Style {
    let f = akzent_fuell();
    let bg = match status {
        button::Status::Hovered => Color { r: (f.r + 0.06).min(1.0), g: (f.g + 0.06).min(1.0), b: (f.b + 0.06).min(1.0), a: 1.0 },
        button::Status::Pressed => Color { r: f.r * 0.82, g: f.g * 0.82, b: f.b * 0.82, a: 1.0 },
        _ => f,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: auf_akzent(),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

/// Knopf für zerstörerische Aktionen (Löschen, Verwerfen): rote Schrift auf neutraler Fläche,
/// beim Überfahren rot gefüllt.
pub fn gefahr_knopf_stil(_theme: &Theme, status: button::Status) -> button::Style {
    let rot = fehler_rot();
    let (bg, schrift) = match status {
        button::Status::Hovered | button::Status::Pressed => (Color { a: 0.85, ..rot }, auf_akzent()),
        _ => (knopf_flaeche(), rot),
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: schrift,
        border: Border { color: Color { a: 0.6, ..rot }, width: 1.0, radius: 6.0.into() },
        ..Default::default()
    }
}

/// Stil für Navigations-Tabs oben.
pub fn tab_knopf_stil(aktiv: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = if aktiv {
            karte_aktiv()
        } else {
            match status {
                button::Status::Hovered => hover_flaeche(),
                _ => Color::TRANSPARENT,
            }
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if aktiv { text_hell() } else { text_gedimmt() },
            border: Border {
                color: if aktiv { akzent() } else { Color::TRANSPARENT },
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }
}

/// Stil für Sekundär- und Icon-Knöpfe.
pub fn sekundaer_knopf_stil(_theme: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => hover_flaeche(),
        button::Status::Pressed => gedrueckt_flaeche(),
        _ => knopf_flaeche(),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: text_hell(),
        border: Border {
            color: rahmen_farbe(),
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

/// Stil für Infoboxen in Bau- und Prüf-Dashboards.
pub fn infobox_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(karte_hintergrund())),
        border: Border {
            color: linie_farbe(),
            width: 1.0,
            radius: 10.0.into(),
        },
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

/// Stil für Codeblöcke im Vorschaumodus.
pub fn codeblock_container_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(code_hintergrund())),
        border: Border {
            color: rahmen_farbe(),
            width: 1.0,
            radius: 6.0.into(),
        },
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

/// Stil für anklickbare Abschnitte in der Live-Vorschau.
///
/// Hebt den aktuell im Editor fokussierten Abschnitt dezent mit
/// Akzentrahmen und leichtem Hintergrund hervor.
pub fn vorschau_block_stil(aktiv: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (bg, border_color, border_width) = if aktiv {
            (akzent_flaeche(), akzent(), 1.0)
        } else {
            match status {
                button::Status::Hovered => (hover_flaeche(), rahmen_farbe(), 1.0),
                _ => (Color::TRANSPARENT, Color::TRANSPARENT, 0.0),
            }
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text_hell(),
            border: Border {
                color: border_color,
                width: border_width,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }
}

/// Flacher Symbolknopf ohne Rahmen für Ansichts-Schalter (Akzentfarbe, wenn aktiv).
pub fn symbol_knopf_stil(aktiv: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match (aktiv, status) {
            (_, button::Status::Hovered) => hover_flaeche(),
            (true, _) => akzent_flaeche(),
            _ => Color::TRANSPARENT,
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if aktiv { akzent() } else { text_gedimmt() },
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }
}

/// Textlink-Stil: gedämpft, beim Überfahren dezent hinterlegt.
pub fn link_knopf_stil(_theme: &Theme, status: button::Status) -> button::Style {
    let (bg, farbe) = match status {
        button::Status::Hovered | button::Status::Pressed => {
            (akzent_flaeche(), akzent())
        }
        _ => (Color::TRANSPARENT, akzent()),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: farbe,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 4.0.into(),
        },
        ..Default::default()
    }
}

/// Suchfeld-Optik für den Befehlspalette-Auslöser in der Kopfleiste.
pub fn suchfeld_knopf_stil(_theme: &Theme, status: button::Status) -> button::Style {
    let rahmen = match status {
        button::Status::Hovered | button::Status::Pressed => akzent(),
        _ => rahmen_farbe(),
    };

    button::Style {
        background: Some(Background::Color(dunkel_hintergrund())),
        text_color: text_gedimmt(),
        border: Border {
            color: rahmen,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

/// Zeile in der Befehlspalette; die per Pfeiltasten gewählte Zeile ist hinterlegt.
pub fn palette_eintrag_stil(aktiv: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match (aktiv, status) {
            (true, _) => karte_aktiv(),
            (false, button::Status::Hovered) => karte_hintergrund(),
            _ => Color::TRANSPARENT,
        };

        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text_hell(),
            border: Border {
                color: if aktiv {
                    akzent()
                } else {
                    Color::TRANSPARENT
                },
                width: if aktiv { 1.0 } else { 0.0 },
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }
}

/// Abgedunkelter Hintergrund hinter der Befehlspalette.
pub fn overlay_dimm_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba8(0, 0, 0, 0.55))),
        ..Default::default()
    }
}

/// Schwebendes Fenster der Befehlspalette.
pub fn palette_box_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(panel_hintergrund())),
        border: Border {
            color: rahmen_farbe(),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.6),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 32.0,
        },
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

/// Kleine Hinweisblase für Tooltips (mit Tastenkürzel).
pub fn tooltip_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(karte_aktiv())),
        border: Border {
            color: rahmen_farbe(),
            width: 1.0,
            radius: 6.0.into(),
        },
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

/// Auswahlliste (Dropdown) im Look der Werkbank.
pub fn auswahl_stil(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let rahmen = match status {
        pick_list::Status::Hovered | pick_list::Status::Opened { .. } => akzent(),
        pick_list::Status::Active => rahmen_farbe(),
    };

    pick_list::Style {
        text_color: text_hell(),
        placeholder_color: text_gedimmt(),
        handle_color: text_gedimmt(),
        background: Background::Color(karte_hintergrund()),
        border: Border {
            color: rahmen,
            width: 1.0,
            radius: 6.0.into(),
        },
    }
}

/// Band-Auswahl in der Kopfleiste: flach wie ein Titel, Rahmen erst beim Überfahren.
pub fn band_auswahl_stil(_theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let (bg, rahmen) = match status {
        pick_list::Status::Hovered | pick_list::Status::Opened { .. } => (hover_flaeche(), linie_farbe()),
        pick_list::Status::Active => (Color::TRANSPARENT, Color::TRANSPARENT),
    };
    pick_list::Style {
        text_color: text_hell(),
        placeholder_color: text_gedimmt(),
        handle_color: text_gedimmt(),
        background: Background::Color(bg),
        border: Border { color: rahmen, width: 1.0, radius: 6.0.into() },
    }
}

/// Aufklappmenü der Auswahlliste.
pub fn auswahl_menue_stil(_theme: &Theme) -> menu::Style {
    menu::Style {
        background: Background::Color(panel_hintergrund()),
        border: Border {
            color: rahmen_farbe(),
            width: 1.0,
            radius: 6.0.into(),
        },
        text_color: text_hell(),
        selected_text_color: auf_akzent(),
        selected_background: Background::Color(akzent_fuell()),
        shadow: Shadow::default(),
    }
}

/// Schieberegler im Akzent.
pub fn regler_stil(_theme: &Theme, status: slider::Status) -> slider::Style {
    let griff = match status {
        slider::Status::Hovered | slider::Status::Dragged => akzent(),
        slider::Status::Active => akzent_fuell(),
    };

    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(akzent()),
                Background::Color(rahmen_farbe()),
            ),
            width: 4.0,
            border: Border {
                radius: 2.0.into(),
                ..Default::default()
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 8.0 },
            background: Background::Color(griff),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}


// ---------------------------------------------------------------- Bausteine der ruhigen Oberfläche
//
// Angelehnt an reduzierte Schreibprogramme: flache Listen statt Karten, Auswahl als weiche Fläche,
// Segment-Schalter statt Reitern, Dialoge mit Titel, Erklärung und Knopfleiste rechts unten.

/// Fette Variante der Oberflächenschrift (Titel in Listen und Köpfen).
pub fn fett() -> iced::Font {
    iced::Font { weight: iced::font::Weight::Semibold, ..iced::Font::DEFAULT }
}

/// Zahl mit schmalem Tausenderabstand, z. B. `54 756`.
pub fn tausender(n: usize) -> String {
    let s = n.to_string();
    let mut aus = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            aus.push('\u{202F}');
        }
        aus.push(c);
    }
    aus
}

/// Weiche Trennlinie (halbtransparent, damit sie auf jeder Fläche gleich zurückhaltend wirkt).
pub fn linie_farbe() -> Color {
    let c = rahmen_farbe();
    Color { a: 0.55, ..c }
}

/// Eine 1 px hohe Trennlinie über die volle Breite.
pub fn trennlinie<'a, M: 'a>() -> iced::Element<'a, M> {
    container(iced::widget::space().height(1).width(iced::Fill))
        .style(|_| container::Style { background: Some(Background::Color(linie_farbe())), ..Default::default() })
        .into()
}

/// Eine 1 px breite senkrechte Trennlinie über die volle Höhe.
pub fn trennlinie_senkrecht<'a, M: 'a>() -> iced::Element<'a, M> {
    container(iced::widget::space().width(1).height(iced::Fill))
        .style(|_| container::Style { background: Some(Background::Color(linie_farbe())), ..Default::default() })
        .into()
}

/// Kleine Abschnittsüberschrift in Versalien (z. B. „TEILE“, „ZIEL“).
pub fn abschnitt_titel<'a, M: 'a>(titel: &str) -> iced::Element<'a, M> {
    iced::widget::text(titel.to_uppercase()).size(10).color(text_gedimmt()).into()
}

/// Bibliothek (linke Spalte): eine Spur dunkler als die Blattliste, wie ein Regal hinter dem Pult.
pub fn bibliothek_hintergrund() -> Color {
    waehle(rgb(33, 38, 45), rgb(236, 235, 230))
}

/// Flächenstil der Bibliothek.
pub fn bibliothek_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(bibliothek_hintergrund())),
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

/// Flächenstil von Blattliste, Inspektor und Kopfleiste (ohne Rahmen; Trennung über Linien).
pub fn flaeche_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(panel_hintergrund())),
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

/// Eintrag einer flachen Liste (Bibliothek, Blattliste): Auswahl als weiche Akzentfläche, kein Rahmen.
pub fn listen_eintrag_stil(aktiv: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match (aktiv, status) {
            (true, _) => akzent_flaeche(),
            (false, button::Status::Hovered) => hover_flaeche(),
            (false, button::Status::Pressed) => gedrueckt_flaeche(),
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text_hell(),
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 7.0.into() },
            ..Default::default()
        }
    }
}

/// Rahmen eines Segment-Schalters (Gruppe gleichwertiger Ansichten).
pub fn segment_rahmen_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(code_hintergrund())),
        border: Border { color: linie_farbe(), width: 1.0, radius: 8.0.into() },
        ..Default::default()
    }
}

/// Ein Feld des Segment-Schalters; das aktive hebt sich als erhabene Fläche ab.
pub fn segment_stil(aktiv: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let bg = match (aktiv, status) {
            (true, _) => karte_hover(),
            (false, button::Status::Hovered) => hover_flaeche(),
            _ => Color::TRANSPARENT,
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if aktiv { text_hell() } else { text_gedimmt() },
            border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 6.0.into() },
            shadow: if aktiv {
                Shadow { color: Color::from_rgba8(0, 0, 0, 0.25), offset: iced::Vector::new(0.0, 1.0), blur_radius: 2.0 }
            } else {
                Shadow::default()
            },
            ..Default::default()
        }
    }
}

/// Farbe eines Bearbeitungsstands (Entwurf grau, in Arbeit orange, überarbeitet Akzent, fertig grün).
pub fn status_farbe(status: werkbank_kern::KapitelStatus) -> Color {
    use werkbank_kern::KapitelStatus as S;
    match status {
        S::Entwurf => text_gedimmt(),
        S::InArbeit => warnung_orange(),
        S::Ueberarbeitet => akzent(),
        S::Fertig => erfolg_gruen(),
    }
}

/// Kleine Marke (Pille) in einer Signalfarbe, z. B. für den Bearbeitungsstand.
pub fn marke_stil(farbe: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(Color { a: 0.16, ..farbe })),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 9.0.into() },
        text_color: Some(farbe),
        ..Default::default()
    }
}

/// Schreibfläche ohne Rahmen: der Text steht frei auf dem Pult.
pub fn schreibflaeche_stil(_theme: &Theme, status: crate::editor::Status) -> crate::editor::Style {
    let _ = status;
    crate::editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: 0.0.into() },
        placeholder: text_gedimmt(),
        value: text_hell(),
        selection: Color { a: 0.45, ..akzent_fuell() },
    }
}

/// Schwebender Dialog: weich gerundet, mit tiefem Schatten.
pub fn dialog_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(panel_hintergrund())),
        border: Border { color: linie_farbe(), width: 1.0, radius: 14.0.into() },
        shadow: Shadow { color: Color::from_rgba8(0, 0, 0, 0.55), offset: iced::Vector::new(0.0, 12.0), blur_radius: 40.0 },
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

/// Fußleiste eines Dialogs (eine Spur abgesetzt, Knöpfe rechts).
pub fn dialog_fuss_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(bibliothek_hintergrund())),
        border: Border { color: Color::TRANSPARENT, width: 0.0, radius: Radius { top_left: 0.0, top_right: 0.0, bottom_right: 14.0, bottom_left: 14.0 } },
        ..Default::default()
    }
}

/// Gruppe zusammengehöriger Einstellungen (wie eine Tafel mit abgerundeten Ecken).
pub fn gruppe_stil(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(karte_hintergrund())),
        border: Border { color: linie_farbe(), width: 1.0, radius: 10.0.into() },
        text_color: Some(text_hell()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linear(c: f32) -> f32 {
        if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    }

    fn leuchtdichte(c: Color) -> f32 {
        0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
    }

    /// Kontrastverhältnis nach WCAG 2.1.
    fn kontrast(a: Color, b: Color) -> f32 {
        let (la, lb) = (leuchtdichte(a), leuchtdichte(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    /// Beide Tests stellen den globalen Hell/Dunkel-Schalter um und dürfen nicht gleichzeitig laufen.
    static THEMA_SPERRE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn kontraste() {
        let _sperre = THEMA_SPERRE.lock().unwrap_or_else(|e| e.into_inner());
        for hell in [false, true] {
            hell_setzen(hell);
            let modus = if hell { "hell" } else { "dunkel" };
            let flaechen = [
                ("Schreibbereich", dunkel_hintergrund()),
                ("Seitenleiste", panel_hintergrund()),
                ("Bibliothek", bibliothek_hintergrund()),
                ("Karte", karte_hintergrund()),
                ("aktive Karte", karte_aktiv()),
            ];
            for (name, f) in flaechen {
                let mindest = if matches!(name, "Schreibbereich" | "Seitenleiste" | "Bibliothek") { 10.0 } else { 7.0 };
                assert!(kontrast(text_hell(), f) >= mindest, "{modus}: Haupttext auf {name}: {:.1}", kontrast(text_hell(), f));
                assert!(kontrast(text_gedimmt(), f) >= 4.5, "{modus}: Sekundärtext auf {name}: {:.1}", kontrast(text_gedimmt(), f));
                assert!(kontrast(akzent(), f) >= 4.5, "{modus}: Akzent auf {name}: {:.1}", kontrast(akzent(), f));
                for (n, c) in [("Erfolg", erfolg_gruen()), ("Warnung", warnung_orange()), ("Fehler", fehler_rot())] {
                    assert!(kontrast(c, f) >= 3.0, "{modus}: {n} auf {name}: {:.1}", kontrast(c, f));
                }
            }
            assert!(kontrast(auf_akzent(), akzent_fuell()) >= 4.5, "{modus}: Schrift auf Akzentfläche {:.1}", kontrast(auf_akzent(), akzent_fuell()));
            assert!(kontrast(text_hell(), code_hintergrund()) >= 10.0, "{modus}: Code");
            assert!(kontrast(akzent(), akzent_flaeche()) >= 4.0, "{modus}: Akzent auf Akzentfläche {:.1}", kontrast(akzent(), akzent_flaeche()));
        }
        hell_setzen(false);
    }

    #[test]
    fn flaechen_haben_keinen_farbstich() {
        let _sperre = THEMA_SPERRE.lock().unwrap_or_else(|e| e.into_inner());
        // Hintergründe bleiben nahezu neutral (Abstand der Kanäle gering): kein „Bonbon“-Farbton
        for hell in [false, true] {
            hell_setzen(hell);
            for f in [dunkel_hintergrund(), panel_hintergrund(), bibliothek_hintergrund(), karte_hintergrund(), rahmen_farbe(), code_hintergrund()] {
                let spanne = f.r.max(f.g).max(f.b) - f.r.min(f.g).min(f.b);
                // Schiefergrau darf leicht ins Blaue gehen, nie aber ins Grüne
                assert!(spanne < 0.10, "Fläche zu bunt: {f:?}");
                assert!(f.g <= f.b && f.r <= f.b || hell, "Fläche nicht schiefergrau: {f:?}");
            }
        }
        hell_setzen(false);
    }
}
