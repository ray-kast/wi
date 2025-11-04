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
        OperatorResult::Abort
    }
}
