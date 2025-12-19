use super::prelude::*;
use crate::{
    traits::GraphWidgetCell, AlignCell, Cursor, CursorUpdate, Port, Side, SidedPort, Step, WCursor,
};

pub(super) mod actions {
    use wi_macros::Kind;

    use crate::{actions::ActionKind, traits::GraphWidgetTypes, Step, WSidedPort};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Kind)]
    #[kind(ActionKind)]
    pub struct GoToOpposite;

    #[derive_where::derive_where(Debug, Clone, Copy, PartialEq, Eq, Hash; W::NodeId, W::PortId)]
    #[derive(Kind)]
    #[kind(ActionKind)]
    pub struct GoToPort<W: GraphWidgetTypes + ?Sized>(pub WSidedPort<W>);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    pub struct StepCursor(
        #[kind(
            Step::Left => ActionKind::StepLeft,
            Step::Down => ActionKind::StepDown,
            Step::Up => ActionKind::StepUp,
            Step::Right => ActionKind::StepRight,
        )]
        pub Step,
    );

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    #[kind(ActionKind)]
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
fn signed_steps(pos: bool, steps: u32) -> (i32, u32) {
    let steps = i32::try_from(steps).unwrap_or(i32::MAX);
    let steps = if pos { steps } else { steps.saturating_neg() };

    (steps, steps.unsigned_abs())
}

fn move_cursor<W: CursorOps + ?Sized>(
    count: Option<NonZeroU32>,
    cx: &mut ActionCx<W>,
    mut f: impl FnMut(
        NonZeroU32,
        WCursor<W>,
        &W,
        &W::Cell,
    ) -> (Option<NonZeroU32>, WCursor<W>, AlignCell),
) -> bool {
    let cell = &mut cx.driver.cell;

    let any = run_with_count(count, |steps| {
        let (dec, cursor, align) = f(
            steps,
            cx.driver.cursor.take().unwrap_or_else(|| unreachable!()),
            cx.widget,
            cell,
        );

        if dec.is_some() {
            cell.align_to_cursor(cx.widget, &cursor, align);
        }
        cx.driver.cursor = Some(cursor);

        dec
    });

    if any {
        cx.run(|w, d, c| {
            w.update_cursor(
                CursorUpdate::Move,
                d.cursor.as_ref().unwrap_or_else(|| unreachable!()),
                c,
            );
        });
    }

    any
}

impl<W: CursorOps + ?Sized> EditorAction<W> for actions::GoToOpposite {
    fn process(&self, count: Option<NonZeroU32>, mut cx: ActionCx<W>) -> bool {
        move_cursor(count, &mut cx, |count, cursor, widget, _| {
            let dec;
            let count = count.get();
            let next = match cursor {
                Cursor::Node(n) => {
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
                c @ (Cursor::Port(..) | Cursor::Edge(..)) if count % 2 == 0 => {
                    dec = count;
                    c
                },
                Cursor::Port(p) => {
                    let c = Cursor::Port(p);
                    let cell = W::Cell::of_cursor(widget, &c);
                    let Cursor::Port(mut p) = c else {
                        unreachable!();
                    };
                    if let Some(p2) = widget.nearest_port(&p.1 .0, p.0.flip(), &cell) {
                        p.0 = p.0.flip();
                        p.1 .1 = p2;
                        dec = count;
                        Cursor::Port(p)
                    } else {
                        dec = 0;
                        Cursor::Port(p)
                    }
                },
                Cursor::Edge(mut e) => {
                    e.anchor = e.anchor.flip();
                    dec = count;
                    Cursor::Edge(e)
                },
                c @ Cursor::FixedPoint(..) => {
                    dec = 0;
                    c
                },
            };

            (NonZero::new(dec), next, AlignCell::Overwrite)
        });

        true
    }
}

impl<W: CursorOps + ?Sized> EditorAction<W> for actions::GoToPort<W> {
    fn process(&self, count: Option<NonZeroU32>, mut cx: ActionCx<W>) -> bool {
        let None = count else { return false };
        let Self(port) = *self;

        move_cursor(None, &mut cx, |_, _, _, _| {
            (NonZero::new(1), Cursor::Port(port), AlignCell::Overwrite)
        });

        true
    }
}

impl<W: CursorOps + ?Sized> EditorAction<W> for actions::StepCursor {
    fn process(&self, count: Option<NonZeroU32>, mut cx: ActionCx<W>) -> bool {
        let Self(step) = *self;

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
                    } else if let anchor = e.anchor_port()
                        && let Some((_, p)) =
                            widget.step_port_by(&anchor, if vert_is_down(step) { 1 } else { -1 })
                        && let Some(mut f) =
                            widget.nearest_edge(&SidedPort(anchor.0, Port(anchor.1 .0, p)), cell)
                    {
                        f.anchor = e.anchor;
                        dec = 1;
                        Cursor::Edge(f)
                    } else {
                        dec = 0;
                        Cursor::Edge(e)
                    }
                },
                (Cursor::FixedPoint(p), s) => {
                    let count = steps.get();
                    dec = count;
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

impl<W: CursorOps + ?Sized> EditorAction<W> for actions::ViewCursor {
    fn process(&self, count: Option<NonZeroU32>, mut cx: ActionCx<W>) -> bool {
        let None = count else { return false };
        cx.run(|w, d, c| {
            w.update_cursor(
                CursorUpdate::CenterInView,
                d.cursor.as_ref().unwrap_or_else(|| unreachable!()),
                c,
            );
        });
        true
    }
}
