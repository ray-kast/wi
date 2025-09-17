use masonry::{core::Widget, kurbo::{PathEl, Point, Size, Stroke}, peniko::color::OpaqueColor};
use smallvec::smallvec;
use xilem::Affine;

use crate::graph::data::Shared;

pub struct Node(Shared, usize);

impl Node {
    pub(super) fn new(shared: Shared, idx: usize) -> Self { Self(shared, idx) }
}

impl Widget for Node {
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
        let rect = self.0.borrow().node_rect(self.1);

        scene.stroke(
            &Stroke::new(2.0),
            Affine::IDENTITY,
            OpaqueColor::from_rgb8(0x3a, 0x3a, 0x3a),
            None,
            &rect,
        );
    }

    // TODO: review
    fn accessibility_role(&self) -> accesskit::Role { accesskit::Role::Form }

    // TODO
    fn accessibility(
        &mut self,
        ctx: &mut masonry::core::AccessCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> smallvec::SmallVec<[masonry::core::WidgetId; 16]> { smallvec![] }
}
