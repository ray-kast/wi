use std::cmp::Ordering;

use ratatui::{
    layout::{Position, Size},
    prelude::{Buffer, Rect},
    style::{Color, Style},
    widgets::Widget,
};

use crate::vector::SignedPosition;

bitflags::bitflags! {
    #[derive(Clone, Copy)]
    struct EdgeFlags: u8 {
        const LEFT   = 0b0001;
        const TOP    = 0b0010;
        const RIGHT  = 0b0100;
        const BOTTOM = 0b1000;

        const HORIZ = Self::LEFT.bits() | Self::RIGHT.bits();
        const VERT = Self::TOP.bits() | Self::BOTTOM.bits();

        const CORNER_LT = Self::LEFT.bits() | Self::TOP.bits();
        const CORNER_LB = Self::LEFT.bits() | Self::BOTTOM.bits();
        const CORNER_RT = Self::RIGHT.bits() | Self::TOP.bits();
        const CORNER_RB = Self::RIGHT.bits() | Self::BOTTOM.bits();
    }
}

#[derive(Debug, Clone, Copy)]
enum CellState {
    Zero,
    One,
    Many,
}

impl CellState {
    const fn inc(self, inc: bool) -> Self {
        if inc {
            match self {
                Self::Zero => Self::One,
                Self::One | Self::Many => Self::Many,
            }
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct EdgeCell {
    left: CellState,
    top: CellState,
    right: CellState,
    bottom: CellState,
    fg: Color,
}

impl EdgeCell {
    const BLANK: Self = Self {
        left: CellState::Zero,
        top: CellState::Zero,
        right: CellState::Zero,
        bottom: CellState::Zero,
        fg: Color::Reset,
    };

    fn write(&mut self, flags: EdgeFlags, fg: Color) {
        if fg != self.fg {
            *self = Self { fg, ..Self::BLANK }
        }

        self.left = self.left.inc(flags.contains(EdgeFlags::LEFT));
        self.top = self.top.inc(flags.contains(EdgeFlags::TOP));
        self.right = self.right.inc(flags.contains(EdgeFlags::RIGHT));
        self.bottom = self.bottom.inc(flags.contains(EdgeFlags::BOTTOM));
    }

    const fn into_bits(self) -> Option<(char, Color)> {
        mod imp {
            use super::{
                CellState,
                CellState::{Many as SX, One as S1, Zero as S0},
            };

            include!(concat!(env!("OUT_DIR"), "/edge_cell_char.rs"));
        }

        let Self {
            left,
            top,
            right,
            bottom,
            fg,
        } = self;

        if let Some(char) = imp::cell_char(left, top, right, bottom) {
            Some((char, fg))
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub struct EdgeBuffer {
    size: Size,
    buf: Box<[EdgeCell]>,
}

impl EdgeBuffer {
    pub fn new(size: Size) -> Self {
        let this = Self {
            size,
            buf: vec![EdgeCell::BLANK; usize::from(size.width).strict_mul(size.height.into())]
                .into_boxed_slice(),
        };

        assert!(this.buf.len() == usize::from(size.width).strict_mul(size.height.into()));

        this
    }

    #[inline]
    fn index_of(&self, pos: Position) -> usize {
        usize::from(pos.y)
            .strict_mul(self.size.width.into())
            .strict_add(pos.x.into())
    }

    #[inline]
    fn cell_mut(&mut self, pos: Position) -> Option<&mut EdgeCell> {
        self.buf.get_mut(self.index_of(pos))
    }

    #[inline]
    fn write(&mut self, pos: SignedPosition, flags: EdgeFlags, fg: Color) {
        let Ok(x) = pos.x.try_into() else { return };
        let Ok(y) = pos.y.try_into() else { return };
        let Some(cell) = self.cell_mut(Position { x, y }) else {
            return;
        };

        cell.write(flags, fg);
    }

    #[inline]
    fn h_seg(&mut self, x: (i32, i32), y: i32, fg: Color) {
        let (from_x, to_x) = x;

        for x in from_x.min(to_x)..=from_x.max(to_x) {
            self.write(SignedPosition { x, y }, EdgeFlags::HORIZ, fg);
        }
    }

    #[inline]
    fn v_seg(&mut self, x: i32, y: (i32, i32), fg: Color) {
        let (from_y, to_y) = y;

        for y in from_y.min(to_y)..=from_y.max(to_y) {
            self.write(SignedPosition { x, y }, EdgeFlags::VERT, fg);
        }
    }

    pub fn push(&mut self, mut from: SignedPosition, mut to: SignedPosition, fg: Color) {
        let to_is_lower = to.y > from.y;
        let y_diff = to.y.abs_diff(from.y);
        let y_sign = if to_is_lower { 1 } else { -1 };

        if to.x < from.x {
            if y_diff < 2 {
                self.write(from, EdgeFlags::CORNER_LB, fg);
                self.write(to, EdgeFlags::CORNER_RB, fg);

                from.y = from.y.saturating_add(1);
                to.y = to.y.saturating_add(1);

                let pos = match to.y.cmp(&from.y) {
                    Ordering::Less => Some(&mut to),
                    Ordering::Equal => None,
                    Ordering::Greater => Some(&mut from),
                };

                if let Some(pos) = pos {
                    self.write(*pos, EdgeFlags::VERT, fg);
                    pos.y = pos.y.saturating_add(1);
                }

                self.write(from, EdgeFlags::CORNER_LT, fg);
                self.write(to, EdgeFlags::CORNER_RT, fg);

                from.x = from.x.saturating_sub(1);
                to.x = to.x.saturating_add(1);

                if to.x <= from.x {
                    self.h_seg((to.x, from.x), from.y, fg);
                }
            } else {
                let (chr1, chr2) = if to_is_lower {
                    (EdgeFlags::CORNER_LB, EdgeFlags::CORNER_RT)
                } else {
                    (EdgeFlags::CORNER_LT, EdgeFlags::CORNER_RB)
                };

                self.write(from, chr1, fg);
                self.write(to, chr2, fg);

                from.y = from.y.saturating_add(y_sign);
                to.y = to.y.saturating_sub(y_sign);

                let y = from.y.midpoint(to.y);

                if y != from.y {
                    self.v_seg(from.x, (from.y, y.saturating_sub(y_sign)), fg);
                }

                if y != to.y {
                    self.v_seg(to.x, (y.saturating_add(y_sign), to.y), fg);
                }

                let mid1 = SignedPosition { x: from.x, y };
                let mid2 = SignedPosition { x: to.x, y };

                let (chr1, chr2) = if to_is_lower {
                    (EdgeFlags::CORNER_LT, EdgeFlags::CORNER_RB)
                } else {
                    (EdgeFlags::CORNER_LB, EdgeFlags::CORNER_RT)
                };

                self.write(mid1, chr1, fg);
                self.write(mid2, chr2, fg);

                from.x = from.x.saturating_sub(1);
                to.x = to.x.saturating_add(1);

                if to.x <= from.x {
                    self.h_seg((from.x, to.x), y, fg);
                }
            }
        } else {
            if from.y == to.y {
                self.h_seg((from.x, to.x), from.y, fg);
                return;
            }

            let x = from.x.midpoint(to.x);

            if x != from.x {
                self.h_seg((from.x, x.saturating_sub(1)), from.y, fg);
            }
            if x != to.x {
                self.h_seg((x.saturating_add(1), to.x), to.y, fg);
            }

            let mid1 = SignedPosition { x, y: from.y };
            let mid2 = SignedPosition { x, y: to.y };

            let (chr1, chr2) = if to.y > from.y {
                (EdgeFlags::CORNER_LB, EdgeFlags::CORNER_RT)
            } else {
                (EdgeFlags::CORNER_LT, EdgeFlags::CORNER_RB)
            };

            self.write(mid1, chr1, fg);
            self.write(mid2, chr2, fg);

            if y_diff == 1 {
                return;
            }

            from.y = from.y.saturating_add(y_sign);
            to.y = to.y.saturating_sub(y_sign);

            self.v_seg(x, (from.y, to.y), fg);
        }
    }
}

impl Widget for &EdgeBuffer {
    fn render(self, mut area: Rect, buf: &mut Buffer) {
        let Position { x: x_off, y: y_off } = area.as_position();
        area.width = area.width.min(self.size.width);
        area.height = area.height.min(self.size.height);

        for pos in area.intersection(buf.area).positions() {
            let Some(x) = pos.x.checked_sub(x_off) else {
                continue;
            };
            let Some(y) = pos.y.checked_sub(y_off) else {
                continue;
            };

            // SAFETY: area is resized to prevent indexing outside buf
            let Some((ch, fg)) =
                unsafe { self.buf.get_unchecked(self.index_of(Position { x, y })) }.into_bits()
            else {
                continue;
            };

            buf[pos].set_char(ch).set_style(Style {
                fg: Some(fg),
                bg: None,
                ..Style::reset()
            });
        }
    }
}
