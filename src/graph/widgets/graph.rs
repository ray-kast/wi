use std::rc::Rc;

use masonry::{core::{Widget, WidgetPod}, kurbo::Point};

use crate::graph::{
    data::shared,
    widgets::{edge::Edge, node::Node},
};

pub struct Graph {
    nodes: Vec<WidgetPod<Node>>,
    edges: Vec<WidgetPod<Edge>>,
}

impl Graph {
    pub fn new(graph: &crate::graph::GraphView) -> Self {
        let shared = shared(graph);

        Self {
            nodes: graph
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(i, n)| n.is_node().then_some(i))
                .map(|i| WidgetPod::new(Node::new(Rc::clone(&shared), i)))
                .collect(),
            edges: graph
                .edges
                .iter()
                .map(|e| WidgetPod::new(Edge::new(Rc::clone(&shared), e)))
                .collect(),
        }
    }
}

impl Widget for Graph {
    fn register_children(&mut self, ctx: &mut masonry::core::RegisterCtx) {
        let Self { nodes, edges } = self;

        for node in nodes {
            ctx.register_child(node);
        }

        for edge in edges {
            ctx.register_child(edge);
        }
    }

    fn layout(
        &mut self,
        ctx: &mut masonry::core::LayoutCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        bc: &masonry::core::BoxConstraints,
    ) -> masonry::kurbo::Size {
        for node in &mut self.nodes {
            ctx.run_layout(node, bc);
            ctx.place_child(node, Point::ZERO);
        }

        for edge in &mut self.edges {
            ctx.run_layout(edge, bc);
            ctx.place_child(edge, Point::ZERO);
        }

        bc.max()
    }

    fn paint(
        &mut self,
        ctx: &mut masonry::core::PaintCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        scene: &mut masonry::vello::Scene,
    ) {
    }

    // TODO: review
    fn accessibility_role(&self) -> accesskit::Role { accesskit::Role::ScrollView }

    // TODO
    fn accessibility(
        &mut self,
        ctx: &mut masonry::core::AccessCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        node: &mut accesskit::Node,
    ) {
    }

    fn children_ids(&self) -> smallvec::SmallVec<[masonry::core::WidgetId; 16]> {
        self.nodes
            .iter()
            .map(|n| n.id())
            .chain(self.edges.iter().map(|e| e.id()))
            .collect()
    }
}
