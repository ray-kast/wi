use ratatui::layout::{Margin, Position, Rect};
use wi_core::{
    opinions::graph::{NodeStyleArity, StyleKind},
    Side,
};

use crate::{
    graph::TuiNode,
    vector::{Point, Vector},
};

const LABEL_MARGIN: Margin = Margin {
    horizontal: 1,
    vertical: 0,
};

pub trait NodeExt: TuiNode {
    fn body_size(&self) -> Vector {
        let height = match self.style() {
            StyleKind::Widget(_) => 1,
            StyleKind::Small(s) => s.in_arity().max(s.out_arity()).max(1),
            StyleKind::Large(l) => (l.in_arity() + l.out_arity()).max(1),
        };

        Vector {
            x: self.width(),
            y: height.into(),
        }
    }

    #[inline]
    fn head_height(&self) -> f32 { matches!(self.style(), StyleKind::Large(_)).into() }

    fn port_inner_row(&self, idx: u16, side: Side) -> u16 {
        match self.style() {
            StyleKind::Widget(_) => {
                assert!(idx == 0);
                0
            },
            StyleKind::Small(s) => {
                let in_arity = s.in_arity();
                let out_arity = s.out_arity();

                in_arity.max(out_arity).saturating_sub(match side {
                    Side::In => in_arity,
                    Side::Out => out_arity,
                }) / 2
                    + idx
            },
            StyleKind::Large(l) => idx + u16::from(matches!(side, Side::In)) * l.out_arity(),
        }
    }

    #[inline]
    fn outer_size(&self) -> Vector {
        let mut ret = self.body_size();
        ret.y += self.head_height();
        ret
    }

    #[inline]
    fn head_rect_offset(&self) -> Option<(Rect, Position)> {
        self.position().to_rect_offset(Vector {
            x: self.width(),
            y: self.head_height(),
        })
    }

    #[inline]
    fn body_position(&self) -> Point {
        self.position()
            + Vector {
                x: 0.0,
                y: self.head_height(),
            }
    }

    #[inline]
    fn body_rect_offset(&self) -> Option<(Rect, Position)> {
        self.body_position().to_rect_offset(self.body_size())
    }

    #[inline]
    fn label_rect(&self) -> Option<(Rect, Position)> {
        match self.style() {
            StyleKind::Widget(_) | StyleKind::Small(_) => self.body_rect_offset(),
            StyleKind::Large(_) => self.head_rect_offset(),
        }
    }

    #[inline]
    fn port_label_rect_offset(&self) -> Option<(Rect, Position)> {
        let (mut rect, mut offset) = self.body_rect_offset()?;
        rect = rect.inner(LABEL_MARGIN);

        let clip_x = rect.x.min(offset.x);
        let clip_y = rect.y.min(offset.y);
        rect.x -= clip_x;
        offset.x -= clip_x;
        rect.y -= clip_y;
        offset.y -= clip_y;

        Some((rect, offset))
    }

    fn port_pos(&self, idx: u16, side: Side, bump_outside: bool) -> Point {
        let size = self.body_size();
        let out = f32::from(matches!(side, Side::Out));

        self.body_position()
            + Vector {
                x: out * size.x + f32::from(bump_outside) * (1.0 * out - 1.0),
                y: self.port_inner_row(idx, side).into(),
            }
    }

    fn edge_midpoint(&self, port: u16, to_node: &Self, to_port: u16) -> Point {
        let from = self.port_pos(port, Side::Out, true);
        let to = to_node.port_pos(to_port, Side::In, true);

        Point::from_vec(from.as_vec().midpoint(to.as_vec()))
    }
}
impl<N: TuiNode> NodeExt for N {}
