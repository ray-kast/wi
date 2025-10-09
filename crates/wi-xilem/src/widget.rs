use masonry::{
    core::{
        keyboard::{Key, KeyState, NamedKey},
        EventCtx, Ime, KeyboardEvent, PaintCtx, PointerButton, PointerEvent, TextEvent, Update,
        Widget,
    },
    kurbo::{Affine, Circle, Point, Rect, Size, Stroke},
    peniko::{color::OpaqueColor, BlendMode, Fill},
    vello::Scene,
};
use smallvec::smallvec;
use wi_core::{
    modifiers::{M_CTRL, M_NONE, M_SHIFT},
    Cursor, GraphWidgetDriver, Port, Side, SidedPort, WPort,
};

use self::{core::GraphCore, edge::Edge, node::Node};

mod cell;
mod core;
mod drag;
mod edge;
mod node;
mod status;
mod view;

#[expect(missing_debug_implementations, reason = "WidgetPod doesn't impl Debug")]
pub struct Graph {
    core: GraphCore,
    driver: GraphWidgetDriver<GraphCore>,
    viewport: Rect,
}

impl Graph {
    #[inline]
    pub fn new(graph: &crate::GraphView) -> Self {
        let core = GraphCore::new(graph);
        Self {
            driver: GraphWidgetDriver::new(&core),
            core,
            viewport: Rect::ZERO,
        }
    }

    fn paint_port(ctx: &mut PaintCtx, scene: &mut Scene, tf: Affine, pos: Point, focused: bool) {
        scene.fill(
            Fill::NonZero,
            tf,
            if ctx.is_focus_target() && focused {
                OpaqueColor::from_rgb8(0x90, 0x37, 0x22)
            } else {
                OpaqueColor::from_rgb8(0x9a, 0x9a, 0x9a)
            },
            None,
            &Circle::new(pos, 6.0),
        );
    }

    fn paint_node(
        ctx: &mut PaintCtx,
        scene: &mut Scene,
        tf: Affine,
        id: usize,
        node: &Node,
        focus_node: Option<&usize>,
        focus_port: Option<(Side, &WPort<GraphCore>)>,
    ) {
        let rect = node.rect();

        scene.fill(
            Fill::NonZero,
            tf,
            OpaqueColor::from_rgb8(0x27, 0x27, 0x27).with_alpha(0.7),
            None,
            &rect,
        );

        scene.stroke(
            &Stroke::new(4.0),
            tf,
            if ctx.is_focus_target() && focus_node == Some(&id) {
                OpaqueColor::from_rgb8(0x90, 0x37, 0x22)
            } else {
                OpaqueColor::from_rgb8(0x3a, 0x3a, 0x3a)
            },
            None,
            &rect,
        );

        for port in 0..node.in_edges.len() {
            Self::paint_port(
                ctx,
                scene,
                tf,
                node.port_pos(port, Side::In),
                focus_port == Some((Side::In, &Port(id, port))),
            );
        }

        for port in 0..node.out_edges.len() {
            Self::paint_port(
                ctx,
                scene,
                tf,
                node.port_pos(port, Side::Out),
                focus_port == Some((Side::Out, &Port(id, port))),
            );
        }
    }

    fn paint_edge(
        ctx: &mut PaintCtx,
        scene: &mut Scene,
        tf: Affine,
        focused: bool,
        from: (&Node, usize),
        to: (&Node, usize),
    ) {
        scene.stroke(
            &Stroke::new(4.0),
            tf,
            if ctx.is_focus_target() && focused {
                OpaqueColor::from_rgb8(0x90, 0x37, 0x22)
            } else {
                OpaqueColor::from_rgb8(0x7f, 0x7f, 0x7f)
            },
            None,
            &Edge::new(
                from.0.port_pos(from.1, Side::Out),
                to.0.port_pos(to.1, Side::In),
                from.1 < from.0.out_edges.len() / 2,
            ),
        );
    }
}

impl Widget for Graph {
    fn accepts_focus(&self) -> bool { true }

    fn accepts_pointer_interaction(&self) -> bool { true }

    fn accepts_text_input(&self) -> bool { true }

    fn register_children(&mut self, ctx: &mut masonry::core::RegisterCtx) {
        ctx.register_child(&mut self.core.statusbar);
    }

    fn layout(
        &mut self,
        ctx: &mut masonry::core::LayoutCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        bc: &masonry::core::BoxConstraints,
    ) -> masonry::kurbo::Size {
        let sb_size = ctx.run_layout(&mut self.core.statusbar, &bc.loosen());
        let viewport_height = (bc.max().height - sb_size.height).max(0.0);

        ctx.place_child(
            &mut self.core.statusbar,
            Point::new(0.0, bc.max().height - sb_size.height),
        );

        self.viewport =
            Rect::from_origin_size(Point::ZERO, Size::new(bc.max().width, viewport_height));

        bc.max()
    }

