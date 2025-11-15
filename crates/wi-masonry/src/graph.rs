use std::{borrow::Cow, fmt, sync::Arc};

use masonry::{core::NoAction, kurbo::Point};
use petgraph::{prelude::*, visit::IntoEdgeReferences};

pub type Graph<N> = StableDiGraph<Arc<N>, Edge>;

pub trait Node: fmt::Debug + Clone + Send + Sync + 'static {
    type Prototype: Clone;
    type Icon;
    type Widget;
    type PortShape;

    #[expect(clippy::must_use_candidate)]
    fn override_prototype() -> Option<Result<Self::Prototype, ()>> { None }

    fn create(proto: Self::Prototype, position: Point) -> Self;

    fn in_arity(&self) -> u16;
    fn out_arity(&self) -> u16;

    fn in_port(&self, index: u16) -> Port<'_, Self::PortShape, InputLabel<'_, Self::Widget>>;
    fn out_port(&self, index: u16) -> Port<'_, Self::PortShape, OutputLabel<'_>>;

    fn name(&self) -> Cow<'_, str>;

    fn position(&self) -> Point;
    fn position_mut(&mut self) -> Option<&mut Point>;

    #[inline]
    fn style(&self) -> NodeStyle { NodeStyle::Large }

    #[inline]
    fn width(&self) -> f64 {
        match self.style() {
            NodeStyle::Small => 84.0,
            NodeStyle::Medium => 96.0,
            NodeStyle::Large => 192.0,
        }
    }

    #[inline]
    fn label(&self) -> NodeLabel<'_, Self::Icon, Self::Widget> { NodeLabel::Name }
}

#[derive(Debug, Clone, Default)]
pub struct Port<'n, S, L> {
    pub name: Cow<'n, str>,
    pub shape: S,
    pub label: L,
}

#[derive(Debug, Clone, Default)]
pub enum InputLabel<'n, W> {
    #[default]
    Name,
    Text(Cow<'n, str>),
    Widget(W),
}

#[derive(Debug, Clone, Default)]
pub enum OutputLabel<'n> {
    #[default]
    Name,
    Text(Cow<'n, str>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeStyle {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Clone)]
pub enum NodeLabel<'n, I, W> {
    Name,
    Text(Cow<'n, str>),
    Icon(I),
    Widget(W),
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

#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub from_port: u16,
    pub to_port: u16,
}

pub const PORT_MAX: u16 = u16::MAX;
#[inline]
#[expect(clippy::must_use_candidate, reason = "Using this is impossible")]
pub fn port_overflow<T>() -> T { panic!("Port number exceeded {PORT_MAX}") }

#[derive(Debug)]
pub struct Checked<G>(Arc<G>);

impl<G> Clone for Checked<G> {
    #[inline]
    fn clone(&self) -> Self { Self(Arc::clone(&self.0)) }
}

impl<G> Checked<G> {
    #[expect(
        clippy::missing_safety_doc,
        reason = "WIP, adding this will suppress missing-docs warnings"
    )]
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(graph: Arc<G>) -> Self { Self(graph) }

    #[must_use]
    pub fn into_inner(self) -> Arc<G> { self.0 }
}

impl<N: Node> Checked<Graph<N>> {
    #[must_use]
    pub fn new(graph: Arc<Graph<N>>) -> Self {
        for edge in graph.edge_references() {
            let weight = edge.weight();

            assert!(
                weight.from_port < graph[edge.source()].out_arity(),
                "Invalid edge source port index"
            );
            assert!(
                weight.to_port < graph[edge.target()].in_arity(),
                "Invalid edge target port index"
            );
        }

        Self(graph)
    }
}
