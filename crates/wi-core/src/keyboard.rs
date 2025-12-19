use std::{borrow::Cow, num::NonZero};

use keyboard_types::{Modifiers, NamedKey as K};
use shibari::Acceptor;
use tracing::{debug, instrument};

use crate::{
    actions::{ActionCx, EditorAction, Kind},
    bindings::{ActionOut, Key},
    continuation::Dispatch,
    modifiers::M_SHIFT,
    operators::{
        CurrentOperator, EditorOperator, Operator, OperatorCx, OperatorFlow, StartCx, StartOperator,
    },
    traits::GraphWidget,
    DriverInner, GraphWidgetDriver, LastChord,
};

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    #[instrument(
        skip(self, widget, cx),
        fields(
            op = ?self.current_operator.as_ref().map(Kind::kind),
            mode = ?self.inner.mode,
            pending = ?self.inner.mode.pending_op(),
        ),
    )]
    #[inline]
    pub fn handle_str_input(
        &mut self,
        widget: &mut W,
        s: &str,
        mods: Modifiers,
        cx: W::Context<'_, '_>,
    ) -> bool {
        self.mutate_check(widget, cx, |inner, curr_op, widget, mut cx| {
            let mut any_handled = false;

            for chr in s.chars() {
                any_handled |=
                    inner.handle_char(curr_op, widget, chr, mods, W::reborrow_cx(&mut cx));
            }

            any_handled
        })
    }

    #[instrument(
        skip(self, widget, cx),
        fields(
            op = ?self.current_operator.as_ref().map(Kind::kind),
            mode = ?self.inner.mode,
            pending = ?self.inner.mode.pending_op(),
        ),
    )]
    #[inline]
    pub fn handle_char_input(
        &mut self,
        widget: &mut W,
        chr: char,
        mods: Modifiers,
        cx: W::Context<'_, '_>,
    ) -> bool {
        self.mutate_check(widget, cx, |inner, curr_op, widget, cx| {
            inner.handle_char(curr_op, widget, chr, mods, cx)
        })
    }

    #[instrument(
        skip(self, widget, cx),
        fields(
            op = ?self.current_operator.as_ref().map(Kind::kind),
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
    fn handle_char(
        &mut self,
        current_operator: &mut CurrentOperator<W>,
        widget: &mut W,
        chr: char,
        mut mods: Modifiers,
        mut cx: W::Context<'_, '_>,
    ) -> bool {
        if chr.is_uppercase() {
            mods.insert(M_SHIFT);

            let mut any_handled = false;
            for chr in chr.to_lowercase() {
                any_handled |= self.handle_key(
                    current_operator,
                    widget,
                    Key::Char(chr, mods),
                    W::reborrow_cx(&mut cx),
                );
            }

            any_handled
        } else {
            mods.remove(M_SHIFT);
            self.handle_key(current_operator, widget, Key::Char(chr, mods), cx)
        }
    }

    fn handle_key(
        &mut self,
        current_operator: &mut CurrentOperator<W>,
        widget: &mut W,
        key: Key,
        mut cx: W::Context<'_, '_>,
    ) -> bool {
        if let Some(operator) = current_operator.as_mut() {
            let mut res = OperatorFlow::Continue;
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
                OperatorFlow::Continue => true,
                OperatorFlow::Finish => {
                    current_operator
                        .pop(&mut self.stashed_operators)
                        .unwrap_or_else(|| unreachable!());
                    true
                },
                OperatorFlow::Abort => {
                    current_operator
                        .pop(&mut self.stashed_operators)
                        .unwrap_or_else(|| unreachable!());
                    false
                },
            };
        }

        let (pending, out) = self.mode.accept(key);
        match out {
            ActionOut::Trap => {
                if self.count.is_some() || !pending.is_empty() {
                    self.last_op = LastChord {
                        count: self.count.map(NonZero::get),
                        operator_prefix: None,
                        chord: pending,
                    };
                }

                self.count = None;
                self.last_action.hide();

                false
            },
            ActionOut::Advance => {
                self.last_action.hide();
                true
            },
            ActionOut::Modifier => false,
            ActionOut::Action(action) => {
                let loud = !action.kind().is_silent();

                if loud {
                    self.last_op = LastChord {
                        count: self.count.map(NonZero::get),
                        operator_prefix: None,
                        chord: pending,
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
                    self.last_action.replace(action.into());
                } else {
                    self.last_action.hide();
                }

                handled
            },
            ActionOut::Operator(operator) => {
                self.last_op = LastChord {
                    count: self.count.map(NonZero::get),
                    operator_prefix: Some(pending.clone()),
                    chord: Cow::Borrowed(""),
                };
                self.last_action.hide();

                self.init_operator(current_operator, widget, pending, operator, cx)
            },
        }
    }

    fn init_operator(
        &mut self,
        current_operator: &mut CurrentOperator<W>,
        widget: &mut W,
        chord: Cow<'static, str>,
        operator: Operator,
        mut cx: W::Context<'_, '_>,
    ) -> bool {
        let mut res = OperatorFlow::Continue;
        let started = operator.start_op(
            self.count.take(),
            StartCx::new(
                widget,
                self,
                W::reborrow_cx(&mut cx),
                Dispatch::Immediate(&mut res),
            ),
        );
        match started {
            Err(e) => {
                debug!(operator = ?operator, "Operator did not start");
                false
            },
            Ok(s) => match res {
                OperatorFlow::Continue => {
                    debug!(operator = s.kind().name(), "Pushing operator");
                    current_operator.push(s, chord, &mut self.stashed_operators);

                    true
                },
                OperatorFlow::Finish => {
                    debug!(operator = s.kind().name(), "Operator finished on init");

                    true
                },
                OperatorFlow::Abort => {
                    debug!(operator = s.kind().name(), "Operator aborted on init");

                    false
                },
            },
        }
    }
}
