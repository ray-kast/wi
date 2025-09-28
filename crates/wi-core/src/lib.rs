use std::fmt;

pub use crate::cursor::Cursor;

mod cursor;
mod keyboard;

pub trait GraphWidget {
    type Node;
    type PortIdx;
    type Point;
    type EventCtx<'a>;

    fn in_edges<'a>(&'a self, node: &Self::Node) -> impl IntoIterator<Item = &'a Self::Node>
    where Self::Node: 'a;

    fn out_edges<'a>(&'a self, node: &Self::Node) -> impl IntoIterator<Item = &'a Self::Node>
    where Self::Node: 'a;

    fn view_cursor(&mut self, cursor: &Cursor<Self>, ctx: &mut Self::EventCtx<'_>);
}

#[must_use]
pub struct GraphWidgetDriver<W: GraphWidget + ?Sized> {
    cursor: Cursor<W>,
    keyboard: keyboard::KeyboardHandler,
}

impl<W: GraphWidget + ?Sized> fmt::Debug for GraphWidgetDriver<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
    W::Point: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { cursor, keyboard } = self;
        f.debug_struct("GraphWidgetDriver")
            .field("cursor", cursor)
            .field("keyboard", keyboard)
            .finish()
    }
}

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    pub fn new(cursor: Cursor<W>) -> Self {
        Self {
            cursor,
            keyboard: keyboard::KeyboardHandler::default(),
        }
    }

    #[inline]
    pub fn cursor(&self) -> &Cursor<W> { &self.cursor }
}
