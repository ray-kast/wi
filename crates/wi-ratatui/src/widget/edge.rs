use ratatui::{
    layout::Position,
    prelude::{Buffer, Rect},
    style::{Color, Style},
    widgets::Widget,
};

const LR: char = '─';
const TB: char = '│';

const BR: char = '┌';
const LB: char = '┐';
const TR: char = '└';
const LT: char = '┘';

#[derive(Debug, Clone, Copy)]
pub struct Edge {
    from: Position,
    to: Position,
    color: Color,
}

impl Edge {
    #[inline]
    pub const fn new(from: Position, to: Position, color: Color) -> Self {
        Self { from, to, color }
    }

    #[inline]
    fn write(&self, area: Rect, buf: &mut Buffer, pos: Position, chr: char) {
        if !area.contains(pos) {
            return;
        }

        let cell = &mut buf[pos];
        cell.set_style(Style {
            bg: None,
            fg: Some(self.color),
            ..Style::reset()
        });
        cell.set_char(chr);
    }

    #[inline]
    fn h_seg(&self, area: Rect, buf: &mut Buffer, x: (u16, u16), y: u16) {
        let (from_x, to_x) = x;
        let row = area.intersection(Rect {
            x: from_x.min(to_x),
            y,
            width: from_x.abs_diff(to_x) + 1,
            height: 1,
        });

        for pos in row.positions() {
            self.write(area, buf, pos, LR);
        }
    }

    #[inline]
    fn v_seg(&self, area: Rect, buf: &mut Buffer, x: u16, y: (u16, u16)) {
        let (from_y, to_y) = y;
        let col = Rect {
            x,
            y: from_y.min(to_y),
            width: 1,
            height: from_y.abs_diff(to_y) + 1,
        };

        for pos in col.positions() {
            self.write(area, buf, pos, TB);
        }
    }
}

impl Widget for Edge {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(buf.area);
        let Self { from, to, .. } = self;

        let y_diff = from.y.abs_diff(to.y);
        let (from, to) = if to.x <= from.x {
            if y_diff < 2 {
                todo!()
            } else {
                todo!()
            }
        } else {
            (from, to)
        };

        if y_diff == 0 {
            self.h_seg(area, buf, (from.x, to.x), from.y);
            return;
        }

        let x = from.x.midpoint(to.x);
        self.h_seg(area, buf, (from.x, x), from.y);
        self.h_seg(area, buf, (x, to.x), to.y);

        let mid1 = Position { x, y: from.y };
        let mid2 = Position { x, y: to.y };

        let (chr1, chr2) = if to.y > from.y { (LB, TR) } else { (LT, BR) };

        self.write(area, buf, mid1, chr1);
        self.write(area, buf, mid2, chr2);

        if y_diff == 1 {
            return;
        }

        self.v_seg(area, buf, x, (from.y.min(to.y) + 1, from.y.max(to.y) - 1));
    }
}
