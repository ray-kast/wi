use std::{mem, num::NonZeroIsize, sync::Arc};

use masonry::{
    core::{EventCtx, PointerInfo, PointerState, WidgetPod},
    dpi::PhysicalPosition,
    kurbo::{Affine, Point, Rect, Size, Vec2},
    widgets::Flex,
};
use petgraph::{
    prelude::*,
    visit::{IntoEdgeReferences, IntoNodeReferences},
};
use wi_core::{
    Cursor, CursorUpdate, EdgeCursor, GraphWidget, Port, Side, SidedPort, Status, WCursor,
    WEdgeCursor, WPort, WSidedPort,
};

use super::{
    cell::Cell,
    status::RenderedStatus,
    view::{Pan, Zoom},
};
use crate::{
    drag::DragHandler,
    graph::{Graph, Node},
    widget::node::NodeExt,
};

pub struct EditorCore<N> {
    pub graph: Arc<Graph<N>>,
    node_drag: DragHandler<(NodeIndex, Point)>,
    pub pan: Pan,
    pub zoom: Zoom,
    pub statusbar: WidgetPod<Flex>,
}

macro_rules! make_mut {
    ($expr:expr) => {
        Arc::make_mut(&mut $expr)
    };
}

impl<N: Node> EditorCore<N> {
    pub fn new(graph: Arc<Graph<N>>) -> Self {
        let pan = graph
            .node_weights()
            .fold(None, |r: Option<Rect>, n| {
                Some(if let Some(rect) = r {
                    rect.union(n.rect())
                } else {
                    n.rect()
                })
            })
            .map_or(Vec2::ZERO, |r| r.center().to_vec2());

        Self {
            graph,
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
impl<N: Node> EditorCore<N> {
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
            .graph
            .node_references()
            .rev()
            .find_map(|(k, v)| v.rect().contains(point).then_some(k))
        else {
            return;
        };

        self.node_drag
            .begin_drag(pointer, state, (node, self.graph[node].position()));
    }

    #[inline]
    fn node_drag_delta(
        nodes: &mut Graph<N>,
        zoom: &Zoom,
        ctx: &mut EventCtx,
    ) -> impl FnOnce(&(NodeIndex, Point), PhysicalPosition<f64>, PhysicalPosition<f64>) {
        |&(node, start_pos), from, to| {
            let delta = ctx.local_position(to) - ctx.local_position(from);
            let node = &mut nodes[node];

            let Some(pos) = make_mut!(*node).position_mut() else {
                return;
            };
            let prev = mem::replace(pos, start_pos + delta / zoom.scale());
            if prev != node.position() {
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
            Self::node_drag_delta(make_mut!(self.graph), &self.zoom, ctx),
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
            Self::node_drag_delta(make_mut!(self.graph), &self.zoom, ctx),
        );
    }

    #[inline]
    pub fn cancel_node_drag(&mut self, pointer: Option<&PointerInfo>, ctx: &mut EventCtx) {
        self.node_drag.cancel_drag(pointer, |&(n, p)| {
            let node = &mut make_mut!(self.graph)[n];
            let prev = mem::replace(&mut make_mut!(*node).position(), p);
            if prev != node.position() {
                ctx.request_render();
            }
        });
    }
}

impl<N: Node> GraphWidget for EditorCore<N> {
    type Cell = Cell<N>;
    type Context<'a> = EventCtx<'a>;
    type Node = NodeIndex;
    type Point = Point;
    type PortIdx = u16;

    fn default_cursor(&self) -> WCursor<Self> {
        self.graph
            .node_references()
            .next()
            .map_or(Cursor::FixedPoint(Point::ZERO), |(k, _)| Cursor::Node(k))
    }

    fn nearest_port(
        &self,
        &n: &Self::Node,
        side: Side,
        cell: &Self::Cell,
    ) -> Option<Self::PortIdx> {
        let node = &self.graph[n];
        let len = match side {
            Side::In => node.in_arity(),
            Side::Out => node.out_arity(),
        };
        let pos = cell.port_target(self, side);

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
        let node = &self.graph[n];

        let res: u16 = usize::from(port)
            .saturating_add_signed(count)
            .min(
                match side {
                    Side::In => node.in_arity(),
                    Side::Out => node.out_arity(),
                }
                .saturating_sub(1)
                .into(),
            )
            .try_into()
            .unwrap_or_else(|_| unreachable!());

        #[expect(clippy::cast_possible_wrap, reason = "The wrap here is intended")]
        NonZeroIsize::new((res.wrapping_sub(port) as i16).into()).map(|d| (d, res))
    }

    fn nearest_edge(
        &self,
        port: &WSidedPort<Self>,
        cell: &Self::Cell,
    ) -> Option<WEdgeCursor<Self>> {
        let SidedPort(side, port) = *port;
        let node = &self.graph[port.0];

        let (from, to) = match side {
            Side::In => (
                self.graph
                    .edge_references()
                    .find(|e| port == Port(e.target(), e.weight().to_port))
                    .map(|e| Port(e.source(), e.weight().from_port))?,
                port,
            ),
            Side::Out => {
                let pos = cell.edge_target(self);
                (
                    port,
                    self.graph
                        .edge_references()
                        .filter(|e| port == Port(e.source(), e.weight().from_port))
                        .map(|e| {
                            let w = e.weight();
                            (
                                Port(e.target(), w.to_port),
                                node.edge_midpoint(w.from_port, &self.graph[e.target()], w.to_port)
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
        let node = &self.graph[port.0];
        let mut ports: Vec<_> = match anchor {
            Side::In => return None,
            Side::Out => self
                .graph
                .edge_references()
                .filter(|e| edge.from == Port(e.source(), e.weight().from_port))
                .map(|e| {
                    let w = e.weight();
                    (
                        Port(e.target(), w.to_port),
                        node.edge_midpoint(w.from_port, &self.graph[e.target()], w.to_port),
                    )
                })
                .collect(),
        };

        ports.sort_unstable_by(|(p1, o1), (p2, o2)| {
            o1.y.total_cmp(&o2.y)
                .then_with(|| o1.x.total_cmp(&o2.x))
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
            .saturating_add_signed(count)
            .min(ports.len().checked_sub(1)?);

        #[expect(clippy::cast_possible_wrap, reason = "The wrap here is intended")]
        NonZeroIsize::new(res.wrapping_sub(idx) as isize).map(|d| (d, ports[res].0))
    }

    fn update_cursor(&mut self, update: CursorUpdate, cursor: &WCursor<Self>, ctx: &mut EventCtx) {
        match update {
            CursorUpdate::Move => (),
            CursorUpdate::CenterInView => {
                self.pan.center_on(match *cursor {
                    Cursor::Node(n) => self.graph[n].rect().center(),
                    Cursor::Port(SidedPort(s, Port(n, p))) => self.graph[n].port_pos(p, s),
                    Cursor::Edge(e) => {
                        self.graph[e.from.0].edge_midpoint(e.from.1, &self.graph[e.to.0], e.to.1)
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