    fn paint(
        &mut self,
        ctx: &mut PaintCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        scene: &mut Scene,
    ) {
        let tf = self.core.view_transform(ctx.size());

        scene.push_layer(BlendMode::default(), 1.0, Affine::IDENTITY, &self.viewport);

        let focus_node;
        let focus_port;
        let focus_edge;
        let focus_point;

        match self.driver.cursor() {
            &Cursor::Node(n) => {
                focus_node = Some(n);
                focus_port = None;
                focus_edge = None;
                focus_point = None;
            },
            &Cursor::Port(SidedPort(s, ref p)) => {
                focus_node = None;
                focus_port = Some((s, p));
                focus_edge = None;
                focus_point = None;
            },
            Cursor::Edge(e) => {
                focus_node = None;
                focus_port = Some(e.anchor_port());
                focus_edge = Some(e);
                focus_point = None;
            },
            &Cursor::FixedPoint(p) => {
                focus_node = None;
                focus_port = None;
                focus_edge = None;
                focus_point = Some(p);
            },
        }

        for (&node_id, node) in &self.core.nodes {
            for (i, port) in node.in_edges.iter().enumerate() {
                let Some(port) = port else { continue };

                if !focus_edge.is_some_and(|e| (e.from, e.to) == (*port, Port(node_id, i))) {
                    Self::paint_edge(
                        ctx,
                        scene,
                        tf,
                        false,
                        (&self.core.nodes[&port.0], port.1),
                        (node, i),
                    );
                }
            }
        }

        if let Some(e) = focus_edge {
            Self::paint_edge(
                ctx,
                scene,
                tf,
                true,
                (&self.core.nodes[&e.from.0], e.from.1),
                (&self.core.nodes[&e.to.0], e.to.1),
            );
        }

        for (&id, node) in &self.core.nodes {
            Self::paint_node(ctx, scene, tf, id, node, focus_node.as_ref(), focus_port);
        }

        if self.driver.view_debug() {
            cell::debug(self.driver.cursor_cell(), &self.core, scene, tf);
        }

        if let Some(p) = focus_point {
            scene.fill(
                Fill::NonZero,
                tf,
                if ctx.is_focus_target() {
                    OpaqueColor::from_rgb8(0x90, 0x37, 0x22)
                } else {
                    OpaqueColor::from_rgb8(0xb3, 0xb3, 0xb3)
                },
                None,
                &Circle::new(p, 6.0),
            );
        }

        scene.pop_layer();
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

    fn children_ids(&self) -> smallvec::SmallVec<[masonry::core::WidgetId; 16]> {
        smallvec![self.core.statusbar.id()]
    }

    fn update(
        &mut self,
        ctx: &mut masonry::core::UpdateCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        event: &Update,
    ) {
        #[expect(clippy::single_match, reason = "for maintainability")]
        match event {
            Update::FocusChanged(_) => ctx.request_render(),
            _ => (),
        }
    }

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
            }) => {
                if !self
                    .driver
                    .handle_char_input(&mut self.core, s, modifiers, ctx)
                {
                    return;
                }
            },
            &TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Named(k),
                modifiers,
                is_composing: false,
                ..
            }) => {
                if !self
                    .driver
                    .handle_named_keypress(&mut self.core, k, modifiers, ctx)
                {
                    return;
                }
            },
            TextEvent::Ime(Ime::Commit(s)) => {
                self.driver
                    .handle_char_input(&mut self.core, s, &M_NONE, ctx);
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
                    && state.modifiers.difference(M_CTRL) == M_NONE
                {
                    self.core.pan.begin_drag(*pointer, state);
                    ctx.capture_pointer();
                } else {
                    self.core.pan.cancel_drag(Some(pointer), ctx);
                }
            },
            PointerEvent::Down {
                button: Some(PointerButton::Primary),
                pointer,
                state,
            } => {
                if state.buttons == PointerButton::Primary.into() && state.modifiers == M_NONE {
                    self.core.begin_node_drag(*pointer, state, ctx);
                    ctx.capture_pointer();
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
            } => match state.modifiers {
                M_NONE => self.core.pan.scroll(delta, false, &self.core.zoom, ctx),
                M_CTRL => self.core.zoom.scroll(delta, ctx),
                M_SHIFT => self.core.pan.scroll(delta, true, &self.core.zoom, ctx),
                _ => return,
            },
            _ => return,
        }

        ctx.set_handled();
    }
}
