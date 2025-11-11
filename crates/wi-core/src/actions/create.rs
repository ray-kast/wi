use super::prelude::*;

pub(super) mod actions {
    use crate::{actions::ActionKind, traits::GraphWidgetTypes};

    #[derive_where::derive_where(Debug, Clone, Copy, PartialEq, Eq, Hash; W::NodeKind, W::Point)]
    #[derive(wi_macros::Kind)]
    #[kind(ActionKind)]
    pub struct CreateNode<W: GraphWidgetTypes + ?Sized>(pub W::NodeKind, pub W::Point);
}

impl<W: NodeOps + ?Sized> EditorAction<W> for actions::CreateNode<W> {
    fn process(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let None = count else { return false };
        let Self(kind, pos) = self;

        cx.widget.create_node(kind, pos, cx.inner);

        true
    }
}
