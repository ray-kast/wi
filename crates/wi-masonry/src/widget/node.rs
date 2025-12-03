use masonry::kurbo::{Insets, Point, Rect, Size, Vec2};
use wi_core::{
    opinions::graph::{NodeStyleArity, StyleKind},
    Side,
};

use crate::graph::WidgetNode;

pub const PORT_HEIGHT_32: f32 = 18.0;
pub const PORT_HEIGHT: f64 = 18.0;
pub const PORT_GAP: f64 = 4.0;

pub trait NodeExt: WidgetNode {
    fn inner_size(&self) -> Size {
        let arity = match self.style() {
            StyleKind::Widget(_) => 1,
            StyleKind::Small(s) => s.in_arity().max(s.out_arity()).max(1),
            StyleKind::Large(l) => (l.in_arity() + l.out_arity()).max(1),
        };

        Size::new(
            self.width(),
            PORT_HEIGHT * f64::from(arity) + PORT_GAP * f64::from(arity - 1),
        )
    }

    #[inline]
    fn padding(&self) -> Insets {
        match self.style() {
            StyleKind::Widget(_) => Insets::uniform(2.0),
            StyleKind::Small(_) => Insets::uniform(4.0),
            StyleKind::Large(_) => Insets::new(12.0, 8.0 + PORT_HEIGHT + 2.0 * 4.0, 12.0, 8.0),
        }
    }

    fn name_rect(&self) -> Rect {
        let vec = match self.style() {
            StyleKind::Widget(_) => Vec2::new(12.0, 2.0),
            StyleKind::Small(_) => {
                Vec2::new(12.0, 4.0 + (self.inner_size().height - PORT_HEIGHT) * 0.5)
            },
            StyleKind::Large(_) => Vec2::new(4.0, 4.0),
        };
        Rect::from_points(
            vec.to_point(),
            Point::new(self.outer_size().width - vec.x, PORT_HEIGHT + vec.y),
        )
    }

    fn port_label_y(&self, idx: u16, side: Side) -> f64 {
        match self.style() {
            StyleKind::Widget(_) => {
                assert!(idx == 0);
                0.0
            },
            StyleKind::Small(s) => {
                let in_arity = s.in_arity();
                let out_arity = s.out_arity();
                let arity = in_arity.max(out_arity).saturating_sub(match side {
                    Side::In => in_arity,
                    Side::Out => out_arity,
                });
                (PORT_HEIGHT * f64::from(arity) + PORT_GAP * f64::from(arity.saturating_sub(1)))
                    * 0.5
                    + (PORT_HEIGHT + PORT_GAP) * f64::from(idx)
            },
            StyleKind::Large(l) => {
                (PORT_HEIGHT + PORT_GAP)
                    * (f64::from(idx)
                        + f64::from(u16::from(matches!(side, Side::In)) * l.out_arity()))
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

    #[inline]
    fn outer_size(&self) -> Size { self.inner_size() + self.padding().size() }

    #[inline]
    fn rect(&self) -> Rect { Rect::from_origin_size(self.position(), self.outer_size()) }

    #[inline]
    fn port_pos(&self, idx: u16, side: Side) -> Point {
        let padding = self.padding();
        self.position() + self.port_inner_offs(idx, side) + Vec2::new(padding.x0, padding.y0)
    }

    #[inline]
    fn edge_midpoint(&self, port: u16, to_node: &Self, to_port: u16) -> Point {
        ((self.port_pos(port, Side::Out).to_vec2() + to_node.port_pos(to_port, Side::In).to_vec2())
            * 0.5)
            .to_point()
    }
}
impl<N: WidgetNode> NodeExt for N {}
