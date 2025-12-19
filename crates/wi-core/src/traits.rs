use std::num::NonZeroI32;

use crate::{
    AlignCell, ContinueOnce, CursorUpdate, Side, Status, Step, WCursor, WEdgeCursor, WPort,
    WSidedPort, Yielded,
};

pub trait GraphWidgetCell<W: GraphWidgetTypes + ?Sized> {
    fn of_cursor(widget: &W, cursor: &WCursor<W>) -> Self;

    fn position(&self, widget: &W) -> W::Point;

    fn align_to_cursor(&mut self, widget: &W, cursor: &WCursor<W>, align: AlignCell);
}

// TODO: drop references for types that are Copy

pub trait GraphWidgetTypes {
    type NodeId: Copy + Eq;
    type PortId: Copy + Eq;

    type Cell: GraphWidgetCell<Self>;
    type Point: Copy;

    type Context<'cx, 'widget: 'cx>;

    type NodeKind: Clone;

    fn reborrow_cx<'widget, 're>(
        cx: &'re mut Self::Context<'_, 'widget>,
    ) -> Self::Context<'re, 'widget>;
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
        count: i32,
    ) -> Option<(NonZeroI32, Self::PortId)>;

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
        count: i32,
    ) -> Option<(NonZeroI32, WPort<Self>)>;

    fn step_point_by(&self, point: &Self::Point, step: Step, count: u32) -> Self::Point;

    fn update_cursor(
        &mut self,
        update: CursorUpdate,
        cursor: &WCursor<Self>,
        cx: Self::Context<'_, '_>,
    );
}

pub trait EdgeOps: GraphWidgetTypes {
    fn create_edge(&mut self, from: WPort<Self>, to: WPort<Self>, cx: Self::Context<'_, '_>);

    fn delete_edge(
        &mut self,
        from: &WPort<Self>,
        to: &WPort<Self>,
        cx: Self::Context<'_, '_>,
    ) -> bool;
}

pub trait NodeOps: GraphWidgetTypes {
    fn prompt_node_kind<'a, Y, C: ContinueOnce<Self, Y, Option<Self::NodeKind>>>(
        &'a mut self,
        then: Yielded<'a, '_, Self, Y, C>,
    );

    fn create_node(
        &mut self,
        kind: Self::NodeKind,
        position: Self::Point,
        cx: Self::Context<'_, '_>,
    );

    fn nudge_node(
        &mut self,
        node: &Self::NodeId,
        step: Step,
        count: u32,
        cx: Self::Context<'_, '_>,
    ) -> bool;

    fn delete_node(&mut self, node: &Self::NodeId, cx: Self::Context<'_, '_>) -> bool;
}

pub trait UiOps: GraphWidgetTypes {
    fn update_status(&mut self, status: Status, cx: Self::Context<'_, '_>);

    fn quit(&mut self, cx: Self::Context<'_, '_>);
}
