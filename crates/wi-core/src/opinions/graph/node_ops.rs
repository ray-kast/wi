use std::{cmp::Ordering, num::NonZeroIsize, sync::Arc};

use petgraph::{graph::NodeIndex, visit::IntoNodeReferences};

use super::Graph;

pub fn min_node_by<
    N,
    T,
    F: Fn(&NodeIndex) -> bool,
    M: Fn(&Arc<N>) -> T,
    C: Fn(&T, &T) -> Ordering,
>(
    graph: &Graph<N>,
    pred: F,
    map: M,
    cmp: C,
) -> Option<NodeIndex> {
    graph
        .node_references()
        .filter(|(i, _)| pred(i))
        .map(|(i, n)| (i, map(n)))
        .min_by(|(_, a), (_, b)| cmp(a, b))
        .map(|(i, _)| i)
}

#[must_use]
pub fn step_port_by(port: u16, arity: u16, steps: isize) -> Option<(NonZeroIsize, u16)> {
    let res: u16 = usize::from(port)
        .saturating_add_signed(steps)
        .min(arity.saturating_sub(1).into())
        .try_into()
        .unwrap_or_else(|_| unreachable!());

    #[expect(clippy::cast_possible_wrap, reason = "The wrap here is intended")]
    NonZeroIsize::new((res.wrapping_sub(port) as i16).into()).map(|d| (d, res))
}
