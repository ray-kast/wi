use ratatui::layout::Position;
pub use wi_core::opinions::graph::*;

pub trait TuiNode: Node<Position = Position> {
    #[inline]
    fn width(&self) -> u16 {
        match self.style() {
            StyleKind::Widget(_) => 8,
            StyleKind::Small(_) => 12,
            StyleKind::Large(_) => 16,
        }
    }
}
