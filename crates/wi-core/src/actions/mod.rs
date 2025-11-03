use std::{
    borrow::Cow,
    num::{NonZero, NonZeroU32},
};

use tracing::{debug, instrument};

use crate::{
    bindings::{ActionOut, ModeKind},
    GraphWidget, GraphWidgetDriver,
};

mod cursor;
mod delete;
mod jump;

pub mod all {
    pub use super::{basic::*, cursor::actions::*, delete::actions::*, jump::actions::*};
}

pub mod prelude {
    pub use std::{
        borrow::Cow,
        num::{NonZero, NonZeroU32},
    };

    pub use super::{all::*, Action, ActionCx, EditorAction, EditorMotion, Motion};
    pub use crate::{
        cursor::{Selection, SelectionExt},
        GraphWidget,
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

pub struct ActionCx<'a, 'w, W: GraphWidget + ?Sized> {
    pub widget: &'a mut W,
    pub driver: &'a mut GraphWidgetDriver<W>,
    pub inner: &'a mut W::Context<'w>,
}

impl<'w, W: GraphWidget + ?Sized> ActionCx<'_, 'w, W> {
    #[inline]
    pub fn reborrow<'b>(&'b mut self) -> ActionCx<'b, 'w, W> {
        ActionCx {
            widget: &mut *self.widget,
            driver: &mut *self.driver,
            inner: &mut *self.inner,
        }
    }
}

mod imp {
    use enum_dispatch::enum_dispatch;

    use super::prelude::*;
    use crate::cursor::NullSelection;

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
        // Basic
        Motion,
        PushCount,
        SetMode,
        ToggleDebug,

        // From cursor
        ViewCursor,

        // From delete
        DeleteAtCursor,
    }

    impl Action {
        #[inline]
        #[must_use]
        pub fn name(self) -> Cow<'static, str> { EditorAction::name(&self) }
    }

    #[enum_dispatch]
    pub trait EditorMotion {
        fn name(&self) -> Cow<'static, str>;

        fn process<W: GraphWidget + ?Sized, S: Selection>(
            self,
            count: Option<NonZeroU32>,
            cx: ActionCx<W>,
            selection: S,
        ) -> bool;
    }

    impl<T: EditorMotion> EditorAction for T {
        #[inline]
        fn is_silent(&self) -> bool { true }

        #[inline]
        fn name(&self) -> Cow<'static, str> { EditorMotion::name(self) }

        fn process<W: GraphWidget + ?Sized>(
            self,
            count: Option<NonZeroU32>,
            cx: ActionCx<W>,
        ) -> bool {
            EditorMotion::process(self, count, cx, NullSelection)
        }
    }

    #[enum_dispatch(EditorMotion)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Motion {
        // From cursor
        GoToOpposite,
        StepCursor,

        // From jump
        JumpToPort,
    }
}

pub use imp::{Action, EditorAction, EditorMotion, Motion};

mod basic {
    use crate::bindings::ModeKind;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PushCount(pub char);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SetMode(pub ModeKind);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct ToggleDebug;
}

impl EditorAction for basic::PushCount {
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

impl EditorAction for basic::SetMode {
    #[inline]
    fn is_silent(&self) -> bool { true }

    fn name(&self) -> Cow<'static, str> {
        let Self(m) = self;
        match m {
            ModeKind::Normal => "normal mode",
        }
        .into()
    }

    fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let Self(m) = self;
        let None = count else { return false };

        cx.driver.mode.change(m)
    }
}

impl EditorAction for basic::ToggleDebug {
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

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    #[instrument(skip(self, widget, action, ctx), fields(count = ?self.count))]
    pub fn process_action(
        &mut self,
        widget: &mut W,
        action: ActionOut,
        ctx: &mut W::Context<'_>,
    ) -> bool {
        let action = match action {
            ActionOut::Trap => {
                self.count = None;
                return false;
            },
            ActionOut::Advance => return true,
            ActionOut::Action(a) => a,
        };

        debug!(action = action.name().as_ref(), "Processing action");
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
