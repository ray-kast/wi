use crate::trie::{trie, Acceptor};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    PushCount(char),
    Step(Step),
    ViewCursor,
}

use tracing::debug;
use Action::*;
use Key::{Char as C, Named as N};
use NamedKey::*;

use crate::modifiers::*;

trie! {
    pub fn NormalAccept(k: Key) -> Option<Action> {
        C(c @ '0'..='9', _, M_NONE) => Count(PushCount(c)) @ "" {
            _ => continue,
        },

        C('h', _, M_NONE) | N(ArrowLeft, M_NONE) => yield Step(Step::Left),
        C('j', _, M_NONE) | N(ArrowDown, M_NONE) => yield Step(Step::Down),
        C('k', _, M_NONE) | N(ArrowUp, M_NONE) => yield Step(Step::Up),
        C('l', _, M_NONE) | N(ArrowRight, M_NONE) => yield Step(Step::Right),

        C('z', _, M_NONE) => View @ "z" {
            . => yield ViewCursor,
            _ => yield,
        },
        N(Home, M_NONE) => yield ViewCursor,
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
    type Output = (bool, Option<Action>);

    fn pending_op(&self) -> &'static str {
        match self {
            Self::Normal { accept, .. } => accept.pending_op(),
        }
    }

    fn accept(&mut self, input: Key) -> Self::Output {
        debug!("Handling keypress");
        match self {
            Self::Normal { accept, .. } => {
                let action = accept.accept(input);
                (*accept != NormalAccept::default(), action)
            },
        }
    }
}
