use std::num::NonZeroIsize;

use crate::{
    ContinueOnce, CursorUpdate, GraphWidgetCell, Side, Status, Step, WCursor, WEdgeCursor, WPort, WSidedPort, Yielded
};

// TODO: drop references for types that are Copy

pub trait GraphWidgetTypes {
    type NodeId: Copy + Eq;
    type PortId: Copy + Eq;

    type Cell: GraphWidgetCell<Self>;
    type Point: Copy;

    type Context<'a>;

    type NodeKind;
}

pub trait GraphWidget: GraphWidgetTypes + CursorOps + EdgeOps + NodeOps + UiOps {}

impl<T: GraphWidgetTypes + CursorOps + EdgeOps + NodeOps + UiOps + ?Sized> GraphWidget for T {}

pub trait CursorOps: GraphWidgetTypes {
    fn default_cursor(&self) -> WCursor<Self>;

    fn nearest_node_where<F: Fn(&Self::NodeId) -> bool>(
        &self,
        cell: &Self::Cell,
        pred: F,
    ) -> Option<Self::NodeId>;

    fn nearest_node(&self, cell: &Self::Cell) -> Option<Self::NodeId> {
        self.nearest_node_where(cell, |_| true)
    }

    fn nearest_port(
        &self,
        node: &Self::NodeId,
        side: Side,
        cell: &Self::Cell,
    ) -> Option<Self::PortId>;

    fn step_port_by(
        &self,
        port: &WSidedPort<Self>,
        count: isize,
    ) -> Option<(NonZeroIsize, Self::PortId)>;

    fn nearest_edge_where<F: Fn(&WEdgeCursor<Self>) -> bool>(
        &self,
        port: &WSidedPort<Self>,
        cell: &Self::Cell,
        pred: F,
    ) -> Option<WEdgeCursor<Self>>;

    #[inline]
    fn nearest_edge(
        &self,
        port: &WSidedPort<Self>,
        cell: &Self::Cell,
    ) -> Option<WEdgeCursor<Self>> {
        self.nearest_edge_where(port, cell, |_| true)
    }

    fn step_edge_by(
        &self,
        edge: &WEdgeCursor<Self>,
        count: isize,
    ) -> Option<(NonZeroIsize, WPort<Self>)>;

    fn step_point_by(&self, point: &Self::Point, step: Step, count: usize) -> Self::Point;

    fn update_cursor(
        &mut self,
        update: CursorUpdate,
        cursor: &WCursor<Self>,
        cx: &mut Self::Context<'_>,
    );
}

pub trait EdgeOps: GraphWidgetTypes {
    fn delete_edge(
        &mut self,
        from: &WPort<Self>,
        to: &WPort<Self>,
        cx: &mut Self::Context<'_>,
    ) -> bool;
}

pub trait NodeOps: GraphWidgetTypes {
    fn prompt_node_kind<Y, C: ContinueOnce<Self, Y, Option<Self::NodeKind>>>(
        &mut self,
        then: Yielded<Self, Y, C>,
        cx: &mut Self::Context<'_>,
    );

    fn create_node(
        &mut self,
        kind: Self::NodeKind,
        position: Self::Point,
        cx: &mut Self::Context<'_>,
    );

    fn delete_node(&mut self, node: &Self::NodeId, cx: &mut Self::Context<'_>) -> bool;
}

pub trait UiOps: GraphWidgetTypes {
    fn update_status(&mut self, status: Status, cx: &mut Self::Context<'_>);

    fn quit(&mut self, cx: &mut Self::Context<'_>);
}
