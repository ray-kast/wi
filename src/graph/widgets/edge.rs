use masonry::{
    core::Widget,
    kurbo::{PathEl, Point, Stroke},
    peniko::color::OpaqueColor,
};
use smallvec::smallvec;
use xilem::{Affine, Vec2};

use crate::graph::data::{PortId, Shared};

pub struct Edge {
    shared: Shared,
    from: PortId,
    to: PortId,
}

impl Edge {
    pub(super) fn new(shared: Shared, edge: &(PortId, PortId)) -> Self {
        let (from, to) = *edge;
        Self { shared, from, to }
    }

    fn endpoints(&self) -> (Point, Point) {
        let &Self {
            ref shared,
            from,
            to,
        } = self;
        let data = shared.borrow();

        (data.port_pos(from, true), data.port_pos(to, false))
    }
}

impl Widget for Edge {
    fn register_children(&mut self, ctx: &mut masonry::core::RegisterCtx) {}

    fn layout(
        &mut self,
        ctx: &mut masonry::core::LayoutCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        bc: &masonry::core::BoxConstraints,
    ) -> masonry::kurbo::Size {
        bc.max()
    }

    fn paint(
        &mut self,
        ctx: &mut masonry::core::PaintCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        scene: &mut masonry::vello::Scene,
    ) {
        let (start, end) = self.endpoints();

        scene.stroke(
            &Stroke::new(2.0),
            Affine::IDENTITY,
            OpaqueColor::from_rgb8(0x7f, 0x7f, 0x7f),
            None,
            &[
                PathEl::MoveTo(start),
                PathEl::CurveTo(
                    start + Vec2::new(16.0, 0.0),
                    end + Vec2::new(-16.0, 0.0),
                    end,
                ),
            ],
        );
    }

    // TODO: review
    fn accessibility_role(&self) -> accesskit::Role { accesskit::Role::Form }

    fn accessibility(
        &mut self,
        ctx: &mut masonry::core::AccessCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> smallvec::SmallVec<[masonry::core::WidgetId; 16]> { smallvec![] }
}
