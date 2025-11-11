use std::sync::Arc;

use petgraph::graph::NodeIndex;

use crate::graph::{Edge, Graph};

#[derive(Debug, Clone)]
pub struct GraphAction<N> {
    pub graph: Arc<Graph<N>>,
    pub kind: GraphActionKind<N>,
}

#[derive(Debug, Clone)]
pub enum GraphActionKind<N> {
    NodeMoved,
    EdgeCreated,
    EdgeDeleted(NodeIndex, NodeIndex, Edge),
    NodeCreated,
    NodeDeleted(Arc<N>),
}
