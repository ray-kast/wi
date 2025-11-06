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

impl<T> AcceptState for Option<T> {
    const ADVANCE: Self = None;
    const TRAP: Self = None;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TrapError;

impl std::fmt::Display for TrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Acceptor received unhandled input")
    }
}

impl std::error::Error for TrapError {}

pub type AcceptResult<T> = Result<Option<T>, TrapError>;

impl<T> AcceptState for AcceptResult<T> {
    const ADVANCE: Self = Ok(None);
    const TRAP: Self = Err(TrapError);
}
