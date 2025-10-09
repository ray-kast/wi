use crate::{
    action::prelude::*, bindings::Step, Cell, CursorUpdate, GraphWidget, Port, PreserveCell, Side,
    SidedPort, WCellRef,
};

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

pub type WEdgeCursor<W> = EdgeCursor<<W as GraphWidget>::Node, <W as GraphWidget>::PortIdx>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cursor<Node, PortIdx, Point> {
    Node(Node),
    Port(SidedPort<Node, PortIdx>),
    Edge(EdgeCursor<Node, PortIdx>),
    FixedPoint(Point),
}

pub type WCursor<W> =
    Cursor<<W as GraphWidget>::Node, <W as GraphWidget>::PortIdx, <W as GraphWidget>::Point>;

pub mod actions {
    use crate::bindings::Step;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct StepCursor(pub Step);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ViewCursor;
}

#[inline]
fn horiz_is_right(step: Step) -> bool {
    match step {
        Step::Left => false,
        Step::Right => true,
        _ => unreachable!(),
    }
}

#[inline]
fn vert_is_down(step: Step) -> bool {
    match step {
        Step::Up => false,
        Step::Down => true,
        _ => unreachable!(),
    }
}

#[inline]
fn horiz_side(step: Step) -> Side {
    if horiz_is_right(step) {
        Side::Out
    } else {
        Side::In
    }
}

#[inline]
fn signed_steps(pos: bool, steps: u32) -> (isize, u32) {
    let steps_abs = isize::try_from(steps).unwrap_or(isize::MAX);
    let steps = if pos { steps_abs } else { -steps_abs };
    let dec = u32::try_from(steps_abs).unwrap_or_else(|_| unreachable!());
    (steps, dec)
}

impl EditorAction for actions::StepCursor {
    fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let Self(step) = self;
        let mut steps = count.map_or(1, NonZero::get);
        while steps > 0 {
            let Cell { anchor, row, col } = cx.driver.cell.get_or_insert_with(|| {
                cx.widget.cursor_cell(
                    cx.driver.cursor.as_ref().unwrap_or_else(|| unreachable!()),
                    PreserveCell::Overwrite,
                )
            });
            let cell: WCellRef<W> = Cell { anchor, row, col };

            let dec;
            let mut reset_cell = false;
            let cursor = match (
                cx.driver.cursor.take().unwrap_or_else(|| unreachable!()),
                step,
            ) {
                (Cursor::Node(n), s @ (Step::Left | Step::Right)) => {
                    let side = horiz_side(s);
                    if let Some(p) = cx.widget.nearest_port(&n, side, cell) {
                        dec = 1;
                        Cursor::Port(SidedPort(side, Port(n, p)))
                    } else {
                        dec = steps;
                        Cursor::Node(n)
                    }
                },
                (c @ Cursor::Node(_), Step::Down | Step::Up) => {
                    dec = steps;
                    c
                },
                (Cursor::Port(p), s @ (Step::Left | Step::Right)) => {
                    let side = horiz_side(s);
                    if side == p.0 {
                        if let Some(e) = cx.widget.nearest_edge(&p, cell) {
                            dec = 1;
                            Cursor::Edge(e)
                        } else {
                            dec = steps;
                            Cursor::Port(p)
                        }
                    } else {
                        dec = 1;
                        Cursor::Node(p.1 .0)
                    }
                },
                (Cursor::Port(p), s @ (Step::Down | Step::Up)) => {
                    let count;
                    (count, dec) = signed_steps(vert_is_down(s), steps);
                    if let Some((_, i)) = cx.widget.step_port_by(&p, count) {
                        Cursor::Port(SidedPort(p.0, Port(p.1 .0, i)))
                    } else {
                        Cursor::Port(p)
                    }
                },
                (Cursor::Edge(e), s @ (Step::Left | Step::Right)) => {
                    let want = horiz_side(s);
                    dec = 1;
                    reset_cell = true;
                    Cursor::Port(e.into_port(want))
                },
                (Cursor::Edge(e), s @ (Step::Down | Step::Up)) => {
                    let count;
                    (count, dec) = signed_steps(vert_is_down(s), steps);
                    if let Some((_, p)) = cx.widget.step_edge_by(&e, count) {
                        Cursor::Edge(e.step(p))
                    } else {
                        Cursor::Edge(e)
                    }
                },
                (Cursor::FixedPoint(_p), _s) => todo!(),
            };

            steps = steps.checked_sub(dec).unwrap_or_else(|| unreachable!());
            Cell {
                anchor: *anchor,
                row: *row,
                col: *col,
            } = cx.widget.cursor_cell(&cursor, match step {
                _ if reset_cell => PreserveCell::Overwrite,
                Step::Left | Step::Right => PreserveCell::Row(anchor, row),
                Step::Down | Step::Up => PreserveCell::Col(anchor, col),
            });
            cx.driver.cursor = Some(cursor);
        }
        cx.widget
            .update_cursor(CursorUpdate::Move, cx.driver.cursor(), cx.inner);

        true
    }
}

impl EditorAction for actions::ViewCursor {
    fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let None = count else { return false };
        cx.widget
            .update_cursor(CursorUpdate::CenterInView, cx.driver.cursor(), cx.inner);
        true
    }
}
