use std::{marker::PhantomData, num::NonZeroI32, sync::Arc};

use petgraph::{
    graph::NodeIndex,
    visit::{EdgeRef, IntoNodeReferences},
};
use wi_core::{
    make_mut,
    opinions::graph::{helpers, Checked, Graph, Node},
    traits::{CursorOps, EdgeOps, GraphWidgetTypes, NodeOps, UiOps},
    ContinueCx, ContinueOnce, Cursor, CursorUpdate, Port, Side, SidedPort, Status, Step, WCursor,
    WEdgeCursor, WPort, WSidedPort, Yielded,
};

use super::{node::NodeExt, GraphEditor};
use crate::{
    graph::TuiNode,
    vector::{Point, Vector},
};

#[derive(Debug)]
pub struct Cx<'w> {
    pub render_requested: bool,
    pub quit_requested: bool,
    pub graph_changed: bool,
    _p: PhantomData<&'w ()>,
}

impl Cx<'_> {
    #[expect(clippy::new_without_default)]
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            render_requested: false,
            quit_requested: false,
            graph_changed: false,
            _p: PhantomData,
        }
    }
}

pub type PrototypeCallback<N> =
    Box<dyn FnOnce(Option<<N as Node>::Prototype>, &mut GraphEditor<N>, &mut Cx<'_>)>;

pub struct EditorCore<N: TuiNode> {
    pub graph: Arc<Graph<N>>,
    pub want_node_prototype: Option<PrototypeCallback<N>>,
}

impl<N: TuiNode> EditorCore<N> {
    #[inline]
    #[must_use]
    pub fn new(graph: Checked<Graph<N>>) -> Self {
        Self {
            graph: graph.into_inner(),
            want_node_prototype: None,
        }
    }
}

impl<N: TuiNode> GraphWidgetTypes for EditorCore<N> {
    type Cell = wi_core::opinions::cell::WCell<EditorCore<N>>;
    type Context<'cx, 'widget: 'cx> = &'cx mut Cx<'widget>;
    type NodeId = NodeIndex;
    type NodeKind = N::Prototype;
    type Point = Point;
    type PortId = u16;

    #[inline]
    fn reborrow_cx<'widget, 're>(
        cx: &'re mut Self::Context<'_, 'widget>,
    ) -> Self::Context<'re, 'widget> {
        cx
    }
}

#[inline]
fn dist_squared(a: Point, b: Point) -> f32 { a.as_vec().distance_squared(b.as_vec()) }

fn step_point(point: Point, step: Step, count: u32) -> Point {
    #![expect(clippy::cast_precision_loss)]

    const STEP: f32 = 1.0;

    point
        + (count as f32)
            * match step {
                Step::Left => Vector::new(-STEP, 0.0),
                Step::Down => Vector::new(0.0, STEP),
                Step::Up => Vector::new(0.0, -STEP),
                Step::Right => Vector::new(STEP, 0.0),
            }
}

impl<N: TuiNode> CursorOps for EditorCore<N> {
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
        helpers::min_node_by(
            &self.graph,
            pred,
            |n| {
                let pos = n.position() + 0.5 * n.outer_size();
                dist_squared(pos, target)
            },
            f32::total_cmp,
        )
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
            .map(|p| (p, dist_squared(node.port_pos(p, side, false), pos)))
            .min_by(|(_, d), (_, e)| d.total_cmp(e))?;

        Some(port)
    }

    fn step_port_by(
        &self,
        port: &WSidedPort<Self>,
        count: i32,
    ) -> Option<(NonZeroI32, Self::PortId)> {
        let &SidedPort(side, Port(node, port)) = port;
        helpers::step_port_by(port, self.graph[node].arity(side), count)
    }

    fn nearest_edge_where<F: Fn(&WEdgeCursor<Self>) -> bool>(
        &self,
        &port: &WSidedPort<Self>,
        cell: &Self::Cell,
        pred: F,
    ) -> Option<WEdgeCursor<Self>> {
        let target = cell.edge_target(self);
        helpers::min_edge_by(
            &self.graph,
            port,
            pred,
            |from, from_port, to, to_port| {
                dist_squared(from.edge_midpoint(from_port, to, to_port), target)
            },
            f32::total_cmp,
        )
    }

    fn step_edge_by(
        &self,
        &edge: &WEdgeCursor<Self>,
        count: i32,
    ) -> Option<(NonZeroI32, WPort<Self>)> {
        helpers::step_edge_by(
            &self.graph,
            edge,
            count,
            |from, from_port, to, to_port| from.edge_midpoint(from_port, to, to_port).as_vec(),
            |o1, o2| o1.y.total_cmp(&o2.y).then_with(|| o1.x.total_cmp(&o2.x)),
        )
    }

    fn step_point_by(&self, &point: &Self::Point, step: Step, count: u32) -> Self::Point {
        step_point(point, step, count)
    }

    fn update_cursor(
        &mut self,
        update: CursorUpdate,
        cursor: &WCursor<Self>,
        cx: Self::Context<'_, '_>,
    ) {
        cx.render_requested = true;
    }
}

