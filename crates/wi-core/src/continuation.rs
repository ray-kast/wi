use crate::{
    operators::{CurrentOperator, OpYielded},
    traits::{GraphWidgetTypes, UiOps},
    DriverInner, GraphWidgetDriver,
};

#[derive(Debug)]
pub enum Dispatch<I, D> {
    Immediate(I),
    Deferred(D),
}

type ContinueDispatch<'c, W, Y> = Dispatch<Y, &'c mut CurrentOperator<W>>;

#[derive_where::derive_where(Debug;
    W, W::Context<'a, 'w>, W::NodeId, W::PortId, W::Cell, W::Point, W::NodeKind, Y)]
pub struct ContinueCx<'a, 'c, 'w: 'a, W: GraphWidgetTypes + ?Sized, Y> {
    widget: &'a mut W,
    driver: &'a mut DriverInner<W>,
    dispatch: ContinueDispatch<'c, W, Y>,
    inner: W::Context<'a, 'w>,
}

impl<'a: 'c, 'c, 'w, W: GraphWidgetTypes + ?Sized, Y> ContinueCx<'a, 'c, 'w, W, Y> {
    pub const fn new_deferred(
        widget: &'a mut W,
        driver: &'a mut GraphWidgetDriver<W>,
        inner: W::Context<'a, 'w>,
    ) -> Self {
        Self {
            widget,
            driver: &mut driver.inner,
            dispatch: Dispatch::Deferred(&mut driver.current_operator),
            inner,
        }
    }
}

impl<'a, 'c, 'w, W: UiOps + ?Sized, Y> ContinueCx<'a, 'c, 'w, W, OpYielded<'a, 'c, W, Y>> {
    pub(crate) fn run_op<
        F: FnOnce(Option<Y>, crate::operators::OperatorCx<'_, '_, 'w, W>) -> T,
        T,
    >(
        self,
        f: F,
    ) -> T {
        match self.dispatch {
            Dispatch::Immediate((dispatch, caller)) => f(
                Some(caller),
                crate::operators::OperatorCx::new(self.widget, self.driver, self.inner, dispatch),
            ),
            Dispatch::Deferred(dispatch) => self.driver.mutate_check(
                dispatch,
                self.widget,
                self.inner,
                |driver, dispatch, widget, inner| {
                    f(
                        None,
                        crate::operators::OperatorCx::new(
                            widget,
                            driver,
                            inner,
                            Dispatch::Deferred(dispatch),
                        ),
                    )
                },
            ),
        }
    }
}

pub trait ContinueOnce<W: GraphWidgetTypes + ?Sized, Y, T>: Send + Sync + 'static {
    fn continue_once(self, value: T, cx: ContinueCx<W, Y>);
}

#[expect(
    missing_debug_implementations,
    reason = "This is usually going to be paramaterized"
)]
pub struct Yielded<'a, 'w: 'a, W: GraphWidgetTypes + ?Sized, Y, C> {
    driver: &'a mut DriverInner<W>,
    cx: W::Context<'a, 'w>,
    caller: Y,
    then: C,
}

impl<'a, 'w, W: GraphWidgetTypes + ?Sized, Y, C> Yielded<'a, 'w, W, Y, C> {
    pub(crate) const fn new(
        then: C,
        driver: &'a mut DriverInner<W>,
        caller: Y,
        cx: W::Context<'a, 'w>,
    ) -> Self {
        Self {
            driver,
            cx,
            caller,
            then,
        }
    }

    #[inline]
    pub fn cx(&mut self) -> W::Context<'_, 'w> { W::reborrow_cx(&mut self.cx) }

    #[inline]
    pub fn resume_now<T>(mut self, widget: &'a mut W, value: T) -> W::Context<'a, 'w>
    where C: ContinueOnce<W, Y, T> {
        self.then.continue_once(value, ContinueCx {
            widget,
            driver: self.driver,
            dispatch: Dispatch::Immediate(self.caller),
            inner: W::reborrow_cx(&mut self.cx),
        });

        self.cx
    }

    #[inline]
    pub fn defer(self) -> (C, W::Context<'a, 'w>) { (self.then, self.cx) }
}
