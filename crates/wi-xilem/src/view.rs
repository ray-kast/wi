use std::sync::Arc;

use wi_masonry::graph::Checked;
use xilem::{
    core::{View, ViewMarker},
    Pod, ViewCtx,
};

use crate::{
    graph::{Graph, Node},
    widget,
};

pub fn graph_editor<N>(graph: Checked<Graph<N>>) -> GraphEditor<N> {
    GraphEditor(graph.into_inner())
}

#[must_use]
#[derive(Debug)]
pub struct GraphEditor<N>(pub(super) Arc<Graph<N>>);

impl<N> ViewMarker for GraphEditor<N> {}
impl<S, A, N: Node + 'static> View<S, A, ViewCtx> for GraphEditor<N> {
    type Element = Pod<widget::GraphEditor<N>>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let graph = widget::GraphEditor::new(Arc::clone(&self.0));
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
