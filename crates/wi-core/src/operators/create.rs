use wi_macros::Kind;

pub use super::prelude::*;
use crate::GraphWidgetCell;

pub(super) mod operators {
    use crate::operators::operator;

    operator!(
        #[uninit_kind = Create]
        #[derive(Debug, Default, PartialEq)]
        pub struct Create(pub(super) super::CreateInner);
    );
}

#[derive(Debug, PartialEq)]
struct Shared(&'static str);

#[derive(Debug, PartialEq, Kind)]
#[kind(const OperatorKind::Create)]
enum CreateInner {
    Init(CreateOpAccept),
    Yielded(Arc<Shared>),
}

impl OperatorState for CreateInner {
    fn pending_op(&self) -> Cow<'static, str> {
        match self {
            Self::Init(a) => a.pending_op().into(),
            Self::Yielded(s) => s.0.into(),
        }
    }
}

impl<W: NodeOps + UiOps + ?Sized> OperatorInner<W> for CreateInner {
    fn new(count: Option<NonZeroU32>, cx: OperatorCx<W>) -> Option<Self> {
        let accept = CreateOpAccept::default();

        let None = count else {
            cx.abort(accept.pending_op().into());
            return None;
        };

        Some(Self::Init(accept))
    }

    fn step(&mut self, key: Key, cx: OperatorCx<W>) {
        match self {
            Self::Init(a) => {
                let (pend, out) = a.accept(key);
                let action = match out {
                    CreateOut::Trap => {
                        cx.abort(pend.into());
                        return;
                    },
                    CreateOut::Advance => return,
                    CreateOut::CreateOpAction(a) => a,
                };

                match action {
                    CreateOpAction::Accept => {
                        let shared = Arc::new(Shared(pend));

                        *self = Self::Yielded(Arc::clone(&shared));
                        let (widget, then) = cx.into_yielded(CreateWithType(shared), self);

                        widget.prompt_node_kind(then);
                    },
                }
            },
            Self::Yielded(..) => (),
        }
    }
}

struct CreateWithType(Arc<Shared>);

impl<W: NodeOps + UiOps + ?Sized>
    ContinueOnce<W, OpYielded<'_, '_, &'_ mut CreateInner>, Option<W::NodeKind>>
    for CreateWithType
{
    fn continue_once(
        self,
        kind: Option<W::NodeKind>,
        cx: ContinueCx<W, OpYielded<&mut CreateInner>>,
    ) {
        cx.run_op(|inner, mut cx| {
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

            if let Some(value) = kind {
                let pos = cx.driver().cell.position(cx.widget());
                cx.run_action(CreateNode(value, pos), None);
            }

            cx.finish(self.0 .0.into());
        });
    }
}
