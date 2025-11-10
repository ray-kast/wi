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

    fn build(&self, ctx: &mut ViewCtx, _app_state: &mut S) -> (Self::Element, Self::ViewState) {
        let graph = widget::GraphEditor::new(Arc::clone(&self.0));
        (ctx.with_action_widget(|c| c.create_pod(graph)), ())
    }

    fn rebuild(
        &self,
        _prev: &Self,
        _view_state: &mut Self::ViewState,
        _ctx: &mut ViewCtx,
        _element: xilem::core::Mut<'_, Self::Element>,
        _app_state: &mut S,
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
        _view_state: &mut Self::ViewState,
        _message: &mut xilem::core::MessageContext,
        _element: xilem::core::Mut<'_, Self::Element>,
        _app_state: &mut S,
    ) -> xilem::core::MessageResult<A> {
        todo!()
    }
}
