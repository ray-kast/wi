use std::num::NonZero;

use keyboard_types::{Modifiers, NamedKey as K};
use shibari::Acceptor;
use tracing::{debug, instrument};

use crate::{
    actions::{ActionCx, EditorAction, Kind},
    bindings::{ActionOut, Key},
    continuation::Dispatch,
    operators::{CurrentOperator, EditorOperator, OperatorCx, OperatorResult},
    traits::GraphWidget,
    DriverInner, GraphWidgetDriver, LastOp,
};

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
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
        cx: W::Context<'_, '_>,
    ) -> bool {
        self.mutate_check(widget, cx, |inner, curr_op, widget, mut cx| {
            let mut any_handled = false;

            for char in chars.to_lowercase().chars() {
                any_handled |= inner.handle_key(
                    curr_op,
                    widget,
                    Key::Char(char, mods),
                    W::reborrow_cx(&mut cx),
                );
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
        cx: W::Context<'_, '_>,
    ) -> bool {
        self.mutate_check(widget, cx, |inner, curr_op, widget, cx| {
            inner.handle_key(curr_op, widget, Key::Named(key, mods), cx)
        })
    }
}

impl<W: GraphWidget + ?Sized> DriverInner<W> {
    fn handle_key(
        &mut self,
        current_operator: &mut CurrentOperator,
        widget: &mut W,
        key: Key,
        mut cx: W::Context<'_, '_>,
    ) -> bool {
        if let Some(ref mut operator) = current_operator.as_mut() {
            let mut res = OperatorResult::Continue;
            operator.step(
                key,
                OperatorCx::new(
                    widget,
                    self,
                    W::reborrow_cx(&mut cx),
                    Dispatch::Immediate(&mut res),
                ),
            );

            return match res {
                OperatorResult::Continue => true,
                OperatorResult::Finish => {
                    current_operator
                        .pop(&mut self.stashed_operators)
                        .unwrap_or_else(|| unreachable!());
                    true
                },
                OperatorResult::Abort => {
                    current_operator
                        .pop(&mut self.stashed_operators)
                        .unwrap_or_else(|| unreachable!());
                    false
                },
            };
        }

        let (pend, out) = self.mode.accept(key);
        match out {
            ActionOut::Trap => {
                self.last_op = LastOp {
                    count: self.count.map(NonZero::get),
                    chord: pend.into(),
                };
                self.count = None;

                false
            },
            ActionOut::Advance => true,
            ActionOut::Action(action) => {
                let loud = !action.kind().is_silent();

                if loud {
                    self.last_op = LastOp {
                        count: self.count.map(NonZero::get),
                        chord: pend.into(),
                    };
                }

                debug!(
                    action = action.kind().name(),
                    count = self.count,
                    "Processing action"
                );
                let handled = action.process(
                    self.count.take(),
                    ActionCx::new(widget, self, W::reborrow_cx(&mut cx)),
                );

                if handled && loud {
                    self.last_action = Some(action.into());
                }

                handled
            },
            ActionOut::Operator(mut operator) => {
                self.last_op = LastOp {
                    count: self.count.map(NonZero::get),
                    chord: pend.into(),
                };

                let mut res = OperatorResult::Continue;
                operator.init(
                    self.count.take(),
                    OperatorCx::new(
                        widget,
                        self,
                        W::reborrow_cx(&mut cx),
                        Dispatch::Immediate(&mut res),
                    ),
                );

                match res {
                    OperatorResult::Continue => {
                        debug!(operator = operator.kind().name(), "Pushing operator");
                        current_operator.push(operator, &mut self.stashed_operators);

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
