use crate::{
    action::{prelude::*, Action},
    modifiers::M_TCTL,
    trie::trie,
    Side,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char, Modifiers),
    Named(NamedKey, Modifiers),
}

use keyboard_types::{Modifiers, NamedKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    Left,
    Down,
    Up,
    Right,
}

use Key::{Char as C, Named as N};
use NamedKey as K;

#[allow(clippy::wildcard_imports)]
use crate::modifiers::*;

trie! {
    #[advance = Nop]
    #[fallthrough = Fallthrough]
    pub fn ConnectAccept(k: Key) -> Action {
        _ => yield,
    }

    #[advance = Nop]
    #[fallthrough = Fallthrough]
    pub fn NormalAccept(k: Key) -> Action {
        C('c', M_SHIFT) => yield SetMode(ModeKind::Connect),

        C('d', M_NONE) => Delete @ "d" {
            _ => yield,
        },
        C('g', M_NONE) => Go @ "g" {
            _ => yield,
        },

        _ => yield,
    }

    #[advance = Nop]
    pub fn GestureAccept(k: Key) -> Action {
        C('h', M_NONE) | N(K::ArrowLeft, M_NONE) => yield StepCursor(Step::Left),
        C('j', M_NONE) | N(K::ArrowDown, M_NONE) => yield StepCursor(Step::Down),
        C('k', M_NONE) | N(K::ArrowUp, M_NONE) => yield StepCursor(Step::Up),
        C('l', M_NONE) | N(K::ArrowRight, M_NONE) => yield StepCursor(Step::Right),

        C('%', M_SHIFT) => yield GoToOpposite,

        C('i', M_NONE) => yield JumpToPort(Side::In),
        C('o', M_NONE) => yield JumpToPort(Side::Out),

        _ => yield,
    }

    #[advance = Nop]
    #[fallthrough = Fallthrough]
    pub fn GlobalAccept(k: Key) -> Action {
        C(c @ '1'..='9', M_NONE) => Count(PushCount(c)) @ "" {
            C(c @ '0'..='9', M_NONE) => Count(PushCount(c)) { .. },
            _ => continue,
        },

        N(K::Escape, M_NONE) | C('[' | 'c', M_TCTL) => yield SetMode(ModeKind::Normal),

        C('z', M_NONE) => View @ "z" {
            . => yield ViewCursor,
            C('d', M_NONE) => yield ToggleDebug,
            _ => yield,
        },
        N(K::Home, M_NONE) => yield ViewCursor,

        _ => yield,
    }
}

mod mode {
    use tracing::debug;

    use super::{ConnectAccept, GestureAccept, GlobalAccept, Key, NormalAccept};
    use crate::{
        trie::{
            accept::{Fallthrough, Overlay},
            Acceptor,
        },
        Action,
    };

    type WithGlobal<A> = Overlay<A, GlobalAccept>;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Mode {
        Connect {
            accept: WithGlobal<Fallthrough<ConnectAccept, GestureAccept>>,
        },
        Normal {
            accept: WithGlobal<Fallthrough<NormalAccept, GestureAccept>>,
        },
    }

    impl Default for Mode {
        #[inline]
        fn default() -> Self {
            Self::Normal {
                accept: Overlay::default(),
            }
        }
    }

    impl Mode {
        pub fn change(&mut self, to: ModeKind) -> bool {
            if self.kind() == to {
                return false;
            }

            *self = match to {
                ModeKind::Connect => Self::Connect {
                    accept: Overlay::default(),
                },
                ModeKind::Normal => Self::Normal {
                    accept: Overlay::default(),
                },
            };

            true
        }

        #[inline]
        pub fn kind(self) -> ModeKind {
            match self {
                Self::Connect { .. } => ModeKind::Connect,
                Self::Normal { .. } => ModeKind::Normal,
            }
        }
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ModeKind {
        Connect,
        #[default]
        Normal,
    }

    impl From<Mode> for ModeKind {
        #[inline]
        fn from(value: Mode) -> Self { value.kind() }
    }

    impl Acceptor<Key> for Mode {
        type Output = Action;

        fn pending_op(&self) -> &'static str {
            match self {
                Self::Connect { accept, .. } => accept.pending_op(),
                Self::Normal { accept, .. } => accept.pending_op(),
            }
        }

        fn accept(&mut self, input: Key) -> Self::Output {
            debug!("Handling keypress");
            match self {
                Self::Connect { accept, .. } => accept.accept(input),
                Self::Normal { accept, .. } => accept.accept(input),
            }
        }
    }

    #[test]
    fn mode_default_kind() {
        assert_eq!(Mode::default().kind(), ModeKind::default());
    }
}

pub use mode::*;
