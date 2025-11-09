mod create;

pub mod all {
    pub use super::create::operators::*;
}

pub mod prelude {
    pub use std::sync::Arc;

    pub use shibari::Acceptor;

    pub use super::{all::*, OpYielded, Operator, OperatorCx, OperatorInner, OperatorKind};
    pub use crate::{
        actions::prelude::*,
        bindings::*,
        continuation::{ContinueCx, ContinueOnce, Dispatch, Yielded},
    };
}

mod imp {
    use std::{hash::Hash, mem};

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

    pub(crate) trait EditorOperator<W: GraphWidget + ?Sized>: Kind<OperatorKind> {
        fn init(&mut self, cx: OperatorCx<W>);

        fn step(&mut self, key: Key, cx: OperatorCx<W>);
    }

    pub trait OperatorInner<W: GraphWidget + ?Sized>: Sized {
        fn new(cx: OperatorCx<W>) -> Self;

        fn step(&mut self, key: Key, cx: OperatorCx<W>);
    }

    #[impl_enum]
    #[derive(Debug, PartialEq)]
    pub enum Operator {
        Create(Create),
    }

    #[impl_enum]
    impl Kind<OperatorKind> for Operator {
        fn kind(&self) -> OperatorKind { dispatch!(self) }
    }

    #[impl_enum]
    impl<W: GraphWidget + ?Sized> EditorOperator<W> for Operator {
        fn init(&mut self, cx: OperatorCx<W>) { dispatch!(self, cx) }

        fn step(&mut self, key: Key, cx: OperatorCx<W>) { dispatch!(self, key, cx) }
    }

    impl Operator {
        #[inline]
        #[must_use]
        pub fn kind(&self) -> OperatorKind { Kind::kind(self) }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum OperatorKind {
        Create,
    }

    impl OperatorKind {
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
    CurrentOperator, OpYielded, Operator, OperatorCx, OperatorInner, OperatorKind, OperatorResult,
};

macro_rules! operator {
    (
        #[uninit_kind = $uninit:ident]
        $(#$attr:tt)*
        $vis:vis struct $op:ident($inner_vis:vis $inner:ty);
    ) => {
        $(#$attr)*
        $vis struct $op($inner_vis Option<$inner>);

        impl crate::actions::Kind<crate::operators::OperatorKind> for $op {
            fn kind(&self) -> crate::operators::OperatorKind {
                if let Some(ref inner) = self.0 {
                    <$inner as crate::actions::Kind<crate::operators::OperatorKind>>::kind(
                        inner
                    )
                } else {
                    #[allow(unused_imports)]
                    use crate::operators::{OperatorKind, OperatorKind::*};
                    $uninit
                }
            }
        }

        impl<W: crate::GraphWidget + ?Sized> crate::operators::EditorOperator<W> for $op {
            fn init(&mut self, mut cx: crate::operators::OperatorCx<W>) {
                if self.0.is_some() { cx.abort() }

                self.0 = Some(<$inner as crate::operators::OperatorInner<W>>::new(cx));
            }

            fn step(&mut self, key: crate::bindings::Key, cx: crate::operators::OperatorCx<W>) {
                let Some(ref mut inner) = self.0 else { unreachable!() };

                <$inner as crate::operators::OperatorInner<W>>::step(inner, key, cx);
            }
        }
    };
}

pub(crate) use operator;
