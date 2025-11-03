use std::num::NonZero;

use shibari::Acceptor;

use crate::{actions::Action, bindings::ModeKind, GraphWidget, GraphWidgetDriver};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Status {
    pub count: Option<u32>,
    pub mode: ModeKind,
    pub last_action: Option<Action>,
    pub pending_op: &'static str,
    pub debug: bool,
}

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    pub fn status(&self) -> Status {
        Status {
            count: self.count.map(NonZero::get),
            mode: self.mode.kind(),
            last_action: self.last_action,
            pending_op: self.mode.pending_op(),
            debug: self.debug,
        }
    }
}
