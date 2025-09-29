use masonry::{
    core::{
        keyboard::{Key, KeyState, NamedKey},
        EventCtx, Ime, KeyboardEvent, Modifiers, PointerButton, PointerEvent, TextEvent, Widget,
    },
    kurbo::{Circle, Stroke},
    peniko::{color::OpaqueColor, Fill},
};
use smallvec::smallvec;
use wi_core::{Cursor, GraphWidgetDriver};

use crate::widget::edge::Edge;

mod core;
mod drag;
mod edge;

#[derive(Debug)]
pub struct Graph {
    core: core::GraphCore,
    driver: GraphWidgetDriver<core::GraphCore>,
}

impl Graph {
    pub fn new(graph: &crate::GraphView) -> Self {
        Self {
            core: core::GraphCore::new(graph),
            driver: GraphWidgetDriver::new(wi_core::Cursor::Node(
                graph.nodes.keys().copied().min().unwrap_or(usize::MAX),
            )),
        }
    }
}

impl Widget for Graph {
    fn accepts_focus(&self) -> bool { true }

    fn accepts_pointer_interaction(&self) -> bool { true }

    fn accepts_text_input(&self) -> bool { true }

    fn register_children(&mut self, _ctx: &mut masonry::core::RegisterCtx) {}

    fn layout(
        &mut self,
        _ctx: &mut masonry::core::LayoutCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        bc: &masonry::core::BoxConstraints,
    ) -> masonry::kurbo::Size {
        bc.max()
    }

    fn paint(
        &mut self,
        ctx: &mut masonry::core::PaintCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        scene: &mut masonry::vello::Scene,
    ) {
        let transform = self.core.view_transform(ctx.size());

        for node in self.core.nodes.values() {
            for (i, port) in node.in_edges.iter().enumerate() {
                let Some(port) = port else { continue };
                let from = &self.core.nodes[&port.node];
                let from_pos = from.port_pos(port.port, true);
                let to_pos = node.port_pos(i, false);

                scene.stroke(
                    &Stroke::new(4.0),
                    transform,
                    OpaqueColor::from_rgb8(0x7f, 0x7f, 0x7f),
                    None,
                    &Edge::new(from_pos, to_pos),
                );
            }
        }

        for (&i, node) in &self.core.nodes {
            let rect = node.rect();

            scene.fill(
                Fill::NonZero,
                transform,
                OpaqueColor::from_rgb8(0x27, 0x27, 0x27).with_alpha(0.7),
                None,
                &rect,
            );

            scene.stroke(
                &Stroke::new(4.0),
                transform,
                if ctx.is_focus_target()
                    && let &Cursor::Node(n) = self.driver.cursor()
                    && n == i
                {
                    OpaqueColor::from_rgb8(0x90, 0x37, 0x22)
                } else {
                    OpaqueColor::from_rgb8(0x3a, 0x3a, 0x3a)
                },
                None,
                &rect,
            );

            for port in 0..node.in_edges.len() {
                scene.fill(
                    Fill::NonZero,
                    transform,
                    if ctx.is_focus_target()
                        && let &Cursor::InPort(n, p) = self.driver.cursor()
                        && n == i
                        && p == port
                    {
                        OpaqueColor::from_rgb8(0x90, 0x37, 0x22)
                    } else {
                        OpaqueColor::from_rgb8(0x9a, 0x9a, 0x9a)
                    },
                    None,
                    &Circle::new(node.port_pos(port, false), 6.0),
                );
            }

            for port in 0..node.out_edges.len() {
                scene.fill(
                    Fill::NonZero,
                    transform,
                    if ctx.is_focus_target()
                        && let &Cursor::InPort(n, p) = self.driver.cursor()
                        && n == i
                        && p == port
                    {
                        OpaqueColor::from_rgb8(0x90, 0x37, 0x22)
                    } else {
                        OpaqueColor::from_rgb8(0x9a, 0x9a, 0x9a)
                    },
                    None,
                    &Circle::new(node.port_pos(port, true), 6.0),
                );
            }
        }

        if let &Cursor::FixedPoint(p) = self.driver.cursor() {
            scene.fill(
                Fill::NonZero,
                transform,
                if ctx.is_focus_target() {
                    OpaqueColor::from_rgb8(0x90, 0x37, 0x22)
                } else {
                    OpaqueColor::from_rgb8(0xb3, 0xb3, 0xb3)
                },
                None,
                &Circle::new(p, 6.0),
            );
        }
    }

