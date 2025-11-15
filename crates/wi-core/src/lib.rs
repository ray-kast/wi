use std::num::NonZeroU32;

use crate::{
    actions::Action,
    mode::Mode,
    operators::{CurrentOperator, Operator},
    traits::{CursorOps, GraphWidgetCell, GraphWidgetTypes, UiOps},
};
pub use crate::{
    actions::{ActionKind, MotionKind},
    continuation::{ContinueCx, ContinueOnce, Yielded},
    cursor::{euclidean_cell, Cursor, EdgeCursor, WCursor, WEdgeCursor},
    mode::ModeKind,
    operators::OperatorKind,
    status::{LastOp, Status},
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
    last_op: LastOp,
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
                last_op: LastOp::default(),
            },
        };

        debug_assert_eq!(me.status(), Status::default());

        me
    }
}

impl<W: UiOps + ?Sized> GraphWidgetDriver<W> {
    #[inline]
    fn mutate_check<'w, T>(
        &mut self,
        widget: &mut W,
        cx: W::Context<'_, 'w>,
        f: impl FnOnce(&mut DriverInner<W>, &mut CurrentOperator, &mut W, W::Context<'_, 'w>) -> T,
    ) -> T {
        self.inner
            .mutate_check(&mut self.current_operator, widget, cx, f)
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

impl<W: UiOps + ?Sized> DriverInner<W> {
    #[inline]
    fn mutate_check<'w, T>(
        &mut self,
        current_operator: &mut CurrentOperator,
        widget: &mut W,
        mut cx: W::Context<'_, 'w>,
        f: impl for<'a> FnOnce(
            &'a mut Self,
            &'a mut CurrentOperator,
            &'a mut W,
            W::Context<'a, 'w>,
        ) -> T,
    ) -> T {
        let pre_status = self.status(current_operator);
        let res = f(self, current_operator, widget, W::reborrow_cx(&mut cx));

        let post_status = self.status(current_operator);
        if pre_status != post_status {
            widget.update_status(post_status, cx);
        }

        res
    }
}
