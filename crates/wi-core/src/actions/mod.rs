use std::num::{NonZero, NonZeroU32};

use crate::traits::GraphWidgetTypes;

mod create;
mod cursor;
mod delete;
mod jump;
mod quit;

pub mod all {
    pub use super::{
        basic::*, create::actions::*, cursor::actions::*, delete::actions::*, jump::actions::*,
        quit::actions::*,
    };
}

pub mod prelude {
    pub use std::num::{NonZero, NonZeroU32};

    pub(crate) use super::{all::*, ActionCx, EditorAction, EditorMotion, Kind};
    pub use crate::{
        cursor::{Selection, SelectionExt},
        traits::*,
    };

    pub fn run_with_count(
        count: Option<NonZeroU32>,
        mut f: impl FnMut(NonZeroU32) -> Option<NonZeroU32>,
    ) -> bool {
        let mut count = count.map_or(1, NonZero::get);
        let mut any = false;

        while let Some(n) = NonZero::new(count) {
            let Some(dec) = f(n) else { break };

            any = true;
            count = count
                .checked_sub(dec.get())
                .unwrap_or_else(|| unreachable!());
        }

        any
    }
}

mod imp {
    use std::hash::Hash;

    use derive_where::derive_where;
    use wi_macros::impl_enum;

    use super::prelude::*;
    use crate::{cursor::NullSelection, DriverInner};

    pub struct ActionCx<'a, 'w, W: GraphWidgetTypes + ?Sized> {
        pub widget: &'a mut W,
        pub driver: &'a mut DriverInner<W>,
        pub inner: &'a mut W::Context<'w>,
    }

    pub(crate) trait Kind<K: Copy + Eq + Hash> {
        fn kind(&self) -> K;
    }

    pub(crate) trait EditorAction<W: GraphWidgetTypes + ?Sized>: Kind<ActionKind> {
        fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool;
    }

    #[impl_enum]
    #[derive_where(Debug, Clone, Copy, PartialEq, Eq, Hash; W::NodeKind, W::Point)]
    pub enum Action<W: GraphWidgetTypes + ?Sized> {
        SimpleAction(SimpleAction),

        // From create
        CreateNode(CreateNode<W>),
    }

    #[impl_enum]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum SimpleAction {
        // Basic
        Motion(Motion),
        PushCount(PushCount),
        SetMode(SetMode),
        ToggleDebug(ToggleDebug),

        // From cursor
        ViewCursor(ViewCursor),

        // From delete
        DeleteAtCursor(DeleteAtCursor),

