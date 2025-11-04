use crate::{GraphWidget, GraphWidgetDriver};

#[expect(
    missing_debug_implementations,
    reason = "This is not publicly re-exported"
)]
pub struct ContinueCx<'a, 'w, W: GraphWidget + ?Sized> {
    pub(crate) widget: &'a mut W,
    pub(crate) driver: &'a mut GraphWidgetDriver<W>,
    inner: &'a mut W::Context<'w>,
}

impl<'a, 'w, W: GraphWidget + ?Sized> ContinueCx<'a, 'w, W> {
    pub(crate) const fn new(
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

    pub(crate) const fn into_op_cx(self) -> crate::operators::OperatorCx<'a, 'w, W> {
        crate::operators::OperatorCx::new(self.widget, self.driver, self.inner)
    }
}

pub trait ContinueOnceImpl<W: GraphWidget + ?Sized, T> {
    fn continue_once(self, value: T, cx: ContinueCx<W>);
}

pub trait ContinueOnce<W: GraphWidget + ?Sized, T> {
    fn finish(
        self,
        value: T,
        widget: &mut W,
        driver: &mut GraphWidgetDriver<W>,
        cx: &mut W::Context<'_>,
    );
}

impl<C: ContinueOnceImpl<W, T>, W: GraphWidget + ?Sized, T> ContinueOnce<W, T> for C {
    fn finish(
        self,
        value: T,
        widget: &mut W,
        driver: &mut GraphWidgetDriver<W>,
        cx: &mut W::Context<'_>,
    ) {
        ContinueOnceImpl::continue_once(self, value, ContinueCx {
            widget,
            driver,
            inner: cx,
        });
    }
}

#[expect(
    missing_debug_implementations,
    reason = "This is usually going to be paramaterized"
)]
pub struct Yielded<'a, 'w, W: GraphWidget + ?Sized, C> {
    driver: &'a mut GraphWidgetDriver<W>,
    cx: &'a mut W::Context<'w>,
    then: C,
}

impl<'a, 'w, W: GraphWidget + ?Sized, C> Yielded<'a, 'w, W, C> {
    pub(crate) const fn new(
        then: C,
        driver: &'a mut GraphWidgetDriver<W>,
        cx: &'a mut W::Context<'w>,
    ) -> Self {
        Self { driver, cx, then }
    }

    #[inline]
    pub fn resume_now<T>(self, widget: &'a mut W, value: T)
    where C: ContinueOnce<W, T> {
        self.then.finish(value, widget, self.driver, self.cx);
    }

    #[inline]
    pub fn defer(self) -> C { self.then }
}
