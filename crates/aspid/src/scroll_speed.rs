//! A pass-through wrapper that scales mouse-wheel deltas before its child sees them.
//!
//! iced's [`scrollable`] hardcodes its scroll speed (one wheel line → 60px, raw pixel
//! deltas → 1×) with a `// TODO: Configurable speed/friction`. On platforms that emit many
//! small high-resolution pixel deltas, that 1× feels sluggish next to other apps. Wrapping
//! a scrollable in [`scroll_speed`] multiplies the wheel delta on the way down, so the
//! inner scrollable moves further per notch without any forked-fork of iced.
//!
//! [`scrollable`]: iced::widget::scrollable

use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{layout, mouse, overlay, renderer, Clipboard, Layout, Shell, Widget};
use iced::mouse::ScrollDelta;
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// Wrap `content` so wheel scrolling moves `factor`× as far.
pub fn scroll_speed<'a, Message, Theme, Renderer>(
    content: impl Into<Element<'a, Message, Theme, Renderer>>,
    factor: f32,
) -> Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: renderer::Renderer + 'a,
{
    Element::new(ScrollSpeed {
        content: content.into(),
        factor,
    })
}

struct ScrollSpeed<'a, Message, Theme, Renderer> {
    content: Element<'a, Message, Theme, Renderer>,
    factor: f32,
}

fn scale(delta: ScrollDelta, factor: f32) -> ScrollDelta {
    match delta {
        ScrollDelta::Lines { x, y } => ScrollDelta::Lines {
            x: x * factor,
            y: y * factor,
        },
        ScrollDelta::Pixels { x, y } => ScrollDelta::Pixels {
            x: x * factor,
            y: y * factor,
        },
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer>
    for ScrollSpeed<'_, Message, Theme, Renderer>
where
    Renderer: renderer::Renderer,
{
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let child = self.content.as_widget_mut();
        if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
            let scaled = Event::Mouse(mouse::Event::WheelScrolled {
                delta: scale(*delta, self.factor),
            });
            child.update(
                &mut tree.children[0],
                &scaled,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        } else {
            child.update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
