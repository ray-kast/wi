use keyboard_types::{Modifiers, NamedKey};
use shibari::{static_acceptors, AcceptorOutput};
use Key::{Char as C, Named as N};
use NamedKey as K;

use crate::{
    actions::{prelude::*, SimpleAction},
    mode::ModeKind,
    modifiers::{M_CTRL, M_NONE, M_SHIFT, M_TCTL},
    operators::prelude::*,
    Side, Step,
};

pub enum CreateOpAction {
    Accept,
    Side(Side),
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

#[derive(Debug, Clone, Copy, AcceptorOutput)]
pub enum ActionOut {
    #[shibari(trap)]
    Trap,
    #[shibari(advance)]
    Advance,
    Modifier,
    #[shibari(from)]
    Action(SimpleAction),
    Operator(Operator),
}

impl From<Operator> for ActionOut {
    #[inline]
    fn from(value: Operator) -> Self { Self::Operator(value) }
}

#[inline]
fn op<T: Into<Operator>>(op: T) -> Operator { op.into() }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Char(char, Modifiers),
    Named(NamedKey, Modifiers),
}

static_acceptors! {
    input = Key;

    token Escape = N(K::Escape, M_NONE) | C('[' | 'c', M_TCTL) => "\u{238b}";
    token Home = N(K::Home, M_NONE) => "\u{21b0}";
    token Accept = C(' ', M_NONE) | N(K::Enter, M_NONE) => "\u{23ce}";

    token CommandOp = C(':', M_NONE | M_SHIFT) => ":";
    token Repeat = C('.', M_NONE | M_SHIFT) => ".";
    token CreateOp = C('c', M_NONE) => "c";
    token DeleteOp = C('d', M_NONE) => "d";
    token GoOp = C('g', M_NONE) => "g";
    token Input = C('i', M_NONE) => "i";
    token Output = C('o', M_NONE) => "o";
    token ViewOp = C('z', M_NONE) => "z";
    token Quit = C('q', M_NONE) => "q";
    token CtrlQuit = C('q', M_CTRL) => "\u{2303}q";

    token Debug = C('d', M_NONE) => "d";

    token Left = C('h', M_NONE) | N(K::ArrowLeft, M_NONE) => "h";
    token Down = C('j', M_NONE) | N(K::ArrowDown, M_NONE) => "j";
    token Up = C('k', M_NONE) | N(K::ArrowUp, M_NONE) => "k";
    token Right = C('l', M_NONE) | N(K::ArrowRight, M_NONE) => "l";

    token NudgeLeft = C('h', M_SHIFT) | N(K::ArrowLeft, M_SHIFT) => "H";
    token NudgeDown = C('j', M_SHIFT) | N(K::ArrowDown, M_SHIFT) => "J";
    token NudgeUp = C('k', M_SHIFT) | N(K::ArrowUp, M_SHIFT) => "K";
    token NudgeRight = C('l', M_SHIFT) | N(K::ArrowRight, M_SHIFT) => "L";

    token Opposite = C('%', M_NONE) => "%";

    token DigitNonzero = C(c @ '1'..='9', M_NONE) => "";
    token Digit = C(c @ '0'..='9', M_NONE) => "";

    pub grammar CreateOp: CreateOut {
        CreateOp | Accept => yield CreateOpAction::Accept;

        Input => yield CreateOpAction::Side(Side::In);
        Output => yield CreateOpAction::Side(Side::Out);
    }

    pub grammar Normal: ActionOut {
        CreateOp => yield op(Create);

        DeleteOp {
            DeleteOp => yield DeleteAtCursor;
        }

        GoOp {}

        NudgeLeft => yield Nudge(Step::Left);
        NudgeDown => yield Nudge(Step::Down);
        NudgeUp => yield Nudge(Step::Up);
        NudgeRight => yield Nudge(Step::Right);

        Repeat => yield Repeat;

        extend Motion;
        extend Global;
    }

    grammar Motion: ActionOut {
        Left => yield StepCursor(Step::Left);
        Down => yield StepCursor(Step::Down);
        Up => yield StepCursor(Step::Up);
        Right => yield StepCursor(Step::Right);

        Input => yield op(JumpToPort::CurrentNode(Side::In));
        Output => yield op(JumpToPort::CurrentNode(Side::Out));

        GoOp {
            Input => yield op(JumpToPort::Global(Side::In));
            Output => yield op(JumpToPort::Global(Side::Out));
        }

        Opposite => yield GoToOpposite;
    }

    grammar Global: ActionOut {
        'count: DigitNonzero {
            yield PushCount(c);

            Digit => goto 'count, PushCount(c);

            ..continue
        }

        Escape => yield SetMode(ModeKind::Normal);

        CommandOp {
            Quit => yield Quit;
        }

        ViewOp {
            ViewOp => yield ViewCursor;
            Debug => yield ToggleDebug;
        }

        Home => yield ViewCursor;
        CtrlQuit => yield Quit;
    }
}
