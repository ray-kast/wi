pub use wi_macros::trie;

pub trait Acceptor<T>: Default + PartialEq {
    type Output;

    fn pending_op(&self) -> &'static str;

    fn accept(&mut self, input: T) -> Self::Output;
}
