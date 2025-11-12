use std::sync::Arc;

use wi_masonry::{graph::Checked, widget::GraphAction};
use xilem::{
    core::{MessageResult, View, ViewMarker},
    Pod, ViewCtx,
};

use crate::{
    graph::{Graph, Node},
    widget,
};

type Callback<N, State, Action> =
    Box<dyn Fn(&mut State, GraphAction<N, Checked<Graph<N>>>) -> Action + Send + Sync>;
pub fn graph_editor<
    N: Node,
    State,
    Action,
    F: Fn(&mut State, GraphAction<N, Checked<Graph<N>>>) -> Action + Send + Sync + 'static,
>(
    graph: Checked<Graph<N>>,
    on_action: F,
) -> GraphEditor<N, State, Action> {
    GraphEditor {
        graph: graph.into_inner(),
        on_action: Box::new(on_action),
    }
}

#[must_use]
#[expect(missing_debug_implementations)]
pub struct GraphEditor<N: Node, State, Action> {
    pub(super) graph: Arc<Graph<N>>,
    on_action: Callback<N, State, Action>,
}

impl<N: Node + 'static, State: 'static, Action: 'static> ViewMarker
    for GraphEditor<N, State, Action>
{
}

impl<N: Node + 'static, State: 'static, Action: 'static> View<State, Action, ViewCtx>
    for GraphEditor<N, State, Action>
{
    type Element = Pod<widget::GraphEditor<N>>;
    type ViewState = ();

    fn build(&self, cx: &mut ViewCtx, _app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let graph = widget::GraphEditor::new(Arc::clone(&self.graph));
        (cx.with_action_widget(|c| c.create_pod(graph)), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        (): &mut Self::ViewState,
        _cx: &mut ViewCtx,
        mut element: xilem::core::Mut<'_, Self::Element>,
        _app_state: &mut State,
    ) {
        if Arc::ptr_eq(&prev.graph, &self.graph) {
            return;
        }

        element
            .widget
            .set_graph(Arc::clone(&self.graph), &mut element.ctx);
    }

    fn teardown(
        &self,
        (): &mut Self::ViewState,
        cx: &mut ViewCtx,
        element: xilem::core::Mut<'_, Self::Element>,
    ) {
        cx.teardown_leaf(element);
    }

    fn message(
        &self,
        (): &mut Self::ViewState,
        message: &mut xilem::core::MessageContext,
        _element: xilem::core::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        let Some(action) = message.take_message::<GraphAction<N>>() else {
            tracing::error!(?message, "Wrong message type in GraphEditor::message");
            return MessageResult::Stale;
        };

        let action = match *action {
            GraphAction::Changed(g, c) => {
                GraphAction::Changed(unsafe { Checked::new_unchecked(g) }, c)
            },
            GraphAction::WantNodePrototype(f) => GraphAction::WantNodePrototype(f),
        };

        MessageResult::Action((self.on_action)(app_state, action))
    }
}
