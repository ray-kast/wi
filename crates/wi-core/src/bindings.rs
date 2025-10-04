use crate::trie::{trie, Acceptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// Tuple of (char code, shift, modifiers \\ shift)
    Char(char, bool, Modifiers),
    Named(NamedKey, Modifiers),
}

use keyboard_types::{Modifiers, NamedKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    PushCount(char),
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
