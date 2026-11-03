//! # Tastaturfokus für Knöpfe, Schalter, Regler und Auswahllisten
//!
//! Iced 0.14 kennt keinen Tab-Fokus für diese Elemente. Die Hülle [`Fokus`] macht jedes
//! Element fokussierbar (Tab/F6 wandern über `focus_next`), zeichnet einen grünen Fokusrahmen und
//! löst bei `Enter`/`Leertaste` einen Klick aus. Regler und Auswahllisten bekommen Pfeiltasten.
//! `button`, `toggler` haben hier dieselbe Schreibweise wie in Iced und ersetzen sie einfach.

use crate::thema;
use iced::keyboard::{self, Key, Modifiers, key::Named};
use iced::widget::Button;
use iced::{Border, Color, Element, Length, Padding, Pixels, Size, Theme};
use iced_core::layout::{self, Layout};
use iced_core::renderer;
use iced_core::widget::operation::{Focusable, Operation};
use iced_core::widget::{self, Tree, tree};
use iced_core::{Clipboard, Event, Rectangle, Shell, Vector, Widget, mouse, overlay};

type Tasten<'a, M> = Box<dyn Fn(&Key, Modifiers) -> Option<M> + 'a>;

/// Hülle, die ein Element per Tastatur erreichbar macht.
pub struct Fokus<'a, Message> {
    inhalt: Element<'a, Message>,
    klick_bei_enter: bool,
    tasten: Option<Tasten<'a, Message>>,
}

#[derive(Default)]
struct Zustand {
    fokussiert: bool,
}

impl Focusable for Zustand {
    fn is_focused(&self) -> bool {
        self.fokussiert
    }
    fn focus(&mut self) {
        self.fokussiert = true;
    }
    fn unfocus(&mut self) {
        self.fokussiert = false;
    }
}

impl<'a, Message: 'a> Fokus<'a, Message> {
    /// Umhüllt ein Element; `klick_bei_enter` simuliert bei Enter/Leertaste einen Mausklick.
    pub fn neu(inhalt: impl Into<Element<'a, Message>>, klick_bei_enter: bool) -> Self {
        Self { inhalt: inhalt.into(), klick_bei_enter, tasten: None }
    }

    /// Eigene Tastenbelegung (z. B. Pfeiltasten bei Reglern).
    pub fn tasten(mut self, f: impl Fn(&Key, Modifiers) -> Option<Message> + 'a) -> Self {
        self.tasten = Some(Box::new(f));
        self
    }
}

impl<Message> Widget<Message, Theme, iced::Renderer> for Fokus<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Zustand>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Zustand::default())
    }

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
        let zustand = tree.state.downcast_mut::<Zustand>();
        operation.focusable(None, layout.bounds(), zustand);
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
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let fokussiert = tree.state.downcast_ref::<Zustand>().fokussiert;

        if fokussiert && let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
            let klick = self.klick_bei_enter && matches!(key, Key::Named(Named::Enter | Named::Space));
            if klick {
                // Enter/Leertaste = Mausklick in die Mitte des Elements
                let mitte = mouse::Cursor::Available(layout.bounds().center());
                for e in [
                    Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                    Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                ] {
                    self.inhalt.as_widget_mut().update(&mut tree.children[0], &e, layout, mitte, renderer, clipboard, shell, viewport);
                }
                shell.capture_event();
                return;
            }
            if let Some(f) = &self.tasten
                && let Some(nachricht) = f(key, *modifiers)
            {
                shell.publish(nachricht);
                shell.capture_event();
                return;
            }
        }

        if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            tree.state.downcast_mut::<Zustand>().fokussiert = cursor.is_over(layout.bounds());
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
        use iced_core::Renderer as _;
        self.inhalt.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
        if tree.state.downcast_ref::<Zustand>().fokussiert {
            renderer.fill_quad(
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: Border { color: thema::akzent(), width: 2.0, radius: 6.0.into() },
                    ..renderer::Quad::default()
                },
                Color::TRANSPARENT,
            );
        }
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
    ) -> Option<overlay::Element<'b, Message, Theme, iced::Renderer>> {
        self.inhalt.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

impl<'a, Message: 'a> From<Fokus<'a, Message>> for Element<'a, Message> {
    fn from(f: Fokus<'a, Message>) -> Self {
        Element::new(f)
    }
}

// ---------------------------------------------------------------- Ersatz für button und toggler

/// Knopf mit Tastaturfokus; gleiche Methoden wie `iced::widget::button`.
pub struct Knopf<'a, M>(Button<'a, M>);

