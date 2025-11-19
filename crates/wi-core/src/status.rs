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
pub struct LastChord {
    pub count: Option<u32>,
    pub operator_prefix: Option<Cow<'static, str>>,
    pub chord: Cow<'static, str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CurrentOperatorStatus {
    pub operator_chord: Cow<'static, str>,
    pub operator: OperatorKind,
    pub pending_op: Cow<'static, str>,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Status {
    pub count: Option<u32>,
    pub mode: ModeKind,
    pub last_action: Option<ActionKind>,
    pub current_operator: Option<CurrentOperatorStatus>,
    pub pending_op: Cow<'static, str>,
    pub last_chord: LastChord,
    pub debug: bool,
}

impl<W: GraphWidgetTypes + ?Sized> GraphWidgetDriver<W> {
    #[inline]
    pub fn status(&self) -> Status { self.inner.status(&self.current_operator) }
}

impl<W: GraphWidgetTypes + ?Sized> DriverInner<W> {
    pub fn status(&self, current_operator: &CurrentOperator<W>) -> Status {
        Status {
            count: self.count.map(NonZero::get),
            mode: self.mode.kind(),
            last_action: self.last_action.as_ref(false).map(Action::kind),
            current_operator: current_operator
                .status()
                .map(|(c, o)| CurrentOperatorStatus {
                    operator_chord: c.clone(),
                    operator: o.kind(),
                    pending_op: o.pending_op(),
                }),
            pending_op: self.mode.pending_op(),
            last_chord: self.last_op.clone(),
            debug: self.debug,
        }
    }
}
