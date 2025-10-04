use std::num::NonZero;

use tracing::{debug, instrument};

use crate::{bindings::Action, CursorUpdate, GraphWidget, GraphWidgetDriver};

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    #[instrument(
        skip(self, widget, ctx),
        fields(count = ?self.count),
    )]
    pub fn process_action(
        &mut self,
        widget: &mut W,
        action: Action,
        ctx: &mut W::Context<'_>,
    ) -> bool {
        debug!("Processing action");
        match (self.count, action) {
            (c, Action::PushCount(d)) => {
                let digit = u32::from(d) - u32::from('0');
                self.count = c
                    .map_or(0, NonZero::get)
                    .checked_mul(10)
                    .and_then(|c| c.checked_add(digit))
                    .and_then(NonZero::new);
            },
            (None, Action::ViewCursor) => {
                widget.update_cursor(CursorUpdate::CenterInView, &self.cursor, ctx);
            },
            (Some(_), _) => return false,
        }

        true
    }
}
