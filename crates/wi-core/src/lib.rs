use std::{
    fmt,
    num::{NonZeroIsize, NonZeroU32},
};

use crate::bindings::Mode;
pub use crate::{
    actions::Action,
    bindings::ModeKind,
    cursor::{euclidean_cell, Cursor, EdgeCursor, WCursor, WEdgeCursor},
    operators::Operator,
    status::Status,
};

pub extern crate shibari;

mod actions;
mod bindings;
mod cursor;
mod keyboard;
pub mod modifiers;
mod operators;
mod status;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    In,
    Out,
}

impl Side {
    #[inline]
    #[must_use]
    pub fn flip(self) -> Self {
        match self {
            Self::In => Self::Out,
            Self::Out => Self::In,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Port<N, P>(pub N, pub P);

pub type WPort<W> = Port<<W as GraphWidget>::NodeId, <W as GraphWidget>::PortId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SidedPort<N, P>(pub Side, pub Port<N, P>);

pub type WSidedPort<W> = SidedPort<<W as GraphWidget>::NodeId, <W as GraphWidget>::PortId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    Left,
    Down,
    Up,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignCell {
    Overwrite,
    KeepRow,
    KeepCol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorUpdate {
    Move,
    CenterInView,
}

pub trait GraphWidgetCell<W: GraphWidget + ?Sized> {
    fn of_cursor(widget: &W, cursor: &WCursor<W>) -> Self;

    fn position(&self, widget: &W) -> W::Point;

    fn align_to_cursor(&mut self, widget: &W, cursor: &WCursor<W>, align: AlignCell);
}

// TODO: drop references for types that are Copy
pub trait GraphWidget {
    type NodeId: Copy + Eq;
    type PortId: Copy + Eq;

    type Cell: GraphWidgetCell<Self>;
    type Point: Copy;

    type Context<'a>;

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
        ctx: &mut Self::Context<'_>,
    );

    fn update_status(&mut self, status: Status, ctx: &mut Self::Context<'_>);

    fn delete_node(&mut self, node: &Self::NodeId) -> bool;

    fn delete_edge(&mut self, from: &WPort<Self>, to: &WPort<Self>) -> bool;
}

#[must_use]
pub struct GraphWidgetDriver<W: GraphWidget + ?Sized> {
    cursor: Option<WCursor<W>>,
    cell: W::Cell,
    debug: bool,

    count: Option<NonZeroU32>,
    operators: Vec<Operator>,
    mode: Mode,
    last_action: Option<Action>,
}

impl<W: GraphWidget + ?Sized> fmt::Debug for GraphWidgetDriver<W>
where
    W::NodeId: fmt::Debug,
    W::PortId: fmt::Debug,
    W::Cell: fmt::Debug,
    W::Point: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            cursor,
            cell,
            debug,
            count,
            operators,
            mode,
            last_action,
        } = self;
        f.debug_struct("GraphWidgetDriver")
            .field("cursor", cursor)
            .field("cell", cell)
            .field("debug", debug)
            .field("count", count)
            .field("operators", operators)
            .field("mode", mode)
            .field("last_action", last_action)
            .finish()
    }
}

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    pub fn new(widget: &W) -> Self {
        let cursor = widget.default_cursor();
        let me = Self {
            cell: W::Cell::of_cursor(widget, &cursor),
            cursor: Some(cursor),
            debug: false,
            count: None,
            operators: vec![],
            mode: Mode::default(),
            last_action: None,
        };

        debug_assert_eq!(me.status(), Status::default());

        me
    }

    #[inline]
    pub fn cursor(&self) -> &WCursor<W> { self.cursor.as_ref().unwrap_or_else(|| unreachable!()) }

    #[inline]
    pub fn cursor_cell(&self) -> &W::Cell { &self.cell }

    #[inline]
    pub fn view_debug(&self) -> bool { self.debug }
}
