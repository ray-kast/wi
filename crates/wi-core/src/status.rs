use std::num::NonZero;

use crate::{action::Action, bindings::Mode, trie::Acceptor, GraphWidget, GraphWidgetDriver};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModeKind {
    #[default]
    Normal,
}

#[test]
fn mode_default_kind() {
    assert_eq!(Mode::default().status(), ModeKind::default());
}

impl Mode {
    pub fn status(self) -> ModeKind {
        match self {
            Self::Normal { .. } => ModeKind::Normal,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Status {
    pub count: Option<u32>,
    pub mode: ModeKind,
    pub last_action: Option<Action>,
    pub pending_op: &'static str,
}

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    pub fn status(&self) -> Status {
        Status {
            count: self.count.map(NonZero::get),
            mode: self.mode.status(),
            last_action: self.last_action,
            pending_op: self.mode.pending_op(),
        }
    }
}
