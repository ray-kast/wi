use wi_macros::Kind;

pub use super::prelude::*;
use crate::{
    jump::{JumpOut, JumpState},
    Cursor, GraphWidgetCell, Side, WSidedPort,
};

pub(super) mod operators {
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct Create;
}

#[derive(Debug, PartialEq)]
struct Shared(Cow<'static, str>);

#[derive(Kind)]
#[derive_where(Debug, PartialEq; W::NodeId, W::PortId)]
#[kind(OperatorKind)]
pub(crate) struct CreateStarted<W: GraphWidgetTypes + ?Sized>(#[kind] State<W>);

#[derive(Kind)]
#[derive_where(Debug, PartialEq; W::NodeId, W::PortId)]
enum State<W: GraphWidgetTypes + ?Sized> {
    #[kind(OperatorKind::Create)]
    Init(CreateOpAccept),
    #[kind(OperatorKind::Create)]
    EdgeStart(JumpState<WSidedPort<W>>),
    #[kind(OperatorKind::Create)]
    EdgeConnect(JumpState<WSidedPort<W>>, WSidedPort<W>),
    #[kind(OperatorKind::Create)]
    Yielded(Arc<Shared>),
}

impl<W: GraphWidgetTypes + ?Sized> StartOperator<W> for operators::Create {
    type Started = CreateStarted<W>;

    fn start_op(
        &self,
        count: Option<NonZeroU32>,
        cx: StartCx<W>,
    ) -> Result<Self::Started, StartError> {
        let accept = CreateOpAccept::default();

        let None = count else {
            return Err(cx.into_aborted());
        };

        Ok(CreateStarted(State::Init(accept)))
    }
}

impl<W: GraphWidgetTypes + ?Sized> OperatorState for CreateStarted<W> {
    fn pending_op(&self) -> Cow<'static, str> {
        match &self.0 {
            State::Init(a) => a.pending_op(),
            State::EdgeStart(j) | State::EdgeConnect(j, ..) => j.pending_op(),
            State::Yielded(s) => s.0.clone(),
        }
    }
}

impl<W: EdgeOps + NodeOps + UiOps + ?Sized> EditorOperator<W> for CreateStarted<W> {
    fn step(&mut self, key: Key, mut cx: OperatorCx<W>) {
        match &mut self.0 {
            State::Init(a) => {
                let (pending, out) = a.accept(key);
                let action = match out {
                    CreateOut::Trap => {
                        cx.abort(pending);
                        return;
                    },
                    CreateOut::Advance => return,
                    CreateOut::CreateOpAction(a) => a,
                };

                match action {
                    CreateOpAction::Accept => {
                        let shared = Arc::new(Shared(pending));

                        self.0 = State::Yielded(Arc::clone(&shared));
                        let (widget, then) = cx.into_yielded(CreateWithType(shared), self);

                        widget.prompt_node_kind(then);
                    },
                    CreateOpAction::Side(s) => {
                        match cx
                            .driver()
                            .cursor
                            .as_ref()
                            .unwrap_or_else(|| unreachable!())
                        {
                            Cursor::Node(_) => self.0 = State::EdgeStart(JumpState::new(pending)),
                            Cursor::Port(p) => {
                                if p.0 != s {
                                    cx.abort(pending);
                                    return;
                                }

                                self.0 = State::EdgeConnect(JumpState::new(pending), *p);
                            },
                            Cursor::Edge(_) | Cursor::FixedPoint(_) => cx.abort(pending),
                        }
                    },
                }
            },
            State::EdgeStart(j) => {
                let (pending, out) = j.accept(key);
                match out {
                    JumpOut::Trap => cx.abort(pending),
                    JumpOut::Advance => (),
                    JumpOut::Accept(p) => self.0 = State::EdgeConnect(JumpState::new(pending), p),
                }
            },
            &mut State::EdgeConnect(ref mut j, p) => {
                let (pending, out) = j.accept(key);
                let other = match out {
                    JumpOut::Trap => {
                        cx.abort(pending);
                        return;
                    },
                    JumpOut::Advance => return,
                    JumpOut::Accept(p) => p,
                };

                debug_assert_eq!(p.0, other.0.flip());

                let (from, to) = match p.0 {
                    Side::In => (p.1, other.1),
                    Side::Out => (other.1, p.1),
                };

                cx.run_action(CreateEdge(from, to), None);
                cx.finish(pending);
            },
            State::Yielded(..) => (),
        }
    }
}

struct CreateWithType(Arc<Shared>);

impl<W: NodeOps + UiOps + ?Sized>
    ContinueOnce<W, OpYielded<'_, '_, W, &'_ mut CreateStarted<W>>, Option<W::NodeKind>>
    for CreateWithType
{
    fn continue_once(
        self,
        kind: Option<W::NodeKind>,
        cx: ContinueCx<W, OpYielded<W, &mut CreateStarted<W>>>,
    ) {
        cx.run_op(|inner, mut cx| {
            let inner = match (inner, cx.dispatch()) {
                (Some(i), Dispatch::Immediate(_)) => Some(&*i),
                (None, Dispatch::Deferred(o)) => o.as_ref().and_then(|o| {
                    if let Started::Create(i) = o {
                        Some(i)
                    } else {
                        None
                    }
                }),
                _ => unreachable!(),
            };

            if inner.is_none_or(|i| {
                if let CreateStarted(State::Yielded(s)) = i {
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

            cx.finish(self.0 .0.clone());
        });
    }
}
