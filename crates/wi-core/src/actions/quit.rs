use super::prelude::*;

pub(super) mod actions {
    use crate::ActionKind;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    #[kind(ActionKind)]
    pub struct Quit;
}

impl<W: UiOps + ?Sized> EditorAction<W> for actions::Quit {
    fn process(&self, count: Option<NonZeroU32>, mut cx: ActionCx<W>) -> bool {
        let None = count else { return false };

        cx.run(|w, _, c| w.quit(c));

        true
    }
}
