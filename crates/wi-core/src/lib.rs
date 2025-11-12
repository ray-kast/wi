use std::num::NonZeroU32;

use crate::{
    actions::Action,
    mode::Mode,
    operators::{CurrentOperator, Operator},
    traits::{CursorOps, GraphWidgetCell, GraphWidgetTypes},
};
pub use crate::{
    actions::{ActionKind, MotionKind},
    continuation::{ContinueCx, ContinueOnce, Yielded},
    cursor::{euclidean_cell, Cursor, EdgeCursor, WCursor, WEdgeCursor},
    mode::ModeKind,
    operators::OperatorKind,
    status::Status,
};

pub mod prelude {
    pub use crate::{continuation::ContinueOnce, traits::*};
}

pub extern crate shibari;

mod actions;
mod bindings;
mod continuation;
mod cursor;
mod keyboard;
mod mode;
pub mod modifiers;
mod operators;
mod status;
pub mod traits;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    In,
    Out,
}

impl Side {
    #[inline]
    #[must_use]
    pub fn flip(self) -> Self {
        match self {
            Self::In => Self::Out,
            Self::Out => Self::In,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Port<N, P>(pub N, pub P);

pub type WPort<W> = Port<<W as GraphWidgetTypes>::NodeId, <W as GraphWidgetTypes>::PortId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SidedPort<N, P>(pub Side, pub Port<N, P>);

pub type WSidedPort<W> =
    SidedPort<<W as GraphWidgetTypes>::NodeId, <W as GraphWidgetTypes>::PortId>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    Left,
    Down,
    Up,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlignCell {
    Overwrite,
    KeepRow,
    KeepCol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CursorUpdate {
    Move,
    CenterInView,
}

#[derive_where::derive_where(Debug; W::NodeId, W::PortId, W::Cell, W::Point, W::NodeKind)]
#[must_use]
pub struct GraphWidgetDriver<W: GraphWidgetTypes + ?Sized> {
    current_operator: CurrentOperator,
    inner: DriverInner<W>,
}

#[derive_where::derive_where(Debug; W::NodeId, W::PortId, W::Cell, W::Point, W::NodeKind)]
struct DriverInner<W: GraphWidgetTypes + ?Sized> {
    cursor: Option<WCursor<W>>,
    cell: W::Cell,
    debug: bool,

    count: Option<NonZeroU32>,
    stashed_operators: Vec<Operator>,
    mode: Mode,
    last_action: Option<Action<W>>,
}

impl<W: CursorOps + ?Sized> GraphWidgetDriver<W> {
    pub fn new(widget: &W) -> Self {
        let cursor = widget.default_cursor();
        let me = Self {
            current_operator: CurrentOperator::default(),
            inner: DriverInner {
                cell: W::Cell::of_cursor(widget, &cursor),
                cursor: Some(cursor),
                debug: false,
                count: None,
                stashed_operators: vec![],
                mode: Mode::default(),
                last_action: None,
            },
        };

        debug_assert_eq!(me.status(), Status::default());

        me
    }
}

impl<W: GraphWidgetTypes + ?Sized> GraphWidgetDriver<W> {
    #[inline]
    pub fn cursor(&self) -> &WCursor<W> {
        self.inner.cursor.as_ref().unwrap_or_else(|| unreachable!())
    }

    #[inline]
    pub fn cursor_cell(&self) -> &W::Cell { &self.inner.cell }

    #[inline]
    pub fn view_debug(&self) -> bool { self.inner.debug }
}
