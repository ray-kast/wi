use keyboard_types::{Modifiers, NamedKey};
use shibari::{static_acceptors, AcceptorOutput};
use Key::{Char as C, Named as N};
use NamedKey as K;

use crate::{
    actions::{prelude::*, Motion, SimpleAction},
    mode::ModeKind,
    modifiers::{M_NONE, M_SHIFT, M_TCTL},
    operators::prelude::*,
    Step,
};

pub enum CreateOpAction {
    Accept,
}

#[derive(AcceptorOutput)]
pub enum CreateOut {
    #[shibari(trap)]
    Trap,
    #[shibari(advance)]
    Advance,
    #[shibari(from)]
    CreateOpAction(CreateOpAction),
}

#[derive(Debug, AcceptorOutput)]
pub enum ActionOut {
    #[shibari(trap)]
    Trap,
    #[shibari(advance)]
    Advance,
    #[shibari(from)]
    Action(SimpleAction),
    Operator(Operator),
}

#[inline]
fn dispatch_op<T: Into<Operator>>(op: T) -> ActionOut { ActionOut::Operator(op.into()) }

#[derive(Debug, Clone, Copy, AcceptorOutput)]
pub enum MotionOut {
    #[shibari(trap)]
    Trap,
    #[shibari(advance)]
    Advance,
    #[shibari(from)]
    Motion(Motion),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char, Modifiers),
    Named(NamedKey, Modifiers),
}

static_acceptors! {
    input = Key;

    token Escape = N(K::Escape, M_NONE) | C('[' | 'c', M_TCTL);
    token Home = N(K::Home, M_NONE);
    token Accept = C(' ', M_NONE) | N(K::Enter, M_NONE);

    token CreateOp = C('c', M_NONE);
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

    pub grammar CreateOp: CreateOut {
        CreateOp | Accept => yield CreateOpAction::Accept;
    }

    pub grammar Normal: ActionOut {
        CreateOp => yield dispatch_op(Create::default());

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
