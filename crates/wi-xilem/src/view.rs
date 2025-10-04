use std::collections::HashMap;

use masonry::kurbo::Point;
use xilem::{
    core::{View, ViewMarker},
    Pod, ViewCtx,
};

use crate::{widget, Port};

pub fn graph() -> GraphView {
    GraphView {
        nodes: HashMap::new(),
        fresh_node: 0,
    }
}

#[derive(Debug)]
pub(super) struct Node {
    pub(super) pos: Point,
    pub(super) in_edges: Vec<Option<Port>>,
    pub(super) out_arity: usize,
}

#[must_use]
#[derive(Debug)]
pub struct GraphView {
    pub(super) nodes: HashMap<usize, Node>,
    fresh_node: usize,
}

impl GraphView {
    pub fn with(mut self, f: impl FnOnce(&mut Self) -> &mut Self) -> Self {
        f(&mut self);
        self
    }

    pub fn node(&mut self, pos: Point, in_arity: usize, out_arity: usize) -> &mut Self {
        use std::collections::hash_map::Entry;

        assert!(
            self.fresh_node != usize::MAX,
            "Maximum node count exceeded!"
        );

        let node = Node {
            pos,
            in_edges: vec![None; in_arity],
            out_arity,
        };

        let Entry::Vacant(v) = self.nodes.entry(self.fresh_node) else {
            unreachable!();
        };

        v.insert(node);
        self.fresh_node += 1;

        self
    }

    pub fn edge(&mut self, from: (usize, usize), to: (usize, usize)) -> &mut Self {
        let (node, port) = from;
        assert!(port < self.nodes[&node].out_arity, "Invalid edge from-port");
        let from = Port { node, port };

        let (node, port) = to;
        assert!(
            self.nodes.get_mut(&node).unwrap().in_edges[port]
                .replace(from)
                .is_none(),
            "Attempted to insert duplicate edge"
        );

        self
    }

    pub(crate) fn out_edge_map(&self) -> HashMap<usize, Vec<Vec<Port>>> {
        self.nodes.iter().fold(
            self.nodes
                .iter()
                .map(|(&k, v)| (k, vec![vec![]; v.out_arity]))
                .collect(),
            |mut h, (&node, v)| {
                for (port, edge) in v.in_edges.iter().enumerate() {
                    let Some(edge) = edge else { continue };

                    h.get_mut(&edge.node).unwrap_or_else(|| unreachable!())[edge.port]
                        .push(Port { node, port });
                }

                h
            },
        )
    }
}

impl ViewMarker for GraphView {}
impl<S, A> View<S, A, ViewCtx> for GraphView {
    type Element = Pod<widget::Graph>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let graph = widget::Graph::new(self);
        (ctx.with_action_widget(|c| c.new_pod(graph)), ())
    }

    fn rebuild(
        &self,
        prev: &Self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: xilem::core::Mut<'_, Self::Element>,
    ) {
        todo!()
    }

    fn teardown(
        &self,
        view_state: &mut Self::ViewState,
        ctx: &mut ViewCtx,
        element: xilem::core::Mut<'_, Self::Element>,
    ) {
        todo!()
    }

    fn message(
        &self,
        view_state: &mut Self::ViewState,
        id_path: &[xilem::core::ViewId],
        message: xilem::core::DynMessage,
        app_state: &mut S,
    ) -> xilem::core::MessageResult<A, xilem::core::DynMessage> {
        todo!()
    }
}
