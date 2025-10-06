use std::num::{NonZero, NonZeroU32};

use tracing::{debug, instrument};

use crate::{GraphWidget, GraphWidgetDriver};

pub mod prelude {
    pub use std::num::{NonZero, NonZeroU32};

    pub use super::{actions::*, ActionCx, EditorAction};
    pub use crate::cursor::actions::*;
}

pub struct ActionCx<'a, 'w, W: GraphWidget + ?Sized> {
    pub widget: &'a mut W,
    pub driver: &'a mut GraphWidgetDriver<W>,
    pub inner: &'a mut W::Context<'w>,
}

mod imp {
    use enum_dispatch::enum_dispatch;

    use crate::GraphWidget;

    use super::prelude::*;

    #[enum_dispatch]
    pub trait EditorAction {
        fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool;
    }

    #[enum_dispatch(EditorAction)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Action {
        // Local
        Nop,
        PushCount,
        Unhandled,

        // From cursor
        StepCursor,
        ViewCursor,
    }

    impl Default for Action {
        fn default() -> Self { Self::Unhandled(Unhandled) }
    }
}

pub use imp::{Action, EditorAction};

mod actions {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Nop;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PushCount(pub char);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Unhandled;
}

impl EditorAction for actions::Nop {
    #[inline]
    fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        cx.driver.count = count;
        true
    }
}

impl EditorAction for actions::PushCount {
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

impl EditorAction for actions::Unhandled {
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
        EditorAction::process(action, self.count.take(), ActionCx {
            widget,
            driver: self,
            inner: ctx,
        })
    }
}
