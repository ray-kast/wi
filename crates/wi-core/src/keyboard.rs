use keyboard_types::{Modifiers as M, NamedKey};
use tracing::{debug, instrument};

use self::State::*;
use crate::{GraphWidget, GraphWidgetDriver};

const M_EMPTY: M = M::empty();

#[derive(Debug, Default, Clone, Copy)]
enum State {
    #[default]
    Init,
    Go,
}

#[derive(Debug, Default)]
pub struct KeyboardHandler {
    state: State,
}

impl<N> GraphWidgetDriver<N> {
    #[instrument(skip(self, widget), fields(state = ?self.keyboard.state))]
    pub fn handle_char_input<W: GraphWidget<N>>(&mut self, widget: &mut W, chars: &str, mods: &M) {
        let shift = mods.shift();
        let mods = mods.difference(M::SHIFT);

        for char in chars.chars() {
            match (char, mods, self.keyboard.state) {
                ('g', M_EMPTY, Init) => self.keyboard.state = Go,
                ('o', M_EMPTY, Init) => {
                },
                _ => {
                    debug!("Unhandled character input");
                    self.keyboard.state = Init;
                },
            }
        }
    }

    #[instrument(skip(self, widget), fields(state = ?self.keyboard.state))]
    pub fn handle_named_keypress<W: GraphWidget<N>>(&mut self, widget: &mut W, key: &NamedKey, mods: &M) {
        match (key, mods) {
            _ => {
                debug!("Unhandled named keypress");
                self.keyboard.state = Init;
            },
        }
    }
}
