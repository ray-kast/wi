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
                let steps = c.map_or(1, NonZero::get);
                for _ in 0..steps {
                    let (row, col) = self.cell.get_or_insert_with(|| {
                        widget.cursor_cell(self.cursor.as_ref().unwrap_or_else(|| unreachable!()))
                    });

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
                                    Cursor::Port(Port(side, n, p))
                                } else {
                                    Cursor::Node(n)
                                }
                            },
                            (c @ Cursor::Node(_), Step::Down | Step::Up) => {
                                let (row2, col2) = widget.cursor_cell(&c);
                                *row = row2;
                                *col = col2;
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
                                        Cursor::Port(p)
                                    } else {
                                        Cursor::Port(p)
                                    }
                                } else {
                                    let c = Cursor::Node(p.1);
                                    let cell = widget.cursor_cell(&c);
                                    *col = cell.1;
                                    c
                                }
                            },
                            (Cursor::Port(p), c @ (Step::Down | Step::Up)) => {
                                let step = if matches!(c, Step::Down) { 1 } else { -1 };
                                if let Some((_, i, r)) = widget.step_port_by(&p, step) {
                                    *row = r;
                                    Cursor::Port(Port(p.0, p.1, i))
                                } else {
                                    Cursor::Port(p)
                                }
                            },
                            (Cursor::FixedPoint(_p), _s) => todo!(),
                        },
                    );
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