    // TODO: review
    fn accessibility_role(&self) -> accesskit::Role { accesskit::Role::ScrollView }

    fn accessibility(
        &mut self,
        _ctx: &mut masonry::core::AccessCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        _node: &mut accesskit::Node,
    ) {
        for _node in &self.core.nodes {
            // TODO
        }
    }

    fn children_ids(&self) -> smallvec::SmallVec<[masonry::core::WidgetId; 16]> { smallvec![] }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Named(NamedKey::Escape),
                ..
            }) if self.core.pan.in_drag() || self.core.in_node_drag() => {
                self.core.pan.cancel_drag(None, ctx);
                self.core.cancel_node_drag(None, ctx);
            },
            TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Character(s),
                modifiers,
                is_composing: false,
                ..
            }) => self
                .driver
                .handle_char_input(&mut self.core, s, modifiers, ctx),
            TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Named(k),
                modifiers,
                is_composing: false,
                ..
            }) => self
                .driver
                .handle_named_keypress(&mut self.core, k, modifiers, ctx),
            TextEvent::Ime(Ime::Commit(s)) => {
                self.driver
                    .handle_char_input(&mut self.core, s, &Modifiers::empty(), ctx);
            },
            _ => return,
        }
        ctx.set_handled();
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        // TODO: how do i do this differently
        ctx.request_focus();

        match event {
            PointerEvent::Down {
                button: Some(PointerButton::Auxiliary),
                pointer,
                state,
            } => {
                if state.buttons == PointerButton::Auxiliary.into()
                    && state.modifiers.difference(Modifiers::CONTROL) == Modifiers::empty()
                {
                    self.core.pan.begin_drag(*pointer, state);
                } else {
                    self.core.pan.cancel_drag(Some(pointer), ctx);
                }
            },
            PointerEvent::Down {
                button: Some(PointerButton::Primary),
                pointer,
                state,
            } => {
                if state.buttons == PointerButton::Primary.into()
                    && state.modifiers == Modifiers::empty()
                {
                    self.core.begin_node_drag(*pointer, state, ctx);
                } else {
                    self.core.cancel_node_drag(Some(pointer), ctx);
                }
            },
            PointerEvent::Move(u) => {
                self.core
                    .pan
                    .update_drag(&u.pointer, &u.current, &self.core.zoom, ctx);
                self.core.update_node_drag(&u.pointer, &u.current, ctx);
            },
            PointerEvent::Up {
                button: Some(PointerButton::Auxiliary),
                pointer,
                state,
            } => {
                self.core
                    .pan
                    .complete_drag(pointer, state, &self.core.zoom, ctx);
            },
            PointerEvent::Up {
                button: Some(PointerButton::Primary),
                pointer,
                state,
            } => {
                self.core.complete_node_drag(pointer, state, ctx);
            },
            PointerEvent::Cancel(i) => {
                self.core.pan.cancel_drag(Some(i), ctx);
                self.core.cancel_node_drag(Some(i), ctx);
            },
            PointerEvent::Scroll {
                pointer: _,
                delta,
                state,
            } => {
                const M_EMPTY: Modifiers = Modifiers::empty();

                match state.modifiers {
                    Modifiers::CONTROL => self.core.zoom.scroll(delta, ctx),
                    M_EMPTY => self.core.pan.scroll(delta, false, ctx),
                    Modifiers::SHIFT => self.core.pan.scroll(delta, true, ctx),
                    _ => (),
                }
            },
            _ => (),
        }
    }
}
