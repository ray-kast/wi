use std::ops;

use ratatui::layout::{Position, Rect};

pub type Vector = glam::Vec2;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Point(Vector);

#[inline]
fn kept_precision(f: f32, u: u16) -> bool { (f - f32::from(u)).abs() <= 0.5 }

struct RoundedRange {
    min: u16,
    size: u16,
    start_offs: u16,
}

fn round_point_offs_to_range(point: f32, offs: f32) -> Option<RoundedRange> {
    #![expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]

    let other = point + offs;
    let min = point.min(other);
    let max = point.max(other);

    let start_offs = (-min) as u16;
    let min_rounded = min as u16;
    let max_rounded = max as u16;

    let size = max_rounded.saturating_sub(min_rounded);

    let min_err = min - f32::from(min_rounded);
    let max_err = max - f32::from(max_rounded);

    if min_err < -0.5 && max_err > 0.5 {
        Some(RoundedRange {
            min: min_rounded,
            size,
            start_offs,
        })
    } else {
        (kept_precision(min, min_rounded)
            && kept_precision(max, max_rounded)
            && (min >= 0.0 || kept_precision(-min, start_offs)))
        .then_some(RoundedRange {
            min: min_rounded,
            size,
            start_offs,
        })
    }
}

impl Point {
    pub const ZERO: Self = Self(Vector::ZERO);

    #[inline]
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self { Self(Vector { x, y }) }

    #[inline]
    #[must_use]
    pub const fn from_vec(vec: Vector) -> Self { Self(vec) }

    #[must_use]
    pub const fn as_position_lossy(self) -> Position {
        #![expect(clippy::cast_sign_loss, clippy::cast_possible_truncation)]

        let Self(vec) = self;
        Position {
            x: vec.x.round_ties_even() as u16,
            y: vec.y.round_ties_even() as u16,
        }
    }

    #[must_use]
    pub fn to_position(self) -> Option<Position> {
        let Self(vec) = self;
        let pos = self.as_position_lossy();

        (kept_precision(vec.x, pos.x) && kept_precision(vec.y, pos.y)).then_some(pos)
    }

    #[must_use]
    pub fn to_rect_offset(self, size: Vector) -> Option<(Rect, Position)> {
        let Self(vec) = self;
        let x = round_point_offs_to_range(vec.x, size.x)?;
        let y = round_point_offs_to_range(vec.y, size.y)?;

        Some((
            Rect {
                x: x.min,
                y: y.min,
                width: x.size,
                height: y.size,
            },
            Position {
                x: x.start_offs,
                y: y.start_offs,
            },
        ))
    }

    #[inline]
    #[must_use]
    pub const fn as_vec(&self) -> Vector { self.0 }
}

impl From<Position> for Point {
    #[inline]
    fn from(value: Position) -> Self {
        Self(Vector {
            x: value.x.into(),
            y: value.y.into(),
        })
    }
}

impl From<Vector> for Point {
    #[inline]
    fn from(value: Vector) -> Self { Self(value) }
}

impl ops::Add<Vector> for Point {
    type Output = Point;

    #[inline]
    fn add(self, rhs: Vector) -> Self::Output { Point(self.0 + rhs) }
}

impl ops::Sub<Vector> for Point {
    type Output = Point;

    #[inline]
    fn sub(self, rhs: Vector) -> Self::Output { Point(self.0 - rhs) }
}
