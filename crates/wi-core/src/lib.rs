use std::{
    fmt,
    num::{NonZeroIsize, NonZeroU32},
};

pub use crate::cursor::Cursor;
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

pub struct Port<W: GraphWidget + ?Sized>(pub W::Node, pub W::PortIdx);

impl<W: GraphWidget + ?Sized> Clone for Port<W>
where
    W::Node: Clone,
    W::PortIdx: Clone,
{
    fn clone(&self) -> Self { Self(self.0.clone(), self.1.clone()) }

    fn clone_from(&mut self, source: &Self) {
        let Self(n, p) = self;
        n.clone_from(&source.0);
        p.clone_from(&source.1);
    }
}

impl<W: GraphWidget + ?Sized> Copy for Port<W>
where
    W::Node: Copy,
    W::PortIdx: Copy,
{
}

impl<W: GraphWidget + ?Sized> fmt::Debug for Port<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(node, port) = self;
        f.debug_tuple("Port").field(node).field(port).finish()
    }
}

impl<W: GraphWidget + ?Sized> PartialEq for Port<W>
where
    W::Node: PartialEq,
    W::PortIdx: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        let Self(n, p) = self;
        *n == other.0 && *p == other.1
    }
}

pub struct SidedPort<W: GraphWidget + ?Sized>(pub Side, pub W::Node, pub W::PortIdx);

impl<W: GraphWidget + ?Sized> Clone for SidedPort<W>
where
    W::Node: Clone,
    W::PortIdx: Clone,
{
    fn clone(&self) -> Self { Self(self.0, self.1.clone(), self.2.clone()) }

    fn clone_from(&mut self, source: &Self) {
        let Self(s, n, p) = self;
        *s = source.0;
        n.clone_from(&source.1);
        p.clone_from(&source.2);
    }
}

impl<W: GraphWidget + ?Sized> Copy for SidedPort<W>
where
    W::Node: Copy,
    W::PortIdx: Copy,
{
}

impl<W: GraphWidget + ?Sized> fmt::Debug for SidedPort<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(side, node, port) = self;
        f.debug_tuple("SidedPort")
            .field(side)
            .field(node)
            .field(port)
            .finish()
    }
}

impl<W: GraphWidget + ?Sized> PartialEq for SidedPort<W>
where
    W::Node: PartialEq,
    W::PortIdx: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        let Self(s, n, p) = self;
        *s == other.0 && *n == other.1 && *p == other.2
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorUpdate {
    Move,
    CenterInView,
}

pub enum PreserveCell<'a, W: GraphWidget + ?Sized> {
    Overwrite,
    Row(&'a W::CellAnchor, &'a W::Row),
    Col(&'a W::CellAnchor, &'a W::Col),
}

impl<W: GraphWidget + ?Sized> fmt::Debug for PreserveCell<'_, W>
where
    W::CellAnchor: fmt::Debug,
    W::Row: fmt::Debug,
    W::Col: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overwrite => f.write_str("Overwrite"),
            Self::Row(a, r) => f.debug_tuple("Row").field(&a).field(&r).finish(),
            Self::Col(a, c) => f.debug_tuple("Col").field(&a).field(&c).finish(),
        }
    }
}

pub trait GraphWidget {
    type Node;
    type PortIdx;
    type EdgeIdx;

    type CellAnchor;
    type Row;
    type Col;

    type Point;

    type Context<'a>;

    fn cursor_cell(
        &self,
        cursor: &Cursor<Self>,
        keep: PreserveCell<Self>,
    ) -> (Self::CellAnchor, Self::Row, Self::Col);

    fn nearest_port(
        &self,
        node: &Self::Node,
        side: Side,
        cell: (&Self::CellAnchor, &Self::Row, &Self::Col),
    ) -> Option<Self::PortIdx>;

    fn step_port_by(
        &self,
        port: &SidedPort<Self>,
        count: isize,
    ) -> Option<(NonZeroIsize, Self::PortIdx)>;

    fn nearest_edge(
        &self,
        port: &SidedPort<Self>,
        cell: (&Self::CellAnchor, &Self::Row, &Self::Col),
    ) -> Option<Self::EdgeIdx>;

    fn step_edge_by(
        &self,
        port: &SidedPort<Self>,
        edge: &Self::EdgeIdx,
        count: isize,
    ) -> Option<(NonZeroIsize, Self::EdgeIdx)>;

    fn edge_port(&self, port: &SidedPort<Self>, edge: &Self::EdgeIdx, side: Side) -> Port<Self>;

    fn update_cursor(
        &mut self,
        update: CursorUpdate,
        cursor: &Cursor<Self>,
        ctx: &mut Self::Context<'_>,
    );

    fn update_status(&mut self, status: Status, ctx: &mut Self::Context<'_>);
}

#[must_use]
pub struct GraphWidgetDriver<W: GraphWidget + ?Sized> {
    cursor: Option<Cursor<W>>,
    cell: Option<(W::CellAnchor, W::Row, W::Col)>,
    count: Option<NonZeroU32>,
    mode: Mode,
}

impl<W: GraphWidget + ?Sized> fmt::Debug for GraphWidgetDriver<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
    W::EdgeIdx: fmt::Debug,
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
    pub fn new(cursor: Cursor<W>) -> Self {
        Self {
            cursor: Some(cursor),
            count: None,
            cell: None,
            mode: Mode::default(),
        }
    }

    #[inline]
    pub fn cursor(&self) -> &Cursor<W> { self.cursor.as_ref().unwrap_or_else(|| unreachable!()) }

    #[inline]
    pub fn cursor_cell(&self) -> Option<(&W::CellAnchor, &W::Row, &W::Col)> {
        let (anchor, row, col) = self.cell.as_ref()?;
        Some((anchor, row, col))
    }
}
