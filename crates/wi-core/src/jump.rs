use std::borrow::Cow;

use derive_where::derive_where;
use shibari::Acceptor;
use smallvec::SmallVec;

use crate::{bindings::Key, traits::GraphWidgetTypes, WPort};

#[derive_where(Debug, PartialEq, Eq, Hash; W::NodeId, W::PortId)]
#[derive_where(Clone, Copy; )]
pub struct JumpAdorner<W: GraphWidgetTypes + ?Sized> {
    target: JumpAdornerTarget<W>,
}

#[derive_where(Debug, PartialEq, Eq, Hash; W::NodeId, W::PortId)]
#[derive_where(Clone, Copy; )]
pub enum JumpAdornerTarget<W: GraphWidgetTypes + ?Sized> {
    Node(W::NodeId),
    Port(W::PortId),
    Edge(WPort<W>, WPort<W>),
}

pub enum JumpResult<T> {
    Abort,
    Accept(T),
    Jump(JumpState<T>),
}

pub trait JumpTarget {}

#[derive(Debug, PartialEq)]
pub struct JumpState<T>(Cow<'static, str>, Vec<T>);

impl<T> JumpState<T> {
    pub fn start(pending: Cow<'static, str>, targets: SmallVec<[T; 1]>) -> JumpResult<T> {
        if targets.len() < 2 {
            let Ok([target]) = targets.into_inner() else {
                return JumpResult::Abort;
            };

            JumpResult::Accept(target)
        } else {
            JumpResult::Jump(Self(pending, targets.into_vec()))
        }
    }
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