impl<N: TuiNode> EdgeOps for EditorCore<N> {
    fn create_edge(&mut self, from: WPort<Self>, to: WPort<Self>, cx: Self::Context<'_, '_>) {
        helpers::create_edge(make_mut!(self.graph), from, to);
        cx.render_requested = true;
    }

    fn delete_edge(
        &mut self,
        &from: &WPort<Self>,
        &to: &WPort<Self>,
        cx: Self::Context<'_, '_>,
    ) -> bool {
        let Some(e) = helpers::find_edge(&self.graph, from, to) else {
            return false;
        };
        let e = e.id();

        make_mut!(self.graph)
            .remove_edge(e)
            .unwrap_or_else(|| unreachable!());

        cx.graph_changed = true;
        cx.render_requested = true;

        true
    }
}

impl<N: TuiNode> NodeOps for EditorCore<N> {
    fn prompt_node_kind<'a, Y, C: ContinueOnce<Self, Y, Option<Self::NodeKind>>>(
        &'a mut self,
        then: Yielded<'a, '_, Self, Y, C>,
    ) {
        if let Some(p) = N::override_prototype() {
            then.resume_now(self, p.ok());
        } else {
            let (then, _cx) = then.defer();
            assert!(
                self.want_node_prototype
                    .replace(Box::new(|value, widget, cx| {
                        then.continue_once(
                            value,
                            ContinueCx::new_deferred(&mut widget.core, &mut widget.driver, cx),
                        );
                    }))
                    .is_none(),
                "Node kind prompted while already in dialog"
            );
        }
    }

    fn create_node(
        &mut self,
        kind: Self::NodeKind,
        position: Self::Point,
        cx: Self::Context<'_, '_>,
    ) {
        make_mut!(self.graph).add_node(N::create(kind, position).into());

        cx.graph_changed = true;
        cx.render_requested = true;
    }

    fn nudge_node(
        &mut self,
        &node: &Self::NodeId,
        step: Step,
        count: u32,
        cx: Self::Context<'_, '_>,
    ) -> bool {
        let Some(pos) = make_mut!(make_mut!(self.graph)[node]).position_mut() else {
            return false;
        };

        *pos = step_point(*pos, step, count);

        cx.graph_changed = true;
        cx.render_requested = true;
        true
    }

    fn delete_node(&mut self, &node: &Self::NodeId, cx: Self::Context<'_, '_>) -> bool {
        if make_mut!(self.graph).remove_node(node).is_none() {
            return false;
        }

        cx.graph_changed = true;
        cx.render_requested = true;

        true
    }
}

impl<N: TuiNode> UiOps for EditorCore<N> {
    fn update_status(&mut self, _status: Status, cx: Self::Context<'_, '_>) {
        cx.render_requested = true;
    }

    #[inline]
    fn quit(&mut self, cx: Self::Context<'_, '_>) { cx.quit_requested = true; }
}