/// Wie `iced::widget::button`, aber per Tab/F6 erreichbar.
pub fn button<'a, M: Clone + 'a>(inhalt: impl Into<Element<'a, M>>) -> Knopf<'a, M> {
    Knopf(iced::widget::button(inhalt))
}

impl<'a, M: Clone + 'a> Knopf<'a, M> {
    pub fn padding(self, p: impl Into<Padding>) -> Self {
        Self(self.0.padding(p))
    }
    pub fn width(self, w: impl Into<Length>) -> Self {
        Self(self.0.width(w))
    }
    pub fn height(self, h: impl Into<Length>) -> Self {
        Self(self.0.height(h))
    }
    pub fn style(self, f: impl Fn(&Theme, iced::widget::button::Status) -> iced::widget::button::Style + 'a) -> Self {
        Self(self.0.style(f))
    }
    pub fn on_press(self, m: M) -> Self {
        Self(self.0.on_press(m))
    }
    pub fn on_press_maybe(self, m: Option<M>) -> Self {
        Self(self.0.on_press_maybe(m))
    }
}

impl<'a, M: Clone + 'a> From<Knopf<'a, M>> for Element<'a, M> {
    fn from(k: Knopf<'a, M>) -> Self {
        Fokus::neu(k.0, true).into()
    }
}

/// Schalter mit Tastaturfokus; gleiche Methoden wie `iced::widget::toggler`.
pub struct Schalter<'a, M>(iced::widget::Toggler<'a, M>);

/// Wie `iced::widget::toggler`, aber per Tab/F6 erreichbar.
pub fn toggler<'a, M: 'a>(an: bool) -> Schalter<'a, M> {
    Schalter(iced::widget::toggler(an))
}

impl<'a, M: 'a> Schalter<'a, M> {
    pub fn label(self, l: impl iced_core::text::IntoFragment<'a>) -> Self {
        Self(self.0.label(l))
    }
    pub fn size(self, s: impl Into<Pixels>) -> Self {
        Self(self.0.size(s))
    }
    pub fn text_size(self, s: impl Into<Pixels>) -> Self {
        Self(self.0.text_size(s))
    }
    pub fn on_toggle(self, f: impl Fn(bool) -> M + 'a) -> Self {
        Self(self.0.on_toggle(f))
    }
}

impl<'a, M: 'a> From<Schalter<'a, M>> for Element<'a, M> {
    fn from(s: Schalter<'a, M>) -> Self {
        Fokus::neu(s.0, true).into()
    }
}

// ---------------------------------------------------------------- Pfeiltasten für Regler und Listen

/// Macht einen Regler per Pfeiltasten bedienbar (links/unten = weniger, rechts/oben = mehr).
pub fn regler_mit_pfeilen<'a, M: 'a>(
    regler: impl Into<Element<'a, M>>,
    aktuell: u32,
    schritt: u32,
    min: u32,
    max: u32,
    aendern: impl Fn(u32) -> M + 'a,
) -> Element<'a, M> {
    Fokus::neu(regler, false)
        .tasten(move |taste, _| match taste {
            Key::Named(Named::ArrowLeft | Named::ArrowDown) => Some(aendern(aktuell.saturating_sub(schritt).max(min))),
            Key::Named(Named::ArrowRight | Named::ArrowUp) => Some(aendern((aktuell + schritt).min(max))),
            _ => None,
        })
        .into()
}

/// Macht eine Auswahlliste per Pfeiltasten bedienbar (springt zum vorherigen/nächsten Eintrag).
pub fn liste_mit_pfeilen<'a, M: 'a, T: Clone + PartialEq + 'a>(
    liste: impl Into<Element<'a, M>>,
    optionen: Vec<T>,
    aktuell: Option<T>,
    waehlen: impl Fn(T) -> M + 'a,
) -> Element<'a, M> {
    Fokus::neu(liste, false)
        .tasten(move |taste, _| {
            let pos = aktuell.as_ref().and_then(|a| optionen.iter().position(|o| o == a)).unwrap_or(0) as i32;
            let schritt = match taste {
                Key::Named(Named::ArrowUp | Named::ArrowLeft) => -1,
                Key::Named(Named::ArrowDown | Named::ArrowRight) => 1,
                _ => return None,
            };
            optionen.get((pos + schritt).clamp(0, optionen.len() as i32 - 1) as usize).cloned().map(&waehlen)
        })
        .into()
}

// Kennung für spätere gezielte Fokussierung
#[allow(dead_code)]
pub fn id(name: &'static str) -> widget::Id {
    widget::Id::new(name)
}
