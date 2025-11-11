use keyboard_types::{Modifiers, NamedKey as K};
use shibari::Acceptor;
use tracing::{debug, instrument};

use crate::{
    actions::{ActionCx, EditorAction, Kind},
    bindings::{ActionOut, Key},
    continuation::Dispatch,
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
        skip(self, widget, cx),
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
        cx: &mut W::Context<'_>,
    ) -> bool {
        self.mutate_check(widget, cx, |me, widget, cx| {
            me.handle_key(widget, Key::Named(key, mods), cx)
        })
    }

    fn handle_key(&mut self, widget: &mut W, key: Key, cx: &mut W::Context<'_>) -> bool {
        if let Some(ref mut operator) = self.current_operator.as_mut() {
            let mut res = OperatorResult::Continue;
            operator.step(
                key,
                OperatorCx::new(widget, &mut self.inner, cx, Dispatch::Immediate(&mut res)),
            );

            return match res {
                OperatorResult::Continue => true,
                OperatorResult::Finish => {
                    self.current_operator
                        .pop(&mut self.inner.stashed_operators)
                        .unwrap_or_else(|| unreachable!());
                    true
                },
                OperatorResult::Abort => {
                    self.current_operator
                        .pop(&mut self.inner.stashed_operators)
                        .unwrap_or_else(|| unreachable!());
                    false
                },
            };
        }

        match self.inner.mode.accept(key) {
            ActionOut::Trap => {
                self.inner.count = None;
                false
            },
            ActionOut::Advance => true,
            ActionOut::Action(action) => {
                debug!(
                    action = action.kind().name(),
                    count = self.inner.count,
                    "Processing action"
                );
                let handled = action.process(self.inner.count.take(), ActionCx {
                    widget,
                    driver: &mut self.inner,
                    inner: cx,
                });

                if handled && !action.kind().is_silent() {
                    self.inner.last_action = Some(action.into());
                }

                handled
            },
            ActionOut::Operator(mut operator) => {
                let mut res = OperatorResult::Continue;
                operator.init(OperatorCx::new(
                    widget,
                    &mut self.inner,
                    cx,
                    Dispatch::Immediate(&mut res),
                ));

                match res {
                    OperatorResult::Continue => {
                        debug!(operator = operator.kind().name(), "Pushing operator");
                        self.current_operator
                            .push(operator, &mut self.inner.stashed_operators);

                        true
                    },
                    OperatorResult::Finish => {
                        debug!(
                            operator = operator.kind().name(),
                            "Operator finished on init"
                        );

                        true
                    },
                    OperatorResult::Abort => {
                        debug!(
                            operator = operator.kind().name(),
                            "Operator aborted on init"
                        );

                        false
                    },
                }
            },
        }
    }
}
