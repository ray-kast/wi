use keyboard_types::{Modifiers, NamedKey as K};
use shibari::Acceptor;
use tracing::{debug, instrument};

use crate::{
    actions::{ActionCx, EditorAction},
    bindings::{ActionOut, Key},
    operators::{EditorOperator, OperatorCx, OperatorResult},
    GraphWidget, GraphWidgetDriver,
};

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    #[inline]
    fn mutate_check<T>(
        &mut self,
        widget: &mut W,
        ctx: &mut W::Context<'_>,
        f: impl FnOnce(&mut Self, &mut W, &mut W::Context<'_>) -> T,
    ) -> T {
        let pre_status = self.status();
        let res = f(self, widget, ctx);

        if pre_status != self.status() {
            widget.update_status(self.status(), ctx);
        }

        res
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
        mods: Modifiers,
        ctx: &mut W::Context<'_>,
    ) -> bool {
        self.mutate_check(widget, ctx, |me, widget, ctx| {
            let mut any_handled = false;

            for char in chars.to_lowercase().chars() {
                any_handled |= me.handle_key(widget, Key::Char(char, mods), ctx);
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
            me.handle_key(widget, Key::Named(key, mods), ctx)
        })
    }

    fn handle_key(&mut self, widget: &mut W, key: Key, ctx: &mut W::Context<'_>) -> bool {
        if let Some(mut operator) = self.operators.pop() {
            match operator.step(key, OperatorCx::new(widget, self, ctx)) {
                OperatorResult::Continue => {
                    self.operators.push(operator);
                    return true;
                },
                OperatorResult::Finish => return true,
                OperatorResult::Abort => (),
            }
        }

        match self.mode.accept(key) {
            ActionOut::Trap => {
                self.count = None;
                false
            },
            ActionOut::Advance => true,
            ActionOut::Action(action) => {
                debug!(
                    action = action.name().as_ref(),
                    count = self.count,
                    "Processing action"
                );
                let handled = action.process(self.count.take(), ActionCx {
                    widget,
                    driver: self,
                    inner: ctx,
                });

                if handled && !action.is_silent() {
                    self.last_action = Some(action);
                }

                handled
            },
            ActionOut::Operator(mut operator) => {
                let handled = operator.init(OperatorCx::new(widget, self, ctx));

                if handled {
                    debug!(operator = operator.name().as_ref(), "Pushing operator");
                    self.operators.push(operator);
                } else {
                    debug!(
                        operator = operator.name().as_ref(),
                        "Operator did not initialize"
                    );
                }

                handled
            },
        }
    }
}
