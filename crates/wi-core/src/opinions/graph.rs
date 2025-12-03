use std::{
    borrow::{Borrow, Cow},
    ops::Deref,
    sync::Arc,
};

use derive_where::derive_where;
use petgraph::{prelude::*, visit::IntoEdgeReferences};
use smallvec::SmallVec;
use wi_macros::impl_enum;

use crate::Side;

pub type Graph<N> = StableDiGraph<Arc<N>, Edge>;

pub trait NodeStyleArity {
    fn in_arity(&self) -> u16;
    fn out_arity(&self) -> u16;
}

type InPort<'n, I, P, W> = (u16, Port<'n, &'n P, Option<&'n WidgetLabel<'n, I, W>>>);
type OutPort<'n, I, P> = (u16, Port<'n, &'n P, Option<&'n Label<'n, I>>>);

pub trait NodeStyle<I, P, W>: NodeStyleArity {
    fn in_ports<'n>(&'n self) -> impl Iterator<Item = InPort<'n, I, P, W>>
    where
        I: 'n,
        P: 'n,
        W: 'n;

    fn out_ports<'n>(&'n self) -> impl Iterator<Item = OutPort<'n, I, P>>
    where
        I: 'n,
        P: 'n,
        W: 'n;
}

pub trait Node {
    type Prototype: Clone;
    type Position;
    type Icon;
    type PortShape;
    type Widget;

    #[expect(clippy::must_use_candidate)]
    fn override_prototype() -> Option<Result<Self::Prototype, ()>> { None }

    fn create(proto: Self::Prototype, position: Self::Position) -> Self;

    fn name(&self) -> Cow<'_, str>;
    fn description(&self) -> Cow<'_, str>;

    fn position(&self) -> Self::Position;
    fn position_mut(&mut self) -> Option<&mut Self::Position>;

    #[inline]
    fn arity(&self, side: Side) -> u16 {
        match side {
            Side::In => self.style().in_arity(),
            Side::Out => self.style().out_arity(),
        }
    }

    fn style(&self) -> StyleKind<'_, Self::Icon, Self::PortShape, Self::Widget>;
}

#[impl_enum]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StyleKind<'n, I, P, W> {
    Large(LargeStyle<'n, I, P, W>),
    Small(SmallStyle<'n, I, P>),
    Widget(WidgetStyle<'n, I, P, W>),
}

#[impl_enum]
impl<'n, I, P, W> NodeStyleArity for StyleKind<'n, I, P, W> {
    fn in_arity(&self) -> u16 { dispatch!(self) }

    fn out_arity(&self) -> u16 { dispatch!(self) }
}

#[impl_enum]
impl<'n, I, P, W> NodeStyle<I, P, W> for StyleKind<'n, I, P, W> {
    fn in_ports<'m>(
        &'m self,
    ) -> impl Iterator<Item = (u16, Port<'m, &'m P, Option<&'m WidgetLabel<'m, I, W>>>)>
    where
        I: 'm,
        P: 'm,
        W: 'm,
    {
        // TODO: enum dispatch, maybe
        dispatch_map!(|i| Box::new(i) as Box<dyn Iterator<Item = _>>; self)
    }

    fn out_ports<'m>(
        &'m self,
    ) -> impl Iterator<Item = (u16, Port<'m, &'m P, Option<&'m Label<'m, I>>>)>
    where
        I: 'm,
        P: 'm,
        W: 'm,
    {
        // TODO: enum dispatch, maybe
        dispatch_map!(|i| Box::new(i) as Box<dyn Iterator<Item = _>>; self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive_where(Default; )]
pub struct LargeStyle<'n, I, P, W> {
    pub label: Label<'n, I>,
    pub in_ports: Ports<'n, Port<'n, P, WidgetLabel<'n, I, W>>>,
    pub out_ports: Ports<'n, Port<'n, P, Label<'n, I>>>,
}

const IN_ARITY_OVERFLOW: &str = "Node input arity overflow";
const OUT_ARITY_OVERFLOW: &str = "Node output arity overflow";

impl<I, P, W> NodeStyleArity for LargeStyle<'_, I, P, W> {
    #[inline]
    fn in_arity(&self) -> u16 { self.in_ports.len().try_into().expect(IN_ARITY_OVERFLOW) }

    #[inline]
    fn out_arity(&self) -> u16 { self.out_ports.len().try_into().expect(OUT_ARITY_OVERFLOW) }
}

