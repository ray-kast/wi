use std::{collections::BTreeMap, mem, num::NonZeroIsize};

use masonry::{
    core::{EventCtx, PointerInfo, PointerState, WidgetPod},
    kurbo::{Affine, Point, Rect, Size, Vec2},
    widgets::Flex,
};
use wi_core::{
    status::Status, Cursor, CursorUpdate, EdgeCursor, GraphWidget, Port, Side, SidedPort, WCursor,
    WEdgeCursor, WPort, WSidedPort,
};
use xilem::dpi::PhysicalPosition;

use super::{
    cell::Cell,
    drag::DragHandler,
    node::Node,
    status::RenderedStatus,
    view::{Pan, Zoom},
};

pub struct GraphCore {
    pub nodes: BTreeMap<usize, Node>,
    node_drag: DragHandler<(usize, Point)>,
    pub pan: Pan,
    pub zoom: Zoom,
    pub statusbar: WidgetPod<Flex>,
}

impl GraphCore {
    pub fn new(graph: &crate::GraphView) -> Self {
        let mut out_edges = graph.out_edge_map();

        let nodes: BTreeMap<_, _> = graph
            .nodes
            .iter()
            .map(|(&i, n)| {
                (i, Node {
                    pos: n.pos,
                    in_edges: n
                        .in_edges
                        .iter()
                        .map(|p| p.map(|p| Port(p.node, p.port)))
                        .collect(),
                    out_edges: out_edges
                        .remove(&i)
                        .unwrap_or_else(|| unreachable!())
                        .into_iter()
                        .map(|p| p.into_iter().map(|p| Port(p.node, p.port)).collect())
                        .collect(),
                })
            })
            .collect();

        let pan = nodes
            .values()
            .fold(None, |r: Option<Rect>, n| {
                Some(if let Some(rect) = r {
                    rect.union(n.rect())
                } else {
                    n.rect()
                })
            })
            .map_or(Vec2::ZERO, |r| r.center().to_vec2());

        Self {
            nodes,
            node_drag: DragHandler::new(),
            pan: Pan::new(pan),
            zoom: Zoom::default(),
            statusbar: RenderedStatus::new(Status::default()).create(),
        }
    }

    #[inline]
    pub fn view_transform(&self, size: Size) -> Affine {
        Affine::scale_about(self.zoom.scale(), (size.to_vec2() * 0.5).to_point())
            * Affine::translate(self.pan.translation(size))
    }

    fn iview_transform(&self, size: Size) -> Affine {
        Affine::translate(-self.pan.translation(size))
            * Affine::scale_about(self.zoom.scale().recip(), (size.to_vec2() * 0.5).to_point())
    }
}

// Node drag behavior
impl GraphCore {
    #[inline]
    pub fn in_node_drag(&self) -> bool { self.node_drag.in_drag() }

    #[inline]
    pub fn begin_node_drag(
        &mut self,
        pointer: PointerInfo,
        state: &PointerState,
        ctx: &mut EventCtx,
    ) {
        let point = self.iview_transform(ctx.size()) * ctx.local_position(state.position);

        let Some(node) = self
            .nodes
            .iter()
            .rev()
            .find_map(|(&k, v)| v.rect().contains(point).then_some(k))
        else {
            return;
        };

        self.node_drag
            .begin_drag(pointer, state, (node, self.nodes[&node].pos));
    }

    #[inline]
    fn node_drag_delta(
        nodes: &mut BTreeMap<usize, Node>,
        zoom: &Zoom,
        ctx: &mut EventCtx,
    ) -> impl FnOnce(&(usize, Point), PhysicalPosition<f64>, PhysicalPosition<f64>) {
        |&(ref node, start_pos), from, to| {
            let delta = ctx.local_position(to) - ctx.local_position(from);
            let node = nodes.get_mut(node).unwrap_or_else(|| unreachable!());

            let prev = mem::replace(&mut node.pos, start_pos + delta / zoom.scale());
            if prev != node.pos {
                ctx.request_render();
            }
        }
    }

    #[inline]
    pub fn update_node_drag(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        ctx: &mut EventCtx,
    ) {
        self.node_drag.update_drag(
            pointer,
            state,
            Self::node_drag_delta(&mut self.nodes, &self.zoom, ctx),
        );
    }

    #[inline]
    pub fn complete_node_drag(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        ctx: &mut EventCtx,
    ) {
        self.node_drag.complete_drag(
            pointer,
            state,
            Self::node_drag_delta(&mut self.nodes, &self.zoom, ctx),
        );
    }

    #[inline]
    pub fn cancel_node_drag(&mut self, pointer: Option<&PointerInfo>, ctx: &mut EventCtx) {
        self.node_drag.cancel_drag(pointer, |&(ref n, p)| {
            let node = self.nodes.get_mut(n).unwrap_or_else(|| unreachable!());
            let prev = mem::replace(&mut node.pos, p);
            if prev != node.pos {
                ctx.request_render();
            }
        });
    }
}

