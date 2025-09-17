use std::collections::HashSet;

use masonry::kurbo::Point;
use xilem::{
    core::{View, ViewMarker},
    Pod, ViewCtx,
};

use crate::graph::{data::PortId, widgets};

pub fn graph() -> GraphView {
    GraphView {
        nodes: vec![],
        free_node: 0,
        edges: HashSet::new(),
    }
}

pub(super) struct Node {
    pub(super) pos: Point,
}

pub(super) enum NodeSlot {
    Free { next: usize },
    Node(Node),
}

impl NodeSlot {
    pub fn is_node(&self) -> bool {
        match self {
            Self::Free {..} => false,
            Self::Node(_) => true,
        }
    }

    pub fn node(&self) -> Option<&Node> {
        match self {
            Self::Free { .. } => None,
            Self::Node(n) => Some(n),
        }
    }
}

pub struct GraphView {
    pub(super) nodes: Vec<NodeSlot>,
    free_node: usize,
    pub(super) edges: HashSet<(PortId, PortId)>,
}

impl GraphView {
    pub fn with(mut self, f: impl FnOnce(&mut Self) -> &mut Self) -> Self {
        f(&mut self);
        self
    }

    pub fn node(&mut self, pos: Point) -> &mut Self {
        if self.free_node == usize::MAX {
            panic!("Maximum node count exceeded!");
        }

        let node = Node { pos };

        match self.nodes.get_mut(self.free_node) {
            None => {
                self.nodes.push(NodeSlot::Node(node));
                self.free_node = self.nodes.len();
            },
            Some(NodeSlot::Node(_)) => unreachable!(),
            Some(n @ &mut NodeSlot::Free { next }) => {
                *n = NodeSlot::Node(node);
                self.free_node = next;
            },
        }

        self
    }

    pub fn edge(&mut self, from: (usize, usize), to: (usize, usize)) -> &mut Self {
        let (node, port) = from;
        let from = PortId { node, port };

        let (node, port) = to;
        let to = PortId { node, port };

        self.edges.insert((from, to));
        self
    }
}

impl ViewMarker for GraphView {}
impl<S, A> View<S, A, ViewCtx> for GraphView {
    type Element = Pod<widgets::Graph>;
    type ViewState = ();

    fn build(&self, ctx: &mut ViewCtx) -> (Self::Element, Self::ViewState) {
        let graph = widgets::Graph::new(self);
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
