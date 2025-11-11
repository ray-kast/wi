use std::sync::Arc;

pub use action::{GraphAction, GraphActionKind};
use masonry::{
    core::{
        keyboard::{Key, KeyState, NamedKey},
        render_text, EventCtx, Ime, KeyboardEvent, MutateCtx, PaintCtx, PointerButton,
        PointerButtonEvent, PointerEvent, PointerScrollEvent, StyleSet, TextEvent, Update, Widget,
    },
    kurbo::{Affine, Circle, Point, Rect, Size, Stroke, Vec2},
    parley::{Alignment, AlignmentOptions, GenericFamily, Layout},
    peniko::{color::OpaqueColor, BlendMode, Brush, Fill},
    vello::Scene,
};
use petgraph::{
    prelude::*,
    visit::{IntoEdgeReferences, IntoNodeReferences},
};
use smallvec::smallvec;
use wi_core::{
    modifiers::{M_CTRL, M_NONE, M_SHIFT},
    Cursor, GraphWidgetDriver, Port, Side, SidedPort, WPort,
};

use self::{core::EditorCore, edge::Edge};
use crate::{
    graph::{self, Graph, InputLabel, Node, NodeLabel, NodeStyle, OutputLabel},
    widget::node::NodeExt,
};

mod action;
mod cell;
mod core;
mod edge;
mod node;
mod status;
mod view;

#[expect(missing_debug_implementations, reason = "WidgetPod doesn't impl Debug")]
pub struct GraphEditor<N: Node> {
    core: EditorCore<N>,
    driver: GraphWidgetDriver<EditorCore<N>>,
    viewport: Rect,
}

impl<N: Node> GraphEditor<N> {
    #[inline]
    #[must_use]
    pub fn new(graph: Arc<Graph<N>>) -> Self {
        let core = EditorCore::new(graph);
        Self {
            driver: GraphWidgetDriver::new(&core),
            core,
            viewport: Rect::ZERO,
        }
    }

    #[inline]
    pub fn set_graph(&mut self, graph: Arc<Graph<N>>, cx: &mut MutateCtx) {
        self.core.set_graph(graph, cx);
        // TODO: fixup cursor and pan/zoom
    }

