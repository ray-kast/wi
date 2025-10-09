use masonry::kurbo::{Point, Vec2};
use wi_core::{AlignCell, Cursor, GraphWidgetCell, Port, SidedPort, WCursor, WPort};

use super::{core::GraphCore, node::Node};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Anchor {
    Fixed,
    Node(usize),
    Edge(WPort<GraphCore>, WPort<GraphCore>),
}

impl Anchor {
    pub fn get(self, graph: &GraphCore) -> Point {
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
    anchor: Anchor,
    offs: Vec2,
}

impl Cell {
    #[inline]
    pub fn point(&self, graph: &GraphCore) -> Point { self.anchor.get(graph) + self.offs }
}

impl GraphWidgetCell<GraphCore> for Cell {
    fn of_cursor(widget: &GraphCore, cursor: &WCursor<GraphCore>) -> Self {
        let (anchor, offs) = match *cursor {
            Cursor::Node(n) => {
                let node = &widget.nodes[&n];
                (
                    Anchor::Node(n),
                    Vec2::new(node.rect().size().width * 0.5, Node::PORT_Y_OFFS),
                )
            },
            Cursor::Port(SidedPort(s, Port(n, p))) => {
                let node = &widget.nodes[&n];
                (Anchor::Node(n), node.port_pos(p, s) - node.pos)
            },
            Cursor::Edge(e) => (Anchor::Edge(e.from, e.to), Vec2::ZERO),
            Cursor::FixedPoint(p) => (Anchor::Fixed, p.to_vec2()),
        };

        Self { anchor, offs }
    }

    fn align_to_cursor(
        &mut self,
        widget: &GraphCore,
        cursor: &WCursor<GraphCore>,
        align: AlignCell,
    ) {
        let mut new = Self::of_cursor(widget, cursor);

        let diff = || {
            if new.anchor == self.anchor {
                Vec2::ZERO
            } else {
                self.anchor.get(widget) - new.anchor.get(widget)
            }
        };

        match align {
            AlignCell::Overwrite => (),
            AlignCell::KeepRow => new.offs.y = self.offs.y + diff().y,
            AlignCell::KeepCol => new.offs.x = self.offs.x + diff().x,
        }

        *self = new;
    }
}
