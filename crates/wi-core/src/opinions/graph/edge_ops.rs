use std::{cmp::Ordering, num::NonZeroI32, sync::Arc};

use petgraph::{
    graph::NodeIndex,
    stable_graph::EdgeReference,
    visit::{EdgeRef, IntoEdgeReferences},
};

use super::{Edge, Graph};
use crate::{EdgeCursor, Port, Side, SidedPort};

pub fn min_edge_by<
    N,
    T,
    F: Fn(&EdgeCursor<NodeIndex, u16>) -> bool,
    M: Fn(&Arc<N>, u16, &Arc<N>, u16) -> T,
    C: Fn(&T, &T) -> Ordering,
>(
    graph: &Graph<N>,
    port: SidedPort<NodeIndex, u16>,
    pred: F,
    map: M,
    cmp: C,
) -> Option<EdgeCursor<NodeIndex, u16>> {
    fn edge_cursor(
        from: Port<NodeIndex, u16>,
        to: Port<NodeIndex, u16>,
    ) -> EdgeCursor<NodeIndex, u16> {
        EdgeCursor {
            from,
            to,
            anchor: Side::In,
        }
    }

    let SidedPort(side, port) = port;
    let node = &graph[port.0];

    match side {
        Side::In => graph.edge_references().find_map(|e| {
            let Edge { from_port, to_port } = *e.weight();
            if port != Port(e.target(), to_port) {
                return None;
            }

            let cur = edge_cursor(Port(e.source(), from_port), port);
            pred(&cur).then_some(cur)
        }),
        Side::Out => graph
            .edge_references()
            .filter_map(|e| {
                let Edge { from_port, to_port } = *e.weight();
                if port != Port(e.source(), from_port) {
                    return None;
                }

                let cur = edge_cursor(port, Port(e.target(), to_port));
                if !pred(&cur) {
                    return None;
                }

                Some((cur, map(node, port.1, &graph[cur.to.0], cur.to.1)))
            })
            .min_by(|(_, d), (_, e)| cmp(d, e))
            .map(|(e, _)| e),
    }
}

pub fn step_edge_by<N, T, M: Fn(&Arc<N>, u16, &Arc<N>, u16) -> T, C: Fn(&T, &T) -> Ordering>(
    graph: &Graph<N>,
    edge: EdgeCursor<NodeIndex, u16>,
    count: i32,
    map: M,
    cmp: C,
) -> Option<(NonZeroI32, Port<NodeIndex, u16>)> {
    let (anchor, &port) = edge.anchor();
    let node = &graph[port.0];
    let mut ports: Vec<_> = match anchor {
        Side::In => return None,
        Side::Out => graph
            .edge_references()
            .filter(|e| edge.from == Port(e.source(), e.weight().from_port))
            .map(|e| {
                let w = e.weight();
                (
                    Port(e.target(), w.to_port),
                    map(node, w.from_port, &graph[e.target()], w.to_port),
                    // node.edge_midpoint(w.from_port, &graph[e.target()], w.to_port),
                )
            })
            .collect(),
    };

    ports.sort_unstable_by(|(p1, o1), (p2, o2)| {
        cmp(o1, o2)
            .then_with(|| p1.0.cmp(&p2.0))
            .then_with(|| p1.1.cmp(&p2.1))
    });

    let port = *edge.free_port();
    let idx = ports
        .iter()
        .enumerate()
        .find_map(|(i, (p, _))| (*p == port).then_some(i))
        .unwrap();

    let res = idx
        .saturating_add_signed(isize::try_from(count).unwrap_or(if count.is_negative() {
            isize::MIN
        } else {
            isize::MAX
        }))
        .min(ports.len().checked_sub(1)?);

    #[expect(clippy::cast_possible_wrap, reason = "The wrap here is intended")]
    NonZeroI32::new(res.wrapping_sub(idx) as i32).map(|d| (d, ports[res].0))
}

pub fn create_edge<N>(graph: &mut Graph<N>, from: Port<NodeIndex, u16>, to: Port<NodeIndex, u16>) {
    graph.add_edge(from.0, to.0, Edge {
        from_port: from.1,
        to_port: to.1,
    });
}

#[must_use]
pub fn find_edge<N>(
    graph: &Graph<N>,
    from: Port<NodeIndex, u16>,
    to: Port<NodeIndex, u16>,
) -> Option<EdgeReference<'_, Edge>> {
    graph
        .edges_connecting(from.0, to.0)
        .find(|e| e.weight().from_port == from.1 && e.weight().to_port == to.1)
}
