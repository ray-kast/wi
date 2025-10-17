use std::{
    fmt,
    num::{NonZeroIsize, NonZeroU32},
};

use crate::bindings::Mode;
pub use crate::{
    action::Action,
    bindings::ModeKind,
    cursor::{Cursor, EdgeCursor, WCursor, WEdgeCursor},
    status::Status,
};

mod action;
mod bindings;
pub mod cell;
mod cursor;
mod jump;
mod keyboard;
pub mod modifiers;
mod selection;
mod status;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignCell {
    Overwrite,
    KeepRow,
    KeepCol,
}

pub trait GraphWidgetCell<W: GraphWidget + ?Sized> {
    fn of_cursor(widget: &W, cursor: &WCursor<W>) -> Self;

    fn align_to_cursor(&mut self, widget: &W, cursor: &WCursor<W>, align: AlignCell);
}

pub trait GraphWidget {
    type Node;
    type PortIdx;

    type Cell: GraphWidgetCell<Self>;
    type Point;

    type Context<'a>;

    fn default_cursor(&self) -> WCursor<Self>;

    fn nearest_port(
        &self,
        node: &Self::Node,
        side: Side,
        cell: &Self::Cell,
    ) -> Option<Self::PortIdx>;

    fn step_port_by(
        &self,
        port: &WSidedPort<Self>,
        count: isize,
    ) -> Option<(NonZeroIsize, Self::PortIdx)>;

    fn nearest_edge(&self, port: &WSidedPort<Self>, cell: &Self::Cell)
        -> Option<WEdgeCursor<Self>>;

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
    cell: W::Cell,
    debug: bool,

    count: Option<NonZeroU32>,
    mode: Mode,
    last_action: Option<Action>,
}

impl<W: GraphWidget + ?Sized> fmt::Debug for GraphWidgetDriver<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
    W::Cell: fmt::Debug,
    W::Point: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            cursor,
            cell,
            debug,
            count,
            mode,
            last_action,
        } = self;
        f.debug_struct("GraphWidgetDriver")
            .field("cursor", cursor)
            .field("cell", cell)
            .field("debug", debug)
            .field("count", count)
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
