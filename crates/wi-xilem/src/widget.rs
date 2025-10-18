use masonry::{
    core::{
        keyboard::{Key, KeyState, NamedKey},
        render_text, EventCtx, Ime, KeyboardEvent, PaintCtx, PointerButton, PointerEvent, StyleSet,
        TextEvent, Update, Widget,
    },
    kurbo::{Affine, Circle, Point, Rect, Size, Stroke, Vec2},
    parley::{Alignment, AlignmentOptions, GenericFamily, Layout},
    peniko::{color::OpaqueColor, BlendMode, Brush, Fill},
    vello::Scene,
};
use petgraph::{
    graph::NodeIndex,
    visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
};
use smallvec::smallvec;
use wi_core::{
    modifiers::{M_CTRL, M_NONE, M_SHIFT},
    Cursor, GraphWidgetDriver, Port, Side, SidedPort, WPort,
};

use self::{core::EditorCore, edge::Edge};
use crate::{
    graph::{GraphMarker, LargeNode, NodeKind, NodeType, WidgetNode},
    widget::node::{NodeExt, NodeWidget},
};

mod cell;
mod core;
mod drag;
mod edge;
mod node;
mod status;
mod view;

#[expect(missing_debug_implementations, reason = "WidgetPod doesn't impl Debug")]
pub struct GraphEditor<G: GraphMarker> {
    core: EditorCore<G>,
    driver: GraphWidgetDriver<EditorCore<G>>,
    viewport: Rect,
}

impl<G: GraphMarker> GraphEditor<G> {
    #[inline]
    #[must_use]
    pub fn new(graph: &crate::GraphEditor<G>) -> Self {
        let core = EditorCore::new(graph);
        Self {
            driver: GraphWidgetDriver::new(&core),
            core,
            viewport: Rect::ZERO,
        }
    }

    fn paint_port(
        ctx: &mut PaintCtx,
        scene: &mut Scene,
        tf: Affine,
        pos: Point,
        focused: bool,
        weight: f64,
    ) {
        scene.fill(
            Fill::NonZero,
            tf,
            if ctx.is_focus_target() && focused {
                OpaqueColor::from_rgb8(0x90, 0x37, 0x22)
            } else {
                OpaqueColor::from_rgb8(0x9a, 0x9a, 0x9a)
            },
            None,
            &Circle::new(pos, 6.0 * weight.clamp(1.0, 2.0)),
        );
    }

