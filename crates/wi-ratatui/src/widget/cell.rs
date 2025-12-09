use wi_core::{
    opinions::cell::{Anchor, GraphEuclidean, WAnchor},
    Port, SidedPort,
};

use super::{core::EditorCore, node::NodeExt};
use crate::{
    graph::TuiNode,
    vector::{Point, Vector},
};

impl<N: TuiNode> GraphEuclidean for EditorCore<N> {
    type Scalar = f32;
    type Vector = Vector;

    const ZERO_VEC: Self::Vector = Vector::ZERO;

    #[inline]
    fn vec_coords(v: Self::Vector) -> (Self::Scalar, Self::Scalar) { (v.x, v.y) }

    #[inline]
    fn point_coords(p: Self::Point) -> (Self::Scalar, Self::Scalar) { Self::vec_coords(p.as_vec()) }

    #[inline]
    fn vec((x, y): (Self::Scalar, Self::Scalar)) -> Self::Vector { Vector { x, y } }

    #[inline]
    fn anchor_point(&self, &anchor: &WAnchor<Self>) -> Self::Point {
        match anchor {
            Anchor::Fixed => Point::ZERO,
            Anchor::Node(n) => {
                let node = &self.graph[n];
                node.position() + 0.5 * node.outer_size()
            },
            Anchor::Port(SidedPort(s, Port(n, p))) => self.graph[n].port_pos(p, s, false),
            Anchor::Edge(Port(f, fp), Port(t, tp)) => {
                self.graph[f].edge_midpoint(fp, &self.graph[t], tp)
            },
        }
    }
}
