use super::prelude::*;

pub(super) mod actions {
    use super::super::prelude::*;
    use crate::WPort;

    #[derive_where(Debug, Clone, Copy, PartialEq, Eq, Hash;
        W::NodeId, W::PortId, W::NodeKind, W::Point)]
    #[derive(Kind)]
    #[kind(ActionKind)]
    pub struct CreateEdge<W: GraphWidgetTypes + ?Sized>(pub WPort<W>, pub WPort<W>);

    #[derive_where(Debug, Clone, Copy, PartialEq, Eq, Hash; W::NodeKind, W::Point)]
    #[derive(Kind)]
    #[kind(ActionKind)]
    pub struct CreateNode<W: GraphWidgetTypes + ?Sized>(pub W::NodeKind, pub W::Point);
}

impl<W: EdgeOps + ?Sized> EditorAction<W> for actions::CreateEdge<W> {
    fn process(&self, count: Option<NonZeroU32>, mut cx: ActionCx<W>) -> bool {
        let None = count else { return false };
        let Self(from, to) = *self;

        cx.run(|w, _, c| w.create_edge(from, to, c));

        true
    }
}

impl<W: NodeOps + ?Sized> EditorAction<W> for actions::CreateNode<W> {
    fn process(&self, count: Option<NonZeroU32>, mut cx: ActionCx<W>) -> bool {
        let None = count else { return false };
        let Self(ref kind, pos) = *self;

        cx.run(|w, _, c| w.create_node(kind.clone(), pos, c));

        true
    }
}
