use masonry::{
    kurbo::{Affine, Circle, PathEl, Point, Stroke, Vec2},
    peniko::color::OpaqueColor,
    vello::Scene,
};
use wi_core::{
    cell::euclidean::{Anchor, GraphEuclidean, State, WAnchor},
    Port, SidedPort,
};

use super::core::EditorCore;
use crate::{graph::Node, widget::node::NodeExt};

impl<N: Node> GraphEuclidean for EditorCore<N> {
    type Scalar = f64;
    type Vector = Vec2;

    const ZERO_VEC: Self::Vector = Vec2::ZERO;

    #[inline]
    fn vec_coords(v: Self::Vector) -> (Self::Scalar, Self::Scalar) { v.into() }

    #[inline]
    fn point_coords(p: Self::Point) -> (Self::Scalar, Self::Scalar) { p.into() }

    #[inline]
    fn vec(coords: (Self::Scalar, Self::Scalar)) -> Self::Vector { coords.into() }

    #[inline]
    fn point_vec(p: Self::Point) -> Self::Vector { p.to_vec2() }

    fn anchor_point(&self, anchor: &WAnchor<Self>) -> Self::Point {
        match *anchor {
            Anchor::Fixed => Point::ZERO,
            Anchor::Node(n) => {
                let node = &self.graph[n];
                node.position()
                    + Vec2::new(node.outer_size().width * 0.5, 0.0)
                    + Vec2::new(0.0, node.padding().y0)
            },
            Anchor::Port(SidedPort(s, Port(n, p))) => self.graph[n].port_pos(p, s),
            Anchor::Edge(i, o) => {
                let Port(from_node, from_port) = i;
                let Port(to_node, to_port) = o;

                let from_node = &self.graph[from_node];
                let to_node = &self.graph[to_node];

                from_node.edge_midpoint(from_port, to_node, to_port)
            },
        }
    }
}

pub type Cell<N> = wi_core::cell::euclidean::WCell<EditorCore<N>>;

pub fn debug<N: Node>(
    cell: &Cell<N>,
    graph: &EditorCore<N>,
    scene: &mut Scene,
    tf: Affine,
    weight: f64,
) {
    let Cell::<N> {
        saved_port,
        saved_edge,
        state,
        anchor,
        offset,
    } = *cell;

    let brush = match state {
        State::Init => OpaqueColor::from_rgb8(0x00, 0xff, 0x00),
        State::Node1 => OpaqueColor::from_rgb8(0xff, 0xff, 0x00),
        State::Node2 => OpaqueColor::from_rgb8(0xff, 0x7f, 0x00),
    }
    .with_alpha(0.5);

    let stroke = Stroke::new(weight);

    if let Some((_, anchor, offset)) = saved_port {
        let p = graph.anchor_point(&anchor) + offset;
        scene.stroke(&stroke, tf, brush, None, &Circle::new(p, 8.0));
    }

    if let Some((anchor, offset)) = saved_edge {
        let p = graph.anchor_point(&anchor) + offset;
        scene.stroke(&stroke, tf, brush, None, &Circle::new(p, 8.0));
    }

    let p = graph.anchor_point(&anchor) + offset;
    scene.stroke(&stroke, tf, brush, None, &[
        PathEl::MoveTo(p + Vec2::new(-36.0, 0.0)),
        PathEl::LineTo(p + Vec2::new(-24.0, 0.0)),
        PathEl::MoveTo(p + Vec2::new(24.0, 0.0)),
        PathEl::LineTo(p + Vec2::new(36.0, 0.0)),
        PathEl::MoveTo(p + Vec2::new(0.0, -36.0)),
        PathEl::LineTo(p + Vec2::new(0.0, -24.0)),
        PathEl::MoveTo(p + Vec2::new(0.0, 24.0)),
        PathEl::LineTo(p + Vec2::new(0.0, 36.0)),
    ]);
}
