use super::prelude::*;

pub(super) mod actions {
    use crate::{actions::MotionKind, Side};

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, wi_macros::Kind)]
    pub struct JumpToPort(
        #[kind(
            Some(Side::In) => MotionKind::JumpToInput,
            Some(Side::Out) => MotionKind::JumpToOutput,
            None => MotionKind::JumpToPort,
        )]
        pub Option<Side>,
    );
}

impl<W: GraphWidgetTypes + ?Sized, S: Selection> EditorMotion<W, S> for actions::JumpToPort {
    fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>, selection: S) -> bool {
        let None = count else { return false };
        let Self(side) = self;

        true
    }
}
