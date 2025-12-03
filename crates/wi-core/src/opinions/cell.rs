use std::ops::{Add, Div, Mul, Sub};

use num_traits::Num;

use crate::{
    traits::{GraphWidgetCell, GraphWidgetTypes},
    AlignCell, Cursor, Port, Side, SidedPort, WCursor,
};

/// Implements several functions that enable the use of a Euclidean [Cell]
pub trait GraphEuclidean:
    GraphWidgetTypes<
    NodeId: PartialEq,
    PortId: PartialEq,
    Point: Add<Self::Vector, Output = Self::Point> + Sub<Self::Vector, Output = Self::Point>,
>
{
    type Scalar: Copy + Num;
    type Vector: Copy
        + Add<Self::Vector, Output = Self::Vector>
        + Sub<Self::Vector, Output = Self::Vector>
        + Mul<Self::Scalar, Output = Self::Vector>
        + Div<Self::Scalar, Output = Self::Vector>;

    const ZERO_VEC: Self::Vector;

    fn vec_coords(v: Self::Vector) -> (Self::Scalar, Self::Scalar);
    fn point_coords(p: Self::Point) -> (Self::Scalar, Self::Scalar);

    fn vec(coords: (Self::Scalar, Self::Scalar)) -> Self::Vector;

    #[inline]
    fn point_vec(p: Self::Point) -> Self::Vector { Self::vec(Self::point_coords(p)) }

    fn anchor_point(&self, anchor: &WAnchor<Self>) -> Self::Point;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor<N, P> {
    Fixed,
    Node(N),
    Port(SidedPort<N, P>),
    Edge(Port<N, P>, Port<N, P>),
}

pub type WAnchor<W> = Anchor<<W as GraphWidgetTypes>::NodeId, <W as GraphWidgetTypes>::PortId>;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum State {
    #[default]
    Init,
    /// Current cell is for a node cursor
    Node1,
    /// Current cell is for a node cursor, and the previous cell was as
    /// well
    Node2,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell<NodeId, PortId, Vector> {
    pub saved_port: Option<(Side, Anchor<NodeId, PortId>, Vector)>,
    pub saved_edge: Option<(Anchor<NodeId, PortId>, Vector)>,
    pub state: State,
    pub anchor: Anchor<NodeId, PortId>,
    pub offset: Vector,
}

pub type WCell<W> = Cell<
    <W as GraphWidgetTypes>::NodeId,
    <W as GraphWidgetTypes>::PortId,
    <W as GraphEuclidean>::Vector,
>;

impl<NodeId, PortId, Vector> Cell<NodeId, PortId, Vector> {
    // Vector: Copy feels like it should be inferred from W, but oh well
    #[inline]
    pub fn node_target<
        W: GraphEuclidean<NodeId = NodeId, PortId = PortId, Vector = Vector> + ?Sized,
    >(
        &self,
        widget: &W,
    ) -> W::Point
    where
        Vector: Copy,
    {
        widget.anchor_point(&self.anchor) + self.offset
    }

    // Vector: Copy feels like it should be inferred from W, but oh well
    #[inline]
    pub fn port_target<
        W: GraphEuclidean<NodeId = NodeId, PortId = PortId, Vector = Vector> + ?Sized,
    >(
        &self,
        widget: &W,
        side: Side,
    ) -> W::Point
    where
        Vector: Copy,
    {
        let (anchor, offset) = self
            .saved_port
            .as_ref()
            .and_then(|(s, a, o)| (*s == side).then_some((a, o)))
            .unwrap_or((&self.anchor, &self.offset));

        widget.anchor_point(anchor) + *offset
    }

    #[inline]
    pub fn edge_target<
        W: GraphEuclidean<NodeId = NodeId, PortId = PortId, Vector = Vector> + ?Sized,
    >(
        &self,
        widget: &W,
    ) -> W::Point
    where
        Vector: Copy,
    {
        let (anchor, offset) = self
            .saved_edge
            .as_ref()
            .map_or((&self.anchor, &self.offset), |(a, o)| (a, o));

        widget.anchor_point(anchor) + *offset
    }
}

impl<W: GraphEuclidean + ?Sized> GraphWidgetCell<W> for Cell<W::NodeId, W::PortId, W::Vector> {
    fn of_cursor(_: &W, cursor: &WCursor<W>) -> Self {
        match cursor {
            Cursor::Node(n) => Self {
                saved_port: None,
                saved_edge: None,
                state: State::Node1,
                anchor: Anchor::Node(*n),
                offset: W::ZERO_VEC,
            },
            &Cursor::Port(ref p @ SidedPort(s, _)) => {
                let anchor = Anchor::Port(*p);
                Self {
                    saved_port: Some((s, anchor, W::ZERO_VEC)),
                    saved_edge: None,
                    state: State::Init,
                    anchor,
                    offset: W::ZERO_VEC,
                }
            },
            Cursor::Edge(e) => {
                let anchor = Anchor::Edge(e.from, e.to);
                let offs = W::ZERO_VEC;

                Self {
                    saved_port: None,
                    saved_edge: Some((anchor, offs)),
                    state: State::Init,
                    anchor,
                    offset: offs,
                }
            },
            &Cursor::FixedPoint(p) => Self {
                saved_port: None,
                saved_edge: None,
                state: State::Init,
                anchor: Anchor::Fixed,
                offset: W::point_vec(p),
            },
        }
    }

    #[inline]
    fn position(&self, widget: &W) -> <W as GraphWidgetTypes>::Point {
        widget.anchor_point(&self.anchor) + self.offset
    }

    fn align_to_cursor(&mut self, widget: &W, cursor: &WCursor<W>, align: AlignCell) {
        let Self {
            saved_port,
            saved_edge,
            state,
            anchor,
            offset,
        } = self;
        let new = Self::of_cursor(widget, cursor);

        let diff = || {
            if new.anchor == *anchor {
                W::vec_coords(W::ZERO_VEC)
            } else {
                W::point_coords(
                    widget.anchor_point(anchor) - W::point_vec(widget.anchor_point(&new.anchor)),
                )
            }
        };

        *state = match (*state, new.state) {
            (State::Node1 | State::Node2, State::Node1) => State::Node2,
            (_, s) => s,
        };

        if let AlignCell::Overwrite = align {
            *saved_port = None;
            *saved_edge = None;
        } else if let State::Node2 = state
            && let Cursor::Node(..) = cursor
        {
            *saved_port = None;
        } else {
            if let Some(c) = new.saved_port {
                *saved_port = Some(c);
            }

            if let Some(c) = new.saved_edge {
                *saved_edge = Some(c);
            }
        }

        let (new_x, new_y) = W::vec_coords(new.offset);
        let (x, y) = W::vec_coords(*offset);
        *offset = W::vec(match align {
            AlignCell::Overwrite => (new_x, new_y),
            AlignCell::KeepRow => (new_x, y + diff().1),
            AlignCell::KeepCol => (x + diff().0, new_y),
        });

        *anchor = new.anchor;
    }
}
