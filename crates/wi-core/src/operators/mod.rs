mod create;

pub mod all {
    pub use super::create::operators::*;
}

pub mod prelude {
    pub use std::{borrow::Cow, sync::Arc};

    pub use shibari::Acceptor;

    pub(crate) use super::{
        all::*, OpYielded, Operator, OperatorCx, OperatorInner, OperatorKind, OperatorState,
    };
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

    pub type OpDispatch<'a, 'c> = Dispatch<&'c mut OperatorResult, &'c mut CurrentOperator>;
    pub type OpYielded<'a, 'c, Y> = (OpDispatch<'a, 'c>, Y);

    pub struct OperatorCx<'a, 'c, 'w: 'a, W: GraphWidgetTypes + ?Sized> {
        widget: &'a mut W,
        driver: &'a mut DriverInner<W>,
        inner: W::Context<'a, 'w>,
        dispatch: OpDispatch<'a, 'c>,
    }

    impl<'a, 'c, 'w, W: GraphWidgetTypes + ?Sized> OperatorCx<'a, 'c, 'w, W> {
        pub const fn new(
            widget: &'a mut W,
            driver: &'a mut DriverInner<W>,
            inner: W::Context<'a, 'w>,
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
        pub fn into_yielded<Y, C>(
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
            action.process(
                count,
                ActionCx::new(self.widget, self.driver, W::reborrow_cx(&mut self.inner)),
            )
        }

        #[inline]
        pub fn abort(mut self, pending: Cow<'static, str>) {
            match self.dispatch {
                Dispatch::Immediate(ref mut r) => **r = OperatorResult::Abort,
                Dispatch::Deferred(ref mut c) => {
                    c.pop(&mut self.driver.stashed_operators);
                    self.driver.last_op.chord = pending;
                },
            }
        }

        #[inline]
        pub fn finish(mut self, pending: Cow<'static, str>) {
            match self.dispatch {
                Dispatch::Immediate(ref mut r) => **r = OperatorResult::Finish,
                Dispatch::Deferred(ref mut c) => {
                    c.pop(&mut self.driver.stashed_operators);
                    self.driver.last_op.chord = pending;
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

    pub(crate) trait OperatorState: Kind<OperatorKind> {
        fn pending_op(&self) -> Cow<'static, str>;
    }

    pub(crate) trait EditorOperator<W: GraphWidgetTypes + ?Sized>: OperatorState {
        fn init(&mut self, count: Option<NonZeroU32>, cx: OperatorCx<W>);

        fn step(&mut self, key: Key, cx: OperatorCx<W>);
    }

    pub trait OperatorInner<W: GraphWidgetTypes + ?Sized>: Sized {
        fn new(count: Option<NonZeroU32>, cx: OperatorCx<W>) -> Option<Self>;

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
    impl OperatorState for Operator {
        fn pending_op(&self) -> Cow<'static, str> { dispatch!(self) }
    }

    #[impl_enum]
    impl<W: GraphWidget + ?Sized> EditorOperator<W> for Operator {
        fn init(&mut self, count: Option<NonZeroU32>, cx: OperatorCx<W>) {
            dispatch!(self, count, cx)
        }

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

pub use imp::{
    CurrentOperator, OpYielded, Operator, OperatorCx, OperatorInner, OperatorKind, OperatorResult,
};
pub(crate) use imp::{EditorOperator, OperatorState};

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

        impl crate::operators::OperatorState for $op {
            fn pending_op(&self) -> ::std::borrow::Cow<'static, str> {
                if let Some(ref inner) = self.0 {
                    <$inner as crate::operators::OperatorState>::pending_op(inner)
                } else {
                    ::std::borrow::Cow::Borrowed("")
                }
            }
        }

        impl<W: crate::traits::GraphWidget + ?Sized> crate::operators::EditorOperator<W> for $op {
            fn init(
                &mut self,
                count: Option<::core::num::NonZeroU32>,
                cx: crate::operators::OperatorCx<W>,
            ) {
                if let Some(ref inner) = self.0 {
                    cx.abort(
                        <$inner as crate::operators::OperatorState>::pending_op(inner)
                    );
                    return;
                }

                self.0 = <$inner as crate::operators::OperatorInner<W>>::new(count, cx);
            }

            fn step(&mut self, key: crate::bindings::Key, cx: crate::operators::OperatorCx<W>) {
                let Some(ref mut inner) = self.0 else { unreachable!() };

                <$inner as crate::operators::OperatorInner<W>>::step(inner, key, cx);
            }
        }
    };
}

pub(crate) use operator;
