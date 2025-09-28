use keyboard_types::{Modifiers, NamedKey as K};
use tracing::{debug, instrument};

#[allow(clippy::enum_glob_use)]
use self::{Action::*, State::*};
use crate::{GraphWidget, GraphWidgetDriver};

const M_EMPTY: Modifiers = Modifiers::empty();
const M_C: Modifiers = Modifiers::CONTROL;

#[derive(Debug, Default, Clone, Copy)]
enum State {
    #[default]
    Init,
    Go,
    View,
}

#[derive(Debug, Clone, Copy)]
enum Action {
    JumpInput,
    JumpOutput,
    Nop,
    PushCount(char),
    Reset,
    StackBack,
    StackFwd,
    ViewFocused,
}

#[derive(Debug, Default)]
pub struct KeyboardHandler {
    count: Option<u32>,
    state: State,
}

impl<W: GraphWidget + ?Sized> GraphWidgetDriver<W> {
    #[instrument(
        skip(self, widget, ctx),
        fields(count = ?self.keyboard.count, state = ?self.keyboard.state),
    )]
    pub fn handle_char_input(
        &mut self,
        widget: &mut W,
        chars: &str,
        mods: &Modifiers,
        ctx: &mut W::EventCtx<'_>,
    ) {
        let shift = mods.shift();
        let mods = mods.difference(Modifiers::SHIFT);

        for char in chars.chars() {
            let (action, next) = match (char, mods, &self.keyboard.state) {
                (c @ '0'..='9', M_EMPTY, &s @ Init)
                    if self.keyboard.count.is_some() || c != '0' =>
                {
                    (PushCount(c), s)
                },

                ('g', M_EMPTY, Init) => (Nop, Go),
                ('z', M_EMPTY, Init) => (Nop, View),

                ('o', M_EMPTY, Init) => (JumpOutput, Init),
                ('i', M_EMPTY, Init) => (JumpInput, Init),

                ('o', M_C, Init) => (StackBack, Init),
                ('i', M_C, Init) => (StackFwd, Init),

                // View
                ('z', M_EMPTY, View) => (ViewFocused, Init),

                _ => {
                    debug!("Unhandled character input");
                    (Reset, Init)
                },
            };

            self.process_action(widget, action, next, ctx);
        }
    }

    #[instrument(
        skip(self, widget, ctx),
        fields(count = ?self.keyboard.count, state = ?self.keyboard.state),
    )]
    pub fn handle_named_keypress(
        &mut self,
        widget: &mut W,
        key: &K,
        mods: &Modifiers,
        ctx: &mut W::EventCtx<'_>,
    ) {
        let (action, next) = match (key, *mods, &self.keyboard.state) {
            (
                K::Unidentified
                | K::Alt
                | K::AltGraph
                | K::CapsLock
                | K::Control
                | K::Fn
                | K::FnLock
                | K::Meta
                | K::NumLock
                | K::ScrollLock
                | K::Shift
                | K::Symbol
                | K::SymbolLock,
                ..,
            ) => return,
            (K::Home, M_EMPTY, Init) => (ViewFocused, Init),
            _ => {
                debug!("Unhandled named keypress");
                (Reset, Init)
            },
        };

        self.process_action(widget, action, next, ctx);
    }

    #[inline]
    #[instrument(
        skip(self, widget, ctx),
        fields(count = ?self.keyboard.count, state = ?self.keyboard.state),
    )]
    fn process_action(
        &mut self,
        widget: &mut W,
        action: Action,
        next: State,
        ctx: &mut W::EventCtx<'_>,
    ) {
        debug!("Processing action");
        let count = match (action, self.keyboard.count) {
            (JumpInput, _) => None,
            (JumpOutput, _) => None,
            (Nop, c) => c,
            (PushCount(c), None) => Some(u32::from(c) - u32::from('0')),
            (PushCount(c), Some(n)) => Some(n * 10 + (u32::from(c) - u32::from('0'))),
            (Reset, _) => None,
            (StackBack, _) => None,
            (StackFwd, _) => None,
            (ViewFocused, None) => {
                widget.view_cursor(&self.cursor, ctx);
                None
            },
            (_, Some(_)) => None,
        };

        self.keyboard.count = count;
        self.keyboard.state = next;
    }
}
