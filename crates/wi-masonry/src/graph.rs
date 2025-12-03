use std::fmt;

use masonry::{core::NoAction, kurbo::Point};
pub use wi_core::opinions::graph::*;

pub trait WidgetNode: Node<Position = Point> + fmt::Debug + Clone + Send + Sync + 'static {
    #[inline]
    fn width(&self) -> f64 {
        match self.style() {
            StyleKind::Large(_) => 192.0,
            StyleKind::Small(_) => 96.0,
            StyleKind::Widget(_) => 84.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoWidget {}

impl masonry::core::Widget for NoWidget {
    type Action = NoAction;

    #[inline]
    fn register_children(&mut self, _: &mut masonry::core::RegisterCtx) { match *self {} }

    #[inline]
    fn layout(
        &mut self,
        _: &mut masonry::core::LayoutCtx,
        _: &mut masonry::core::PropertiesMut<'_>,
        _: &masonry::core::BoxConstraints,
    ) -> masonry::kurbo::Size {
        match *self {}
    }

    #[inline]
    fn paint(
        &mut self,
        _: &mut masonry::core::PaintCtx,
        _: &masonry::core::PropertiesRef<'_>,
        _: &mut masonry::vello::Scene,
    ) {
        match *self {}
    }

    #[inline]
    fn accessibility_role(&self) -> accesskit::Role { match *self {} }

    #[inline]
    fn accessibility(
        &mut self,
        _: &mut masonry::core::AccessCtx,
        _: &masonry::core::PropertiesRef<'_>,
        _: &mut accesskit::Node,
    ) {
        match *self {}
    }

    #[inline]
    fn children_ids(&self) -> smallvec::SmallVec<[masonry::core::WidgetId; 16]> { match *self {} }
}
