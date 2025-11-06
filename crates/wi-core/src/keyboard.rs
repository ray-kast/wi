use keyboard_types::{Modifiers, NamedKey as K};
use shibari::Acceptor;
use tracing::{debug, instrument};

use crate::{
    actions::{ActionCx, EditorAction},
    bindings::{Key, NormalOut},
    operators::{EditorOperator, OperatorCx, OperatorResult},
    GraphWidget, GraphWidgetDriver,
};

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    #[inline]
    fn mutate_check<T>(
        &mut self,
        widget: &mut W,
        cx: &mut W::Context<'_>,
        f: impl FnOnce(&mut Self, &mut W, &mut W::Context<'_>) -> T,
    ) -> T {
        let pre_status = self.status();
        let res = f(self, widget, cx);

        if pre_status != self.status() {
            widget.update_status(self.status(), cx);
        }

        res
    }

    #[instrument(
        skip(self, widget, cx),
        fields(
            op = ?self.current_operator,
            mode = ?self.inner.mode,
            pending = ?self.inner.mode.pending_op(),
        ),
    )]
    #[inline]
    pub fn handle_char_input(
        &mut self,
        widget: &mut W,
        chars: &str,
        mods: Modifiers,
        cx: &mut W::Context<'_>,
    ) -> bool {
        self.mutate_check(widget, cx, |me, widget, cx| {
            let mut any_handled = false;

            for char in chars.to_lowercase().chars() {
                any_handled |= me.handle_key(widget, Key::Char(char, mods), cx);
            }

            any_handled
        })
    }

    #[instrument(
        skip(self, widget, ctx),
        fields(
            op = ?self.current_operator,
            mode = ?self.inner.mode,
            pending = ?self.inner.mode.pending_op(),
        ),
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

    fn handle_key(&mut self, widget: &mut W, key: Key, cx: &mut W::Context<'_>) -> bool {
        if let Some(ref mut operator) = self.current_operator {
            return match operator.step(key, OperatorCx::new(widget, &mut self.inner, cx)) {
                OperatorResult::Continue => true,
                OperatorResult::Finish => {
                    self.pop_operator().unwrap_or_else(|| unreachable!());
                    true
                },
                OperatorResult::Abort => {
                    self.pop_operator().unwrap_or_else(|| unreachable!());
                    false
                },
            };
        }

        match self.inner.mode.accept(key) {
            NormalOut::Trap => {
                self.inner.count = None;
                false
            },
            NormalOut::Advance => true,
            NormalOut::Action(action) => {
                debug!(
                    action = action.name().as_ref(),
                    count = self.inner.count,
                    "Processing action"
                );
                let handled = action.process(self.inner.count.take(), ActionCx {
                    widget,
                    driver: self,
                    inner: cx,
                });

                if handled && !action.is_silent() {
                    self.inner.last_action = Some(action);
                }

                handled
            },
            NormalOut::Operator(mut operator) => {
                let handled = operator.init(OperatorCx::new(widget, &mut self.inner, cx));

                if handled {
                    debug!(operator = operator.state().name(), "Pushing operator");
                    self.push_operator(operator);
                } else {
                    debug!(
                        operator = operator.state().name(),
                        "Operator did not initialize"
                    );
                }

                handled
            },
        }
    }
}
