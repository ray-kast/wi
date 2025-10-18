use std::sync::Arc;

use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use xilem::{
    core::{View, ViewMarker},
    Pod, ViewCtx,
};

use crate::{
    graph::{Graph, Node},
    widget,
};

pub fn graph_editor<N>(graph: Checked<Graph<N>>) -> GraphEditor<N> { GraphEditor(graph.0) }

#[derive(Debug)]
pub struct Checked<G>(Arc<G>);

impl<G> Checked<G> {
    #[expect(
        clippy::missing_safety_doc,
        reason = "WIP, adding this will suppress missing-docs warnings"
    )]
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(graph: Arc<G>) -> Self { Self(graph) }
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

#[must_use]
#[derive(Debug)]
pub struct GraphEditor<N>(pub(super) Arc<Graph<N>>);

impl<N> ViewMarker for GraphEditor<N> {}
impl<S, A, N: Node + 'static> View<S, A, ViewCtx> for GraphEditor<N> {
    type Element = Pod<widget::GraphEditor<N>>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let graph = widget::GraphEditor::new(self);
        (ctx.with_action_widget(|c| c.new_pod(graph)), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        (): &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        _element: xilem::core::Mut<'_, Self::Element>,
    ) {
        todo!()
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: xilem::core::Mut<'_, Self::Element>,
    ) {
        ctx.teardown_leaf(element);
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        _id_path: &[xilem::core::ViewId],
        _message: xilem::core::DynMessage,
        _app_state: &mut S,
    ) -> xilem::core::MessageResult<A> {
        todo!()
    }
}
