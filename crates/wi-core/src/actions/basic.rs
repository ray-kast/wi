use super::prelude::*;

pub mod actions {
    use crate::{actions::ActionKind, mode::ModeKind};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    #[kind(ActionKind)]
    pub struct PushCount(pub char);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    #[kind(ActionKind)]
    pub struct Repeat;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    pub struct SetMode(#[kind(ModeKind::Normal => ActionKind::ModeNormal)] pub ModeKind);

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    #[kind(ActionKind)]
    pub struct ToggleDebug;
}

impl<W: GraphWidgetTypes + ?Sized> EditorAction<W> for actions::PushCount {
    fn process(&self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let Self(digit) = *self;
        let digit = u32::from(digit) - u32::from('0');
        cx.driver.count = count
            .map_or(0, NonZero::get)
            .checked_mul(10)
            .and_then(|c| c.checked_add(digit))
            .and_then(NonZero::new);
        true
    }
}

impl<W: GraphWidget + ?Sized> EditorAction<W> for actions::Repeat {
    fn process(&self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let Some(action) = cx.driver.last_action.as_ref(true) else {
            return false;
        };

        action.clone().process(count, cx)
    }
}

impl<W: GraphWidgetTypes + ?Sized> EditorAction<W> for actions::SetMode {
    fn process(&self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let Self(m) = *self;
        let None = count else { return false };

        cx.driver.mode.change(m)
    }
}

impl<W: GraphWidgetTypes + ?Sized> EditorAction<W> for actions::ToggleDebug {
    fn process(&self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let None = count else { return false };

        cx.driver.debug = !cx.driver.debug;

        true
    }
}
