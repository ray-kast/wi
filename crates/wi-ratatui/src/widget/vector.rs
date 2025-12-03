use std::ops;

use ratatui::layout::Position;
use wi_core::opinions::cell::Anchor;

use super::{core::EditorCore, TuiNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vector(pub Position);

impl ops::Add<Vector> for Vector {
    type Output = Vector;

    #[inline]
    fn add(self, rhs: Vector) -> Self::Output {
        Self(Position {
            x: self.0.x + rhs.0.x,
            y: self.0.y + rhs.0.y,
        })
    }
}

impl ops::Sub<Vector> for Vector {
    type Output = Vector;

    #[inline]
    fn sub(self, rhs: Vector) -> Self::Output {
        Self(Position {
            x: self.0.x - rhs.0.x,
            y: self.0.y - rhs.0.y,
        })
    }
}

impl ops::Mul<u16> for Vector {
    type Output = Vector;

    #[inline]
    fn mul(self, rhs: u16) -> Self::Output {
        Self(Position {
            x: self.0.x * rhs,
            y: self.0.y * rhs,
        })
    }
}

impl ops::Div<u16> for Vector {
    type Output = Vector;

    #[inline]
    fn div(self, rhs: u16) -> Self::Output {
        Self(Position {
            x: self.0.x / rhs,
            y: self.0.y / rhs,
        })
    }
}

impl ops::Add<Vector> for Position {
    type Output = Position;

    #[inline]
    fn add(self, rhs: Vector) -> Self::Output { (Vector(self) + rhs).0 }
}

impl ops::Sub<Vector> for Position {
    type Output = Position;

    #[inline]
    fn sub(self, rhs: Vector) -> Self::Output { (Vector(self) - rhs).0 }
}

impl<N: TuiNode> wi_core::opinions::cell::GraphEuclidean for EditorCore<N> {
    type Scalar = u16;
    type Vector = Vector;

    const ZERO_VEC: Self::Vector = Vector(Position::ORIGIN);

    #[inline]
    fn vec_coords(v: Self::Vector) -> (Self::Scalar, Self::Scalar) { (v.0.x, v.0.y) }

    #[inline]
    fn point_coords(p: Self::Point) -> (Self::Scalar, Self::Scalar) { (p.x, p.y) }

    #[inline]
    fn vec(coords: (Self::Scalar, Self::Scalar)) -> Self::Vector {
        Vector(Position {
            x: coords.0,
            y: coords.1,
        })
    }

    #[inline]
    fn anchor_point(&self, anchor: &wi_core::opinions::cell::WAnchor<Self>) -> Self::Point {
        match anchor {
            Anchor::Fixed => Position::ORIGIN,
            Anchor::Node(n) => todo!(),
            Anchor::Port(p) => todo!(),
            Anchor::Edge(f, t) => todo!(),
        }
    }
}
