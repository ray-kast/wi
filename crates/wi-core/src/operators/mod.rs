mod create;
mod jump;

pub mod all {
    pub use super::{create::operators::*, jump::operators::*};
}

pub mod prelude {
    pub use std::{borrow::Cow, sync::Arc};

    pub use shibari::Acceptor;

    pub(crate) use super::{
        all::*, EditorOperator, OpYielded, Operator, OperatorCx, OperatorKind, OperatorState,
        StartCx, StartError, StartOperator, Started,
    };
    pub use crate::{
        actions::prelude::*,
        bindings::*,
        continuation::{ContinueCx, ContinueOnce, Dispatch, Yielded},
    };
}

mod imp {
    use std::{fmt, hash::Hash, mem};

    use wi_macros::impl_enum;

    use super::prelude::*;
    use crate::{actions::Action, DriverInner};

    #[derive_where(Debug; W::NodeId, W::PortId)]
    #[derive_where(Default; )]
    pub(crate) struct CurrentOperator<W: GraphWidgetTypes + ?Sized>(
        Option<(Cow<'static, str>, Started<W>)>,
    );

    impl<W: GraphWidgetTypes + ?Sized> CurrentOperator<W> {
        #[inline]
        pub fn status(&self) -> Option<(&Cow<'static, str>, &Started<W>)> {
            self.0.as_ref().map(|(c, o)| (c, o))
        }

        #[inline]
        pub fn as_ref(&self) -> Option<&Started<W>> { self.0.as_ref().map(|(_, o)| o) }

        #[inline]
        pub fn as_mut(&mut self) -> Option<&mut Started<W>> { self.0.as_mut().map(|(_, o)| o) }

        pub fn push(
            &mut self,
            operator: Started<W>,
            chord: Cow<'static, str>,
            stashed: &mut Vec<(Cow<'static, str>, Started<W>)>,
        ) -> (&Cow<'static, str>, &mut Started<W>) {
            match &mut self.0 {
                o @ None => {
                    debug_assert!(stashed.is_empty());
                    *o = Some((chord, operator));
                    let (chord, operator) = o.as_mut().unwrap_or_else(|| unreachable!());
                    (chord, operator)
                },
                Some(o) => {
                    stashed.push(mem::replace(o, (chord, operator)));
                    let (chord, operator) = o;
                    (chord, operator)
                },
            }
        }

        pub fn pop(
            &mut self,
            stashed: &mut Vec<(Cow<'static, str>, Started<W>)>,
        ) -> Option<(Cow<'static, str>, Started<W>)> {
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

    pub type OpDispatch<'a, 'c, W> = Dispatch<&'c mut OperatorFlow, &'c mut CurrentOperator<W>>;
    pub type OpYielded<'a, 'c, W, Y> = (OpDispatch<'a, 'c, W>, Y);

    pub struct OperatorCx<'a, 'c, 'w: 'a, W: GraphWidgetTypes + ?Sized> {
        widget: &'a mut W,
        driver: &'a mut DriverInner<W>,
        inner: W::Context<'a, 'w>,
        dispatch: OpDispatch<'a, 'c, W>,
    }

    impl<'a, 'c, 'w, W: GraphWidgetTypes + ?Sized> OperatorCx<'a, 'c, 'w, W> {
        pub const fn new(
            widget: &'a mut W,
            driver: &'a mut DriverInner<W>,
            inner: W::Context<'a, 'w>,
            dispatch: OpDispatch<'a, 'c, W>,
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
        pub const fn dispatch(&self) -> &OpDispatch<'a, 'c, W> { &self.dispatch }

        #[inline]
        pub fn into_yielded<Y, C>(
            self,
            then: C,
            yielded: Y,
        ) -> (&'a mut W, Yielded<'a, 'w, W, OpYielded<'a, 'c, W, Y>, C>) {
            (
                self.widget,
                Yielded::new(then, self.driver, (self.dispatch, yielded), self.inner),
            )
        }

        pub(crate) fn run_action<A: EditorAction<W> + Into<Action<W>>>(
            &mut self,
            action: A,
            count: Option<NonZeroU32>,
        ) -> bool {
            let loud = !action.kind().is_silent();
            let handled = action.process(
                count,
                ActionCx::new(self.widget, self.driver, W::reborrow_cx(&mut self.inner)),
            );

            if handled && loud {
                self.driver.last_action.replace(action.into());
            }

            handled
        }

        #[inline]
        pub fn abort(mut self, pending: Cow<'static, str>) {
            self.driver.last_op.chord = pending;
            match self.dispatch {
                Dispatch::Immediate(ref mut r) => **r = OperatorFlow::Abort,
                Dispatch::Deferred(ref mut c) => {
                    c.pop(&mut self.driver.stashed_operators);
                },
            }
        }

        #[inline]
        pub fn finish(mut self, pending: Cow<'static, str>) {
            self.driver.last_op.chord = pending;
            match self.dispatch {
                Dispatch::Immediate(ref mut r) => **r = OperatorFlow::Finish,
                Dispatch::Deferred(ref mut c) => {
                    c.pop(&mut self.driver.stashed_operators);
                },
            }
        }
    }

    pub struct StartCx<'a, 'c, 'w, W: GraphWidgetTypes + ?Sized>(OperatorCx<'a, 'c, 'w, W>);

    impl<'a, 'c, 'w, W: GraphWidgetTypes + ?Sized> StartCx<'a, 'c, 'w, W> {
        #[inline]
        pub const fn new(
            widget: &'a mut W,
            driver: &'a mut DriverInner<W>,
            inner: W::Context<'a, 'w>,
            dispatch: OpDispatch<'a, 'c, W>,
        ) -> Self {
            Self(OperatorCx::new(widget, driver, inner, dispatch))
        }

        #[inline]
        pub fn into_started(self) -> OperatorCx<'a, 'c, 'w, W> { self.0 }

        #[inline]
        pub fn into_aborted(self) -> StartError {
            let _ = self;
            StartError(())
        }
    }

    #[derive(Debug)]
    pub struct StartError(());

    impl fmt::Display for StartError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("Operator did not start")
        }
    }

    impl std::error::Error for StartError {}

    #[derive(Debug, Clone, Copy)]
    pub enum OperatorFlow {
        Continue,
        Finish,
        Abort,
    }

    pub(crate) trait StartOperator<W: GraphWidgetTypes + ?Sized>: Sized {
        type Started;

        fn start_op(
            &self,
            count: Option<NonZeroU32>,
            cx: StartCx<W>,
        ) -> Result<Self::Started, StartError>;
    }

    pub(crate) trait OperatorState: Kind<OperatorKind> {
        fn pending_op(&self) -> Cow<'static, str>;
    }

    pub(crate) trait EditorOperator<W: GraphWidgetTypes + ?Sized>: OperatorState {
        fn step(&mut self, key: Key, cx: OperatorCx<W>);
    }

    #[impl_enum]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum Operator {
        // From create
        Create(Create),

        // From jump
        JumpToPort(JumpToPort),
    }

    #[impl_enum]
    #[derive_where(Debug; W::NodeId, W::PortId)]
    pub(crate) enum Started<W: GraphWidgetTypes + ?Sized> {
        Create(super::create::CreateStarted<W>),
        Jump(super::jump::JumpToPortStarted<W>),
    }

    #[impl_enum]
    impl<W: GraphWidget + ?Sized> StartOperator<W> for Operator {
        type Started = Started<W>;

        fn start_op(
            &self,
            count: Option<NonZeroU32>,
            cx: StartCx<W>,
        ) -> Result<Self::Started, StartError> {
            dispatch_map!(|s| s.map(Into::into); self, count, cx)
        }
    }

    #[impl_enum]
    impl<W: GraphWidgetTypes + ?Sized> Kind<OperatorKind> for Started<W> {
        fn kind(&self) -> OperatorKind { dispatch!(self) }
    }

    #[impl_enum]
    impl<W: GraphWidgetTypes + ?Sized> OperatorState for Started<W> {
        fn pending_op(&self) -> Cow<'static, str> { dispatch!(self) }
    }

    #[impl_enum]
    impl<W: GraphWidget + ?Sized> EditorOperator<W> for Started<W> {
        fn step(&mut self, key: Key, cx: OperatorCx<W>) { dispatch!(self, key, cx) }
    }

    impl<W: GraphWidgetTypes + ?Sized> Started<W> {
        #[inline]
        #[must_use]
        pub fn kind(&self) -> OperatorKind { Kind::kind(self) }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum OperatorKind {
        Create,
        JumpToPort,
    }

    impl OperatorKind {
        #[must_use]
        pub const fn name(self) -> &'static str {
            match self {
                Self::Create => "create",
                Self::JumpToPort => "jump to port",
            }
        }
    }
}

pub(crate) use imp::{CurrentOperator, EditorOperator, OperatorState, StartOperator, Started};
pub use imp::{OpYielded, Operator, OperatorCx, OperatorFlow, OperatorKind, StartCx, StartError};
