use std::{collections::BTreeMap, fmt::Write, mem, num::NonZeroIsize};

use masonry::{
    core::{EventCtx, PointerInfo, PointerState, ScrollDelta, StyleProperty, WidgetMut, WidgetPod},
    kurbo::{Affine, Point, Rect, Size, Vec2},
    widgets::{Flex, Label},
};
use wi_core::{
    status::{ModeKind, Status},
    Cell, Cursor, CursorUpdate, EdgeCursor, GraphWidget, GraphWidgetDriver, Port, PreserveCell,
    Side, SidedPort, WCell, WCellRef, WCursor, WEdgeCursor, WPort, WPreserveCellRef, WSidedPort,
};
use xilem::dpi::PhysicalPosition;

use crate::widget::drag::DragHandler;

#[derive(Debug)]
pub struct Node {
    pos: Point,
    pub in_edges: Vec<Option<WPort<GraphCore>>>,
    pub out_edges: Vec<Vec<WPort<GraphCore>>>,
}

impl Node {
    pub const PORT_Y_OFFS: f64 = 20.0;

    #[inline]
    fn size(&self) -> Size {
        #[expect(clippy::cast_precision_loss, reason = "Necessary cast")]
        Size::new(
            128.0,
            16.0 + 24.0 * (self.in_edges.len().max(self.out_edges.len()).max(1) as f64),
        )
    }

    pub fn rect(&self) -> Rect { Rect::from_origin_size(self.pos, self.size()) }

    #[inline]
    pub fn port_offs(&self, idx: usize, side: Side) -> Vec2 {
        #[expect(clippy::cast_precision_loss, reason = "Necessary cast")]
        {
            Vec2::new(0.0, Self::PORT_Y_OFFS)
                + Vec2::new(0.0, 24.0) * idx as f64
                + f64::from(matches!(side, Side::Out)) * Vec2::new(self.size().width, 0.0)
        }
    }

    #[inline]
    pub fn port_pos(&self, idx: usize, side: Side) -> Point { self.pos + self.port_offs(idx, side) }
}

const SCROLL_PAGE_LINES: f64 = 10.0;
const SCROLL_LINE_PX: f64 = 16.0;

fn scroll_pixels(delta: &ScrollDelta) -> Vec2 {
    match delta {
        &ScrollDelta::PageDelta(x, y) => Vec2::new(
            f64::from(x) * SCROLL_PAGE_LINES,
            f64::from(y) * SCROLL_PAGE_LINES,
        ),
        &ScrollDelta::LineDelta(x, y) => Vec2::new(x.into(), y.into()),
        ScrollDelta::PixelDelta(p) => Vec2::new(p.x / SCROLL_LINE_PX, p.y / SCROLL_LINE_PX),
    }
}

#[derive(Debug)]
pub struct Pan {
    pan: Vec2,
    drag: DragHandler<Vec2>,
}

impl Pan {
    #[inline]
    fn translation(&self, size: Size) -> Vec2 { size.to_vec2() * 0.5 - self.pan }

    #[inline]
    pub fn in_drag(&self) -> bool { self.drag.in_drag() }

    #[inline]
    pub fn begin_drag(&mut self, pointer: PointerInfo, state: &PointerState) {
        self.drag.begin_drag(pointer, state, self.pan);
    }

    #[inline]
    fn drag_delta(
        pan: &mut Vec2,
        zoom: &Zoom,
        ctx: &mut EventCtx,
    ) -> impl FnOnce(&Vec2, PhysicalPosition<f64>, PhysicalPosition<f64>) {
        |&start_pan, from, to| {
            let delta = ctx.local_position(from) - ctx.local_position(to);

            let prev = mem::replace(pan, start_pan + delta / zoom.scale());
            if prev != *pan {
                ctx.request_render();
            }
        }
    }

    #[inline]
    pub fn update_drag(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        zoom: &Zoom,
        ctx: &mut EventCtx,
    ) {
        self.drag
            .update_drag(pointer, state, Self::drag_delta(&mut self.pan, zoom, ctx));
    }

    #[inline]
    pub fn complete_drag(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        zoom: &Zoom,
        ctx: &mut EventCtx,
    ) {
        self.drag
            .complete_drag(pointer, state, Self::drag_delta(&mut self.pan, zoom, ctx));
    }

