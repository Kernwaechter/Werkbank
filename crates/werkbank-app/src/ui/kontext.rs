//! # Kontextmenüs
//!
//! Iced kennt keine Kontextmenüs. [`Rechtsklick`] umhüllt ein Element und meldet einen
//! Rechtsklick samt Mausposition; [`menue`] zeichnet daraufhin ein schwebendes Menü an dieser
//! Stelle. Ein Klick daneben oder `Esc` schließt es.

use crate::thema;
use crate::ui::fokus::button;
use crate::Message;
use werkbank_kern::Teil;
use iced::widget::{column, container, mouse_area, row, space, text};
use iced::{Alignment, Element, Fill, Length, Padding, Point, Size, Theme};
use iced_core::layout::{self, Layout};
use iced_core::renderer;
use iced_core::widget::operation::Operation;
use iced_core::widget::{Tree, Widget};
use iced_core::{Clipboard, Event, Rectangle, Shell, Vector, mouse, overlay};
use werkbank_kern::KapitelStatus;

/// Worauf sich ein Kontextmenü bezieht.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ziel {
    Kapitel(String),
    Teil(String),
    /// Markierte Stelle im Editor (Rechtschreibung oder Sprachprüfung).
    Wort(WortZiel),
}

/// Eine markierte Stelle samt Vorschlägen für das Korrekturmenü.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WortZiel {
    pub zeile: usize,
    pub von: usize,
    pub bis: usize,
    pub wort: String,
    /// Rechtschreibung (Hunspell) oder Sprachprüfung (LanguageTool).
    pub rechtschreibung: bool,
    /// Korrekturvorschläge (höchstens fünf).
    pub vorschlaege: Vec<String>,
    /// Index des LanguageTool-Treffers, falls es einer ist.
    pub treffer: Option<usize>,
    /// Kurzbeschreibung (bei LanguageTool die Meldung).
    pub meldung: String,
    /// LanguageTool-Tippfehler: lässt sich wie ein Wort merken.
    pub tippfehler: bool,
}

/// Offenes Kontextmenü: Ziel und Fensterposition des Rechtsklicks.
#[derive(Debug, Clone)]
pub struct Kontext {
    pub ziel: Ziel,
    pub position: Point,
}

/// Hülle, die bei Rechtsklick eine Nachricht mit der Mausposition (im Fenster) auslöst.
pub struct Rechtsklick<'a, M> {
    inhalt: Element<'a, M>,
    nachricht: Box<dyn Fn(Point) -> M + 'a>,
}

/// Macht ein Element rechtsklickbar.
pub fn rechtsklick<'a, M: 'a>(inhalt: impl Into<Element<'a, M>>, nachricht: impl Fn(Point) -> M + 'a) -> Rechtsklick<'a, M> {
    Rechtsklick { inhalt: inhalt.into(), nachricht: Box::new(nachricht) }
}

