use std::{
    borrow::Cow,
    num::{NonZero, NonZeroU32},
};

use tracing::{debug, instrument};

use crate::{GraphWidget, GraphWidgetDriver};

pub mod prelude {
    pub use std::num::{NonZero, NonZeroU32};

    pub use super::{actions::*, ActionCx, EditorAction};
    pub use crate::{cursor::actions::*, jump::actions::*};

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

pub struct ActionCx<'a, 'w, W: GraphWidget + ?Sized> {
    pub widget: &'a mut W,
    pub driver: &'a mut GraphWidgetDriver<W>,
    pub inner: &'a mut W::Context<'w>,
}

mod imp {
    use std::borrow::Cow;

    use enum_dispatch::enum_dispatch;

    use super::prelude::*;
    use crate::GraphWidget;

    #[enum_dispatch]
    pub trait EditorAction {
        #[inline]
        fn is_silent(&self) -> bool { false }

        fn name(&self) -> Cow<'static, str>;

        fn process<W: GraphWidget + ?Sized>(
            self,
            count: Option<NonZeroU32>,
            cx: ActionCx<W>,
        ) -> bool;
    }

    #[enum_dispatch(EditorAction)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Action {
        // Local
        Nop,
        PushCount,
        ToggleDebug,
        Unhandled,
        GoToOpposite,

        // From cursor
        StepCursor,
        ViewCursor,

        // From jump
        JumpToPort,
    }

    impl Default for Action {
        fn default() -> Self { Self::Unhandled(Unhandled) }
    }

    impl Action {
        #[inline]
        #[must_use]
        pub fn name(self) -> Cow<'static, str> { EditorAction::name(&self) }
    }
}

pub use imp::{Action, EditorAction};

mod actions {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Nop;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PushCount(pub char);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ToggleDebug;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Unhandled;
}

impl EditorAction for actions::Nop {
    #[inline]
    fn is_silent(&self) -> bool { true }

    #[inline]
    fn name(&self) -> Cow<'static, str> { "nop".into() }

    #[inline]
    fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        cx.driver.count = count;
        true
    }
}

impl EditorAction for actions::PushCount {
    #[inline]
    fn is_silent(&self) -> bool { true }

    #[inline]
    fn name(&self) -> Cow<'static, str> { "push count".into() }

    fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
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

impl EditorAction for actions::ToggleDebug {
    #[inline]
    fn is_silent(&self) -> bool { true }

    #[inline]
    fn name(&self) -> Cow<'static, str> { "toggle debug".into() }

    fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let None = count else { return false };

        cx.driver.debug = !cx.driver.debug;

        true
    }
}

impl EditorAction for actions::Unhandled {
    #[inline]
    fn is_silent(&self) -> bool { true }

    #[inline]
    fn name(&self) -> Cow<'static, str> { "<unhandled>".into() }

    #[inline]
    fn process<W: GraphWidget + ?Sized>(self, _: Option<NonZeroU32>, _: ActionCx<W>) -> bool {
        false
    }
}

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    #[instrument(
        skip(self, widget, ctx),
        fields(count = ?self.count),
    )]
    pub fn process_action(
        &mut self,
        widget: &mut W,
        action: Action,
        ctx: &mut W::Context<'_>,
    ) -> bool {
        debug!("Processing action");
        let handled = action.process(self.count.take(), ActionCx {
            widget,
            driver: self,
            inner: ctx,
        });

        if handled && !action.is_silent() {
            self.last_action = Some(action);
        }

        handled
    }
}
