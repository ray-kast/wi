use super::prelude::*;
use crate::jump::{JumpOut, JumpState};

pub(super) mod operators {
    use crate::Side;

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub enum JumpToPort {
        CurrentNode(Side),
        Global(Side),
    }
}

#[derive(Kind)]
#[derive_where(Debug, PartialEq; )]
#[kind(const OperatorKind::JumpToPort)]
pub(crate) struct JumpStarted<W: GraphWidgetTypes + ?Sized>(JumpState<W::PortId>);

impl<W: GraphWidgetTypes + ?Sized> StartOperator<W> for operators::JumpToPort {
    type Started = JumpStarted<W>;

    fn start_op(
        &self,
        count: Option<NonZeroU32>,
        cx: StartCx<W>,
    ) -> Result<Self::Started, StartError> {
        let None = count else {
            return Err(cx.into_aborted());
        };

        Ok(JumpStarted(JumpState::new(Cow::Borrowed(""))))
    }
}

impl<W: GraphWidgetTypes + ?Sized> OperatorState for JumpStarted<W> {
    #[inline]
    fn pending_op(&self) -> Cow<'static, str> { self.0.pending_op() }
}

impl<W: GraphWidgetTypes + ?Sized> EditorOperator<W> for JumpStarted<W> {
    fn step(&mut self, key: Key, cx: OperatorCx<W>) {
        let (pending, out) = self.0.accept(key);
        match out {
            JumpOut::Trap => cx.abort(pending),
            JumpOut::Advance => (),
            JumpOut::Accept(p) => {},
        }
    }
}
