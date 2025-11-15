use std::{borrow::Cow, num::NonZero};

use shibari::Acceptor;

use crate::{
    actions::{Action, ActionKind},
    mode::ModeKind,
    operators::{CurrentOperator, OperatorKind, OperatorState},
    traits::GraphWidgetTypes,
    DriverInner, GraphWidgetDriver,
};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct LastOp {
    pub count: Option<u32>,
    pub chord: Cow<'static, str>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Status {
    pub count: Option<u32>,
    pub mode: ModeKind,
    pub last_action: Option<ActionKind>,
    pub current_operator: Option<(OperatorKind, Cow<'static, str>)>,
    pub pending_op: &'static str,
    pub last_op: LastOp,
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
            last_action: self.last_action.as_ref(false).map(Action::kind),
            current_operator: current_operator
                .as_ref()
                .map(|o| (o.kind(), o.pending_op())),
            pending_op: self.mode.pending_op(),
            last_op: self.last_op.clone(),
            debug: self.debug,
        }
    }
}
