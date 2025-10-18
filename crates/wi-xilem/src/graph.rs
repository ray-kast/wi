use std::{
    borrow::{Borrow, BorrowMut, Cow},
    sync::Arc,
};

use masonry::kurbo::Point;
use petgraph::{graph::IndexType, prelude::StableDiGraph};

pub type Graph<I, W, P, Ix = u32> = StableDiGraph<Arc<Node<I, W, P>>, Edge, Ix>;

#[derive(Debug, Clone)]
pub struct Node<I, W, P> {
    pub name: Cow<'static, str>,
    pub icon: I,
    pub position: Point,
    pub width: f64,
    pub kind: NodeKind<W, P>,
}

#[derive(Debug, Clone)]
pub enum NodeKind<W, P> {
    Widget(WidgetNode<W, P>),
    Small(SmallNode<P>),
    Large(LargeNode<W, P>),
}

#[derive(Debug, Clone)]
pub struct WidgetNode<W, P> {
    pub widget: Option<W>,
    pub input: Option<PortInfo<P>>,
    pub output: Option<PortInfo<P>>,
}

#[derive(Debug, Clone)]
pub struct SmallNode<P> {
    pub inputs: Vec<PortInfo<P>>,
    pub outputs: Vec<PortInfo<P>>,
}

#[derive(Debug, Clone)]
pub struct LargeNode<W, P> {
    pub inputs: Vec<(PortInfo<P>, Option<W>)>,
    pub outputs: Vec<PortInfo<P>>,
}

#[derive(Debug, Clone)]
pub struct PortInfo<T> {
    pub name: Cow<'static, str>,
    pub data: T,
}

#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub from_port: u16,
    pub to_port: u16,
}

impl<I, W, P> Node<I, W, P> {
    #[inline]
    pub fn in_arity(&self) -> u16 {
        match &self.kind {
            NodeKind::Widget(w) => w.input.iter().len(),
            NodeKind::Small(s) => s.inputs.len(),
            NodeKind::Large(l) => l.inputs.len(),
        }
        .try_into()
        .unwrap_or_else(|_| port_overflow())
    }

    #[inline]
    pub fn out_arity(&self) -> u16 {
        match &self.kind {
            NodeKind::Widget(w) => w.output.iter().len(),
            NodeKind::Small(s) => s.outputs.len(),
            NodeKind::Large(l) => l.outputs.len(),
        }
        .try_into()
        .unwrap_or_else(|_| port_overflow())
    }
}

pub const PORT_MAX: u16 = u16::MAX;
#[inline]
#[expect(clippy::must_use_candidate, reason = "Using this is impossible")]
pub fn port_overflow<T>() -> T { panic!("Port number exceeded {PORT_MAX}") }

pub type GraphType<G> = Graph<
    <G as GraphMarker>::Icon,
    <G as GraphMarker>::Widget,
    <G as GraphMarker>::PortData,
    <G as GraphMarker>::Index,
>;

pub type NodeType<G> =
    Node<<G as GraphMarker>::Icon, <G as GraphMarker>::Widget, <G as GraphMarker>::PortData>;

pub trait GraphMarker:
    Clone
    + From<GraphType<Self>>
    + Into<GraphType<Self>>
    + Borrow<GraphType<Self>>
    + BorrowMut<GraphType<Self>>
{
    type Icon: Clone;
    type Widget: Clone;
    type PortData: Clone;
    type Index: IndexType;

    #[inline]
    fn as_graph(&self) -> &GraphType<Self> { self.borrow() }

    #[inline]
    fn as_graph_mut(&mut self) -> &mut GraphType<Self> { self.borrow_mut() }
}

impl<I: Clone, W: Clone, P: Clone, Ix: IndexType> GraphMarker for Graph<I, W, P, Ix> {
    type Icon = I;
    type Index = Ix;
    type PortData = P;
    type Widget = W;
}