impl GraphWidget for GraphCore {
    type Cell = Cell;
    type Context<'a> = EventCtx<'a>;
    type Node = usize;
    type Point = Point;
    type PortIdx = usize;

    fn default_cursor(&self) -> WCursor<Self> {
        self.nodes
            .first_key_value()
            .map_or(Cursor::FixedPoint(Point::ZERO), |(&k, _)| Cursor::Node(k))
    }

    fn nearest_port(&self, n: &Self::Node, side: Side, cell: &Self::Cell) -> Option<Self::PortIdx> {
        let node = &self.nodes[n];
        let len = match side {
            Side::In => node.in_edges.len(),
            Side::Out => node.out_edges.len(),
        };
        let pos = cell.point(self);

        let (port, _) = (0..len)
            .map(|p| (p, node.port_pos(p, side).distance_squared(pos)))
            .min_by(|(_, d), (_, e)| d.total_cmp(e))?;

        Some(port)
    }

    fn step_port_by(
        &self,
        port: &WSidedPort<Self>,
        count: isize,
    ) -> Option<(NonZeroIsize, Self::PortIdx)> {
        let SidedPort(side, Port(n, port)) = *port;
        let node = &self.nodes[&n];

        let res = port.saturating_add_signed(count).min(
            match side {
                Side::In => node.in_edges.len(),
                Side::Out => node.out_edges.len(),
            }
            .saturating_sub(1),
        );

        #[expect(clippy::cast_possible_wrap, reason = "The wrap here is intended")]
        NonZeroIsize::new(res.wrapping_sub(port) as isize).map(|d| (d, res))
    }

    fn nearest_edge(
        &self,
        port: &WSidedPort<Self>,
        cell: &Self::Cell,
    ) -> Option<WEdgeCursor<Self>> {
        let SidedPort(side, port) = *port;
        let node = &self.nodes[&port.0];

        let (from, to) = match side {
            Side::In => (node.in_edges[port.1]?, port),
            Side::Out => {
                let pos = cell.point(self);
                (
                    port,
                    node.out_edges[port.1]
                        .iter()
                        .map(|&p| {
                            (
                                p,
                                node.edge_midpoint(port.1, &self.nodes[&p.0], p.1)
                                    .distance_squared(pos),
                            )
                        })
                        .min_by(|(_, d), (_, e)| d.total_cmp(e))?
                        .0,
                )
            },
        };

        Some(EdgeCursor {
            from,
            to,
            anchor: Side::In,
        })
    }

    fn step_edge_by(
        &self,
        edge: &WEdgeCursor<Self>,
        count: isize,
    ) -> Option<(NonZeroIsize, WPort<Self>)> {
        let (anchor, &port) = edge.anchor_port();
        let node = &self.nodes[&port.0];
        let ports = match anchor {
            Side::In => return None,
            Side::Out => &node.out_edges[port.1],
        };

        let mut sorted: Vec<_> = ports
            .iter()
            .map(|&p| {
                let n = &self.nodes[&p.0];
                (p, match anchor {
                    Side::In => n.edge_midpoint(p.1, node, port.1),
                    Side::Out => node.edge_midpoint(port.1, n, p.1),
                })
            })
            .collect();
        sorted.sort_unstable_by(|(p1, o1), (p2, o2)| {
            o1.y.total_cmp(&o2.y)
                .then_with(|| o1.x.total_cmp(&o2.x))
                .then_with(|| p1.0.cmp(&p2.0))
                .then_with(|| p1.1.cmp(&p2.1))
        });

        let port = *edge.free_port();
        let idx = sorted
            .iter()
            .enumerate()
            .find_map(|(i, (p, _))| (*p == port).then_some(i))
            .unwrap();

        let res = idx
            .saturating_add_signed(count)
            .min(sorted.len().checked_sub(1)?);

        #[expect(clippy::cast_possible_wrap, reason = "The wrap here is intended")]
        NonZeroIsize::new(res.wrapping_sub(idx) as isize).map(|d| (d, sorted[res].0))
    }

    fn update_cursor(&mut self, update: CursorUpdate, cursor: &WCursor<Self>, ctx: &mut EventCtx) {
        match update {
            CursorUpdate::Move => (),
            CursorUpdate::CenterInView => {
                self.pan.center_on(match *cursor {
                    Cursor::Node(n) => self.nodes[&n].rect().center(),
                    Cursor::Port(SidedPort(s, Port(n, p))) => self.nodes[&n].port_pos(p, s),
                    Cursor::Edge(e) => {
                        self.nodes[&e.from.0].edge_midpoint(e.from.1, &self.nodes[&e.to.0], e.to.1)
                    },
                    Cursor::FixedPoint(p) => p,
                });
                self.zoom.reset();
            },
        }
        ctx.request_render();
    }

    fn update_status(&mut self, status: Status, ctx: &mut EventCtx) {
        let rendered = RenderedStatus::new(status);
        ctx.mutate_later(&mut self.statusbar, move |b| rendered.update(b));
    }
}
