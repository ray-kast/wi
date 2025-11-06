use std::num::NonZero;

use shibari::Acceptor;

use crate::{
    actions::Action,
    mode::ModeKind,
    operators::{Operator, OperatorState},
    GraphWidget, GraphWidgetDriver,
};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Status {
    pub count: Option<u32>,
    pub mode: ModeKind,
    pub last_action: Option<Action>,
    pub current_operator: Option<OperatorState>,
    pub pending_op: &'static str,
    pub debug: bool,
}

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    pub fn status(&self) -> Status {
        Status {
            count: self.inner.count.map(NonZero::get),
            mode: self.inner.mode.kind(),
            last_action: self.inner.last_action,
            current_operator: self.current_operator.as_ref().map(Operator::state),
            pending_op: self.inner.mode.pending_op(),
            debug: self.inner.debug,
        }
    }
}
