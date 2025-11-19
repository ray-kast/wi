use std::{borrow::Cow, marker::PhantomData};

use shibari::Acceptor;

use crate::{bindings::Key, traits::GraphWidgetTypes, WPort};

pub struct JumpAdorner<W: GraphWidgetTypes + ?Sized> {
    target: JumpAdornerTarget<W>,
}

pub enum JumpAdornerTarget<W: GraphWidgetTypes + ?Sized> {
    Node(W::NodeId),
    Port(W::PortId),
    Edge(WPort<W>, WPort<W>),
}

#[derive_where::derive_where(Debug, PartialEq; )]
pub struct JumpState<T>(Cow<'static, str>, PhantomData<T>);

impl<T> JumpState<T> {
    pub fn new(pending: Cow<'static, str>) -> Self { Self(pending, todo!()) }
}

pub enum JumpOut<T> {
    Trap,
    Advance,
    Accept(T),
}

impl<T> Acceptor<Key> for JumpState<T> {
    type Output = JumpOut<T>;

    #[inline]
    fn pending_op(&self) -> Cow<'static, str> { self.0.clone() }

    fn accept(&mut self, input: Key) -> (Cow<'static, str>, Self::Output) { todo!() }
}
