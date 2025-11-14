use std::num::NonZero;

use shibari::Acceptor;

use crate::{
    actions::{Action, ActionKind},
    mode::ModeKind,
    operators::{CurrentOperator, Operator, OperatorKind},
    traits::GraphWidgetTypes,
    DriverInner, GraphWidgetDriver,
};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Status {
    pub count: Option<u32>,
    pub mode: ModeKind,
    pub last_action: Option<ActionKind>,
    pub current_operator: Option<OperatorKind>,
    pub pending_op: &'static str,
    pub debug: bool,
}

impl<W: GraphWidgetTypes + ?Sized> GraphWidgetDriver<W> {
    #[inline]
    pub fn status(&self) -> Status { self.inner.status(&self.current_operator) }
}

impl<W: GraphWidgetTypes + ?Sized> DriverInner<W> {
    pub fn status(&self, current_operator: &CurrentOperator) -> Status {
        Status {
            count: self.count.map(NonZero::get),
            mode: self.mode.kind(),
            last_action: self.last_action.as_ref().map(Action::kind),
            current_operator: current_operator.as_ref().map(Operator::kind),
            pending_op: self.mode.pending_op(),
            debug: self.debug,
        }
    }
}
