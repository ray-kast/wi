use crate::{
    action::{Action, prelude::*},
    trie::{Acceptor, trie},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Tuple of (char code, shift, modifiers \\ shift)
    Char(char, bool, Modifiers),
    Named(NamedKey, Modifiers),
}

use keyboard_types::{Modifiers, NamedKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Step {
    Left,
    Down,
    Up,
    Right,
}

use tracing::debug;
use Key::{Char as C, Named as N};
use NamedKey as K;

use crate::modifiers::M_NONE;

trie! {
    #[advance = Nop]
    pub fn NormalAccept(k: Key) -> Action {
        C(c @ '1'..='9', _, M_NONE) => Count(PushCount(c)) @ "" {
            C(c @ '0'..='9', _, M_NONE) => Count(PushCount(c)) { .. },
            _ => continue,
        },

        C('h', _, M_NONE) | N(K::ArrowLeft, M_NONE) => yield StepCursor(Step::Left),
        C('j', _, M_NONE) | N(K::ArrowDown, M_NONE) => yield StepCursor(Step::Down),
        C('k', _, M_NONE) | N(K::ArrowUp, M_NONE) => yield StepCursor(Step::Up),
        C('l', _, M_NONE) | N(K::ArrowRight, M_NONE) => yield StepCursor(Step::Right),

        C('z', _, M_NONE) => View @ "z" {
            . => yield ViewCursor,
            _ => yield,
        },
        N(K::Home, M_NONE) => yield ViewCursor,
        _ => yield,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Mode {
    Normal { accept: NormalAccept },
}

impl Default for Mode {
    #[inline]
    fn default() -> Self {
        Self::Normal {
            accept: NormalAccept::default(),
        }
    }
}

impl Acceptor<Key> for Mode {
    type Output = Action;

    fn pending_op(&self) -> &'static str {
        match self {
            Self::Normal { accept, .. } => accept.pending_op(),
        }
    }

    fn accept(&mut self, input: Key) -> Self::Output {
        debug!("Handling keypress");
        match self {
            Self::Normal { accept, .. } => {
                accept.accept(input)
            },
        }
    }
}