    #[inline]
    pub fn cancel_drag(&mut self, pointer: Option<&PointerInfo>, ctx: &mut EventCtx) {
        self.drag.cancel_drag(pointer, |&s| {
            let prev = mem::replace(&mut self.pan, s);
            if prev != self.pan {
                ctx.request_render();
            }
        });
    }

    pub fn scroll(&mut self, delta: &ScrollDelta, transp: bool, zoom: &Zoom, ctx: &mut EventCtx) {
        let delta = scroll_pixels(delta) / zoom.scale();
        let delta = if transp {
            Vec2::new(delta.y, delta.x)
        } else {
            delta
        };
        let prev = self.pan;
        self.pan -= delta * SCROLL_LINE_PX;
        if prev != self.pan {
            ctx.request_render();
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Zoom(f64);

impl Zoom {
    fn scale(&self) -> f64 { 1.5_f64.powf(self.0) }

    pub fn scroll(&mut self, delta: &ScrollDelta, ctx: &mut EventCtx) {
        let delta = scroll_pixels(delta);
        let primary = if !delta.y.is_finite() || delta.x.abs() > delta.y.abs() {
            delta.x
        } else {
            delta.y
        };

        let delta =
            (delta.x * delta.x + delta.y * delta.y).sqrt() * if primary < 0.0 { -1.0 } else { 1.0 };

        let prev = self.0;
        self.0 += delta;

        #[expect(clippy::float_cmp, reason = "Imprecision fails safe here")]
        if self.0 != prev {
            ctx.request_render();
        }
    }
}

pub struct GraphCore {
    pub nodes: BTreeMap<usize, Node>,
    node_drag: DragHandler<(usize, Point)>,
    pub pan: Pan,
    pub zoom: Zoom,
    pub statusbar: WidgetPod<Flex>,
}

fn edge_midpoint(from_node: &Node, from_port: usize, to_node: &Node, to_port: usize) -> Point {
    ((from_node.port_pos(from_port, Side::Out).to_vec2()
        + to_node.port_pos(to_port, Side::In).to_vec2())
        * 0.5)
        .to_point()
}

impl GraphCore {
    pub fn new(graph: &crate::GraphView, driver: &GraphWidgetDriver<Self>) -> Self {
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
            pan: Pan {
                pan,
                drag: DragHandler::new(),
            },
            zoom: Zoom(0.0),
            statusbar: RenderedStatus::new(driver.status()).create(),
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

    #[inline]
    pub fn cell_point(&self, cell: WCellRef<Self>) -> Point {
        let Cell { anchor, row, col } = cell;
        anchor.get(self) + Vec2::new(*col, *row)
    }

    fn edge_midpoint(&self, from: WPort<Self>, to: WPort<Self>) -> Point {
        let Port(from_node, from_port) = from;
        let Port(to_node, to_port) = to;

        let from_node = &self.nodes[&from_node];
        let to_node = &self.nodes[&to_node];

        edge_midpoint(from_node, from_port, to_node, to_port)
    }
}

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

struct RenderedStatus {
    mode: &'static str,
    chord: String,
}

impl RenderedStatus {
    fn new(status: Status) -> Self {
        let Status {
            count,
            mode,
            pending_op,
        } = status;
        let mut chord = String::new();

        if let Some(count) = count {
            write!(chord, "{count}").unwrap();
        }

        write!(chord, "{pending_op}").unwrap();

        let mode = match mode {
            ModeKind::Normal => "Normal",
        };

        Self { mode, chord }
    }

    fn create(self) -> WidgetPod<Flex> {
        let Self { mode, chord } = self;

        WidgetPod::new(
            Flex::row()
                .gap(8.0)
                .with_spacer(4.0)
                .with_child(Label::new(mode).with_style(StyleProperty::FontSize(18.0)))
                .with_flex_spacer(1.0)
                .with_child(Label::new(chord).with_style(StyleProperty::FontSize(18.0)))
                .with_spacer(4.0),
        )
    }

    fn update(self, mut bar: WidgetMut<Flex>) {
        let Self { mode, chord } = self;

        Label::set_text(&mut Flex::child_mut(&mut bar, 1).unwrap().downcast(), mode);

        Label::set_text(&mut Flex::child_mut(&mut bar, 3).unwrap().downcast(), chord);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    Fixed,
    Node(usize),
    Edge(WPort<GraphCore>, WPort<GraphCore>),
}

impl Anchor {
    fn get(self, graph: &GraphCore) -> Point {
        match self {
            Self::Fixed => Point::ZERO,
            Self::Node(n) => graph.nodes[&n].pos,
            Self::Edge(i, o) => graph.edge_midpoint(i, o),
        }
    }
}

impl GraphWidget for GraphCore {
    type CellAnchor = Anchor;
    type Col = f64;
    type Context<'a> = EventCtx<'a>;
    type Node = usize;
    type Point = Point;
    type PortIdx = usize;
    type Row = f64;

    fn cursor_cell(&self, cursor: &WCursor<Self>, keep: WPreserveCellRef<Self>) -> WCell<Self> {
        let (anchor, pos) = match *cursor {
            Cursor::Node(n) => {
                let node = &self.nodes[&n];
                (
                    Anchor::Node(n),
                    Vec2::new(node.rect().size().width * 0.5, Node::PORT_Y_OFFS),
                )
            },
            Cursor::Port(SidedPort(s, Port(n, p))) => {
                let node = &self.nodes[&n];
                (Anchor::Node(n), node.port_pos(p, s) - node.pos)
            },
            Cursor::Edge(e) => (Anchor::Edge(e.from, e.to), Vec2::ZERO),
            Cursor::FixedPoint(p) => (Anchor::Fixed, p.to_vec2()),
        };

        match keep {
            PreserveCell::Overwrite => Cell {
                anchor,
                row: pos.y,
                col: pos.x,
            },
            PreserveCell::Row(&a, &row) if a == anchor => Cell {
                anchor,
                row,
                col: pos.x,
            },
            PreserveCell::Row(a, &r) => Cell {
                anchor,
                row: r + a.get(self).y - anchor.get(self).y,
                col: pos.x,
            },
            PreserveCell::Col(&a, &col) if a == anchor => Cell {
                anchor,
                row: pos.y,
                col,
            },
            PreserveCell::Col(a, &c) => Cell {
                anchor,
                row: pos.y,
                col: c + a.get(self).x - anchor.get(self).x,
            },
        }
    }

    fn nearest_port(
        &self,
        n: &Self::Node,
        side: Side,
        cell: WCellRef<Self>,
    ) -> Option<Self::PortIdx> {
        let node = &self.nodes[n];
        let len = match side {
            Side::In => node.in_edges.len(),
            Side::Out => node.out_edges.len(),
        };
        let pos = self.cell_point(cell);

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
        cell: WCellRef<Self>,
    ) -> Option<WEdgeCursor<Self>> {
        let SidedPort(side, port) = *port;
        let node = &self.nodes[&port.0];

        let (from, to) = match side {
            Side::In => (node.in_edges[port.1]?, port),
            Side::Out => {
                let pos = self.cell_point(cell);
                (
                    port,
                    node.out_edges[port.1]
                        .iter()
                        .map(|&p| {
                            (
                                p,
                                edge_midpoint(node, port.1, &self.nodes[&p.0], p.1)
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
                    Side::In => edge_midpoint(n, p.1, node, port.1),
                    Side::Out => edge_midpoint(node, port.1, n, p.1),
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
                self.pan.pan = match *cursor {
                    Cursor::Node(n) => self.nodes[&n].rect().center(),
                    Cursor::Port(SidedPort(s, Port(n, p))) => self.nodes[&n].port_pos(p, s),
                    Cursor::Edge(e) => self.edge_midpoint(e.from, e.to),
                    Cursor::FixedPoint(p) => p,
                }
                .to_vec2();
                self.zoom.0 = 0.0;
            },
        }
        ctx.request_render();
    }

    fn update_status(&mut self, status: Status, ctx: &mut EventCtx) {
        let rendered = RenderedStatus::new(status);
        ctx.mutate_later(&mut self.statusbar, move |b| rendered.update(b));
    }
}
