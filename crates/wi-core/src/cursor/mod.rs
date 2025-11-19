use crate::{traits::GraphWidgetTypes, Port, Side, SidedPort};

pub mod euclidean_cell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeCursor<N, P> {
    pub from: Port<N, P>,
    pub to: Port<N, P>,
    pub anchor: Side,
}

impl<N, P> EdgeCursor<N, P> {
    pub fn new(port: SidedPort<N, P>, opp: Port<N, P>) -> Self {
        let SidedPort(side, port) = port;
        let (from, to) = match side {
            Side::In => (opp, port),
            Side::Out => (port, opp),
        };

        Self {
            from,
            to,
            anchor: side.flip(),
        }
    }

    pub fn anchor_port(&self) -> (Side, &Port<N, P>) {
        (self.anchor.flip(), match self.anchor {
            Side::In => &self.from,
            Side::Out => &self.to,
        })
    }

    pub fn free_port(&self) -> &Port<N, P> {
        match self.anchor {
            Side::In => &self.to,
            Side::Out => &self.from,
        }
    }

    pub fn into_port(self, want: Side) -> SidedPort<N, P> {
        SidedPort(want.flip(), match want {
            Side::In => self.from,
            Side::Out => self.to,
        })
    }

    #[must_use]
    pub fn step(self, port: Port<N, P>) -> Self {
        let Self { from, to, anchor } = self;

        let (from, to) = match anchor {
            Side::In => (from, port),
            Side::Out => (port, to),
        };

        Self { from, to, anchor }
    }
}

pub type WEdgeCursor<W> =
    EdgeCursor<<W as GraphWidgetTypes>::NodeId, <W as GraphWidgetTypes>::PortId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cursor<NodeId, PortId, Point> {
    Node(NodeId),
    Port(SidedPort<NodeId, PortId>),
    Edge(EdgeCursor<NodeId, PortId>),
    FixedPoint(Point),
}

pub type WCursor<W> = Cursor<
    <W as GraphWidgetTypes>::NodeId,
    <W as GraphWidgetTypes>::PortId,
    <W as GraphWidgetTypes>::Point,
>;
