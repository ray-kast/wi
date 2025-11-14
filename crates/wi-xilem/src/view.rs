use std::sync::Arc;

use spin::mutex::SpinMutex;
use wi_masonry::{
    graph::Checked,
    widget::{Change, GraphAction, PrototypeCallback},
};
use xilem::{
    core::{MessageResult, View, ViewMarker},
    Pod, ViewCtx,
};

use crate::{
    graph::{Graph, Node},
    widget,
};

mod modal;

pub use modal::*;

#[expect(missing_debug_implementations)]
pub struct Params<N: Node> {
    update: SpinMutex<Option<Update<N>>>,
}

impl<N: Node> Default for Params<N> {
    #[inline]
    fn default() -> Self {
        Self {
            update: None.into(),
        }
    }
}

enum Update<N: Node> {
    FoundNodePrototype(Option<N::Prototype>),
}

type OnChange<N, State, Action> =
    Box<dyn Fn(&mut State, Checked<Graph<N>>, Change<N>) -> Action + Send + Sync>;

pub fn graph_editor<
    N: Node,
    State,
    Action,
    C: Fn(&mut State, Checked<Graph<N>>, Change<N>) -> Action + Send + Sync + 'static,
>(
    graph: Checked<Graph<N>>,
    params: Params<N>,
    on_change: C,
) -> GraphEditor<N, State, Action> {
    GraphEditor {
        graph: graph.into_inner(),
        params,
        on_change: Box::new(on_change),
    }
}

#[must_use]
#[expect(missing_debug_implementations)]
pub struct GraphEditor<N: Node, State, Action> {
    graph: Arc<Graph<N>>,
    params: Params<N>,
    on_change: OnChange<N, State, Action>,
}

#[doc(hidden)]
#[expect(missing_debug_implementations)]
pub struct ViewState<N: Node> {
    want_node_prototype: Option<PrototypeCallback<N>>,
}

#[expect(missing_debug_implementations)]
pub struct GraphEditorAction<N: Node, Action>(ActionKind<N, Action>);

enum ActionKind<N: Node, Action> {
    WantNodePrototype,
    FoundNodePrototype(Option<N::Prototype>),
    Inner(Action),
}

impl<N: Node, Action> From<ActionKind<N, Action>> for GraphEditorAction<N, Action> {
    #[inline]
    fn from(value: ActionKind<N, Action>) -> Self { Self(value) }
}

impl<N: Node + 'static, State: 'static, Action: 'static> ViewMarker
    for GraphEditor<N, State, Action>
{
}

impl<N: Node + 'static, State: 'static, Action: 'static>
    View<State, GraphEditorAction<N, Action>, ViewCtx> for GraphEditor<N, State, Action>
{
    type Element = Pod<widget::GraphEditor<N>>;
    type ViewState = ViewState<N>;

    fn build(&self, cx: &mut ViewCtx, _app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let graph = widget::GraphEditor::new(Arc::clone(&self.graph));

        let Params { update } = &self.params;

        assert!(
            update.lock().is_none(),
            "Update fired while constructing GraphEditor"
        );

        (cx.with_action_widget(|c| c.create_pod(graph)), ViewState {
            want_node_prototype: None,
        })
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        _cx: &mut ViewCtx,
        mut element: xilem::core::Mut<'_, Self::Element>,
        _app_state: &mut State,
    ) {
        if !Arc::ptr_eq(&prev.graph, &self.graph) {
            element
                .widget
                .set_graph(Arc::clone(&self.graph), &mut element.ctx);
        }

        if let Some(update) = self.params.update.lock().take() {
            match update {
                Update::FoundNodePrototype(p) => (view_state
                    .want_node_prototype
                    .take()
                    .unwrap_or_else(|| panic!("Received FoundNodePrototype with no callback")))(
                    p, element,
                ),
            }
        }
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        cx: &mut ViewCtx,
        element: xilem::core::Mut<'_, Self::Element>,
    ) {
        cx.teardown_leaf(element);
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        message: &mut xilem::core::MessageContext,
        _element: xilem::core::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<GraphEditorAction<N, Action>> {
        let Some(action) = message.take_message::<GraphAction<N>>() else {
            tracing::error!(?message, "Wrong message type in GraphEditor::message");
            return MessageResult::Stale;
        };

        match *action {
            GraphAction::Changed(g, c) => {
                MessageResult::Action(ActionKind::Inner((self.on_change)(app_state, g, c)).into())
            },
            GraphAction::WantNodePrototype(f) => {
                assert!(
                    view_state.want_node_prototype.replace(f).is_none(),
                    "Attempted to store duplicate PrototypeCallback"
                );
                MessageResult::Action(ActionKind::WantNodePrototype.into())
            },
        }
    }
}
