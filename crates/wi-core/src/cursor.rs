use std::fmt;

use crate::{action::prelude::*, bindings::Step, CursorUpdate, GraphWidget, Port, Side};

pub enum Cursor<W: GraphWidget + ?Sized> {
    Node(W::Node),
    Port(Port<W>),
    Edge(W::Node, W::OutEdgeIdx),
    FixedPoint(W::Point),
}

impl<W: GraphWidget + ?Sized> fmt::Debug for Cursor<W>
where
    W::Node: fmt::Debug,
    W::PortIdx: fmt::Debug,
    W::OutEdgeIdx: fmt::Debug,
    W::Point: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node(n) => f.debug_tuple("Node").field(n).finish(),
            Self::Port(p) => f.debug_tuple("Port").field(p).finish(),
            Self::Edge(n, e) => f.debug_tuple("Edge").field(n).field(e).finish(),
            Self::FixedPoint(p) => f.debug_tuple("FixedPoint").field(p).finish(),
        }
    }
}

pub mod actions {
    use crate::bindings::Step;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct StepCursor(pub Step);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ViewCursor;
}

impl EditorAction for actions::StepCursor {
    fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let Self(step) = self;
        let mut steps = count.map_or(1, NonZero::get);
        while steps > 0 {
            let (row, col) = cx.driver.cell.get_or_insert_with(|| {
                cx.widget
                    .cursor_cell(cx.driver.cursor.as_ref().unwrap_or_else(|| unreachable!()))
            });

            let dec;
            cx.driver.cursor = Some(
                match (
                    cx.driver.cursor.take().unwrap_or_else(|| unreachable!()),
                    step,
                ) {
                    (Cursor::Node(n), s @ (Step::Left | Step::Right)) => {
                        let side = if matches!(s, Step::Right) {
                            Side::Out
                        } else {
                            Side::In
                        };
                        if let Some((p, c)) = cx.widget.nearest_port(&n, side, (row, col)) {
                            *col = c;
                            dec = 1;
                            Cursor::Port(Port(side, n, p))
                        } else {
                            dec = steps;
                            Cursor::Node(n)
                        }
                    },
                    (c @ Cursor::Node(_), Step::Down | Step::Up) => {
                        let (r, _) = cx.widget.cursor_cell(&c);
                        *row = r;
                        dec = steps;
                        c
                    },
                    (Cursor::Port(p), s @ (Step::Left | Step::Right)) => {
                        let side = if matches!(s, Step::Right) {
                            Side::Out
                        } else {
                            Side::In
                        };
                        if side == p.0 {
                            if let Some((p, r, c)) = cx.widget.port_connection(&p) {
                                *row = r;
                                *col = c;
                                dec = 1;
                                Cursor::Port(p)
                            } else {
                                dec = steps;
                                Cursor::Port(p)
                            }
                        } else {
                            let c = Cursor::Node(p.1);
                            let cell = cx.widget.cursor_cell(&c);
                            *col = cell.1;
                            dec = 1;
                            c
                        }
                    },
                    (Cursor::Port(p), s @ (Step::Down | Step::Up)) => {
                        let steps = isize::try_from(steps).unwrap_or(isize::MAX);
                        let step = if matches!(s, Step::Down) {
                            steps
                        } else {
                            -steps
                        };
                        let (port, r) = cx.widget.step_port_by(&p, step);
                        *row = r;
                        dec = u32::try_from(steps).unwrap_or_else(|_| unreachable!());
                        if let Some((_, i)) = port {
                            Cursor::Port(Port(p.0, p.1, i))
                        } else {
                            Cursor::Port(p)
                        }
                    },
                    (c @ Cursor::Edge(..), Step::Down | Step::Up) => todo!(),
                    (Cursor::Edge(..), Step::Left | Step::Right) => todo!(),
                    (Cursor::FixedPoint(_p), _s) => todo!(),
                },
            );

            steps = steps.checked_sub(dec).unwrap_or_else(|| unreachable!());
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