    fn paint_port(
        cx: &mut PaintCtx,
        scene: &mut Scene,
        tf: Affine,
        pos: Point,
        focused: bool,
        weight: f64,
    ) {
        scene.fill(
            Fill::NonZero,
            tf,
            if cx.is_focus_target() && focused {
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
        cx: &mut PaintCtx,
        scene: &mut Scene,
        tf: Affine,
        node: (NodeIndex, &N),
        focus_node: Option<NodeIndex>,
        focus_port: Option<(Side, &WPort<EditorCore<N>>)>,
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
        let clip = match node.style() {
            NodeStyle::Small => rect.to_rounded_rect(rect.height()),
            NodeStyle::Medium | NodeStyle::Large => rect.to_rounded_rect(6.0 * rounding_weight),
        };

        scene.push_layer(BlendMode::default(), 1.0, tf, &clip);

        scene.fill(
            Fill::NonZero,
            tf,
            OpaqueColor::from_rgb8(0x27, 0x27, 0x27).with_alpha(0.7),
            None,
            &rect,
        );

        match node.style() {
            NodeStyle::Small => (),
            NodeStyle::Medium => {
                let offs = node.name_rect().origin().x;
                scene.fill(
                    Fill::NonZero,
                    tf,
                    OpaqueColor::from_rgb8(0x20, 0x73, 0x20),
                    None,
                    &Rect::from_origin_size(
                        node.position() + Vec2::new(offs, 0.0),
                        Size::new(rect.width() - offs * 2.0, rect.height()),
                    ),
                );
            },
            NodeStyle::Large => {
                // HACK: Using the bottom padding as a shorthand for "inner padding" is bad
                scene.fill(
                    Fill::NonZero,
                    tf,
                    OpaqueColor::from_rgb8(0x20, 0x73, 0x20),
                    None,
                    &Rect::from_origin_size(
                        node.position(),
                        Size::new(rect.width(), node.padding().y0 - node.padding().y1),
                    ),
                );
            },
        }

        let name_rect = node.name_rect();
        let (fcx, lcx) = cx.text_contexts();

        match (node.name(), node.label()) {
            (s, NodeLabel::Name) | (_, NodeLabel::Text(s)) => {
                #[expect(clippy::cast_possible_truncation)]
                let layout = hack_layout_text(
                    &s,
                    lcx.ranged_builder(fcx, &s, 1.0, true),
                    name_rect.width() as f32,
                    match (node.style(), node.in_arity(), node.out_arity()) {
                        (NodeStyle::Small, 0, 0) => Alignment::Center,
                        (NodeStyle::Small, _, 0) => Alignment::Left,
                        (NodeStyle::Small, 0, _) => Alignment::Right,
                        _ => Alignment::Center,
                    },
                );

                render_text(
                    scene,
                    tf * Affine::translate(
                        node.position().to_vec2() + name_rect.origin().to_vec2(),
                    ),
                    &layout,
                    &[Brush::Solid(masonry::theme::TEXT_COLOR)],
                    true,
                );
            },
            (_, NodeLabel::Icon(i)) => todo!(),
            (_, NodeLabel::Widget(w)) => todo!(),
        }

        scene.pop_layer();

        if node.style() == NodeStyle::Large {
            let padding = node.padding();

            let tf = tf
                * Affine::translate(node.position().to_vec2() + Vec2::new(padding.x0, padding.y0));
            #[expect(clippy::cast_possible_truncation)]
            let width = node.inner_size().width as f32;

            for i in 0..node.in_arity() {
                let graph::Port { name, shape, label } = node.in_port(i);
                let label = match label {
                    InputLabel::Name => name,
                    InputLabel::Text(s) => s,
                    InputLabel::Widget(w) => todo!(),
                };

                let layout = hack_layout_text(
                    &label,
                    lcx.ranged_builder(fcx, &label, 1.0, true),
                    width,
                    Alignment::Left,
                );

                render_text(
                    scene,
                    tf * Affine::translate(Vec2::new(0.0, node.port_label_y(i, Side::In))),
                    &layout,
                    &[Brush::Solid(masonry::theme::TEXT_COLOR)],
                    true,
                );
            }

            for i in 0..node.out_arity() {
                let graph::Port { name, shape, label } = node.out_port(i);
                let label = match label {
                    OutputLabel::Name => name,
                    OutputLabel::Text(s) => s,
                };

                let layout = hack_layout_text(
                    &label,
                    lcx.ranged_builder(fcx, &label, 1.0, true),
                    width,
                    Alignment::Right,
                );

                render_text(
                    scene,
                    tf * Affine::translate(Vec2::new(0.0, node.port_label_y(i, Side::Out))),
                    &layout,
                    &[Brush::Solid(masonry::theme::TEXT_COLOR)],
                    true,
                );
            }
        }

        if cx.is_focus_target() && focus_node == Some(id) {
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
                cx,
                scene,
                tf,
                node.port_pos(port, Side::In),
                focus_port == Some((Side::In, &Port(id, port))),
                weight,
            );
        }

        for port in 0..node.out_arity() {
            Self::paint_port(
                cx,
                scene,
                tf,
                node.port_pos(port, Side::Out),
                focus_port == Some((Side::Out, &Port(id, port))),
                weight,
            );
        }
    }

    fn paint_edge(
        cx: &mut PaintCtx,
        scene: &mut Scene,
        tf: Affine,
        focused: bool,
        from: (&N, u16),
        to: (&N, u16),
        weight: f64,
    ) {
        scene.stroke(
            &Stroke::new(5.0 * weight.max(1.0)),
            tf,
            if cx.is_focus_target() && focused {
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

impl<N: Node + 'static> Widget for GraphEditor<N> {
    type Action = GraphAction<N>;

    fn accepts_focus(&self) -> bool { true }

    fn accepts_pointer_interaction(&self) -> bool { true }

    fn accepts_text_input(&self) -> bool { true }

    fn register_children(&mut self, cx: &mut masonry::core::RegisterCtx) {
        cx.register_child(&mut self.core.statusbar);
    }

    fn layout(
        &mut self,
        cx: &mut masonry::core::LayoutCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        bc: &masonry::core::BoxConstraints,
    ) -> masonry::kurbo::Size {
        let sb_size = cx.run_layout(&mut self.core.statusbar, &bc.loosen());
        let viewport_height = (bc.max().height - sb_size.height).max(0.0);

        cx.place_child(
            &mut self.core.statusbar,
            Point::new(0.0, bc.max().height - sb_size.height),
        );

        self.viewport =
            Rect::from_origin_size(Point::ZERO, Size::new(bc.max().width, viewport_height));

        bc.max()
    }

    fn paint(
        &mut self,
        cx: &mut PaintCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        scene: &mut Scene,
    ) {
        let tf = self.core.view_transform(cx.size());
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

        for edge in self.core.graph.edge_references() {
            let from = edge.source();
            let to = edge.target();
            let &graph::Edge { from_port, to_port } = edge.weight();

            if !focus_edge
                .is_some_and(|e| (e.from, e.to) == (Port(from, from_port), Port(to, to_port)))
            {
                Self::paint_edge(
                    cx,
                    scene,
                    tf,
                    false,
                    (&self.core.graph[from], from_port),
                    (&self.core.graph[to], to_port),
                    weight,
                );
            }
        }

        if let Some(e) = focus_edge {
            Self::paint_edge(
                cx,
                scene,
                tf,
                true,
                (&self.core.graph[e.from.0], e.from.1),
                (&self.core.graph[e.to.0], e.to.1),
                weight,
            );
        }

        for (id, node) in self.core.graph.node_references() {
            Self::paint_node(cx, scene, tf, (id, node), focus_node, focus_port, weight);
        }

        if self.driver.view_debug() {
            cell::debug(self.driver.cursor_cell(), &self.core, scene, tf, weight);
        }

        if let Some(p) = focus_point {
            scene.fill(
                Fill::NonZero,
                tf,
                if cx.is_focus_target() {
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
        _cx: &mut masonry::core::AccessCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        _node: &mut accesskit::Node,
    ) {
        for _node in self.core.graph.node_weights() {
            // TODO
        }
    }

    fn children_ids(&self) -> smallvec::SmallVec<[masonry::core::WidgetId; 16]> {
        smallvec![self.core.statusbar.id()]
    }

    fn update(
        &mut self,
        cx: &mut masonry::core::UpdateCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        event: &Update,
    ) {
        #[expect(clippy::single_match, reason = "for maintainability")]
        match event {
            Update::FocusChanged(_) => cx.request_render(),
            _ => (),
        }
    }

    fn on_text_event(
        &mut self,
        cx: &mut EventCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Named(NamedKey::Escape),
                ..
            }) if self.core.pan.in_drag() || self.core.in_node_drag() => {
                self.core.pan.cancel_drag(None, cx);
                self.core.cancel_node_drag(None, || cx.request_render());
            },
            &TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Character(ref s),
                modifiers,
                is_composing: false,
                ..
            }) => {
                if !self
                    .driver
                    .handle_char_input(&mut self.core, s, modifiers, cx)
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
                    .handle_named_keypress(&mut self.core, k, modifiers, cx)
                {
                    return;
                }
            },
            TextEvent::Ime(Ime::Commit(s)) => {
                self.driver.handle_char_input(&mut self.core, s, M_NONE, cx);
            },
            _ => return,
        }

        cx.set_handled();
    }

    fn on_pointer_event(
        &mut self,
        cx: &mut EventCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        // TODO: how do i do this differently
        cx.request_focus();

        match event {
            PointerEvent::Down(PointerButtonEvent {
                button: Some(PointerButton::Auxiliary),
                pointer,
                state,
            }) => {
                if state.buttons == PointerButton::Auxiliary.into()
                    && state.modifiers.difference(M_CTRL) == M_NONE
                {
                    self.core.pan.begin_drag(*pointer, state);
                    cx.capture_pointer();
                } else {
                    self.core.pan.cancel_drag(Some(pointer), cx);
                }
            },
            PointerEvent::Down(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                pointer,
                state,
            }) => {
                if state.buttons == PointerButton::Primary.into() && state.modifiers == M_NONE {
                    self.core.begin_node_drag(*pointer, state, cx);
                    cx.capture_pointer();
                } else {
                    self.core
                        .cancel_node_drag(Some(pointer), || cx.request_render());
                }
            },
            PointerEvent::Move(u) => {
                self.core
                    .pan
                    .update_drag(&u.pointer, &u.current, &self.core.zoom, cx);
                self.core.update_node_drag(&u.pointer, &u.current, cx);
            },
            PointerEvent::Up(PointerButtonEvent {
                button: Some(PointerButton::Auxiliary),
                pointer,
                state,
            }) => {
                self.core
                    .pan
                    .complete_drag(pointer, state, &self.core.zoom, cx);
            },
            PointerEvent::Up(PointerButtonEvent {
                button: Some(PointerButton::Primary),
                pointer,
                state,
            }) => {
                self.core.complete_node_drag(pointer, state, cx);
            },
            PointerEvent::Cancel(i) => {
                self.core.pan.cancel_drag(Some(i), cx);
                self.core.cancel_node_drag(Some(i), || cx.request_render());
            },
            PointerEvent::Scroll(PointerScrollEvent {
                pointer: _,
                delta,
                state,
            }) => match state.modifiers {
                M_NONE => self.core.pan.scroll(delta, false, &self.core.zoom, cx),
                M_CTRL => self.core.zoom.scroll(delta, cx),
                M_SHIFT => self.core.pan.scroll(delta, true, &self.core.zoom, cx),
                _ => return,
            },
            _ => return,
        }

        cx.set_handled();
    }
}
