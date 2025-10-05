use std::num::NonZero;

use tracing::{debug, instrument};

use crate::{
    bindings::{Action, Step},
    Cursor, CursorUpdate, GraphWidget, GraphWidgetDriver, Port, Side,
};

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    #[instrument(
        skip(self, widget, ctx),
        fields(count = ?self.count),
    )]
    pub fn process_action(
        &mut self,
        widget: &mut W,
        action: Action,
        ctx: &mut W::Context<'_>,
    ) -> bool {
        debug!("Processing action");
        match (self.count.take(), action) {
            (c, Action::PushCount(d)) => {
                let digit = u32::from(d) - u32::from('0');
                self.count = c
                    .map_or(0, NonZero::get)
                    .checked_mul(10)
                    .and_then(|c| c.checked_add(digit))
                    .and_then(NonZero::new);
            },
            (c, Action::Step(s)) => {
                let mut steps = c.map_or(1, NonZero::get);
                while steps > 0 {
                    let (row, col) = self.cell.get_or_insert_with(|| {
                        widget.cursor_cell(self.cursor.as_ref().unwrap_or_else(|| unreachable!()))
                    });

                    let dec;
                    self.cursor = Some(
                        match (self.cursor.take().unwrap_or_else(|| unreachable!()), s) {
                            (Cursor::Node(n), c @ (Step::Left | Step::Right)) => {
                                let side = if matches!(c, Step::Right) {
                                    Side::Out
                                } else {
                                    Side::In
                                };
                                if let Some((p, c)) = widget.nearest_port(&n, side, (row, col)) {
                                    *col = c;
                                    dec = 1;
                                    Cursor::Port(Port(side, n, p))
                                } else {
                                    dec = steps;
                                    Cursor::Node(n)
                                }
                            },
                            (c @ Cursor::Node(_), Step::Down | Step::Up) => {
                                let (r, _) = widget.cursor_cell(&c);
                                *row = r;
                                dec = steps;
                                c
                            },
                            (Cursor::Port(p), c @ (Step::Left | Step::Right)) => {
                                let side = if matches!(c, Step::Right) {
                                    Side::Out
                                } else {
                                    Side::In
                                };
                                if side == p.0 {
                                    if let Some((p, r, c)) = widget.port_connection(&p) {
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
                                    let cell = widget.cursor_cell(&c);
                                    *col = cell.1;
                                    dec = 1;
                                    c
                                }
                            },
                            (Cursor::Port(p), c @ (Step::Down | Step::Up)) => {
                                let steps = isize::try_from(steps).unwrap_or(isize::MAX);
                                let step = if matches!(c, Step::Down) { steps } else { -steps };
                                let (port, r) = widget.step_port_by(&p, step);
                                *row = r;
                                dec = u32::try_from(steps).unwrap_or_else(|_| unreachable!());
                                if let Some((_, i)) = port {
                                    Cursor::Port(Port(p.0, p.1, i))
                                } else {
                                    Cursor::Port(p)
                                }
                            },
                            (Cursor::FixedPoint(_p), _s) => todo!(),
                        },
                    );

                    steps = steps.checked_sub(dec).unwrap_or_else(|| unreachable!());
                }
                widget.update_cursor(CursorUpdate::Move, self.cursor(), ctx);
            },
            (None, Action::ViewCursor) => {
                widget.update_cursor(CursorUpdate::CenterInView, self.cursor(), ctx);
            },
            (Some(_), _) => return false,
        }

        true
    }
}
