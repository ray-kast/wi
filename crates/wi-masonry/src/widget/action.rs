use std::{fmt, sync::Arc};

use masonry::core::WidgetMut;
use petgraph::graph::NodeIndex;

use crate::graph::{Checked, Edge, Graph, Node};

pub type PrototypeCallback<N> =
    Box<dyn FnOnce(Option<<N as Node>::Prototype>, WidgetMut<super::GraphEditor<N>>) + Send + Sync>;
pub enum GraphAction<N: Node> {
    Changed(Checked<Graph<N>>, Change<N>),
    WantNodePrototype(PrototypeCallback<N>),
}

impl<N: fmt::Debug + Node> fmt::Debug for GraphAction<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Changed(g, c) => f.debug_tuple("Changed").field(g).field(c).finish(),
            Self::WantNodePrototype(_) => {
                f.debug_tuple("WantNodePrototype").finish_non_exhaustive()
            },
        }
    }
}

#[derive(Debug)]
pub enum Change<N> {
    NodeMoved,
    EdgeCreated,
    EdgeDeleted(NodeIndex, NodeIndex, Edge),
    NodeCreated,
    NodeDeleted(Arc<N>),
}
