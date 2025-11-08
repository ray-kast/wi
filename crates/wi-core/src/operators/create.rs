pub use super::prelude::*;
use crate::GraphWidgetCell;

pub(super) mod operators {
    use crate::operators::operator;

    operator!(
        #[derive(Debug, Default, PartialEq)]
        pub struct Create(pub(super) super::CreateInner);
    );
}

#[derive(Debug, PartialEq)]
struct Shared;

#[derive(Debug, PartialEq)]
enum CreateInner {
    Init(CreateOpAccept),
    Yielded(Arc<Shared>),
}

impl OperatorInner for CreateInner {
    const DEFAULT_STATE: OperatorState = OperatorState::Create;

    fn new<W: GraphWidget + ?Sized>(cx: OperatorCx<W>) -> Self {
        Self::Init(CreateOpAccept::default())
    }

    fn step<W: GraphWidget + ?Sized>(&mut self, key: Key, cx: OperatorCx<W>) -> OperatorResult {
        match self {
            Self::Init(a) => {
                let action = match a.accept(key) {
                    CreateOut::Trap => return OperatorResult::Abort,
                    CreateOut::Advance => return OperatorResult::Continue,
                    CreateOut::CreateOpAction(a) => a,
                };

                match action {
                    CreateOpAction::Accept => {
                        let shared = Arc::new(Shared);

                        *self = Self::Yielded(Arc::clone(&shared));
                        let (widget, then) = cx.into_yielded(CreateWithType(shared), self);

                        widget.prompt_node_kind(then);
                        OperatorResult::Continue
                    },
                }
            },
            Self::Yielded(_) => OperatorResult::Continue,
        }
    }
}

struct CreateWithType(Arc<Shared>);

impl<W: GraphWidget + ?Sized>
    ContinueOnce<W, OpYielded<'_, '_, &'_ mut CreateInner>, Option<W::NodeKind>>
    for CreateWithType
{
    fn continue_once(
        self,
        kind: Option<W::NodeKind>,
        cx: ContinueCx<W, OpYielded<&mut CreateInner>>,
    ) {
        let (inner, mut cx) = cx.into_op_cx();

        let inner = match (inner, cx.dispatch()) {
            (Some(i), Dispatch::Immediate(_)) => Some(&*i),
            (None, Dispatch::Deferred(o)) => o.as_ref().and_then(|o| {
                if let Operator::Create(operators::Create(Some(i))) = o {
                    Some(i)
                } else {
                    None
                }
            }),
            _ => unreachable!(),
        };

        if inner.is_none_or(|i| {
            if let CreateInner::Yielded(s) = i {
                !Arc::ptr_eq(s, &self.0)
            } else {
                true
            }
        }) {
            panic!("Continued Add operator while it was not active");
        }

        let Some(value) = kind else { return };
        let pos = cx.driver().cell.position(cx.widget());

        cx.run_action(CreateNode(value, pos), None);
        cx.finish();
    }
}
