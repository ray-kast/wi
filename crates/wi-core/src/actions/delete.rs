use super::prelude::*;
use crate::{
    AlignCell, Cursor, EdgeCursor, GraphWidgetCell, Port, Side, SidedPort, WCursor, WPort,
};

pub(super) mod actions {
    use crate::actions::ActionKind;

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    #[kind(ActionKind)]
    pub struct DeleteAtCursor;
}

fn fixup_cursor_missing<W: GraphWidget + ?Sized>(
    widget: &W,
    cell: &W::Cell,
    delete_node: impl Fn(W::NodeId) -> bool,
) -> WCursor<W> {
    if let Some(n) = widget.nearest_node_where(cell, |n| !delete_node(*n)) {
        Cursor::Node(n)
    } else {
        Cursor::FixedPoint(cell.position(widget))
    }
}

fn fixup_cursor<W: GraphWidget + ?Sized>(
    widget: &W,
    cursor: &WCursor<W>,
    cell: &W::Cell,
    delete_node: impl Fn(W::NodeId) -> bool,
    delete_edge: impl Fn(WPort<W>, WPort<W>) -> bool,
) -> Option<WCursor<W>> {
    let fixed = 'change: {
        match *cursor {
            Cursor::Node(n) | Cursor::Port(SidedPort(_, Port(n, _))) => {
                if delete_node(n) {
                    break 'change fixup_cursor_missing(widget, cell, delete_node);
                }
            },
            Cursor::Edge(EdgeCursor { from, to, anchor }) => {
                if delete_node(from.0) || delete_node(to.0) {
                    break 'change fixup_cursor_missing(widget, cell, delete_node);
                }

                if delete_edge(from, to) {
                    let next = widget.nearest_edge_where(
                        &match anchor {
                            Side::In => SidedPort(Side::Out, from),
                            Side::Out => SidedPort(Side::In, to),
                        },
                        cell,
                        |e| !delete_edge(e.from, e.to),
                    );

                    if let Some(next) = next {
                        break 'change Cursor::Edge(next);
                    }

                    break 'change fixup_cursor_missing(widget, cell, delete_node);
                }
            },
            Cursor::FixedPoint(_) => (),
        }

        return None;
    };

    Some(fixed)
}

impl<W: GraphWidget + ?Sized> EditorAction<W> for actions::DeleteAtCursor {
    fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let None = count else { return false };

        let cursor = cx.driver.cursor.as_mut().unwrap_or_else(|| unreachable!());
        let cell = &mut cx.driver.cell;
        let fixed = fixup_cursor(
            cx.widget,
            cursor,
            cell,
            |n| matches!(&*cursor, Cursor::Node(m) if n == *m),
            |f, t| matches!(&*cursor, Cursor::Edge(e) if e.from == f && e.to == t),
        );
        let changed = match cursor {
            crate::Cursor::Node(n) => cx.widget.delete_node(n),
            crate::Cursor::Edge(EdgeCursor {
                from,
                to,
                anchor: _,
            }) => cx.widget.delete_edge(from, to),
            crate::Cursor::Port(_) | crate::Cursor::FixedPoint(_) => false,
        };

        if !changed {
            return false;
        }

        if let Some(fixed) = fixed {
            *cursor = fixed;

            cell.align_to_cursor(cx.widget, cursor, AlignCell::Overwrite);

            cx.widget
                .update_cursor(crate::CursorUpdate::Move, cursor, cx.inner);
        }

        true
    }
}
