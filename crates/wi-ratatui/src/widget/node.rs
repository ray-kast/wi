use ratatui::layout::{Margin, Position, Rect, Size};
use wi_core::{
    opinions::graph::{NodeStyleArity, StyleKind},
    Side,
};

use crate::graph::TuiNode;

const LABEL_MARGIN: Margin = Margin {
    horizontal: 1,
    vertical: 0,
};

pub trait NodeExt: TuiNode {
    fn body_size(&self) -> Size {
        let height = match self.style() {
            StyleKind::Widget(_) => 1,
            StyleKind::Small(s) => s.in_arity().max(s.out_arity()).max(1),
            StyleKind::Large(l) => (l.in_arity() + l.out_arity()).max(1),
        };

        Size {
            width: self.width(),
            height,
        }
    }

    #[inline]
    fn head_height(&self) -> u16 { matches!(self.style(), StyleKind::Large(_)).into() }

    fn port_inner_y(&self, idx: u16, side: Side) -> u16 {
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
    fn outer_size(&self) -> Size {
        let mut ret = self.body_size();
        ret.height += self.head_height();
        ret
    }

    #[inline]
    fn head_rect(&self) -> Rect {
        let Position { x, y } = self.position();
        Rect {
            x,
            y,
            width: self.width(),
            height: self.head_height(),
        }
    }

    fn body_rect(&self) -> Rect {
        let Position { x, mut y } = self.position();
        let Size { width, height } = self.body_size();
        y += self.head_height();
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[inline]
    fn label_rect(&self) -> Rect {
        match self.style() {
            StyleKind::Widget(_) | StyleKind::Small(_) => self.body_rect(),
            StyleKind::Large(_) => self.head_rect(),
        }
    }

    #[inline]
    fn port_label_rect(&self) -> Rect { self.body_rect().inner(LABEL_MARGIN) }

    #[inline]
    fn outer_rect(&self) -> Rect {
        let Position { x, y } = self.position();
        let Size { width, height } = self.outer_size();
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}
impl<N: TuiNode> NodeExt for N {}
