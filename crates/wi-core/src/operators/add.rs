pub use super::prelude::*;

pub(super) mod operators {
    use crate::operators::operator;

    operator!(
        #[derive(Debug, Default, PartialEq)]
        pub struct Add(pub(super) super::AddInner);
    );
}

#[derive(Debug, PartialEq)]
struct Shared;

#[derive(Debug, PartialEq)]
enum AddInner {
    Init(AddOpAccept),
    Yielded(Arc<Shared>),
}

impl OperatorInner for AddInner {
    const DEFAULT_STATE: OperatorState = OperatorState::Add;

    fn new<W: GraphWidget + ?Sized>(cx: OperatorCx<W>) -> Option<Self> {
        Some(Self::Init(AddOpAccept::default()))
    }

    fn step<W: GraphWidget + ?Sized>(&mut self, key: Key, cx: OperatorCx<W>) -> OperatorResult {
        match self {
            Self::Init(a) => {
                let action = match a.accept(key) {
                    AddOut::Trap => return OperatorResult::Abort,
                    AddOut::Advance => return OperatorResult::Continue,
                    AddOut::AddOpAction(a) => a,
                };

                match action {
                    AddOpAction::Accept => {
                        let shared = Arc::new(Shared);

                        let (widget, then) = cx.into_yielded(AddWithType(Arc::clone(&shared)));
                        *self = Self::Yielded(shared);

                        widget.prompt_node_kind(then);
                        OperatorResult::Continue
                    },
                }
            },
            Self::Yielded(_) => OperatorResult::Continue,
        }
    }
}

struct AddWithType(Arc<Shared>);

impl<W: GraphWidget + ?Sized> ContinueOnce<W, Option<W::NodeKind>> for AddWithType {
    fn continue_once(self, value: Option<W::NodeKind>, cx: ContinueCx<W>) {
        let (dispatch, cx) = cx.into_op_cx();

        match dispatch {
            Dispatch::Immediate(()) => (),
            Dispatch::Deferred(o) => {
                if o.is_none_or(|o| {
                    if let Operator::Add(operators::Add(Some(AddInner::Yielded(s)))) = o {
                        !Arc::ptr_eq(s, &self.0)
                    } else {
                        true
                    }
                }) {
                    panic!("Continued Add operator while it was not active");
                }
            },
        }

        tracing::warn!("hi");

        let Some(value) = value else { return };

        todo!();
    }
}
