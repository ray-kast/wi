mod create;

pub mod all {
    pub use super::create::operators::*;
}

pub mod prelude {
    pub use std::sync::Arc;

    pub use shibari::Acceptor;

    pub use super::{
        all::*, OpYielded, Operator, OperatorCx, OperatorInner, OperatorResult, OperatorState,
    };
    pub use crate::{
        actions::prelude::*,
        bindings::*,
        continuation::{ContinueCx, ContinueOnce, Dispatch, Yielded},
    };
}

mod imp {
    use std::mem;

    use wi_macros::impl_enum;

    use super::prelude::*;
    use crate::DriverInner;

    #[derive(Debug, Default)]
    pub struct CurrentOperator(Option<Operator>);

    impl CurrentOperator {
        #[inline]
        pub fn as_ref(&self) -> Option<&Operator> { self.0.as_ref() }

        #[inline]
        pub fn as_mut(&mut self) -> Option<&mut Operator> { self.0.as_mut() }

        pub fn push(&mut self, operator: Operator, stashed: &mut Vec<Operator>) -> &mut Operator {
            match &mut self.0 {
                o @ None => {
                    debug_assert!(stashed.is_empty());
                    *o = Some(operator);
                    o.as_mut().unwrap_or_else(|| unreachable!())
                },
                Some(o) => {
                    stashed.push(mem::replace(o, operator));
                    o
                },
            }
        }

        pub fn pop(&mut self, stashed: &mut Vec<Operator>) -> Option<Operator> {
            match self.0.take() {
                None => {
                    debug_assert!(stashed.is_empty());
                    None
                },
                Some(o) => {
                    self.0 = stashed.pop();
                    Some(o)
                },
            }
        }
    }

    pub type OpDispatch<'a, 'c> = Dispatch<&'c mut OperatorResult, &'a mut CurrentOperator>;
    pub type OpYielded<'a, 'c, Y> = (OpDispatch<'a, 'c>, Y);

    pub struct OperatorCx<'a, 'c, 'w, W: GraphWidget + ?Sized> {
        widget: &'a mut W,
        driver: &'a mut DriverInner<W>,
        inner: &'a mut W::Context<'w>,
        dispatch: OpDispatch<'a, 'c>,
    }

    impl<'a, 'c, 'w, W: GraphWidget + ?Sized> OperatorCx<'a, 'c, 'w, W> {
        pub const fn new(
            widget: &'a mut W,
            driver: &'a mut DriverInner<W>,
            inner: &'a mut W::Context<'w>,
            dispatch: OpDispatch<'a, 'c>,
        ) -> Self {
            Self {
                widget,
                driver,
                inner,
                dispatch,
            }
        }

        #[inline]
        pub const fn widget(&self) -> &W { self.widget }

        #[inline]
        pub const fn driver(&self) -> &DriverInner<W> { self.driver }

        #[inline]
        pub const fn dispatch(&self) -> &OpDispatch<'a, 'c> { &self.dispatch }

        #[inline]
        pub const fn into_yielded<Y, C>(
            self,
            then: C,
            yielded: Y,
        ) -> (&'a mut W, Yielded<'a, 'w, W, OpYielded<'a, 'c, Y>, C>) {
            (
                self.widget,
                Yielded::new(then, self.driver, (self.dispatch, yielded), self.inner),
            )
        }

        pub(crate) fn run_action<A: EditorAction<W>>(
            &mut self,
            action: A,
            count: Option<NonZeroU32>,
        ) -> bool {
            action.process(count, ActionCx {
                widget: self.widget,
                driver: self.driver,
                inner: self.inner,
            })
        }

        #[inline]
        pub fn abort(&mut self) {
            match self.dispatch {
                Dispatch::Immediate(ref mut r) => **r = OperatorResult::Abort,
                Dispatch::Deferred(ref mut c) => {
                    c.pop(&mut self.driver.stashed_operators);
                },
            }
        }

        #[inline]
        pub fn finish(&mut self) {
            match self.dispatch {
                Dispatch::Immediate(ref mut r) => **r = OperatorResult::Finish,
                Dispatch::Deferred(ref mut c) => {
                    c.pop(&mut self.driver.stashed_operators);
                },
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum OperatorResult {
        Continue,
        Finish,
        Abort,
    }

    pub(crate) trait EditorOperator {
        fn state(&self) -> OperatorState;

        fn init<W: GraphWidget + ?Sized>(&mut self, cx: OperatorCx<W>);

        fn step<W: GraphWidget + ?Sized>(&mut self, key: Key, cx: OperatorCx<W>);
    }

    pub trait OperatorInner: Sized {
        const DEFAULT_STATE: OperatorState;

        fn new<W: GraphWidget + ?Sized>(cx: OperatorCx<W>) -> Self;

        #[inline]
        fn state(&self) -> Option<OperatorState> { None }

        fn step<W: GraphWidget + ?Sized>(&mut self, key: Key, cx: OperatorCx<W>) -> OperatorResult;
    }

    #[impl_enum]
    #[derive(Debug, PartialEq)]
    pub enum Operator {
        Create(Create),
    }

    #[impl_enum]
    impl EditorOperator for Operator {
        fn state(&self) -> OperatorState { dispatch!(self) }

        fn init<W: GraphWidget + ?Sized>(&mut self, cx: OperatorCx<W>) { dispatch!(self, cx) }

        fn step<W: GraphWidget + ?Sized>(&mut self, key: Key, cx: OperatorCx<W>) {
            dispatch!(self, key, cx)
        }
    }

    impl Operator {
        #[inline]
        #[must_use]
        pub fn state(&self) -> OperatorState { EditorOperator::state(self) }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum OperatorState {
        Create,
    }

    impl OperatorState {
        #[must_use]
        pub const fn name(self) -> &'static str {
            match self {
                Self::Create => "create",
            }
        }
    }
}

pub(crate) use imp::EditorOperator;
pub use imp::{
    CurrentOperator, OpDispatch, OpYielded, Operator, OperatorCx, OperatorInner, OperatorResult,
    OperatorState,
};

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
                mut cx: crate::operators::OperatorCx<W>,
            ) {
                if self.0.is_some() { cx.abort() }

                self.0 = Some(<$inner as crate::operators::OperatorInner>::new(cx));
            }

            fn step<W: crate::GraphWidget + ?Sized>(
                &mut self,
                key: crate::bindings::Key,
                cx: crate::operators::OperatorCx<W>,
            ) {
                let Some(ref mut inner) = self.0 else { unreachable!() };

                let ret = <$inner as crate::operators::OperatorInner>::step(inner, key, cx);

                if !matches!(ret, crate::operators::OperatorResult::Continue) {
                    self.0 = None;
                }
            }
        }
    };
}

pub(crate) use operator;
