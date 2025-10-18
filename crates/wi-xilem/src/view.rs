use std::sync::Arc;

use petgraph::{
    graph::IndexType,
    visit::{EdgeRef, IntoEdgeReferences},
};
use xilem::{
    core::{View, ViewMarker},
    Pod, ViewCtx,
};

use crate::{
    graph::{port_overflow, Graph, GraphMarker, NodeKind},
    widget,
};

pub fn graph_editor<I, W, P, Ix: IndexType>(
    graph: Checked<Graph<I, W, P, Ix>>,
) -> GraphEditor<Graph<I, W, P, Ix>> {
    GraphEditor(graph.0)
}

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

impl<G: GraphMarker> Checked<G> {
    #[must_use]
    pub fn new(graph: Arc<G>) -> Self {
        let g = graph.as_graph();

        for node in g.node_weights() {
            match &node.kind {
                NodeKind::Widget(_) => (),
                NodeKind::Small(s) => {
                    u16::try_from(s.inputs.len()).unwrap_or_else(|_| port_overflow());
                    u16::try_from(s.outputs.len()).unwrap_or_else(|_| port_overflow());
                },
                NodeKind::Large(l) => {
                    u16::try_from(l.inputs.len()).unwrap_or_else(|_| port_overflow());
                    u16::try_from(l.outputs.len()).unwrap_or_else(|_| port_overflow());
                },
            }
        }

        for edge in g.edge_references() {
            let weight = edge.weight();

            assert!(
                weight.from_port < g[edge.source()].out_arity(),
                "Invalid edge source port index"
            );
            assert!(
                weight.to_port < g[edge.target()].in_arity(),
                "Invalid edge target port index"
            );
        }

        Self(graph)
    }
}

#[must_use]
#[derive(Debug)]
pub struct GraphEditor<G>(pub(super) Arc<G>);

impl<G: GraphMarker> ViewMarker for GraphEditor<G> {}
impl<S, A, G: GraphMarker + 'static> View<S, A, ViewCtx> for GraphEditor<G> {
    type Element = Pod<widget::GraphEditor<G>>;
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
