use std::{
    fmt,
    num::{NonZeroIsize, NonZeroU32},
};

pub use crate::cursor::{Cursor, EdgeCursor, WCursor, WEdgeCursor};
use crate::{bindings::Mode, status::Status};

mod action;
mod bindings;
mod cursor;
mod keyboard;
pub mod modifiers;
pub mod status;
mod trie;

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

pub type WPort<W> = Port<<W as GraphWidget>::Node, <W as GraphWidget>::PortIdx>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SidedPort<N, P>(pub Side, pub Port<N, P>);

pub type WSidedPort<W> = SidedPort<<W as GraphWidget>::Node, <W as GraphWidget>::PortIdx>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorUpdate {
    Move,
    CenterInView,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cell<A, R, C> {
    pub anchor: A,
    pub row: R,
    pub col: C,
}

impl<A, R, C> Cell<A, R, C> {
    #[inline]
    fn as_ref(&self) -> Cell<&A, &R, &C> {
        Cell {
            anchor: &self.anchor,
            row: &self.row,
            col: &self.col,
        }
    }
}

pub type WCell<W> =
    Cell<<W as GraphWidget>::CellAnchor, <W as GraphWidget>::Row, <W as GraphWidget>::Col>;

pub type WCellRef<'a, W> = Cell<
    &'a <W as GraphWidget>::CellAnchor,
    &'a <W as GraphWidget>::Row,
    &'a <W as GraphWidget>::Col,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreserveCell<A, R, C> {
    Overwrite,
    Row(A, R),
    Col(A, C),
}

pub type WPreserveCell<W> =
    PreserveCell<<W as GraphWidget>::CellAnchor, <W as GraphWidget>::Row, <W as GraphWidget>::Col>;

pub type WPreserveCellRef<'a, W> = PreserveCell<
    &'a <W as GraphWidget>::CellAnchor,
    &'a <W as GraphWidget>::Row,
    &'a <W as GraphWidget>::Col,
>;

pub trait GraphWidget {
    type Node;
    type PortIdx;

    type CellAnchor;
    type Row;
    type Col;

    type Point;

    type Context<'a>;

    fn cursor_cell(&self, cursor: &WCursor<Self>, keep: WPreserveCellRef<Self>) -> WCell<Self>;

    fn nearest_port(
        &self,
        node: &Self::Node,
        side: Side,
        cell: WCellRef<Self>,
    ) -> Option<Self::PortIdx>;

    fn step_port_by(
        &self,
        port: &WSidedPort<Self>,
        count: isize,
    ) -> Option<(NonZeroIsize, Self::PortIdx)>;

    fn nearest_edge(&self, port: &WSidedPort<Self>, cell: WCellRef<Self>) -> Option<WEdgeCursor<Self>>;

    fn step_edge_by(
        &self,
        edge: &WEdgeCursor<Self>,
        count: isize,
    ) -> Option<(NonZeroIsize, WPort<Self>)>;

    fn update_cursor(
        &mut self,
        update: CursorUpdate,
        cursor: &WCursor<Self>,
        ctx: &mut Self::Context<'_>,
    );

    fn update_status(&mut self, status: Status, ctx: &mut Self::Context<'_>);
}

#[must_use]
pub struct GraphWidgetDriver<W: GraphWidget + ?Sized> {
    cursor: Option<WCursor<W>>,
    cell: Option<WCell<W>>,
    count: Option<NonZeroU32>,
    mode: Mode,
}

impl<W: GraphWidget + ?Sized> fmt::Debug for GraphWidgetDriver<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
    W::CellAnchor: fmt::Debug,
    W::Row: fmt::Debug,
    W::Col: fmt::Debug,
    W::Point: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            cursor,
            cell,
            count,
            mode,
        } = self;
        f.debug_struct("GraphWidgetDriver")
            .field("cursor", cursor)
            .field("cell", cell)
            .field("count", count)
            .field("mode", mode)
            .finish()
    }
}

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    pub fn new(cursor: WCursor<W>) -> Self {
        Self {
            cursor: Some(cursor),
            count: None,
            cell: None,
            mode: Mode::default(),
        }
    }

    #[inline]
    pub fn cursor(&self) -> &WCursor<W> { self.cursor.as_ref().unwrap_or_else(|| unreachable!()) }

    #[inline]
    pub fn cursor_cell(&self) -> Option<WCellRef<'_, W>> { Some(self.cell.as_ref()?.as_ref()) }
}
