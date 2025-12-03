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
    prelude::*, ContinueCx, Cursor, CursorUpdate, EdgeCursor, Port, Side, SidedPort, Status, Step,
    WCursor, WEdgeCursor, WPort, WSidedPort, Yielded,
};

use super::{
    cell::Cell,
    context::{AnyContext, Context},
    status::RenderedStatus,
    view::{Pan, Zoom},
    Change, GraphAction,
};
use crate::{
    drag::DragHandler,
    graph::{Checked, Edge, Graph, WidgetNode},
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

impl<N: WidgetNode> EditorCore<N> {
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

    pub fn set_graph(&mut self, graph: Arc<Graph<N>>, cx: &mut impl Context) -> bool {
        if Arc::ptr_eq(&self.graph, &graph) {
            return false;
        }

        self.cancel_node_drag(None, &mut *cx);
        self.graph = graph;
        cx.request_render();

        true
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

    #[inline]
    fn change(&self, change: Change<N>) -> GraphAction<N> {
        let graph = Arc::clone(&self.graph);
        GraphAction::Changed(
            if cfg!(debug_assertions) {
                Checked::new(graph)
            } else {
                unsafe { Checked::new_unchecked(graph) }
            },
            change,
        )
    }
}

// Node drag behavior
impl<N: WidgetNode> EditorCore<N> {
    #[inline]
    pub fn in_node_drag(&self) -> bool { self.node_drag.in_drag() }

    #[inline]
    pub fn begin_node_drag(
        &mut self,
        pointer: PointerInfo,
        state: &PointerState,
        cx: &mut EventCtx,
    ) {
        let point = self.iview_transform(cx.size()) * cx.local_position(state.position);

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
        cx: &mut EventCtx,
    ) -> impl FnOnce(&(NodeIndex, Point), PhysicalPosition<f64>, PhysicalPosition<f64>) {
        |&(node, start_pos), from, to| {
            let delta = cx.local_position(to) - cx.local_position(from);
            let node = &mut nodes[node];

            let Some(pos) = make_mut!(*node).position_mut() else {
                return;
            };
            let prev = mem::replace(pos, start_pos + delta / zoom.scale());
            if prev != node.position() {
                cx.request_render();
            }
        }
    }

    #[inline]
    pub fn update_node_drag(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        cx: &mut EventCtx,
    ) {
        self.node_drag.update_drag(
            pointer,
            state,
            Self::node_drag_delta(make_mut!(self.graph), &self.zoom, cx),
        );
    }

    #[inline]
    pub fn complete_node_drag(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        cx: &mut EventCtx,
    ) {
        self.node_drag.complete_drag(
            pointer,
            state,
            Self::node_drag_delta(make_mut!(self.graph), &self.zoom, cx),
        );

        cx.submit_action::<GraphAction<N>>(self.change(Change::NodeMoved));
    }

    #[inline]
    pub fn cancel_node_drag(&mut self, pointer: Option<&PointerInfo>, cx: &mut impl Context) {
        self.node_drag.cancel_drag(pointer, |&(n, p)| {
            let node = &mut make_mut!(self.graph)[n];
            let prev = mem::replace(&mut make_mut!(*node).position(), p);
            if prev != node.position() {
                cx.request_render();
            }
        });
    }
}

impl<N: WidgetNode> GraphWidgetTypes for EditorCore<N> {
    type Cell = Cell<N>;
    type Context<'cx, 'widget: 'cx> = AnyContext<'cx, 'widget>;
    type NodeId = NodeIndex;
    type NodeKind = N::Prototype;
    type Point = Point;
    type PortId = u16;

    fn reborrow_cx<'widget, 're>(
        cx: &'re mut Self::Context<'_, 'widget>,
    ) -> Self::Context<'re, 'widget> {
        cx.reborrow()
    }
}

impl<N: WidgetNode> CursorOps for EditorCore<N> {
    fn default_cursor(&self) -> WCursor<Self> {
        self.graph
            .node_references()
            .next()
            .map_or(Cursor::FixedPoint(Point::ZERO), |(k, _)| Cursor::Node(k))
    }

    fn nearest_node_where<F: Fn(&Self::NodeId) -> bool>(
        &self,
        cell: &Self::Cell,
        pred: F,
    ) -> Option<Self::NodeId> {
        let target = cell.node_target(self);
        self.graph
            .node_references()
            .filter(|(i, _)| pred(i))
            .map(|(i, n)| {
                (
                    i,
                    (n.position() + n.outer_size().to_vec2() * 0.5).distance_squared(target),
                )
            })
            .min_by(|(_, d), (_, e)| d.total_cmp(e))
            .map(|(i, _)| i)
    }

    fn nearest_port(
        &self,
        &n: &Self::NodeId,
        side: Side,
        cell: &Self::Cell,
    ) -> Option<Self::PortId> {
        let node = &self.graph[n];
        let len = node.arity(side);
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
    ) -> Option<(NonZeroIsize, Self::PortId)> {
        let SidedPort(side, Port(n, port)) = *port;
        let node = &self.graph[n];

        let res: u16 = usize::from(port)
            .saturating_add_signed(count)
            .min(node.arity(side).saturating_sub(1).into())
            .try_into()
            .unwrap_or_else(|_| unreachable!());

        #[expect(clippy::cast_possible_wrap, reason = "The wrap here is intended")]
        NonZeroIsize::new((res.wrapping_sub(port) as i16).into()).map(|d| (d, res))
    }

    fn nearest_edge_where<F: Fn(&WEdgeCursor<Self>) -> bool>(
        &self,
        port: &WSidedPort<Self>,
        cell: &Self::Cell,
        pred: F,
    ) -> Option<WEdgeCursor<Self>> {
        fn edge_cursor<W: GraphWidgetTypes>(from: WPort<W>, to: WPort<W>) -> WEdgeCursor<W> {
            EdgeCursor {
                from,
                to,
                anchor: Side::In,
            }
        }

        let SidedPort(side, port) = *port;
        let node = &self.graph[port.0];

        match side {
            Side::In => self.graph.edge_references().find_map(|e| {
                let Edge { from_port, to_port } = *e.weight();
                if port != Port(e.target(), to_port) {
                    return None;
                }

                let cur = edge_cursor::<Self>(Port(e.source(), from_port), port);
                pred(&cur).then_some(cur)
            }),
            Side::Out => {
                let pos = cell.edge_target(self);

                self.graph
                    .edge_references()
                    .filter_map(|e| {
                        let Edge { from_port, to_port } = *e.weight();
                        if port != Port(e.source(), from_port) {
                            return None;
                        }

                        let cur = edge_cursor::<Self>(port, Port(e.target(), to_port));
                        if !pred(&cur) {
                            return None;
                        }

                        Some((
                            cur,
                            node.edge_midpoint(port.1, &self.graph[cur.to.0], cur.to.1)
                                .distance_squared(pos),
                        ))
                    })
                    .min_by(|(_, d), (_, e)| d.total_cmp(e))
                    .map(|(e, _)| e)
            },
        }
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

    fn step_point_by(&self, point: &Self::Point, step: Step, count: usize) -> Self::Point {
        #![expect(clippy::cast_precision_loss)]

        const STEP: f64 = 16.0;

        *point
            + (count as f64)
                * match step {
                    Step::Left => Vec2::new(-STEP, 0.0),
                    Step::Down => Vec2::new(0.0, STEP),
                    Step::Up => Vec2::new(0.0, -STEP),
                    Step::Right => Vec2::new(STEP, 0.0),
                }
    }

    fn update_cursor(
        &mut self,
        update: CursorUpdate,
        cursor: &WCursor<Self>,
        mut cx: Self::Context<'_, '_>,
    ) {
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
        cx.request_render();
    }
}

impl<N: WidgetNode> EdgeOps for EditorCore<N> {
    fn create_edge(&mut self, from: WPort<Self>, to: WPort<Self>, mut cx: Self::Context<'_, '_>) {
        make_mut!(self.graph).add_edge(from.0, to.0, Edge {
            from_port: from.1,
            to_port: to.1,
        });
        cx.request_render();
    }

    fn delete_edge(
        &mut self,
        from: &WPort<Self>,
        to: &WPort<Self>,
        mut cx: Self::Context<'_, '_>,
    ) -> bool {
        let Some(e) = self
            .graph
            .edges_connecting(from.0, to.0)
            .find(|e| e.weight().from_port == from.1 && e.weight().to_port == to.1)
        else {
            return false;
        };
        let e = e.id();

        let edge = make_mut!(self.graph)
            .remove_edge(e)
            .unwrap_or_else(|| unreachable!());

        cx.submit_action(self.change(Change::EdgeDeleted(from.0, to.0, edge)));
        cx.request_render();

        true
    }
}

impl<N: WidgetNode> NodeOps for EditorCore<N> {
    fn prompt_node_kind<'a, Y, C: ContinueOnce<Self, Y, Option<Self::NodeKind>>>(
        &'a mut self,
        then: Yielded<'a, '_, Self, Y, C>,
    ) {
        if let Some(p) = N::override_prototype() {
            then.resume_now(self, p.ok());
        } else {
            let (then, mut cx) = then.defer();
            cx.submit_action(GraphAction::<N>::WantNodePrototype(Box::new(
                |value, mut widget| {
                    then.continue_once(
                        value,
                        ContinueCx::new_deferred(
                            &mut widget.widget.core,
                            &mut widget.widget.driver,
                            (&mut widget.ctx).into(),
                        ),
                    );
                },
            )));
        }
    }

    fn create_node(
        &mut self,
        kind: Self::NodeKind,
        position: Self::Point,
        mut cx: Self::Context<'_, '_>,
    ) {
        make_mut!(self.graph).add_node(N::create(kind, position).into());

        cx.submit_action(self.change(Change::NodeCreated));
        cx.request_render();
    }

    fn delete_node(&mut self, &node: &Self::NodeId, mut cx: Self::Context<'_, '_>) -> bool {
        let Some(n) = make_mut!(self.graph).remove_node(node) else {
            return false;
        };

        cx.submit_action(self.change(Change::NodeDeleted(n)));
        cx.request_render();

        true
    }
}

impl<N: WidgetNode> UiOps for EditorCore<N> {
    fn update_status(&mut self, status: Status, mut cx: Self::Context<'_, '_>) {
        let rendered = RenderedStatus::new(status);
        cx.mutate_later(&mut self.statusbar, move |b| rendered.update(b));
    }

    fn quit(&mut self, mut cx: Self::Context<'_, '_>) { cx.exit(); }
}
