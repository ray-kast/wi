use keyboard_types::{Modifiers, NamedKey as K};
use tracing::instrument;

use crate::{bindings::Key, modifiers::M_SHIFT, trie::Acceptor, GraphWidget, GraphWidgetDriver};

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

        if !handled {
            self.unhandled_keypress();
        }

        if pre_status != self.status() {
            widget.update_status(self.status(), ctx);
        }

        handled
    }

    #[instrument(
        skip(self, widget, ctx),
        fields(state = ?self.mode),
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
            let shift = mods.contains(M_SHIFT);
            let mods = mods.difference(M_SHIFT);
            let mut any_handled = false;

            for char in chars.chars() {
                let handled = match me.mode.accept(Key::Char(char, shift, mods)) {
                    (_, Some(a)) => me.process_action(widget, a, ctx),
                    (h, None) => h,
                };

                any_handled |= handled;
            }

            any_handled
        })
    }

    #[instrument(
        skip(self, widget, ctx),
        fields(state = ?self.mode),
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
            match me.mode.accept(Key::Named(key, mods)) {
                (_, Some(a)) => me.process_action(widget, a, ctx),
                (h, None) => h,
            }
        })
    }

    #[inline]
    fn unhandled_keypress(&mut self) { self.count = None; }
}