    #[expect(
        clippy::too_many_lines,
        reason = "The logic here would be difficult to refactor"
    )]
    fn paint_node(
        ctx: &mut PaintCtx,
        scene: &mut Scene,
        tf: Affine,
        node: (NodeIndex<G::Index>, &NodeType<G>),
        focus_node: Option<NodeIndex<G::Index>>,
        focus_port: Option<(Side, &WPort<EditorCore<G>>)>,
        weight: f64,
    ) {
        // HACK: this should be in layout()
        fn hack_layout_text(
            s: &str,
            mut b: masonry::parley::RangedBuilder<'_, masonry::core::BrushIndex>,
            max_advance: f32,
            alignment: Alignment,
        ) -> Layout<masonry::core::BrushIndex> {
            let mut styles = StyleSet::new(node::PORT_HEIGHT_32);
            styles.insert(GenericFamily::SystemUi.into());
            let mut layout = Layout::new();
            styles
                .inner()
                .values()
                .for_each(|v| b.push_default(v.to_owned()));

            b.build_into(&mut layout, s);
            layout.break_all_lines(Some(max_advance));
            layout.align(Some(max_advance), alignment, AlignmentOptions::default());
            layout
        }

        let (id, node) = node;
        let rect = node.rect();

        let rounding_weight = weight.clamp(0.5, 2.0);
        let clip = match &node.kind {
            NodeKind::Widget(_) => rect.to_rounded_rect(rect.height()),
            NodeKind::Small(_) | NodeKind::Large(_) => rect.to_rounded_rect(6.0 * rounding_weight),
        };

        scene.push_layer(BlendMode::default(), 1.0, tf, &clip);

        scene.fill(
            Fill::NonZero,
            tf,
            OpaqueColor::from_rgb8(0x27, 0x27, 0x27).with_alpha(0.7),
            None,
            &rect,
        );

        match &node.kind {
            NodeKind::Widget(WidgetNode { widget, .. }) => (),
            NodeKind::Small(_) => {
                let offs = node.name_rect().origin().x;
                scene.fill(
                    Fill::NonZero,
                    tf,
                    OpaqueColor::from_rgb8(0x20, 0x73, 0x20),
                    None,
                    &Rect::from_origin_size(
                        node.position + Vec2::new(offs, 0.0),
                        Size::new(rect.width() - offs * 2.0, rect.height()),
                    ),
                );
            },
            NodeKind::Large(_) => {
                // HACK: Using the bottom padding as a shorthand for "inner padding" is bad
                scene.fill(
                    Fill::NonZero,
                    tf,
                    OpaqueColor::from_rgb8(0x20, 0x73, 0x20),
                    None,
                    &Rect::from_origin_size(
                        node.position,
                        Size::new(rect.width(), node.padding().y0 - node.padding().y1),
                    ),
                );
            },
        }

        let name_rect = node.name_rect();
        let (fcx, lcx) = ctx.text_contexts();

        {
            #[expect(clippy::cast_possible_truncation)]
            let layout = hack_layout_text(
                &node.name,
                lcx.ranged_builder(fcx, &node.name, 1.0, true),
                name_rect.width() as f32,
                match node.kind {
                    NodeKind::Widget(WidgetNode {
                        input: Some(_),
                        output: None,
                        ..
                    }) => Alignment::Left,
                    NodeKind::Widget(WidgetNode {
                        input: None,
                        output: Some(_),
                        ..
                    }) => Alignment::Right,
                    _ => Alignment::Middle,
                },
            );

            render_text(
                scene,
                tf * Affine::translate(node.position.to_vec2() + name_rect.origin().to_vec2()),
                &layout,
                &[Brush::Solid(masonry::theme::TEXT_COLOR)],
                true,
            );
        }

        scene.pop_layer();

        if let NodeKind::Large(LargeNode { inputs, outputs }) = &node.kind {
            let padding = node.padding();

            let tf =
                tf * Affine::translate(node.position.to_vec2() + Vec2::new(padding.x0, padding.y0));
            #[expect(clippy::cast_possible_truncation)]
            let width = node.inner_size().width as f32;

            for (i, (input, widget)) in inputs.iter().enumerate() {
                let layout = hack_layout_text(
                    &input.name,
                    lcx.ranged_builder(fcx, &input.name, 1.0, true),
                    width,
                    Alignment::Left,
                );

                render_text(
                    scene,
                    tf * Affine::translate(Vec2::new(
                        0.0,
                        node.port_label_y(
                            i.try_into().unwrap_or_else(|_| unreachable!()),
                            Side::In,
                        ),
                    )),
                    &layout,
                    &[Brush::Solid(masonry::theme::TEXT_COLOR)],
                    true,
                );
            }

            for (i, output) in outputs.iter().enumerate() {
                let layout = hack_layout_text(
                    &output.name,
                    lcx.ranged_builder(fcx, &output.name, 1.0, true),
                    width,
                    Alignment::Right,
                );

                render_text(
                    scene,
                    tf * Affine::translate(Vec2::new(
                        0.0,
                        node.port_label_y(
                            i.try_into().unwrap_or_else(|_| unreachable!()),
                            Side::Out,
                        ),
                    )),
                    &layout,
                    &[Brush::Solid(masonry::theme::TEXT_COLOR)],
                    true,
                );
            }
        }

        if ctx.is_focus_target() && focus_node == Some(id) {
            scene.stroke(
                &Stroke::new(4.0 * weight.max(0.5)),
                tf,
                OpaqueColor::from_rgb8(0x90, 0x37, 0x22),
                None,
                &clip,
            );
        }

        for port in 0..node.in_arity() {
            Self::paint_port(
                ctx,
                scene,
                tf,
                node.port_pos(port, Side::In),
                focus_port == Some((Side::In, &Port(id, port))),
                weight,
            );
        }

        for port in 0..node.out_arity() {
            Self::paint_port(
                ctx,
                scene,
                tf,
                node.port_pos(port, Side::Out),
                focus_port == Some((Side::Out, &Port(id, port))),
                weight,
            );
        }
    }

    fn paint_edge(
        ctx: &mut PaintCtx,
        scene: &mut Scene,
        tf: Affine,
        focused: bool,
        from: (&NodeType<G>, u16),
        to: (&NodeType<G>, u16),
        weight: f64,
    ) {
        scene.stroke(
            &Stroke::new(4.0 * weight.max(1.0)),
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
                24.0,
                from.1 < from.0.out_arity() / 2,
            ),
        );
    }
}

impl<G: GraphMarker + 'static> Widget for GraphEditor<G> {
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
        let weight = self.core.zoom.scale().recip();

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

        for edge in self.core.graph().edge_references() {
            let from = edge.source();
            let to = edge.target();
            let &crate::graph::Edge { from_port, to_port } = edge.weight();

            if !focus_edge
                .is_some_and(|e| (e.from, e.to) == (Port(from, from_port), Port(to, to_port)))
            {
                Self::paint_edge(
                    ctx,
                    scene,
                    tf,
                    false,
                    (&self.core.graph()[from], from_port),
                    (&self.core.graph()[to], to_port),
                    weight,
                );
            }
        }

        if let Some(e) = focus_edge {
            Self::paint_edge(
                ctx,
                scene,
                tf,
                true,
                (&self.core.graph()[e.from.0], e.from.1),
                (&self.core.graph()[e.to.0], e.to.1),
                weight,
            );
        }

        for (id, node) in self.core.graph().node_references() {
            Self::paint_node(ctx, scene, tf, (id, node), focus_node, focus_port, weight);
        }

        if self.driver.view_debug() {
            cell::debug(self.driver.cursor_cell(), &self.core, scene, tf, weight);
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
                &Circle::new(p, 6.0 * weight),
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
        for _node in self.core.graph().node_weights() {
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
