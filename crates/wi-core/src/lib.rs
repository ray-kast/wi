use std::{fmt, num::NonZeroU32};

use crate::{bindings::Mode, status::Status};
pub use crate::cursor::Cursor;

mod actions;
mod bindings;
mod cursor;
mod keyboard;
pub mod modifiers;
pub mod status;
mod trie;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorUpdate {
    Move,
    CenterInView,
}

pub trait GraphWidget {
    type Node;
    type PortIdx;
    type Point;
    type Context<'a>;

    fn in_edges<'a>(&'a self, node: &Self::Node) -> impl IntoIterator<Item = &'a Self::Node>
    where Self::Node: 'a;

    fn out_edges<'a>(&'a self, node: &Self::Node) -> impl IntoIterator<Item = &'a Self::Node>
    where Self::Node: 'a;

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
    cursor: Cursor<W>,
    count: Option<NonZeroU32>,
    mode: Mode,
}

impl<W: GraphWidget + ?Sized> fmt::Debug for GraphWidgetDriver<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
    W::Point: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            cursor,
            count,
            mode,
        } = self;
        f.debug_struct("GraphWidgetDriver")
            .field("cursor", cursor)
            .field("count", count)
            .field("mode", mode)
            .finish()
    }
}

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    pub fn new(cursor: Cursor<W>) -> Self {
        Self {
            cursor,
            count: None,
            mode: Mode::default(),
        }
    }

    #[inline]
    pub fn cursor(&self) -> &Cursor<W> { &self.cursor }
}