impl<I, P, W> NodeStyle<I, P, W> for LargeStyle<'_, I, P, W> {
    fn in_ports<'n>(
        &'n self,
    ) -> impl Iterator<Item = (u16, Port<'n, &'n P, Option<&'n WidgetLabel<'n, I, W>>>)>
    where
        I: 'n,
        P: 'n,
        W: 'n,
    {
        self.in_ports.iter().enumerate().map(|(i, p)| {
            (
                i.try_into().expect(IN_ARITY_OVERFLOW),
                p.as_ref().map_label(Some),
            )
        })
    }

    fn out_ports<'n>(
        &'n self,
    ) -> impl Iterator<Item = (u16, Port<'n, &'n P, Option<&'n Label<'n, I>>>)>
    where
        I: 'n,
        P: 'n,
        W: 'n,
    {
        self.out_ports.iter().enumerate().map(|(i, p)| {
            (
                i.try_into().expect(OUT_ARITY_OVERFLOW),
                p.as_ref().map_label(Some),
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive_where(Default; )]
pub struct SmallStyle<'n, I, P> {
    pub label: Label<'n, I>,
    pub in_ports: Ports<'n, Port<'n, P, ()>>,
    pub out_ports: Ports<'n, Port<'n, P, ()>>,
}

impl<I, P> NodeStyleArity for SmallStyle<'_, I, P> {
    fn in_arity(&self) -> u16 {
        self.in_ports
            .len()
            .try_into()
            .expect("Node input arity overflow")
    }

    fn out_arity(&self) -> u16 {
        self.out_ports
            .len()
            .try_into()
            .expect("Node output arity overflow")
    }
}

impl<I, P, W> NodeStyle<I, P, W> for SmallStyle<'_, I, P> {
    fn in_ports<'n>(
        &'n self,
    ) -> impl Iterator<Item = (u16, Port<'n, &'n P, Option<&'n WidgetLabel<'n, I, W>>>)>
    where
        I: 'n,
        P: 'n,
        W: 'n,
    {
        self.in_ports.iter().enumerate().map(|(i, p)| {
            (
                i.try_into().expect(IN_ARITY_OVERFLOW),
                p.as_ref().map_label(|()| None),
            )
        })
    }

    fn out_ports<'n>(
        &'n self,
    ) -> impl Iterator<Item = (u16, Port<'n, &'n P, Option<&'n Label<'n, I>>>)>
    where
        I: 'n,
        P: 'n,
        W: 'n,
    {
        self.out_ports.iter().enumerate().map(|(i, p)| {
            (
                i.try_into().expect(OUT_ARITY_OVERFLOW),
                p.as_ref().map_label(|()| None),
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive_where(Default; )]
pub struct WidgetStyle<'n, I, P, W> {
    pub label: WidgetLabel<'n, I, W>,
    pub in_port: Option<Port<'n, P, ()>>,
    pub out_port: Option<Port<'n, P, ()>>,
}

impl<I, P, W> NodeStyleArity for WidgetStyle<'_, I, P, W> {
    fn in_arity(&self) -> u16 { self.in_port.as_ref().map_or(0, |_| 1) }

    fn out_arity(&self) -> u16 { self.out_port.as_ref().map_or(0, |_| 1) }
}

impl<I, P, W> NodeStyle<I, P, W> for WidgetStyle<'_, I, P, W> {
    fn in_ports<'n>(
        &'n self,
    ) -> impl Iterator<Item = (u16, Port<'n, &'n P, Option<&'n WidgetLabel<'n, I, W>>>)>
    where
        I: 'n,
        P: 'n,
        W: 'n,
    {
        self.in_port
            .as_ref()
            .map(|p| (0, p.as_ref().map_label(|()| None)))
            .into_iter()
    }

    fn out_ports<'n>(
        &'n self,
    ) -> impl Iterator<Item = (u16, Port<'n, &'n P, Option<&'n Label<'n, I>>>)>
    where
        I: 'n,
        P: 'n,
        W: 'n,
    {
        self.out_port
            .as_ref()
            .map(|p| (0, p.as_ref().map_label(|()| None)))
            .into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WidgetLabel<'n, I, W> {
    Label(Label<'n, I>),
    Widget(W),
}

impl<I, W> WidgetLabel<'_, I, W> {
    pub fn as_ref(&self) -> WidgetLabel<'_, &I, &W> {
        match self {
            Self::Label(l) => WidgetLabel::Label(l.as_ref()),
            Self::Widget(w) => WidgetLabel::Widget(w),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[derive_where(Default; )]
pub struct Label<'n, I> {
    pub content: Option<Cow<'n, str>>,
    pub icon: Option<I>,
}

impl<I> Label<'_, I> {
    pub fn as_ref(&self) -> Label<'_, &I> {
        Label {
            content: self.content.as_ref().map(|s| Cow::Borrowed(s.as_ref())),
            icon: self.icon.as_ref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Ports<'n, P> {
    Slice(&'n [P]),
    Single(Option<P>),
    Vec(Vec<P>),
    // This causes StyleKind to become invariant over 'n due to what looks like a trait solver bug
    // Vec(SmallVec<[P; 1]>),
}

impl<'a, P> From<&'a [P]> for Ports<'a, P> {
    #[inline]
    fn from(value: &'a [P]) -> Self { Self::Slice(value) }
}

impl<P> From<SmallVec<[P; 1]>> for Ports<'_, P> {
    #[inline]
    fn from(value: SmallVec<[P; 1]>) -> Self {
        // Self::Vec(value)
        match value.into_inner() {
            Ok([port]) => Self::Single(Some(port)),
            Err(v) if v.is_empty() => Self::Single(None),
            Err(v) => Self::Vec(v.into_vec()),
        }
    }
}

impl<P> From<[P; 1]> for Ports<'_, P> {
    #[inline]
    fn from(value: [P; 1]) -> Self {
        // Self::Vec(SmallVec::from_buf(value))
        let [value] = value;
        Self::Single(Some(value))
    }
}

