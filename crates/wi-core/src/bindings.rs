use shibari::AcceptState;

use crate::{actions::prelude::*, modifiers::M_TCTL, operators::Operator, Step};

#[derive(Debug, Clone, Copy)]
pub enum ActionOut {
    Trap,
    Advance,
    Action(Action),
    Operator(Operator),
}

impl<T: Into<Action>> From<T> for ActionOut {
    #[inline]
    fn from(value: T) -> Self { Self::Action(value.into()) }
}

#[inline]
fn dispatch_op<T: Into<Operator>>(op: T) -> ActionOut { ActionOut::Operator(op.into()) }

impl AcceptState for ActionOut {
    const ADVANCE: Self = Self::Advance;
    const TRAP: Self = Self::Trap;
}

#[derive(Debug, Clone, Copy)]
pub enum MotionOut {
    Trap,
    Advance,
    Motion(Motion),
}

impl<T: Into<Motion>> From<T> for MotionOut {
    #[inline]
    fn from(value: T) -> Self { Self::Motion(value.into()) }
}

impl From<MotionOut> for ActionOut {
    #[inline]
    fn from(value: MotionOut) -> Self {
        match value {
            MotionOut::Trap => Self::Trap,
            MotionOut::Advance => Self::Advance,
            MotionOut::Motion(m) => Self::Action(m.into()),
        }
    }
}

impl AcceptState for MotionOut {
    const ADVANCE: Self = Self::Advance;
    const TRAP: Self = Self::Trap;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char, Modifiers),
    Named(NamedKey, Modifiers),
}

use keyboard_types::{Modifiers, NamedKey};
use Key::{Char as C, Named as N};
use NamedKey as K;

#[allow(clippy::wildcard_imports)]
use crate::modifiers::*;

shibari::static_acceptors! {
    input = Key;

    token Escape = N(K::Escape, M_NONE) | C('[' | 'c', M_TCTL);
    token Home = N(K::Home, M_NONE);

    token AddOp = C('a', M_NONE) => "a";
    token ConnectOp = C('c', M_NONE) => "c";
    token DeleteOp = C('d', M_NONE) => "d";
    token GoOp = C('g', M_NONE) => "g";
    token ViewOp = C('z', M_NONE) => "z";

    token Debug = C('d', M_NONE);

    token Left = C('h', M_NONE) | N(K::ArrowLeft, M_NONE);
    token Down = C('j', M_NONE) | N(K::ArrowDown, M_NONE);
    token Up = C('k', M_NONE) | N(K::ArrowUp, M_NONE);
    token Right = C('l', M_NONE) | N(K::ArrowRight, M_NONE);

    token Opposite = C('%', M_NONE | M_SHIFT);

    token DigitNonzero = C(c @ '1'..='9', M_NONE | M_SHIFT) => "";
    token Digit = C(c @ '0'..='9', M_NONE | M_SHIFT);

    pub grammar Normal: ActionOut {
        AddOp {}
        ConnectOp {}

        DeleteOp {
            DeleteOp => yield DeleteAtCursor;
        }

        GoOp {}

        extend Motion;
        extend Global;
    }

    grammar Motion: MotionOut {
        Left => yield StepCursor(Step::Left);
        Down => yield StepCursor(Step::Down);
        Up => yield StepCursor(Step::Up);
        Right => yield StepCursor(Step::Right);

        Opposite => yield GoToOpposite;
    }

    grammar Global: ActionOut {
        'count: DigitNonzero {
            yield PushCount(c);

            Digit => goto 'count, PushCount(c);

            ..continue
        }

        Escape => yield SetMode(ModeKind::Normal);
        ViewOp {
            ViewOp => yield ViewCursor;
            Debug => yield ToggleDebug;
        }

        Home => yield ViewCursor;
    }
}

mod mode {
    use keyboard_types::NamedKey;
    use shibari::Acceptor;
    use tracing::debug;

    use super::{ActionOut, Key, NormalAccept};

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Mode {
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
                ModeKind::Normal => Self::Normal {
                    accept: NormalAccept::default(),
                },
            };

            true
        }

        #[inline]
        pub fn kind(self) -> ModeKind {
            match self {
                Self::Normal { .. } => ModeKind::Normal,
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
                return ActionOut::Advance;
            }

            debug!("Handling keypress");
            match self {
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
