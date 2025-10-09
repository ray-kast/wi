use std::borrow::Cow;

use crate::{action::prelude::*, GraphWidget};

pub mod actions {
    use crate::Side;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct JumpToPort(pub Side);
}

impl EditorAction for actions::JumpToPort {
    fn name(&self) -> Cow<'static, str> {
        let Self(side) = self;
        match side {
            crate::Side::In => "jump to input",
            crate::Side::Out => "jump to output",
        }
        .into()
    }

    fn process<W: GraphWidget + ?Sized>(self, count: Option<NonZeroU32>, cx: ActionCx<W>) -> bool {
        let None = count else { return false };

        true
    }
}
