use crate::{
    operators::{CurrentOperator, OpYielded},
    DriverInner, GraphWidget, GraphWidgetDriver,
};

#[derive(Debug)]
pub enum Dispatch<I, D> {
    Immediate(I),
    Deferred(D),
}

type ContinueDispatch<'a, Y> = Dispatch<Y, &'a mut CurrentOperator>;

#[derive_where::derive_where(Debug;
    W, W::Context<'w>, W::NodeId, W::PortId, W::Cell, W::Point, W::NodeKind, Y)]
pub struct ContinueCx<'a, 'w, W: GraphWidget + ?Sized, Y> {
    widget: &'a mut W,
    driver: &'a mut DriverInner<W>,
    dispatch: ContinueDispatch<'a, Y>,
    inner: &'a mut W::Context<'w>,
}

impl<'a, 'w, W: GraphWidget + ?Sized, Y> ContinueCx<'a, 'w, W, Y> {
    pub const fn new_deferred(
        widget: &'a mut W,
        driver: &'a mut GraphWidgetDriver<W>,
        inner: &'a mut W::Context<'w>,
    ) -> Self {
        Self {
            widget,
            driver: &mut driver.inner,
            dispatch: Dispatch::Deferred(&mut driver.current_operator),
            inner,
        }
    }
}

impl<'a, 'c, 'w, W: GraphWidget + ?Sized, Y> ContinueCx<'a, 'w, W, OpYielded<'a, 'c, Y>> {
    pub(crate) fn into_op_cx(self) -> (Option<Y>, crate::operators::OperatorCx<'a, 'c, 'w, W>) {
        let (caller, dispatch) = match self.dispatch {
            Dispatch::Immediate((d, y)) => (Some(y), d),
            Dispatch::Deferred(c) => (None, Dispatch::Deferred(c)),
        };

        (
            caller,
            crate::operators::OperatorCx::new(self.widget, self.driver, self.inner, dispatch),
        )
    }
}

pub trait ContinueOnce<W: GraphWidget + ?Sized, Y, T> {
    fn continue_once(self, value: T, cx: ContinueCx<W, Y>);
}

#[expect(
    missing_debug_implementations,
    reason = "This is usually going to be paramaterized"
)]
pub struct Yielded<'a, 'w, W: GraphWidget + ?Sized, Y, C> {
    driver: &'a mut DriverInner<W>,
    cx: &'a mut W::Context<'w>,
    caller: Y,
    then: C,
}

impl<'a, 'w, W: GraphWidget + ?Sized, Y, C> Yielded<'a, 'w, W, Y, C> {
    pub(crate) const fn new(
        then: C,
        driver: &'a mut DriverInner<W>,
        caller: Y,
        cx: &'a mut W::Context<'w>,
    ) -> Self {
        Self {
            driver,
            cx,
            caller,
            then,
        }
    }

    #[inline]
    pub fn resume_now<T>(self, widget: &'a mut W, value: T)
    where C: ContinueOnce<W, Y, T> {
        self.then.continue_once(value, ContinueCx {
            widget,
            driver: self.driver,
            dispatch: Dispatch::Immediate(self.caller),
            inner: self.cx,
        });
    }

    #[inline]
    pub fn defer(self) -> C { self.then }
}
