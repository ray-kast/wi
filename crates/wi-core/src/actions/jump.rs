use super::prelude::*;
use crate::Side;

pub(super) mod actions {
    use crate::Side;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct JumpToPort(pub Option<Side>);
}

impl EditorMotion for actions::JumpToPort {
    fn name(&self) -> Cow<'static, str> {
        let Self(side) = self;
        match side {
            Some(Side::In) => "jump to input",
            Some(Side::Out) => "jump to output",
            None => "jump to port",
        }
        .into()
    }

    fn process<W: GraphWidget + ?Sized, S: Selection>(
        self,
        count: Option<NonZeroU32>,
        cx: ActionCx<W>,
        selection: S,
    ) -> bool {
        let None = count else { return false };

        true
    }
}
