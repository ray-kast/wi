use super::prelude::*;
use crate::{
    AlignCell, Cursor, CursorUpdate, GraphWidgetCell, Port, Side, SidedPort, Step, WCursor,
};

pub(super) mod actions {
    use crate::Step;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct StepCursor(pub Step);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ViewCursor;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct GoToOpposite;
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
fn unsigned_steps(steps: u32) -> (usize, u32) {
    let steps = usize::try_from(steps).unwrap_or(usize::MAX);
    let dec = u32::try_from(steps).unwrap_or_else(|_| unreachable!());
    (steps, dec)
}

#[inline]
fn signed_steps(pos: bool, steps: u32) -> (isize, u32) {
    let steps_abs = isize::try_from(steps).unwrap_or(isize::MAX);
    let steps = if pos { steps_abs } else { -steps_abs };
    let dec = u32::try_from(steps_abs).unwrap_or_else(|_| unreachable!());
    (steps, dec)
}

fn move_cursor<W: GraphWidget + ?Sized>(
    count: Option<NonZeroU32>,
    cx: &mut ActionCx<W>,
    mut f: impl FnMut(
        NonZeroU32,
        WCursor<W>,
        &W,
        &W::Cell,
    ) -> (Option<NonZeroU32>, WCursor<W>, AlignCell),
) -> bool {
    let cell = &mut cx.driver.inner.cell;

    let any = run_with_count(count, |steps| {
        let (dec, cursor, align) = f(
            steps,
            cx.driver
                .inner
                .cursor
                .take()
                .unwrap_or_else(|| unreachable!()),
            cx.widget,
            cell,
        );

        if dec.is_some() {
            cell.align_to_cursor(cx.widget, &cursor, align);
        }
        cx.driver.inner.cursor = Some(cursor);

        dec
    });

    if any {
        cx.widget
            .update_cursor(CursorUpdate::Move, cx.driver.cursor(), cx.inner);
    }

    any
}

impl EditorMotion for actions::StepCursor {
    fn name(&self) -> Cow<'static, str> {
        let Self(step) = self;
        match step {
            Step::Left => "step left",
            Step::Down => "step down",
            Step::Up => "step up",
            Step::Right => "step right",
        }
        .into()
    }

    fn process<W: GraphWidget + ?Sized, S: Selection>(
        self,
        count: Option<NonZeroU32>,
        mut cx: ActionCx<W>,
        selection: S,
    ) -> bool {
        let Self(step) = self;

        move_cursor(count, &mut cx, |steps, cursor, widget, cell| {
            let dec;
            let next = match (cursor, step) {
                (Cursor::Node(n), s @ (Step::Left | Step::Right)) => {
                    let side = horiz_side(s);
                    if let Some(p) = widget.nearest_port(&n, side, cell) {
                        dec = 1;
                        Cursor::Port(SidedPort(side, Port(n, p)))
                    } else {
                        dec = 0;
                        Cursor::Node(n)
                    }
                },
                (c @ Cursor::Node(_), Step::Down | Step::Up) => {
                    dec = steps.get();
                    c
                },
                (Cursor::Port(p), s @ (Step::Left | Step::Right)) => {
                    if horiz_side(s) == p.0 {
                        if let Some(e) = widget.nearest_edge(&p, cell) {
                            dec = 1;
                            Cursor::Edge(e)
                        } else {
                            dec = 0;
                            Cursor::Port(p)
                        }
                    } else {
                        dec = 1;
                        Cursor::Node(p.1 .0)
                    }
                },
                (Cursor::Port(p), s @ (Step::Down | Step::Up)) => {
                    let (count, n) = signed_steps(vert_is_down(s), steps.get());

                    if let Some((_, i)) = widget.step_port_by(&p, count) {
                        dec = n;
                        Cursor::Port(SidedPort(p.0, Port(p.1 .0, i)))
                    } else {
                        dec = 0;
                        Cursor::Port(p)
                    }
                },
                (Cursor::Edge(e), s @ (Step::Left | Step::Right)) => {
                    dec = 1;
                    Cursor::Port(e.into_port(horiz_side(s)))
                },
                (Cursor::Edge(e), s @ (Step::Down | Step::Up)) => {
                    let (count, n) = signed_steps(vert_is_down(s), steps.get());

                    if let Some((_, p)) = widget.step_edge_by(&e, count) {
                        dec = n;
                        Cursor::Edge(e.step(p))
                    } else {
                        dec = 0;
                        Cursor::Edge(e)
                    }
                },
                (Cursor::FixedPoint(p), s) => {
                    let (count, n) = unsigned_steps(steps.get());
                    dec = n;
                    Cursor::FixedPoint(widget.step_point_by(&p, s, count))
                },
            };

            let align = match step {
                Step::Left | Step::Right => AlignCell::KeepRow,
                Step::Down | Step::Up => AlignCell::KeepCol,
            };

            (NonZero::new(dec), next, align)
        });

        true
    }
}

impl EditorAction for actions::ViewCursor {
    #[inline]
    fn name(&self) -> Cow<'static, str> {
        let Self = self;
        "view cursor".into()
    }

    fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let None = count else { return false };
        cx.widget
            .update_cursor(CursorUpdate::CenterInView, cx.driver.cursor(), cx.inner);
        true
    }
}

impl EditorMotion for actions::GoToOpposite {
    #[inline]
    fn name(&self) -> Cow<'static, str> { "go to opposite".into() }

    fn process<W: GraphWidget + ?Sized, S: Selection>(
        self,
        count: Option<NonZeroU32>,
        mut cx: ActionCx<W>,
        selection: S,
    ) -> bool {
        move_cursor(count, &mut cx, |count, cursor, widget, _| {
            let dec;
            let next = match (count.get(), cursor) {
                (_, Cursor::Node(n)) => {
                    // HACK: pattern types wen eta ;;
                    let c = Cursor::Node(n);
                    let cell = W::Cell::of_cursor(widget, &c);
                    let Cursor::Node(n) = c else { unreachable!() };
                    if let Some(p) = widget.nearest_port(&n, Side::In, &cell) {
                        dec = 1;
                        Cursor::Port(SidedPort(Side::In, Port(n, p)))
                    } else {
                        dec = 0;
                        Cursor::Node(n)
                    }
                },
                (k, c @ (Cursor::Port(..) | Cursor::Edge(..))) if k % 2 == 0 => {
                    dec = k;
                    c
                },
                (k, Cursor::Port(p)) => {
                    let c = Cursor::Port(p);
                    let cell = W::Cell::of_cursor(widget, &c);
                    let Cursor::Port(mut p) = c else {
                        unreachable!();
                    };
                    if let Some(p2) = widget.nearest_port(&p.1 .0, p.0.flip(), &cell) {
                        p.0 = p.0.flip();
                        p.1 .1 = p2;
                        dec = k;
                        Cursor::Port(p)
                    } else {
                        dec = 0;
                        Cursor::Port(p)
                    }
                },
                (k, Cursor::Edge(mut e)) => {
                    e.anchor = e.anchor.flip();
                    dec = k;
                    Cursor::Edge(e)
                },
                (_, c @ Cursor::FixedPoint(..)) => {
                    dec = 0;
                    c
                },
            };

            (NonZero::new(dec), next, AlignCell::Overwrite)
        });

        true
    }
}
