use std::{fmt, marker::PhantomData};

use wi_masonry::graph::WidgetNode;
use xilem::{
    core::{MessageResult, View, ViewMarker},
    ViewCtx,
};

use super::{ActionKind, GraphEditorAction, Params};
use crate::view::Update;

pub struct ModalState<'a, N: WidgetNode> {
    state: &'a ViewState<N>,
    took_node_proto: bool,
}

impl<'a, N: WidgetNode> ModalState<'a, N> {
    fn new(state: &'a ViewState<N>) -> Self {
        Self {
            state,
            took_node_proto: false,
        }
    }

    pub fn want_node_prototype(&mut self) -> Option<WantNodePrototype> {
        self.took_node_proto = true;
        matches!(self.state.modal, Some(Modal::WantNodePrototype)).then_some(WantNodePrototype(()))
    }
}

impl<N: WidgetNode> Drop for ModalState<'_, N> {
    fn drop(&mut self) {
        const REASON: &str = "(All modals must be handled or else the editor may lock up)";

        let Self {
            state: _,
            took_node_proto,
        } = *self;

        assert!(
            took_node_proto,
            "WantNodePrototype modal was not handled {REASON}"
        );
    }
}

impl<N: WidgetNode> fmt::Debug for ModalState<'_, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            state: _,
            took_node_proto,
        } = self;

        f.debug_struct("ModalState")
            .field("took_node_proto", took_node_proto)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy)]
#[expect(missing_debug_implementations)]
pub struct WantNodePrototype(());

impl WantNodePrototype {
    pub fn respond<N: WidgetNode, Action>(
        self,
        proto: Option<N::Prototype>,
    ) -> GraphEditorAction<N, Action> {
        ActionKind::FoundNodePrototype(proto).into()
    }
}

pub fn graph_editor_modals<
    N: WidgetNode,
    V,
    State,
    F: Fn(&mut State, Params<N>, ModalState<N>) -> V + Send + Sync + 'static,
>(
    inner: F,
) -> GraphEditorModals<N, impl Fn(&mut State, Params<N>, ModalState<N>) -> V> {
    GraphEditorModals {
        inner,
        _phantom: PhantomData,
    }
}

#[must_use]
#[expect(missing_debug_implementations)]
pub struct GraphEditorModals<N, F> {
    inner: F,
    _phantom: PhantomData<N>,
}

#[doc(hidden)]
#[expect(missing_debug_implementations)]
pub struct ViewState<N: WidgetNode> {
    modal: Option<Modal>,
    update: Option<Update<N>>,
}

enum Modal {
    WantNodePrototype,
}

impl<N, F> ViewMarker for GraphEditorModals<N, F> {}

impl<
        N: WidgetNode,
        V: View<State, GraphEditorAction<N, Action>, ViewCtx>,
        State,
        Action,
        F: Fn(&mut State, Params<N>, ModalState<N>) -> V + Send + Sync + 'static,
    > View<State, Action, ViewCtx> for GraphEditorModals<N, F>
{
    type Element = V::Element;
    type ViewState = (V, ViewState<N>, V::ViewState);

    fn build(&self, ctx: &mut ViewCtx, app_state: &mut State) -> (Self::Element, Self::ViewState) {
        let view_state = ViewState {
            modal: None,
            update: None,
        };
        let view = (self.inner)(app_state, Params::default(), ModalState::new(&view_state));
        let (el, inner_state) = view.build(ctx, app_state);

        (el, (view, view_state, inner_state))
    }

    fn rebuild(
        &self,
        _: &Self,
        (inner, view_state, inner_state): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: xilem::core::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) {
        let view = (self.inner)(
            app_state,
            Params {
                update: view_state.update.take().into(),
            },
            ModalState::new(view_state),
        );
        view.rebuild(inner, inner_state, ctx, element, app_state);
        *inner = view;
    }

    fn teardown(
        &self,
        (inner, _, inner_state): &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: xilem::core::Mut<'_, Self::Element>,
    ) {
        inner.teardown(inner_state, ctx, element);
    }

    fn message(
        &self,
        (inner, view_state, inner_state): &mut Self::ViewState,
        message: &mut xilem::core::MessageContext,
        element: xilem::core::Mut<'_, Self::Element>,
        app_state: &mut State,
    ) -> MessageResult<Action> {
        match inner.message(inner_state, message, element, app_state) {
            MessageResult::Action(GraphEditorAction(k)) => match k {
                ActionKind::WantNodePrototype => {
                    assert!(
                        view_state.modal.replace(Modal::WantNodePrototype).is_none(),
                        "Requested node prototype while already in a modal"
                    );

                    MessageResult::RequestRebuild
                },
                ActionKind::FoundNodePrototype(p) => {
                    let Some(Modal::WantNodePrototype) = view_state.modal.take() else {
                        panic!("Responded to stale node prototype modal");
                    };

                    assert!(
                        view_state
                            .update
                            .replace(Update::FoundNodePrototype(p))
                            .is_none(),
                        "Attempted to store duplicate update"
                    );

                    MessageResult::RequestRebuild
                },
                ActionKind::Inner(a) => MessageResult::Action(a),
            },
            MessageResult::RequestRebuild => MessageResult::RequestRebuild,
            MessageResult::Nop => MessageResult::Nop,
            MessageResult::Stale => MessageResult::Stale,
        }
    }
}
