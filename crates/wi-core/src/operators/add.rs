pub use super::prelude::*;

pub(super) mod operators {
    use crate::operators::operator;

    operator!(
        #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        pub Add => pub(super) AddInner
    );
}

impl OperatorInner for operators::AddInner {
    const DEFAULT_NAME: &'static str = "add";

    fn new<W: GraphWidget + ?Sized>(cx: OperatorCx<W>) -> Option<Self> { Some(Self) }

    fn step<W: GraphWidget + ?Sized>(&mut self, key: Key, cx: OperatorCx<W>) -> OperatorResult {
        let (widget, then) = cx.into_yielded(AddWithType);
        widget.prompt_node_kind(then);

        OperatorResult::Abort
    }
}

struct AddWithType;

impl<W: GraphWidget + ?Sized> ContinueOnceImpl<W, Option<W::NodeKind>> for AddWithType {
    fn continue_once(self, value: Option<W::NodeKind>, cx: ContinueCx<W>) { todo!() }
}
