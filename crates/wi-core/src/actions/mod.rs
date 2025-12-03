mod basic;
mod create;
mod cursor;
mod delete;
mod quit;

pub mod all {
    pub use super::{
        basic::actions::*, create::actions::*, cursor::actions::*, delete::actions::*,
        quit::actions::*,
    };
}

pub mod prelude {
    pub use std::num::{NonZero, NonZeroU32};

    pub use derive_where::derive_where;
    pub use wi_macros::Kind;

    pub(crate) use super::{all::*, ActionCx, ActionKind, EditorAction, Kind};
    pub use crate::traits::*;

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
    use crate::DriverInner;

    #[derive_where(Debug; W::NodeId, W::PortId, W::NodeKind, W::Point)]
    #[derive_where(Default; )]
    pub struct LastAction<W: GraphWidgetTypes + ?Sized> {
        action: Option<Action<W>>,
        hidden: bool,
    }

    impl<W: GraphWidgetTypes + ?Sized> LastAction<W> {
        #[inline]
        pub fn as_ref(&self, show_hidden: bool) -> Option<&Action<W>> {
            (show_hidden || !self.hidden)
                .then_some(self.action.as_ref())
                .flatten()
        }

        pub fn replace(&mut self, action: Action<W>) -> Option<Action<W>> {
            if matches!(action, Action::SimpleAction(SimpleAction::Repeat(_))) {
                return None;
            }

            self.hidden = false;
            self.action.replace(action)
        }

        #[inline]
        pub fn hide(&mut self) { self.hidden = true; }
    }

    pub struct ActionCx<'a, 'w: 'a, W: GraphWidgetTypes + ?Sized> {
        pub widget: &'a mut W,
        pub driver: &'a mut DriverInner<W>,
        inner: W::Context<'a, 'w>,
    }

    impl<'a, 'w, W: GraphWidgetTypes + ?Sized> ActionCx<'a, 'w, W> {
        pub const fn new(
            widget: &'a mut W,
            driver: &'a mut DriverInner<W>,
            inner: W::Context<'a, 'w>,
        ) -> Self {
            Self {
                widget,
                driver,
                inner,
            }
        }

        #[inline]
        pub fn run<T>(
            &mut self,
            f: impl FnOnce(&mut W, &mut DriverInner<W>, W::Context<'_, 'w>) -> T,
        ) -> T {
            f(self.widget, self.driver, W::reborrow_cx(&mut self.inner))
        }
    }

    pub(crate) trait Kind<K: Copy + Eq + Hash> {
        fn kind(&self) -> K;
    }

    pub(crate) trait EditorAction<W: GraphWidgetTypes + ?Sized>: Kind<ActionKind> {
        fn process(&self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool;
    }

    #[impl_enum]
    #[derive_where(Debug, Clone, Copy, PartialEq, Eq, Hash;
        W::NodeId, W::PortId, W::NodeKind, W::Point)]
    pub enum Action<W: GraphWidgetTypes + ?Sized> {
        SimpleAction(SimpleAction),

        // From create
        CreateEdge(CreateEdge<W>),
        CreateNode(CreateNode<W>),

        // From cursor
        GoToPort(GoToPort<W>),
    }

    #[impl_enum]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum SimpleAction {
        // Basic
        PushCount(PushCount),
        Repeat(Repeat),
        SetMode(SetMode),
        ToggleDebug(ToggleDebug),

        // From cursor
        GoToOpposite(GoToOpposite),
        StepCursor(StepCursor),
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
        fn process(&self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
            dispatch!(self, count, cx)
        }
    }

    #[impl_enum]
    impl<W: GraphWidget + ?Sized> EditorAction<W> for SimpleAction {
        #[inline]
        fn process(&self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
            dispatch!(self, count, cx)
        }
    }

    impl<W: GraphWidgetTypes + ?Sized> Action<W> {
        pub fn kind(&self) -> ActionKind { Kind::kind(self) }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum ActionKind {
        CreateEdge,
        CreateNode,
        DeleteAtCursor,
        GoToOpposite,
        GoToPort,
        ModeNormal,
        PushCount,
        Quit,
        Repeat,
        StepDown,
        StepLeft,
        StepRight,
        StepUp,
        ToggleDebug,
        ViewCursor,
    }

    impl ActionKind {
        #[inline]
        #[must_use]
        pub fn name(self) -> &'static str {
            match self {
                Self::CreateEdge => "create edge",
                Self::CreateNode => "create node",
                Self::DeleteAtCursor => "delete at cursor",
                Self::GoToOpposite => "go to opposite",
                Self::GoToPort => "go to port",
                Self::ModeNormal => "normal mode",
                Self::PushCount => "push count",
                Self::Quit => "quit",
                Self::Repeat => "repeat",
                Self::StepDown => "step down",
                Self::StepLeft => "step left",
                Self::StepRight => "step right",
                Self::StepUp => "step up",
                Self::ToggleDebug => "toggle debug",
                Self::ViewCursor => "view cursor",
            }
        }

        #[inline]
        #[must_use]
        pub fn is_silent(self) -> bool {
            matches!(
                self,
                Self::PushCount | Self::StepDown | Self::StepLeft | Self::StepRight | Self::StepUp
            )
        }
    }
}

pub use imp::{Action, ActionCx, ActionKind, LastAction, SimpleAction};
pub(crate) use imp::{EditorAction, Kind};
