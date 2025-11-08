use keyboard_types::NamedKey;
use shibari::Acceptor;
use tracing::debug;

use crate::bindings::{ActionOut, Key, NormalAccept};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    Normal(NormalAccept),
}

impl Default for Mode {
    #[inline]
    fn default() -> Self { Self::Normal(NormalAccept::default()) }
}

impl Mode {
    pub fn change(&mut self, to: ModeKind) -> bool {
        if self.kind() == to {
            return false;
        }

        *self = match to {
            ModeKind::Normal => Self::Normal(NormalAccept::default()),
        };

        true
    }

    #[inline]
    pub fn kind(self) -> ModeKind {
        match self {
            Self::Normal(_) => ModeKind::Normal,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModeKind {
    #[default]
    Normal,
}

impl From<Mode> for ModeKind {
    #[inline]
    fn from(value: Mode) -> Self { value.kind() }
}

impl Acceptor<Key> for Mode {
    type Output = ActionOut;

    fn pending_op(&self) -> &'static str {
        match self {
            Self::Normal(a) => a.pending_op(),
        }
    }

    fn accept(&mut self, input: Key) -> Self::Output {
        if matches!(
            input,
            Key::Named(
                NamedKey::Unidentified
                    | NamedKey::Alt
                    | NamedKey::AltGraph
                    | NamedKey::CapsLock
                    | NamedKey::Control
                    | NamedKey::Fn
                    | NamedKey::FnLock
                    | NamedKey::Meta
                    | NamedKey::NumLock
                    | NamedKey::ScrollLock
                    | NamedKey::Shift
                    | NamedKey::Symbol
                    | NamedKey::SymbolLock,
                _
            )
        ) {
            return ActionOut::Advance;
        }

        debug!("Handling keypress");
        match self {
            Self::Normal(a) => a.accept(input),
        }
    }
}

#[test]
fn mode_default_kind() {
    assert_eq!(Mode::default().kind(), ModeKind::default());
}
