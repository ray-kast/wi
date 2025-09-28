use std::fmt;

use crate::GraphWidget;

pub enum Cursor<W: GraphWidget + ?Sized> {
    Node(W::Node),
    InPort(W::Node, W::PortIdx),
    OutPort(W::Node, W::PortIdx),
    FixedPoint(W::Point),
}

impl<W: GraphWidget + ?Sized> fmt::Debug for Cursor<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
    W::Point: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Cursor::Node(n) => f.debug_tuple("Node").field(n).finish(),
            Cursor::InPort(n, p) => f.debug_tuple("InPort").field(n).field(p).finish(),
            Cursor::OutPort(n, p) => f.debug_tuple("OutPort").field(n).field(p).finish(),
            Cursor::FixedPoint(p) => f.debug_tuple("FixedPoint").field(p).finish(),
        }
    }
}
