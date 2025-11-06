mod add;

pub mod all {
    pub use super::add::operators::*;
}

pub mod prelude {
    pub use std::sync::Arc;

    pub use shibari::Acceptor;

    pub use super::{all::*, Operator, OperatorCx, OperatorInner, OperatorResult, OperatorState};
    pub use crate::{
        actions::prelude::*,
        bindings::*,
        continuation::{ContinueCx, ContinueOnce, Dispatch, Yielded},
    };
}

mod imp {
    use enum_dispatch::enum_dispatch;

    use super::prelude::*;
    use crate::DriverInner;

    pub struct OperatorCx<'a, 'w, W: GraphWidget + ?Sized> {
        pub(super) widget: &'a mut W,
        pub(super) driver: &'a mut DriverInner<W>,
        inner: &'a mut W::Context<'w>,
    }

    impl<'a, 'w, W: GraphWidget + ?Sized> OperatorCx<'a, 'w, W> {
        pub const fn new(
            widget: &'a mut W,
            driver: &'a mut DriverInner<W>,
            inner: &'a mut W::Context<'w>,
        ) -> Self {
            Self {
                widget,
                driver,
                inner,
            }
        }

        #[inline]
        pub const fn into_yielded<C>(self, then: C) -> (&'a mut W, Yielded<'a, 'w, W, C>) {
            (self.widget, Yielded::new(then, self.driver, self.inner))
        }
    }

    pub enum OperatorResult {
        Continue,
        Finish,
        Abort,
    }

    #[enum_dispatch]
    pub trait EditorOperator {
        fn state(&self) -> OperatorState;

        fn init<W: GraphWidget + ?Sized>(&mut self, cx: OperatorCx<W>) -> bool;

        fn step<W: GraphWidget + ?Sized>(&mut self, key: Key, cx: OperatorCx<W>) -> OperatorResult;
    }

    pub trait OperatorInner: Sized {
        const DEFAULT_STATE: OperatorState;

        fn new<W: GraphWidget + ?Sized>(cx: OperatorCx<W>) -> Option<Self>;

        #[inline]
        fn state(&self) -> Option<OperatorState> { None }

        fn step<W: GraphWidget + ?Sized>(&mut self, key: Key, cx: OperatorCx<W>) -> OperatorResult;
    }

    #[enum_dispatch(EditorOperator)]
    #[derive(Debug, PartialEq)]
    pub enum Operator {
        Add,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum OperatorState {
        Add,
    }

    impl OperatorState {
        #[must_use]
        pub const fn name(self) -> &'static str {
            match self {
                Self::Add => "add",
            }
        }
    }

    impl Operator {
        #[inline]
        #[must_use]
        pub fn state(&self) -> OperatorState { EditorOperator::state(self) }
    }
}

pub use imp::{EditorOperator, Operator, OperatorCx, OperatorInner, OperatorResult, OperatorState};

macro_rules! operator {
    ($(#$attr:tt)* $vis:vis struct $op:ident($inner_vis:vis $inner:ty);) => {
        $(#$attr)*
        $vis struct $op($inner_vis Option<$inner>);

        impl crate::operators::EditorOperator for $op {
            fn state(&self) -> crate::operators::OperatorState {
                self.0.as_ref().and_then(
                    <$inner as crate::operators::OperatorInner>::state
                ).unwrap_or_else(|| {
                    <$inner as crate::operators::OperatorInner>::DEFAULT_STATE.into()
                })
            }

            fn init<W: crate::GraphWidget + ?Sized>(
                &mut self,
                cx: crate::operators::OperatorCx<W>,
            ) -> bool {
                if self.0.is_some() { return false; }

                self.0 = <$inner as crate::operators::OperatorInner>::new(cx);
                self.0.is_some()
            }

            fn step<W: crate::GraphWidget + ?Sized>(
                &mut self,
                key: crate::bindings::Key,
                cx: crate::operators::OperatorCx<W>,
            ) -> crate::operators::OperatorResult {
                let Some(ref mut inner) = self.0 else {
                    return crate::operators::OperatorResult::Abort;
                };

                let ret = <$inner as crate::operators::OperatorInner>::step(inner, key, cx);

                if !matches!(ret, crate::operators::OperatorResult::Continue) {
                    self.0 = None;
                }

                ret
            }
        }
    };
}

pub(crate) use operator;
