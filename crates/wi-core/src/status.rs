use std::num::NonZero;

use crate::{bindings::Mode, trie::Acceptor, GraphWidget, GraphWidgetDriver};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModeKind {
    Normal,
}

impl Mode {
    pub fn status(self) -> ModeKind {
        match self {
            Self::Normal { .. } => ModeKind::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Status {
    pub count: Option<u32>,
    pub mode: ModeKind,
    pub pending_op: &'static str,
}

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    pub fn status(&self) -> Status {
        Status {
            count: self.count.map(NonZero::get),
            mode: self.mode.status(),
            pending_op: self.mode.pending_op(),
        }
    }
}