impl<M> Widget<M, Theme, iced::Renderer> for Rechtsklick<'_, M> {
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.inhalt)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.inhalt));
    }

    fn size(&self) -> Size<Length> {
        self.inhalt.as_widget().size()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        self.inhalt.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &iced::Renderer, operation: &mut dyn Operation) {
        self.inhalt.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) = event
            && let Some(p) = cursor.position_over(layout.bounds())
        {
            shell.publish((self.nachricht)(p));
            shell.capture_event();
            return;
        }
        self.inhalt.as_widget_mut().update(&mut tree.children[0], event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.inhalt.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &iced::Renderer) -> mouse::Interaction {
        self.inhalt.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, Theme, iced::Renderer>> {
        self.inhalt.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, M: 'a> From<Rechtsklick<'a, M>> for Element<'a, M> {
    fn from(r: Rechtsklick<'a, M>) -> Self {
        Element::new(r)
    }
}

/// Ein Menüeintrag: Beschriftung, Kürzel und Aktion; `gefaehrlich` färbt ihn rot.
fn eintrag<'a>(name: &'a str, kuerzel: &'a str, aktion: Message, gefaehrlich: bool) -> Element<'a, Message> {
    let farbe = if gefaehrlich { thema::fehler_rot() } else { thema::text_hell() };
    button(
        row![
            text(name).size(13).color(farbe).width(Fill),
            text(kuerzel).size(11).color(thema::text_gedimmt()),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([5, 10])
    .style(thema::listen_eintrag_stil(false))
    .on_press(Message::KontextAktion(Box::new(aktion)))
    .into()
}

/// Das schwebende Menü an der Klickposition, über einer unsichtbaren Fläche, die bei Klick schließt.
pub fn menue<'a>(k: &'a Kontext, teile: &'a [Teil], fenster: Size) -> Element<'a, Message> {
    let mut liste = column![].spacing(1).width(Fill);
    match &k.ziel {
        Ziel::Kapitel(id) => {
            let kapitel = teile.iter().flat_map(|t| t.kapitel.iter()).find(|x| &x.id == id);
            liste = liste
                .push(eintrag("Umbenennen …", "F2", Message::KapitelUmbenennenDialog, false))
                .push(eintrag("Verlauf (frühere Stände) …", "Strg+H", Message::VerlaufOeffnen, false))
                .push(eintrag("Verweis auf dieses Kapitel einfügen", "", Message::VerweisEinfuegen(id.clone()), false))
                .push(thema::trennlinie());
            // Bearbeitungsstand: der aktuelle ist markiert
            for st in [KapitelStatus::Entwurf, KapitelStatus::InArbeit, KapitelStatus::Ueberarbeitet, KapitelStatus::Fertig] {
                let aktiv = kapitel.is_some_and(|x| x.status == st);
                liste = liste.push(
                    button(
                        row![
                            text("●").size(9).color(thema::status_farbe(st)),
                            text(st.name()).size(13).width(Fill),
                            text(if aktiv { "✓" } else { "" }).size(12).color(thema::akzent()),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .width(Fill)
                    .padding([5, 10])
                    .style(thema::listen_eintrag_stil(false))
                    .on_press(Message::KontextAktion(Box::new(Message::StatusAendern(st)))),
                );
            }
            liste = liste
                .push(thema::trennlinie())
                .push(eintrag("Nach oben schieben", "Alt+Umschalt+↑", Message::KapitelVerschieben(-1), false))
                .push(eintrag("Nach unten schieben", "Alt+Umschalt+↓", Message::KapitelVerschieben(1), false))
                .push(thema::trennlinie())
                .push(eintrag("Kapitel löschen …", "", Message::KapitelLoeschenFrage, true));
        }
        Ziel::Wort(w) => {
            let kopf = if w.rechtschreibung {
                format!("„{}“ steht nicht im Wörterbuch", w.wort)
            } else {
                let m: String = w.meldung.chars().take(70).collect();
                if w.meldung.chars().count() > 70 { format!("{m} …") } else { m }
            };
            liste = liste.push(container(text(kopf).size(11).color(thema::text_gedimmt())).padding([4, 10]));
            if w.vorschlaege.is_empty() {
                liste = liste.push(container(text("Keine Vorschläge").size(13).color(thema::text_gedimmt())).padding([5, 10]));
            }
            for (i, v) in w.vorschlaege.iter().enumerate() {
                let aktion = match w.treffer {
                    Some(t) => Message::TrefferErsetzen(t, i),
                    None => Message::WortErsetzen { zeile: w.zeile, von: w.von, bis: w.bis, ersatz: v.clone() },
                };
                liste = liste.push(
                    button(text(v).size(13).font(thema::fett()))
                        .width(Fill)
                        .padding([5, 10])
                        .style(thema::listen_eintrag_stil(false))
                        .on_press(Message::KontextAktion(Box::new(aktion))),
                );
            }
            liste = liste.push(thema::trennlinie());
            match w.treffer {
                None => {
                    liste = liste
                        .push(eintrag("Merken", "für den Band", Message::WortMerken(w.wort.clone()), false))
                        .push(eintrag("Ignorieren", "bis zum Neustart", Message::WortIgnorieren(w.wort.clone()), false));
                }
                Some(t) => {
                    if w.tippfehler {
                        liste = liste.push(eintrag("Wort merken", "für den Band", Message::TrefferWortMerken(t), false));
                    }
                    liste = liste
                        .push(eintrag("Ignorieren", "diese Stelle", Message::TrefferIgnorieren(t), false))
                        .push(eintrag("Regel ausschalten", "für den Band", Message::TrefferRegelAus(t), false));
                }
            }
        }
        Ziel::Teil(_) => {
            liste = liste
                .push(eintrag("Neues Kapitel in diesem Teil …", "Strg+Umschalt+N", Message::NeuesKapitel, false))
                .push(eintrag("Umbenennen …", "", Message::TeilUmbenennenDialog, false))
                .push(thema::trennlinie())
                .push(eintrag("Nach oben schieben", "", Message::TeilVerschieben(-1), false))
                .push(eintrag("Nach unten schieben", "", Message::TeilVerschieben(1), false))
                .push(thema::trennlinie())
                .push(eintrag("Teil entfernen …", "", Message::TeilEntfernenFrage, true));
        }
    }

    // Menü möglichst an der Klickstelle, aber nie über den Fensterrand hinaus
    let (breite, hoehe) = match &k.ziel {
        Ziel::Kapitel(_) => (290.0, 360.0),
        Ziel::Teil(_) => (290.0, 200.0),
        Ziel::Wort(w) => (260.0, 110.0 + 32.0 * w.vorschlaege.len().max(1) as f32),
    };
    let x = k.position.x.min(fenster.width - breite - 8.0).max(8.0);
    let y = if k.position.y + hoehe > fenster.height { (k.position.y - hoehe).max(8.0) } else { k.position.y };

    let karte = container(liste).padding(5).width(breite).style(thema::dialog_stil);
    mouse_area(
        container(column![space::vertical().height(y), row![space::horizontal().width(x), iced::widget::opaque(karte)]])
            .padding(Padding::ZERO)
            .width(Fill)
            .height(Fill),
    )
    .on_press(Message::KontextSchliessen)
    .on_right_press(Message::KontextSchliessen)
    .into()
}
