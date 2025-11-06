use keyboard_types::{Modifiers, NamedKey};
use shibari::{static_acceptors, AcceptState};
use Key::{Char as C, Named as N};
use NamedKey as K;

use crate::{
    actions::prelude::*,
    mode::ModeKind,
    modifiers::{M_NONE, M_SHIFT, M_TCTL},
    operators::prelude::*,
    Step,
};

macro_rules! out {
    ($(#$attr:tt)* $vis:vis enum $ty:ident {
        $inner:ident
        $(, $($body:tt)*)?
    }) => {
        $(#$attr)*
        $vis enum $ty {
            Trap,
            Advance,
            $inner($inner),
            $($($body)*)?
        }

        impl<T: Into<$inner>> From<T> for $ty {
            #[inline]
            fn from(value: T) -> Self { Self::$inner(value.into()) }
        }

        impl AcceptState for $ty {
            const TRAP: Self = $ty::Trap;
            const ADVANCE: Self = $ty::Advance;
        }
    };
}

pub enum AddOpAction {
    Accept,
}

out! {
    pub enum AddOut {
        AddOpAction,
    }
}

out! {
    #[derive(Debug)]
    pub enum NormalOut {
        Action,
        Operator(Operator),
    }
}

#[inline]
fn dispatch_op<T: Into<Operator>>(op: T) -> NormalOut { NormalOut::Operator(op.into()) }

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

impl From<MotionOut> for NormalOut {
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

static_acceptors! {
    input = Key;

    token Escape = N(K::Escape, M_NONE) | C('[' | 'c', M_TCTL);
    token Home = N(K::Home, M_NONE);
    token Accept = C(' ', M_NONE) | N(K::Enter, M_NONE);

    token AddOp = C('a', M_NONE);
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

    pub grammar AddOp: AddOut {
        AddOp | Accept => yield AddOpAction::Accept;
    }

    pub grammar Normal: NormalOut {
        AddOp => yield dispatch_op(Add::default());
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

    grammar Global: NormalOut {
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