impl<P> Deref for Ports<'_, P> {
    type Target = [P];

    fn deref(&self) -> &Self::Target {
        match self {
            Ports::Slice(s) => s,
            Ports::Single(p) => p.as_slice(),
            Ports::Vec(v) => v,
            // Ports::Vec(v) => v,
        }
    }
}

impl<P> AsRef<[P]> for Ports<'_, P> {
    #[inline]
    fn as_ref(&self) -> &[P] { self }
}

impl<P> Borrow<[P]> for Ports<'_, P> {
    #[inline]
    fn borrow(&self) -> &[P] { self }
}

impl<P> Default for Ports<'_, P> {
    #[inline]
    fn default() -> Self {
        // Self::Vec(SmallVec::new_const())
        Self::Single(None)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct Port<'n, S, L> {
    pub name: Cow<'n, str>,
    pub description: Cow<'n, str>,
    pub shape: S,
    pub label: L,
}

impl<'n, S, L> Port<'n, S, L> {
    pub fn as_ref(&self) -> Port<'_, &S, &L> {
        Port {
            name: Cow::Borrowed(&self.name),
            description: Cow::Borrowed(&self.description),
            shape: &self.shape,
            label: &self.label,
        }
    }

    pub fn map_label<M, F: FnOnce(L) -> M>(self, f: F) -> Port<'n, S, M> {
        Port {
            name: self.name,
            description: self.description,
            shape: self.shape,
            label: f(self.label),
        }
    }
}

impl<I, W> Default for WidgetLabel<'_, I, W> {
    #[inline]
    fn default() -> Self { Self::Label(Label::default()) }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Edge {
    pub from_port: u16,
    pub to_port: u16,
}

pub const PORT_MAX: u16 = u16::MAX;
#[inline]
#[expect(clippy::must_use_candidate, reason = "Using this is impossible")]
pub fn port_overflow<T>() -> T { panic!("Port number exceeded {PORT_MAX}") }

#[derive(Debug)]
pub struct Checked<G>(Arc<G>);

impl<G> Clone for Checked<G> {
    #[inline]
    fn clone(&self) -> Self { Self(Arc::clone(&self.0)) }
}

impl<G> Checked<G> {
    #[expect(
        clippy::missing_safety_doc,
        reason = "WIP, adding this will suppress missing-docs warnings"
    )]
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(graph: Arc<G>) -> Self { Self(graph) }

    #[must_use]
    pub fn into_inner(self) -> Arc<G> { self.0 }
}

impl<N: Node> Checked<Graph<N>> {
    #[must_use]
    pub fn new(graph: Arc<Graph<N>>) -> Self {
        for edge in graph.edge_references() {
            let weight = edge.weight();

            assert!(
                weight.from_port < graph[edge.source()].arity(Side::Out),
                "Invalid edge source port index"
            );
            assert!(
                weight.to_port < graph[edge.target()].arity(Side::In),
                "Invalid edge target port index"
            );
        }

        Self(graph)
    }
}
