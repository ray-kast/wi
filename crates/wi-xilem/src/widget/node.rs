use masonry::kurbo::{Point, Rect, Size, Vec2};
use wi_core::{Side, WPort};

use super::core::GraphCore;

#[derive(Debug)]
pub struct Node {
    pub pos: Point,
    pub in_edges: Vec<Option<WPort<GraphCore>>>,
    pub out_edges: Vec<Vec<WPort<GraphCore>>>,
}

impl Node {
    pub const PORT_Y_OFFS: f64 = 20.0;

    #[inline]
    fn size(&self) -> Size {
        #[expect(clippy::cast_precision_loss, reason = "Necessary cast")]
        Size::new(
            128.0,
            16.0 + 24.0 * (self.in_edges.len().max(self.out_edges.len()).max(1) as f64),
        )
    }

    pub fn rect(&self) -> Rect { Rect::from_origin_size(self.pos, self.size()) }

    #[inline]
    pub fn port_offs(&self, idx: usize, side: Side) -> Vec2 {
        #[expect(clippy::cast_precision_loss, reason = "Necessary cast")]
        {
            Vec2::new(0.0, Self::PORT_Y_OFFS)
                + Vec2::new(0.0, 24.0) * idx as f64
                + f64::from(matches!(side, Side::Out)) * Vec2::new(self.size().width, 0.0)
        }
    }

    #[inline]
    pub fn port_pos(&self, idx: usize, side: Side) -> Point { self.pos + self.port_offs(idx, side) }

    pub fn edge_midpoint(&self, port: usize, to_node: &Node, to_port: usize) -> Point {
        ((self.port_pos(port, Side::Out).to_vec2() + to_node.port_pos(to_port, Side::In).to_vec2())
            * 0.5)
            .to_point()
    }
}
