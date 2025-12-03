use super::prelude::*;
use crate::{
    jump::{JumpOut, JumpResult, JumpState},
    WSidedPort,
};

pub(super) mod operators {
    use crate::Side;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum JumpToPort {
        CurrentNode(Side),
        Global(Side),
    }
}

#[derive(Kind)]
#[derive_where(Debug, PartialEq; W::NodeId, W::PortId)]
#[kind(const OperatorKind::JumpToPort)]
pub(crate) struct JumpToPortStarted<W: GraphWidgetTypes + ?Sized>(JumpState<WSidedPort<W>>);

impl<W: GraphWidgetTypes + ?Sized> StartOperator<W> for operators::JumpToPort {
    type Started = JumpToPortStarted<W>;

    fn start_op(
        &self,
        count: Option<NonZeroU32>,
        cx: StartCx<W>,
    ) -> Result<Self::Started, StartError> {
        let None = count else {
            return Err(cx.into_aborted());
        };

        match JumpState::start(Cow::Borrowed(""), todo!()) {
            JumpResult::Abort => Err(cx.into_aborted()),
            JumpResult::Accept(t) => todo!(),
            JumpResult::Jump(s) => Ok(JumpToPortStarted(s)),
        }
    }
}

impl<W: GraphWidgetTypes + ?Sized> OperatorState for JumpToPortStarted<W> {
    #[inline]
    fn pending_op(&self) -> Cow<'static, str> { self.0.pending_op() }
}

impl<W: CursorOps + ?Sized> EditorOperator<W> for JumpToPortStarted<W> {
    fn step(&mut self, key: Key, mut cx: OperatorCx<W>) {
        let (pending, out) = self.0.accept(key);
        match out {
            JumpOut::Trap => cx.abort(pending),
            JumpOut::Advance => (),
            JumpOut::Accept(p) => {
                cx.run_action(GoToPort(p), None);
            },
        }
    }
}
