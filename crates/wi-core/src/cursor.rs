use std::fmt;

use crate::{GraphWidget, Port};

pub enum Cursor<W: GraphWidget + ?Sized> {
    Node(W::Node),
    Port(Port<W>),
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
            Self::Node(n) => f.debug_tuple("Node").field(n).finish(),
            Self::Port(p) => f.debug_tuple("Port").field(p).finish(),
            Self::FixedPoint(p) => f.debug_tuple("FixedPoint").field(p).finish(),
        }
    }
}
