use std::fmt;

use crate::{operators::Operator, DriverInner, GraphWidget, GraphWidgetDriver};

#[derive(Debug)]
pub enum Dispatch<I, D> {
    Immediate(I),
    Deferred(D),
}

type ContinueDispatch<'a> = Dispatch<(), Option<&'a mut Operator>>;

pub struct ContinueCx<'a, 'w, W: GraphWidget + ?Sized> {
    pub(crate) widget: &'a mut W,
    pub(crate) driver: &'a mut DriverInner<W>,
    pub(crate) dispatch: ContinueDispatch<'a>,
    inner: &'a mut W::Context<'w>,
}

impl<W: fmt::Debug + GraphWidget + ?Sized> fmt::Debug for ContinueCx<'_, '_, W>
where
    W::NodeId: fmt::Debug,
    W::PortId: fmt::Debug,
    W::Cell: fmt::Debug,
    W::Point: fmt::Debug,
    for<'a> W::Context<'a>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            widget,
            driver,
            dispatch,
            inner,
        } = self;

        f.debug_struct("ContinueCx")
            .field("widget", widget)
            .field("driver", driver)
            .field("dispatch", dispatch)
            .field("inner", inner)
            .finish()
    }
}

impl<'a, 'w, W: GraphWidget + ?Sized> ContinueCx<'a, 'w, W> {
    pub const fn new(
        widget: &'a mut W,
        driver: &'a mut GraphWidgetDriver<W>,
        inner: &'a mut W::Context<'w>,
    ) -> Self {
        Self {
            widget,
            driver: &mut driver.inner,
            dispatch: Dispatch::Deferred(driver.current_operator.as_mut()),
            inner,
        }
    }

    pub(crate) const fn into_op_cx(
        self,
    ) -> (
        ContinueDispatch<'a>,
        crate::operators::OperatorCx<'a, 'w, W>,
    ) {
        (
            self.dispatch,
            crate::operators::OperatorCx::new(self.widget, self.driver, self.inner),
        )
    }
}

pub trait ContinueOnce<W: GraphWidget + ?Sized, T> {
    fn continue_once(self, value: T, cx: ContinueCx<W>);
}

#[expect(
    missing_debug_implementations,
    reason = "This is usually going to be paramaterized"
)]
pub struct Yielded<'a, 'w, W: GraphWidget + ?Sized, C> {
    driver: &'a mut DriverInner<W>,
    cx: &'a mut W::Context<'w>,
    then: C,
}

impl<'a, 'w, W: GraphWidget + ?Sized, C> Yielded<'a, 'w, W, C> {
    pub(crate) const fn new(
        then: C,
        driver: &'a mut DriverInner<W>,
        cx: &'a mut W::Context<'w>,
    ) -> Self {
        Self { driver, cx, then }
    }

    #[inline]
    pub fn resume_now<T>(self, widget: &'a mut W, value: T)
    where C: ContinueOnce<W, T> {
        self.then.continue_once(value, ContinueCx {
            widget,
            driver: self.driver,
            dispatch: Dispatch::Immediate(()),
            inner: self.cx,
        });
    }

    #[inline]
    pub fn defer(self) -> C { self.then }
}
