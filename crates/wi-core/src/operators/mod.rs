mod add;

pub mod all {
    pub use super::add::operators::*;
}

pub mod prelude {
    pub use super::{all::*, Operator, OperatorCx, OperatorInner, OperatorResult};
    pub use crate::{
        actions::prelude::*,
        bindings::Key,
        continuation::{ContinueCx, ContinueOnceImpl, Yielded},
    };
}

mod imp {
    use enum_dispatch::enum_dispatch;

    use super::prelude::*;
    use crate::GraphWidgetDriver;

    pub struct OperatorCx<'a, 'w, W: GraphWidget + ?Sized> {
        pub widget: &'a mut W,
        pub driver: &'a mut GraphWidgetDriver<W>,
        inner: &'a mut W::Context<'w>,
    }

    impl<'a, 'w, W: GraphWidget + ?Sized> OperatorCx<'a, 'w, W> {
        pub const fn new(
            widget: &'a mut W,
            driver: &'a mut GraphWidgetDriver<W>,
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
        fn name(&self) -> Cow<'static, str>;

        fn init<W: GraphWidget + ?Sized>(&mut self, cx: OperatorCx<W>) -> bool;

        fn step<W: GraphWidget + ?Sized>(&mut self, key: Key, cx: OperatorCx<W>) -> OperatorResult;
    }

    pub trait OperatorInner: Sized {
        const DEFAULT_NAME: &'static str;

        fn new<W: GraphWidget + ?Sized>(cx: OperatorCx<W>) -> Option<Self>;

        #[inline]
        fn name(&self) -> Option<Cow<'static, str>> { None }

        fn step<W: GraphWidget + ?Sized>(&mut self, key: Key, cx: OperatorCx<W>) -> OperatorResult;
    }

    #[enum_dispatch(EditorOperator)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Operator {
        Add,
    }

    impl Operator {
        #[inline]
        #[must_use]
        pub fn name(self) -> Cow<'static, str> { EditorOperator::name(&self) }
    }
}

pub use imp::{EditorOperator, Operator, OperatorCx, OperatorInner, OperatorResult};

macro_rules! operator {
    ($(#$attr:tt)* $vis:vis $op:ident => $inner_vis:vis $inner:ident) => {
        $(#$attr)*
        $vis struct $op(Option<$inner>);

        $(#$attr)*
        $inner_vis struct $inner;

        impl crate::operators::EditorOperator for $op {
            fn name(&self) -> ::std::borrow::Cow<'static, str> {
                self.0.as_ref().and_then(
                    <$inner as crate::operators::OperatorInner>::name
                ).unwrap_or_else(|| {
                    <$inner as crate::operators::OperatorInner>::DEFAULT_NAME.into()
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
