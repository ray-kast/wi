use keyboard_types::{Modifiers as M, NamedKey as K};
use tracing::{debug, instrument};

#[allow(clippy::enum_glob_use)]
use self::State::*;
use crate::{GraphWidget, GraphWidgetDriver};

const M_EMPTY: M = M::empty();

#[derive(Debug, Default)]
enum State {
    #[default]
    Init,
    Go,
    View,
}

#[derive(Debug, Default)]
pub struct KeyboardHandler {
    state: State,
}

impl<N> GraphWidgetDriver<N> {
    #[instrument(skip(self, widget, ctx), fields(state = ?self.keyboard.state))]
    pub fn handle_char_input<W: GraphWidget<N>>(
        &mut self,
        widget: &mut W,
        chars: &str,
        mods: &M,
        ctx: &mut W::EventCtx<'_>,
    ) {
        let shift = mods.shift();
        let mods = mods.difference(M::SHIFT);

        for char in chars.chars() {
            match (char, mods, &self.keyboard.state) {
                ('g', M_EMPTY, Init) => self.keyboard.state = Go,
                ('z', M_EMPTY, Init) => self.keyboard.state = View,

                ('o', M_EMPTY, Init) => {},
                ('i', M_EMPTY, Init) => {},

                ('o', M::CONTROL, Init) => {},
                ('i', M::CONTROL, Init) => {},

                // View
                ('z', M_EMPTY, View) => widget.view_node(&self.focus_node, ctx),

                _ => {
                    debug!("Unhandled character input");
                    self.keyboard.state = Init;
                },
            }
        }
    }

    #[instrument(skip(self, widget, ctx), fields(state = ?self.keyboard.state))]
    pub fn handle_named_keypress<W: GraphWidget<N>>(
        &mut self,
        widget: &mut W,
        key: &K,
        mods: &M,
        ctx: &mut W::EventCtx<'_>,
    ) {
        match (key, *mods, &self.keyboard.state) {
            (K::Home, M_EMPTY, Init) => widget.view_node(&self.focus_node, ctx),
            _ => {
                debug!("Unhandled named keypress");
                self.keyboard.state = Init;
            },
        }
    }
}
