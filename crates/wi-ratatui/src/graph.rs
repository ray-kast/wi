pub use wi_core::opinions::graph::*;

use crate::vector::Point;

pub trait TuiNode: Node<Position = Point> + Clone {
    #[inline]
    fn width(&self) -> f32 {
        match self.style() {
            StyleKind::Widget(_) => 8.0,
            StyleKind::Small(_) => 12.0,
            StyleKind::Large(_) => 16.0,
        }
    }
}
