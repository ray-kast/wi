use std::ops;

use ratatui::layout::{HorizontalAlignment, Position, Rect, VerticalAlignment};

pub type Vector = glam::Vec2;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Point(Vector);

#[inline]
fn kept_precision(f: f32, u: u16) -> bool { (f - f32::from(u)).abs() <= 0.5 }

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
            x: vec.x.round() as u16,
            y: vec.y.round() as u16,
        }
    }

    #[must_use]
    pub fn to_position(self) -> Option<Position> {
        let Self(vec) = self;

        if !(vec.x.is_finite() && vec.y.is_finite()) {
            return None;
        }

        let pos = self.as_position_lossy();

        (kept_precision(vec.x, pos.x) && kept_precision(vec.y, pos.y)).then_some(pos)
    }

    #[must_use]
    pub fn to_position_signed(self) -> Option<SignedPosition> {
        let Self(vec) = self;

        if !(vec.x.is_finite() && vec.y.is_finite()) {
            return None;
        }

        let x_rounded = vec.x.abs().round() as u16;
        let y_rounded = vec.y.abs().round() as u16;

        if !(kept_precision(vec.x.abs(), x_rounded) && kept_precision(vec.y.abs(), y_rounded)) {
            return None;
        }

        Some(SignedPosition {
            x: i32::from(x_rounded) * (1 - 2 * i32::from(vec.x.signum() == -1.0)),
            y: i32::from(y_rounded) * (1 - 2 * i32::from(vec.y.signum() == -1.0)),
        })
    }

    #[must_use]
    pub fn to_rect(self, size: Vector) -> Option<SignedRect> {
        let Self(vec) = self;
        let x = RoundedRange::from_f32(vec.x, size.x)?;
        let y = RoundedRange::from_f32(vec.y, size.y)?;

        Some(SignedRect {
            x: x.start,
            y: y.start,
            width: x.len,
            height: y.len,
        })
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

#[derive(Debug, Default, Clone, Copy)]
pub struct SignedPosition {
    pub x: i32,
    pub y: i32,
}

struct RoundedRange {
    start: i32,
    len: u32,
}

impl RoundedRange {
    fn from_f32(start: f32, len: f32) -> Option<Self> {
        #![expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

        if !(start.is_finite() && len.is_finite()) {
            return None;
        }

        let end = start + len;
        let min = start.min(end);
        let max = start.max(end);

        let min_rounded = min.abs().round() as u16;
        let max_rounded = max.round() as u16;

        if !(kept_precision(min.abs(), min_rounded) && kept_precision(max, max_rounded)) {
            return None;
        }

        #[expect(clippy::float_cmp, reason = "It's okay for signum()")]
        let start = i32::from(min_rounded) * (1 - 2 * i32::from(min.signum() == -1.0));

        Some(Self {
            start,
            len: u32::from(max_rounded).checked_sub_signed(start)?,
        })
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Insets {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl Insets {
    pub const ZERO: Self = Self::uniform(0);

    #[inline]
    #[must_use]
    pub const fn uniform(len: u32) -> Self {
        Self {
            left: len,
            top: len,
            right: len,
            bottom: len,
        }
    }

    #[inline]
    #[must_use]
    pub const fn symmetric(horiz: u32, vert: u32) -> Self {
        Self {
            left: horiz,
            top: vert,
            right: horiz,
            bottom: vert,
        }
    }

    #[inline]
    #[must_use]
    pub const fn new(left: u32, top: u32, right: u32, bottom: u32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    #[inline]
    #[must_use]
    pub const fn left(self, left: u32) -> Self { Self { left, ..self } }

    #[inline]
    #[must_use]
    pub const fn top(self, top: u32) -> Self { Self { top, ..self } }

    #[inline]
    #[must_use]
    pub const fn right(self, right: u32) -> Self { Self { right, ..self } }

    #[inline]
    #[must_use]
    pub const fn bottom(self, bottom: u32) -> Self { Self { bottom, ..self } }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SignedRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl From<Rect> for SignedRect {
    fn from(value: Rect) -> Self {
        let Rect {
            x,
            y,
            width,
            height,
        } = value;

        Self {
            x: x.into(),
            y: y.into(),
            width: width.into(),
            height: height.into(),
        }
    }
}

impl SignedRect {
    #[inline]
    #[must_use]
    pub const fn is_empty(self) -> bool { self.width == 0 || self.height == 0 }

    #[must_use]
    pub fn as_bounding_rect(self) -> Rect {
        let Self {
            x,
            y,
            width,
            height,
        } = self;

        Rect {
            x: u16::from(x >= 0) * x.try_into().unwrap_or(u16::MAX),
            y: u16::from(y >= 0) * y.try_into().unwrap_or(u16::MAX),
            width: width
                .saturating_add_signed(i32::from(x < 0) * x)
                .try_into()
                .unwrap_or(u16::MAX),
            height: height
                .saturating_add_signed(i32::from(y < 0) * y)
                .try_into()
                .unwrap_or(u16::MAX),
        }
    }

    #[must_use]
    pub fn as_top_left(self) -> Option<Position> {
        let Self {
            x,
            y,
            width: _,
            height: _,
        } = self;

        Some(Position {
            x: x.try_into().ok()?,
            y: y.try_into().ok()?,
        })
    }

    #[must_use]
    pub fn as_top_right(self, inside: bool) -> Option<Position> {
        let Self {
            x,
            y,
            width,
            height: _,
        } = self;

        Some(Position {
            x: x.saturating_add_unsigned(width.saturating_sub(inside.into()))
                .try_into()
                .ok()?,
            y: y.try_into().ok()?,
        })
    }

    #[must_use]
    pub fn as_bottom_left(self, inside: bool) -> Option<Position> {
        let Self {
            x,
            y,
            width: _,
            height,
        } = self;

        Some(Position {
            x: x.try_into().ok()?,
            y: y.saturating_add_unsigned(height.saturating_sub(inside.into()))
                .try_into()
                .ok()?,
        })
    }

    #[must_use]
    pub fn as_bottom_right(self, inside: bool) -> Option<Position> {
        let Self {
            x,
            y,
            width,
            height,
        } = self;

        Some(Position {
            x: x.saturating_add_unsigned(width.saturating_sub(inside.into()))
                .try_into()
                .ok()?,
            y: y.saturating_add_unsigned(height.saturating_sub(inside.into()))
                .try_into()
                .ok()?,
        })
    }

    #[must_use]
    pub fn as_center(self) -> Option<Position> {
        let Self {
            x,
            y,
            width,
            height,
        } = self;

        Some(Position {
            x: x.saturating_add_unsigned(width / 2).try_into().ok()?,
            y: y.saturating_add_unsigned(height / 2).try_into().ok()?,
        })
    }

    #[must_use]
    pub const fn inset(self, insets: Insets) -> Self {
        let Self {
            x,
            y,
            width,
            height,
        } = self;
        let Insets {
            left,
            top,
            right,
            bottom,
        } = insets;

        Self {
            x: x.saturating_add_unsigned(left),
            y: y.saturating_add_unsigned(top),
            width: width.saturating_sub(left).saturating_sub(right),
            height: height.saturating_sub(top).saturating_sub(bottom),
        }
    }

    #[must_use]
    pub const fn nudge_add(self, x: u32, y: u32) -> Self {
        Self {
            x: self.x.saturating_add_unsigned(x),
            y: self.y.saturating_add_unsigned(y),
            ..self
        }
    }

    #[must_use]
    pub const fn nudge_sub(self, x: u32, y: u32) -> Self {
        Self {
            x: self.x.saturating_sub_unsigned(x),
            y: self.y.saturating_sub_unsigned(y),
            ..self
        }
    }

    #[must_use]
    pub const fn nudge_add_signed(self, x: i32, y: i32) -> Self {
        Self {
            x: self.x.saturating_add(x),
            y: self.y.saturating_add(y),
            ..self
        }
    }

    #[must_use]
    pub const fn nudge_sub_signed(self, x: i32, y: i32) -> Self {
        Self {
            x: self.x.saturating_sub(x),
            y: self.y.saturating_sub(y),
            ..self
        }
    }

    #[inline]
    #[must_use]
    pub const fn width(self, width: u32) -> Self { Self { width, ..self } }

    #[inline]
    #[must_use]
    pub const fn height(self, height: u32) -> Self { Self { height, ..self } }

    #[inline]
    #[must_use]
    pub const fn width_aligned(self, width: u32, align: HorizontalAlignment) -> Self {
        let gap = self.width.abs_diff(width);
        let bigger = width > self.width;

        Self {
            x: match (align, bigger) {
                (HorizontalAlignment::Left, _) => self.x,
                (HorizontalAlignment::Center, true) => self.x.saturating_sub_unsigned(gap / 2),
                (HorizontalAlignment::Center, false) => self.x.saturating_add_unsigned(gap / 2),
                (HorizontalAlignment::Right, true) => self.x.saturating_sub_unsigned(gap),
                (HorizontalAlignment::Right, false) => self.x.saturating_add_unsigned(gap),
            },
            width,
            ..self
        }
    }

    #[inline]
    #[must_use]
    pub const fn height_aligned(self, height: u32, align: VerticalAlignment) -> Self {
        let gap = self.height.abs_diff(height);
        let bigger = height > self.height;

        Self {
            y: match (align, bigger) {
                (VerticalAlignment::Top, _) => self.y,
                (VerticalAlignment::Center, true) => self.y.saturating_sub_unsigned(gap / 2),
                (VerticalAlignment::Center, false) => self.y.saturating_add_unsigned(gap / 2),
                (VerticalAlignment::Bottom, true) => self.y.saturating_sub_unsigned(gap),
                (VerticalAlignment::Bottom, false) => self.y.saturating_add_unsigned(gap),
            },
            height,
            ..self
        }
    }
}
