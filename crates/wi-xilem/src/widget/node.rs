use masonry::kurbo::{Insets, Point, Rect, Size, Vec2};
use wi_core::Side;

use crate::graph::{Node, NodeKind};

pub trait NodeWidget {
    #[deprecated = "Don't use this directly"]
    fn pos(&self) -> Point;

    fn inner_size(&self) -> Size;

    fn padding(&self) -> Insets;

    fn name_rect(&self) -> Rect;

    fn port_label_y(&self, idx: u16, side: Side) -> f64;

    fn port_inner_offs(&self, idx: u16, side: Side) -> Vec2;
}

pub trait NodeExt: NodeWidget {
    #[inline]
    fn outer_size(&self) -> Size { self.inner_size() + self.padding().size() }

    #[expect(deprecated, reason = "Using pos()")]
    #[inline]
    fn rect(&self) -> Rect { Rect::from_origin_size(self.pos(), self.outer_size()) }

    #[expect(deprecated, reason = "Using pos()")]
    #[inline]
    fn port_pos(&self, idx: u16, side: Side) -> Point {
        let padding = self.padding();
        self.pos() + self.port_inner_offs(idx, side) + Vec2::new(padding.x0, padding.y0)
    }

    fn edge_midpoint(&self, port: u16, to_node: &Self, to_port: u16) -> Point {
        ((self.port_pos(port, Side::Out).to_vec2() + to_node.port_pos(to_port, Side::In).to_vec2())
            * 0.5)
            .to_point()
    }
}
impl<T: NodeWidget> NodeExt for T {}

pub const PORT_HEIGHT_32: f32 = 24.0;
pub const PORT_HEIGHT: f64 = 24.0;
pub const PORT_GAP: f64 = 4.0;

impl<I, W, P> NodeWidget for Node<I, W, P> {
    #[inline]
    fn pos(&self) -> Point { self.position }

    #[inline]
    fn inner_size(&self) -> Size {
        let arity = match &self.kind {
            NodeKind::Widget(_) => 1,
            NodeKind::Small(s) => {
                let in_arity = u16::try_from(s.inputs.len()).unwrap_or_else(|_| unreachable!());
                let out_arity = u16::try_from(s.outputs.len()).unwrap_or_else(|_| unreachable!());
                in_arity.max(out_arity)
            },
            NodeKind::Large(l) => {
                let in_arity = u16::try_from(l.inputs.len()).unwrap_or_else(|_| unreachable!());
                let out_arity = u16::try_from(l.outputs.len()).unwrap_or_else(|_| unreachable!());
                in_arity + out_arity
            },
        };

        Size::new(
            self.width,
            PORT_HEIGHT * f64::from(arity) + PORT_GAP * f64::from(arity.saturating_sub(1)),
        )
    }

    #[inline]
    fn padding(&self) -> Insets {
        match self.kind {
            NodeKind::Widget(_) => Insets::uniform(2.0),
            NodeKind::Small(_) => Insets::uniform(4.0),
            NodeKind::Large(_) => Insets::new(12.0, 8.0 + PORT_HEIGHT + 2.0 * 4.0, 12.0, 8.0),
        }
    }

    fn name_rect(&self) -> Rect {
        let vec = match self.kind {
            NodeKind::Widget(_) => Vec2::new(12.0, 2.0),
            NodeKind::Small(_) => {
                Vec2::new(12.0, 4.0 + (self.inner_size().height - PORT_HEIGHT) * 0.5)
            },
            NodeKind::Large(_) => Vec2::new(4.0, 4.0),
        };
        Rect::from_points(
            vec.to_point(),
            Point::new(self.outer_size().width - vec.x, PORT_HEIGHT + vec.y),
        )
    }

    #[inline]
    fn port_label_y(&self, idx: u16, side: Side) -> f64 {
        match &self.kind {
            NodeKind::Widget(_) => {
                assert!(idx == 0);
                0.0
            },
            NodeKind::Small(s) => {
                let in_arity = u16::try_from(s.inputs.len()).unwrap_or_else(|_| unreachable!());
                let out_arity = u16::try_from(s.outputs.len()).unwrap_or_else(|_| unreachable!());
                let arity = in_arity.max(out_arity).saturating_sub(match side {
                    Side::In => in_arity,
                    Side::Out => out_arity,
                });
                (PORT_HEIGHT * f64::from(arity) + PORT_GAP * f64::from(arity.saturating_sub(1)))
                    * 0.5
                    + (PORT_HEIGHT + PORT_GAP) * f64::from(idx)
            },
            NodeKind::Large(l) => {
                (PORT_HEIGHT + PORT_GAP)
                    * (f64::from(idx)
                        + f64::from(
                            u16::from(matches!(side, Side::In))
                                * u16::try_from(l.outputs.len()).unwrap_or_else(|_| unreachable!()),
                        ))
            },
        }
    }

    #[inline]
    fn port_inner_offs(&self, idx: u16, side: Side) -> Vec2 {
        Vec2::new(
            -self.padding().x0,
            self.port_label_y(idx, side) + PORT_HEIGHT * 0.5,
        ) + f64::from(matches!(side, Side::Out)) * Vec2::new(self.outer_size().width, 0.0)
    }
}
