use keyboard_types::{Modifiers, NamedKey as K};
use tracing::instrument;

use crate::{
    bindings::{Acceptor, Key},
    GraphWidget, GraphWidgetDriver,
};

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    #[inline]
    fn mutate_check(
        &mut self,
        widget: &mut W,
        ctx: &mut W::Context<'_>,
        f: impl FnOnce(&mut Self, &mut W, &mut W::Context<'_>) -> bool,
    ) -> bool {
        let pre_status = self.status();
        let handled = f(self, widget, ctx);

        if pre_status != self.status() {
            widget.update_status(self.status(), ctx);
        }

        handled
    }

    #[instrument(
        skip(self, widget, ctx),
        fields(mode = ?self.mode, pending = ?self.mode.pending_op()),
    )]
    #[inline]
    pub fn handle_char_input(
        &mut self,
        widget: &mut W,
        chars: &str,
        mods: &Modifiers,
        ctx: &mut W::Context<'_>,
    ) -> bool {
        self.mutate_check(widget, ctx, |me, widget, ctx| {
            let mut any_handled = false;

            for char in chars.to_lowercase().chars() {
                let action = me.mode.accept(Key::Char(char, *mods));
                any_handled |= me.process_action(widget, action, ctx);
            }

            any_handled
        })
    }

    #[instrument(
        skip(self, widget, ctx),
        fields(mode = ?self.mode, pending = ?self.mode.pending_op()),
    )]
    #[inline]
    pub fn handle_named_keypress(
        &mut self,
        widget: &mut W,
        key: K,
        mods: Modifiers,
        ctx: &mut W::Context<'_>,
    ) -> bool {
        self.mutate_check(widget, ctx, |me, widget, ctx| {
            let action = me.mode.accept(Key::Named(key, mods));
            me.process_action(widget, action, ctx)
        })
    }
}
