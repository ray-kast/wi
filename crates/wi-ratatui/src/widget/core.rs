use std::{marker::PhantomData, sync::Arc};

use petgraph::graph::NodeIndex;
use ratatui::layout::Position;
use wi_core::{
    opinions::graph::{Checked, Graph},
    traits::{CursorOps, GraphWidgetTypes},
};

use crate::graph::TuiNode;

pub struct Cx<'w>(PhantomData<&'w ()>);

#[derive(Debug)]
pub struct EditorCore<N> {
    pub graph: Arc<Graph<N>>,
}

impl<N> EditorCore<N> {
    #[inline]
    #[must_use]
    pub fn new(graph: Checked<Graph<N>>) -> Self {
        Self {
            graph: graph.into_inner(),
        }
    }
}

impl<N: TuiNode> GraphWidgetTypes for EditorCore<N> {
    type Cell = wi_core::opinions::cell::WCell<EditorCore<N>>;
    type Context<'cx, 'widget: 'cx> = &'cx mut Cx<'widget>;
    type NodeId = NodeIndex;
    type NodeKind = N::Prototype;
    type Point = Position;
    type PortId = u16;

    #[inline]
    fn reborrow_cx<'widget, 're>(
        cx: &'re mut Self::Context<'_, 'widget>,
    ) -> Self::Context<'re, 'widget> {
        cx
    }
}

impl<N: TuiNode> CursorOps for EditorCore<N> {
    fn default_cursor(&self) -> wi_core::WCursor<Self> {
        wi_core::Cursor::FixedPoint(Position::ORIGIN)
    }

    fn nearest_node_where<F: Fn(&Self::NodeId) -> bool>(
        &self,
        cell: &Self::Cell,
        pred: F,
    ) -> Option<Self::NodeId> {
        todo!()
    }

    fn nearest_port(
        &self,
        node: &Self::NodeId,
        side: wi_core::Side,
        cell: &Self::Cell,
    ) -> Option<Self::PortId> {
        todo!()
    }

    fn step_port_by(
        &self,
        port: &wi_core::WSidedPort<Self>,
        count: isize,
    ) -> Option<(std::num::NonZeroIsize, Self::PortId)> {
        todo!()
    }

    fn nearest_edge_where<F: Fn(&wi_core::WEdgeCursor<Self>) -> bool>(
        &self,
        port: &wi_core::WSidedPort<Self>,
        cell: &Self::Cell,
        pred: F,
    ) -> Option<wi_core::WEdgeCursor<Self>> {
        todo!()
    }

    fn step_edge_by(
        &self,
        edge: &wi_core::WEdgeCursor<Self>,
        count: isize,
    ) -> Option<(std::num::NonZeroIsize, wi_core::WPort<Self>)> {
        todo!()
    }

    fn step_point_by(&self, point: &Self::Point, step: wi_core::Step, count: usize) -> Self::Point {
        todo!()
    }

    fn update_cursor(
        &mut self,
        update: wi_core::CursorUpdate,
        cursor: &wi_core::WCursor<Self>,
        cx: Self::Context<'_, '_>,
    ) {
        todo!()
    }
}
