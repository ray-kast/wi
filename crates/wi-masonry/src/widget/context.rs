use std::fmt;

use masonry::{
    core::{EventCtx, MutateCtx, Widget, WidgetMut, WidgetPod},
    kurbo::Size,
};

pub trait Context {
    fn size(&self) -> Size;

    fn request_render(&mut self);

    fn exit(&mut self);

    fn submit_action<A: fmt::Debug + Send + Sync + 'static>(&mut self, action: A);

    fn mutate_later<W: Widget>(
        &mut self,
        child: &mut WidgetPod<W>,
        f: impl FnOnce(WidgetMut<W>) + Send + 'static,
    );
}

macro_rules! impl_trivial {
    ($ty:ty { $($params:tt)* } => $impl_ty:ty) => {
        impl<$($params)*> Context for $ty {
            #[inline]
            fn size(&self) -> Size { <$impl_ty>::size(self) }

            #[inline]
            fn request_render(&mut self) { <$impl_ty>::request_render(self) }

            #[inline]
            fn exit(&mut self) { <$impl_ty>::exit(self) }

            #[inline]
            fn submit_action<A: fmt::Debug + Send + Sync + 'static>(&mut self, action: A) {
                <$impl_ty>::submit_action::<A>(self, action)
            }

            #[inline]
            fn mutate_later<W: Widget>(
                &mut self,
                child: &mut WidgetPod<W>,
                f: impl FnOnce(WidgetMut<W>) + Send + 'static,
            ) {
                <$impl_ty>::mutate_later(self, child, f)
            }
        }
    };
}

impl_trivial!(&mut C { C: Context } => C);
impl_trivial!(EventCtx<'_> {} => EventCtx);
impl_trivial!(MutateCtx<'_> {} => MutateCtx);

pub enum AnyContext<'a, 'w> {
    Event(&'a mut EventCtx<'w>),
    Mutate(&'a mut MutateCtx<'w>),
}

impl<'w> AnyContext<'_, 'w> {
    pub fn reborrow<'b>(&'b mut self) -> AnyContext<'b, 'w> {
        match self {
            Self::Event(c) => AnyContext::Event(c),
            Self::Mutate(c) => AnyContext::Mutate(c),
        }
    }
}

impl Context for AnyContext<'_, '_> {
    fn size(&self) -> Size {
        match self {
            Self::Event(c) => c.size(),
            Self::Mutate(c) => c.size(),
        }
    }

    fn request_render(&mut self) {
        match self {
            Self::Event(c) => c.request_render(),
            Self::Mutate(c) => c.request_render(),
        }
    }

    fn exit(&mut self) {
        match self {
            Self::Event(c) => c.exit(),
            Self::Mutate(c) => c.exit(),
        }
    }

    fn submit_action<A: fmt::Debug + Send + Sync + 'static>(&mut self, action: A) {
        match self {
            Self::Event(c) => c.submit_action(action),
            Self::Mutate(c) => c.submit_action(action),
        }
    }

    fn mutate_later<W: Widget>(
        &mut self,
        child: &mut WidgetPod<W>,
        f: impl FnOnce(WidgetMut<W>) + Send + 'static,
    ) {
        match self {
            Self::Event(c) => c.mutate_later(child, f),
            Self::Mutate(c) => c.mutate_later(child, f),
        }
    }
}

impl<'a, 'w> From<&'a mut EventCtx<'w>> for AnyContext<'a, 'w> {
    #[inline]
    fn from(value: &'a mut EventCtx<'w>) -> Self { Self::Event(value) }
}

impl<'a, 'w> From<&'a mut MutateCtx<'w>> for AnyContext<'a, 'w> {
    #[inline]
    fn from(value: &'a mut MutateCtx<'w>) -> Self { Self::Mutate(value) }
}
