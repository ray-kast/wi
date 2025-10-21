use crate::{actions::prelude::*, modifiers::M_TCTL, Side, Step};

pub trait Acceptor<T>: Default + PartialEq {
    type Output;

    fn pending_op(&self) -> &'static str;

    fn accept(&mut self, input: T) -> Self::Output;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char, Modifiers),
    Named(NamedKey, Modifiers),
}

use keyboard_types::{Modifiers, NamedKey};
use wi_macros::trie;
use Key::{Char as C, Named as N};
use NamedKey as K;

#[allow(clippy::wildcard_imports)]
use crate::modifiers::*;

trie! {
    input = Key;
    acceptor = Acceptor;

    token Escape = N(K::Escape, M_NONE) | C('[' | 'c', M_TCTL);
    token Home = N(K::Home, M_NONE);

    token ConnectMode = C('c', M_SHIFT);
    token DeleteOp = C('d', M_NONE) => "d";
    token GoOp = C('g', M_NONE) => "g";
    token ViewOp = C('z', M_NONE) => "z";

    token Left = C('h', M_NONE) | N(K::ArrowLeft, M_NONE);
    token Down = C('j', M_NONE) | N(K::ArrowDown, M_NONE);
    token Up = C('k', M_NONE) | N(K::ArrowUp, M_NONE);
    token Right = C('l', M_NONE) | N(K::ArrowRight, M_NONE);

    token Opposite = C('%', M_NONE | M_SHIFT);

    token In = C('i', M_NONE);
    token Out = C('o', M_NONE);

    token DigitNonzero = C(c @ '1'..='9', M_NONE | M_SHIFT) => "";
    token Digit = C(c @ '0'..='9', M_NONE | M_SHIFT);

    pub grammar Connect: Action {
        advance = Nop;

        extend Motion;
        extend Global;
    }

    pub grammar Normal: Action {
        advance = Nop;

        ConnectMode => yield SetMode(ModeKind::Connect);

        DeleteOp {
            DeleteOp => yield DeleteAtCursor;
        }

        GoOp {}

        extend Motion;
        extend Global;
    }

    grammar Motion: Motion {
        advance = Nop;

        Left => yield StepCursor(Step::Left);
        Down => yield StepCursor(Step::Down);
        Up => yield StepCursor(Step::Up);
        Right => yield StepCursor(Step::Right);

        Opposite => yield GoToOpposite;

        In => yield JumpToPort(Side::In);
        Out => yield JumpToPort(Side::Out);
    }

    grammar Global: Action {
        advance = Nop;

        'count: DigitNonzero {
            yield PushCount(c);

            Digit => goto 'count, PushCount(c);

            ..continue
        }

        Escape => yield SetMode(ModeKind::Normal);
        ViewOp {
            ViewOp => yield ViewCursor;
        }

        Home => yield ViewCursor;
    }
}

mod mode {
    use keyboard_types::NamedKey;
    use tracing::debug;

    use super::{ConnectAccept, Key, NormalAccept};
    use crate::{actions::all::Nop, bindings::Acceptor, Action};

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Mode {
        Connect { accept: ConnectAccept },
        Normal { accept: NormalAccept },
    }

    impl Default for Mode {
        #[inline]
        fn default() -> Self {
            Self::Normal {
                accept: NormalAccept::default(),
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
                    accept: ConnectAccept::default(),
                },
                ModeKind::Normal => Self::Normal {
                    accept: NormalAccept::default(),
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
                return Nop.into();
            }

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
