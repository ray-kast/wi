pub use shibari_grammar::*;
#[cfg(feature = "proc-macros")]
pub use shibari_macros::*;

pub trait Acceptor<T> {
    type Output;

    fn pending_op(&self) -> &'static str;

    fn accept(&mut self, input: T) -> Self::Output;
}

pub trait AcceptState {
    const TRAP: Self;
    const ADVANCE: Self;
}
