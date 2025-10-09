use masonry::{
    kurbo::{Affine, Circle, PathEl, Point, Stroke, Vec2},
    peniko::color::OpaqueColor,
    vello::Scene,
};
use wi_core::{AlignCell, Cursor, GraphWidgetCell, Port, Side, SidedPort, WCursor, WPort};

use super::{core::GraphCore, node::Node};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Anchor {
    Fixed,
    Node(usize),
    Edge(WPort<GraphCore>, WPort<GraphCore>),
}

impl Anchor {
    fn get(self, graph: &GraphCore) -> Point {
        match self {
            Self::Fixed => Point::ZERO,
            Self::Node(n) => graph.nodes[&n].pos,
            Self::Edge(i, o) => {
                let Port(from_node, from_port) = i;
                let Port(to_node, to_port) = o;

                let from_node = &graph.nodes[&from_node];
                let to_node = &graph.nodes[&to_node];

                from_node.edge_midpoint(from_port, to_node, to_port)
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    saved_port: Option<(Side, Anchor, Vec2)>,
    saved_edge: Option<(Anchor, Vec2)>,
    anchor: Anchor,
    offs: Vec2,
}

impl Cell {
    #[inline]
    pub fn port_target(&self, graph: &GraphCore, side: Side) -> Point {
        let (anchor, offs) = self
            .saved_port
            .and_then(|(s, a, o)| (s == side).then_some((a, o)))
            .unwrap_or((self.anchor, self.offs));

        anchor.get(graph) + offs
    }

    #[inline]
    pub fn edge_target(&self, graph: &GraphCore) -> Point {
        let (anchor, offs) = self.saved_edge.unwrap_or((self.anchor, self.offs));

        anchor.get(graph) + offs
    }

    #[cfg_attr(not(debug_assertions), expect(unused_method))]
    pub fn debug(&self, graph: &GraphCore, scene: &mut Scene, tf: Affine) {
        let Self {
            saved_port,
            saved_edge,
            anchor,
            offs,
        } = *self;

        if let Some((_, anchor, offs)) = saved_port {
            let p = anchor.get(graph) + offs;
            scene.stroke(
                &Stroke::new(1.0),
                tf,
                OpaqueColor::from_rgb8(0x00, 0xff, 0x00).with_alpha(0.5),
                None,
                &Circle::new(p, 8.0),
            );
        }

        if let Some((anchor, offs)) = saved_edge {
            let p = anchor.get(graph) + offs;
            scene.stroke(
                &Stroke::new(1.0),
                tf,
                OpaqueColor::from_rgb8(0x00, 0xff, 0x00).with_alpha(0.5),
                None,
                &Circle::new(p, 8.0),
            );
        }

        let p = anchor.get(graph) + offs;
        scene.stroke(
            &Stroke::new(1.0),
            tf,
            OpaqueColor::from_rgb8(0x00, 0xff, 0x00).with_alpha(0.5),
            None,
            &[
                PathEl::MoveTo(p + Vec2::new(-36.0, 0.0)),
                PathEl::LineTo(p + Vec2::new(-24.0, 0.0)),
                PathEl::MoveTo(p + Vec2::new(24.0, 0.0)),
                PathEl::LineTo(p + Vec2::new(36.0, 0.0)),
                PathEl::MoveTo(p + Vec2::new(0.0, -36.0)),
                PathEl::LineTo(p + Vec2::new(0.0, -24.0)),
                PathEl::MoveTo(p + Vec2::new(0.0, 24.0)),
                PathEl::LineTo(p + Vec2::new(0.0, 36.0)),
            ],
        );
    }
}

impl GraphWidgetCell<GraphCore> for Cell {
    fn of_cursor(widget: &GraphCore, cursor: &WCursor<GraphCore>) -> Self {
        match *cursor {
            Cursor::Node(n) => {
                let node = &widget.nodes[&n];
                Self {
                    saved_port: None,
                    saved_edge: None,
                    anchor: Anchor::Node(n),
                    offs: Vec2::new(node.rect().size().width * 0.5, Node::PORT_Y_OFFS),
                }
            },
            Cursor::Port(SidedPort(s, Port(n, p))) => {
                let node = &widget.nodes[&n];
                let anchor = Anchor::Node(n);
                let offs = node.port_pos(p, s) - node.pos;
                Self {
                    saved_port: Some((s, anchor, offs)),
                    saved_edge: None,
                    anchor,
                    offs,
                }
            },
            Cursor::Edge(e) => {
                let anchor = Anchor::Edge(e.from, e.to);
                let offs = Vec2::ZERO;

                Self {
                    saved_port: None,
                    saved_edge: Some((anchor, offs)),
                    anchor,
                    offs,
                }
            },
            Cursor::FixedPoint(p) => Self {
                saved_port: None,
                saved_edge: None,
                anchor: Anchor::Fixed,
                offs: p.to_vec2(),
            },
        }
    }

    fn align_to_cursor(
        &mut self,
        widget: &GraphCore,
        cursor: &WCursor<GraphCore>,
        align: AlignCell,
    ) {
        let Self {
            saved_port,
            saved_edge,
            anchor,
            offs,
        } = self;
        let mut new = Self::of_cursor(widget, cursor);

        let diff = || {
            if new.anchor == *anchor {
                Vec2::ZERO
            } else {
                anchor.get(widget) - new.anchor.get(widget)
            }
        };

        if let Some(c) = new.saved_port {
            *saved_port = Some(c);
        }

        if let Some(c) = new.saved_edge {
            *saved_edge = Some(c);
        }

        match align {
            AlignCell::Overwrite => (),
            AlignCell::KeepRow => new.offs.y = offs.y + diff().y,
            AlignCell::KeepCol => new.offs.x = offs.x + diff().x,
        }

        *anchor = new.anchor;
        *offs = new.offs;
    }
}
