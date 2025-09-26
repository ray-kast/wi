use masonry::{
    core::{
        keyboard::{Key, KeyState},
        EventCtx, Ime, KeyboardEvent, Modifiers, PointerButton, PointerEvent, TextEvent, Widget,
    },
    kurbo::{Circle, PathEl, Stroke},
    peniko::{color::OpaqueColor, Fill},
};
use smallvec::smallvec;
use wi_core::GraphWidgetDriver;
use xilem::Vec2;

mod core;

#[derive(Debug)]
pub struct Graph {
    core: core::GraphCore,
    driver: GraphWidgetDriver<usize>,
}

impl Graph {
    pub fn new(graph: &crate::GraphView) -> Self {
        Self {
            core: core::GraphCore::new(graph),
            driver: GraphWidgetDriver::new(graph.nodes.keys().copied().min().unwrap_or(usize::MAX)),
        }
    }
}

impl Widget for Graph {
    fn accepts_focus(&self) -> bool { true }

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
                    &[
                        PathEl::MoveTo(from_pos),
                        PathEl::CurveTo(
                            from_pos + Vec2::new(16.0, 0.0),
                            to_pos + Vec2::new(-16.0, 0.0),
                            to_pos,
                        ),
                    ],
                );
            }
        }

        for (&i, node) in &self.core.nodes {
            let rect = node.rect();

            scene.stroke(
                &Stroke::new(4.0),
                transform,
                if *self.driver.focus_node() == i {
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
                    OpaqueColor::from_rgb8(0x9a, 0x9a, 0x9a),
                    None,
                    &Circle::new(node.port_pos(port, false), 4.0),
                );
            }

            for port in 0..node.out_edges.len() {
                scene.fill(
                    Fill::NonZero,
                    transform,
                    OpaqueColor::from_rgb8(0x9a, 0x9a, 0x9a),
                    None,
                    &Circle::new(node.port_pos(port, true), 4.0),
                );
            }
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
                if state.buttons == PointerButton::Auxiliary.into() {
                    self.core.pan.begin_drag(*pointer, state);
                } else {
                    self.core.pan.cancel_drag(pointer, ctx);
                }
            },
            PointerEvent::Move(u) => {
                self.core
                    .pan
                    .update_drag(&u.pointer, &u.current, &self.core.zoom, ctx);
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
            PointerEvent::Cancel(i) => self.core.pan.cancel_drag(i, ctx),
            PointerEvent::Scroll {
                pointer: _,
                delta,
                state,
            } => {
                if state.modifiers == Modifiers::CONTROL {
                    self.core.zoom.scroll(delta, ctx);
                }
            },
            _ => (),
        }
    }
}