        // From quit
        Quit(Quit),
    }

    #[impl_enum]
    impl<W: GraphWidgetTypes + ?Sized> Kind<ActionKind> for Action<W> {
        fn kind(&self) -> ActionKind { dispatch!(self) }
    }

    #[impl_enum]
    impl Kind<ActionKind> for SimpleAction {
        fn kind(&self) -> ActionKind { dispatch!(self) }
    }

    #[impl_enum]
    impl<W: GraphWidget + ?Sized> EditorAction<W> for Action<W> {
        #[inline]
        fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
            dispatch!(self, count, cx)
        }
    }

    #[impl_enum]
    impl<W: GraphWidget + ?Sized> EditorAction<W> for SimpleAction {
        #[inline]
        fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
            dispatch!(self, count, cx)
        }
    }

    impl<W: GraphWidgetTypes + ?Sized> Action<W> {
        pub fn kind(&self) -> ActionKind { Kind::kind(self) }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ActionKind {
        CreateNode,
        DeleteAtCursor,
        ModeNormal,
        Motion(MotionKind),
        PushCount,
        Quit,
        ToggleDebug,
        ViewCursor,
    }

    impl From<MotionKind> for ActionKind {
        #[inline]
        fn from(value: MotionKind) -> Self { Self::Motion(value) }
    }

    impl ActionKind {
        #[inline]
        #[must_use]
        pub fn name(self) -> &'static str {
            match self {
                Self::CreateNode => "create node",
                Self::DeleteAtCursor => "delete at cursor",
                Self::ModeNormal => "normal mode",
                Self::Motion(m) => m.name(),
                Self::PushCount => "push count",
                Self::Quit => "quit",
                Self::ToggleDebug => "toggle debug",
                Self::ViewCursor => "view cursor",
            }
        }

        #[inline]
        #[must_use]
        pub fn is_silent(self) -> bool {
            match self {
                Self::Motion(m) => m.is_silent(),
                Self::PushCount => true,
                _ => false,
            }
        }
    }

    pub(crate) trait EditorMotion<W: GraphWidgetTypes + ?Sized, S: Selection>:
        Kind<MotionKind>
    {
        fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>, selection: S) -> bool;
    }

    impl<T: Kind<MotionKind>> Kind<ActionKind> for T {
        #[inline]
        fn kind(&self) -> ActionKind { ActionKind::Motion(Kind::<MotionKind>::kind(self)) }
    }

    impl<W: GraphWidgetTypes + ?Sized, T: EditorMotion<W, NullSelection>> EditorAction<W> for T {
        fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
            EditorMotion::process(self, count, cx, NullSelection)
        }
    }

    #[impl_enum]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Motion {
        // From cursor
        GoToOpposite(GoToOpposite),
        StepCursor(StepCursor),

        // From jump
        JumpToPort(JumpToPort),
    }

    #[impl_enum]
    impl Kind<MotionKind> for Motion {
        fn kind(&self) -> MotionKind { dispatch!(self) }
    }

    #[impl_enum]
    impl<W: CursorOps + UiOps + ?Sized, S: Selection> EditorMotion<W, S> for Motion {
        #[inline]
        fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>, selection: S) -> bool {
            dispatch!(self, count, cx, selection)
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum MotionKind {
        GoToOpposite,
        JumpToInput,
        JumpToOutput,
        JumpToPort,
        StepDown,
        StepLeft,
        StepRight,
        StepUp,
    }

    impl MotionKind {
        #[inline]
        #[must_use]
        pub fn name(self) -> &'static str {
            match self {
                Self::GoToOpposite => "go to opposite",
                Self::JumpToInput => "jump to input",
                Self::JumpToOutput => "jump to output",
                Self::JumpToPort => "jump to port",
                Self::StepDown => "step down",
                Self::StepLeft => "step left",
                Self::StepRight => "step right",
                Self::StepUp => "step up",
            }
        }

        #[inline]
        #[must_use]
        pub fn is_silent(self) -> bool {
            matches!(
                self,
                Self::StepDown | Self::StepLeft | Self::StepRight | Self::StepUp
            )
        }
    }
}

pub use imp::{Action, ActionCx, ActionKind, Motion, MotionKind, SimpleAction};
pub(crate) use imp::{EditorAction, EditorMotion, Kind};

mod basic {
    use crate::{actions::ActionKind, mode::ModeKind};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    #[kind(ActionKind)]
    pub struct PushCount(pub char);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    pub struct SetMode(#[kind(ModeKind::Normal => ActionKind::ModeNormal)] pub ModeKind);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    #[kind(ActionKind)]
    pub struct ToggleDebug;
}

impl<W: GraphWidgetTypes + ?Sized> EditorAction<W> for basic::PushCount {
    fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let Self(digit) = self;
        let digit = u32::from(digit) - u32::from('0');
        cx.driver.count = count
            .map_or(0, NonZero::get)
            .checked_mul(10)
            .and_then(|c| c.checked_add(digit))
            .and_then(NonZero::new);
        true
    }
}

impl<W: GraphWidgetTypes + ?Sized> EditorAction<W> for basic::SetMode {
    fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let Self(m) = self;
        let None = count else { return false };

        cx.driver.mode.change(m)
    }
}

impl<W: GraphWidgetTypes + ?Sized> EditorAction<W> for basic::ToggleDebug {
    fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let None = count else { return false };

        cx.driver.debug = !cx.driver.debug;

        true
    }
}
