use std::{
    fmt,
    num::{NonZeroIsize, NonZeroU32},
};

pub use crate::cursor::Cursor;
use crate::{bindings::Mode, status::Status};

mod actions;
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

pub struct Port<W: GraphWidget + ?Sized>(pub Side, pub W::Node, pub W::PortIdx);

impl<W: GraphWidget + ?Sized> fmt::Debug for Port<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
    W::Point: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self(side, node, port) = self;
        f.debug_tuple("Port").field(side).field(node).field(port).finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorUpdate {
    Move,
    CenterInView,
}

pub trait GraphWidget {
    type Node;
    type PortIdx;
    type Row;
    type Col;
    type Point;
    type Context<'a>;

    fn cursor_cell(&self, cursor: &Cursor<Self>) -> (Self::Row, Self::Col);

    fn nearest_port(
        &self,
        node: &Self::Node,
        side: Side,
        cell: (&Self::Row, &Self::Col),
    ) -> Option<(Self::PortIdx, Self::Col)>;

    fn step_port_by(
        &self,
        port: &Port<Self>,
        count: isize,
    ) -> (Option<(NonZeroIsize, Self::PortIdx)>, Self::Row);

    fn port_connection(&self, port: &Port<Self>) -> Option<(Port<Self>, Self::Row, Self::Col)>;

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
    cell: Option<(W::Row, W::Col)>,
    count: Option<NonZeroU32>,
    mode: Mode,
}

impl<W: GraphWidget + ?Sized> fmt::Debug for GraphWidgetDriver<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
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
    pub fn cursor_cell(&self) -> Option<(&W::Row, &W::Col)> {
        let (row, col) = self.cell.as_ref()?;
        Some((row, col))
    }
}
