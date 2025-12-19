use super::prelude::*;
use crate::Cursor;

pub mod actions {
    use crate::{ActionKind, Step};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    pub struct Nudge(
        #[kind(
            Step::Left => ActionKind::NudgeLeft,
            Step::Down => ActionKind::NudgeDown,
            Step::Up => ActionKind::NudgeUp,
            Step::Right => ActionKind::NudgeRight,
        )]
        pub Step,
    );
}

impl<W: NodeOps + ?Sized> EditorAction<W> for actions::Nudge {
    fn process(&self, count: Option<NonZeroU32>, mut cx: ActionCx<W>) -> bool {
        let count = count.map_or(1, NonZero::get);
        let Self(step) = *self;

        cx.run(
            |w, d, c| match d.cursor.as_ref().unwrap_or_else(|| unreachable!()) {
                Cursor::Node(node) => w.nudge_node(node, step, count, c),
                _ => false,
            },
        )
    }
}
