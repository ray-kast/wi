use std::{cell::RefCell, collections::HashMap, rc::Rc};

use masonry::kurbo::{Point, Rect, Size};
use xilem::Vec2;

use crate::graph::GraphView;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PortId {
    pub node: usize,
    pub port: usize,
}

pub struct NodeData {
    pub pos: Point,
}

impl NodeData {
    pub fn size(&self) -> Size { Size::new(64.0, 96.0) }
}

pub struct GraphData {
    pub nodes: HashMap<usize, NodeData>,
}

impl GraphData {
    pub fn node_rect(&self, node: usize) -> Rect {
        let node = &self.nodes[&node];

        Rect::from_points(node.pos, node.pos + node.size().to_vec2())
    }

    pub fn port_pos(&self, port: PortId, out: bool) -> Point {
        let node = &self.nodes[&port.node];
        let mut pos = node.pos + Vec2::new(0.0, 16.0) * (port.port as f64);

        if out {
            pos.x += node.size().width;
        }

        pos
    }
}

pub type Shared = Rc<RefCell<GraphData>>;

pub fn shared(view: &GraphView) -> Shared {
    Rc::new(RefCell::new(GraphData {
        nodes: view
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| n.node().map(|n| (i, n)))
            .map(|(i, n)| (i, NodeData { pos: n.pos }))
            .collect(),
    }))
}
