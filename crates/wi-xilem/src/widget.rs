use std::collections::HashMap;

use masonry::{
    core::{
        keyboard::{Key, KeyState},
        Ime, KeyboardEvent, TextEvent, Widget,
    },
    kurbo::{Circle, PathEl, Point, Rect, Size, Stroke},
    peniko::{color::OpaqueColor, Fill},
};
use smallvec::smallvec;
use tracing::info;
use xilem::{Affine, Vec2};

use crate::Port;

struct Node {
    pos: Point,
    in_edges: Vec<Option<Port>>,
    out_arity: usize,
}

impl Node {
    fn size(&self) -> Size {
        Size::new(
            64.0,
            16.0 * (self.in_edges.len().max(self.out_arity) as f64),
        )
    }

    fn rect(&self) -> Rect { Rect::from_origin_size(self.pos, self.size()) }

    fn port_pos(&self, idx: usize, out: bool) -> Point {
        let mut pos = self.pos + Vec2::new(0.0, 8.0) + Vec2::new(0.0, 16.0) * (idx as f64);

        if out {
            pos.x += self.size().width;
        }

        pos
    }
}

pub struct Graph {
    nodes: HashMap<usize, Node>,
    focus_node: usize,
}

impl Graph {
    pub fn new(graph: &crate::GraphView) -> Self {
        Self {
            nodes: graph
                .nodes
                .iter()
                .map(|(&i, n)| {
                    (i, Node {
                        pos: n.pos,
                        in_edges: n.in_edges.clone(),
                        out_arity: n.out_arity,
                    })
                })
                .collect(),
            focus_node: graph.nodes.keys().copied().min().unwrap_or(usize::MAX),
        }
    }
}

impl Widget for Graph {
    fn accepts_focus(&self) -> bool { true }

    fn accepts_text_input(&self) -> bool { true }

    fn register_children(&mut self, ctx: &mut masonry::core::RegisterCtx) {}

    fn layout(
        &mut self,
        ctx: &mut masonry::core::LayoutCtx,
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
        for node in self.nodes.values() {
            for (i, port) in node.in_edges.iter().enumerate() {
                let Some(port) = port else { continue };
                let from = &self.nodes[&port.node];
                let from_pos = from.port_pos(port.port, true);
                let to_pos = node.port_pos(i, false);

                scene.stroke(
                    &Stroke::new(4.0),
                    Affine::IDENTITY,
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

        for (&i, node) in &self.nodes {
            let rect = node.rect();

            scene.stroke(
                &Stroke::new(4.0),
                Affine::IDENTITY,
                if self.focus_node == i {
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
                    Affine::IDENTITY,
                    OpaqueColor::from_rgb8(0x9a, 0x9a, 0x9a),
                    None,
                    &Circle::new(node.port_pos(port, false), 4.0),
                );
            }

            for port in 0..node.out_arity {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
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
        ctx: &mut masonry::core::AccessCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        node: &mut accesskit::Node,
    ) {
        for node in &self.nodes {
            // TODO
        }
    }

    fn children_ids(&self) -> smallvec::SmallVec<[masonry::core::WidgetId; 16]> { smallvec![] }

    fn on_text_event(
        &mut self,
        ctx: &mut masonry::core::EventCtx,
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
            }) => info!(?s, ?modifiers),
            TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Named(k),
                modifiers,
                is_composing: false,
                ..
            }) => info!(?k, ?modifiers),
            TextEvent::Ime(Ime::Commit(s)) => info!(?s),
            _ => return,
        }
        ctx.set_handled();
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut masonry::core::EventCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        _event: &masonry::core::PointerEvent,
    ) {
        // TODO: how do i do this differently
        ctx.request_focus();
    }
}
