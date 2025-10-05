use std::{collections::BTreeMap, fmt::Write, mem, num::NonZeroIsize};

use masonry::{
    core::{EventCtx, PointerInfo, PointerState, ScrollDelta, StyleProperty, WidgetMut, WidgetPod},
    kurbo::{Affine, Point, Rect, Size, Vec2},
    widgets::{Flex, Label},
};
use wi_core::{
    status::{ModeKind, Status},
    Cursor, CursorUpdate, GraphWidget, GraphWidgetDriver, Side,
};
use xilem::dpi::PhysicalPosition;

use crate::{widget::drag::DragHandler, Port};

#[derive(Debug)]
pub struct Node {
    pos: Point,
    pub in_edges: Vec<Option<Port>>,
    pub out_edges: Vec<Vec<Port>>,
}

impl Node {
    fn size(&self) -> Size {
        #[expect(clippy::cast_precision_loss, reason = "Necessary cast")]
        Size::new(
            128.0,
            16.0 + 24.0 * (self.in_edges.len().max(self.out_edges.len()) as f64),
        )
    }

    pub fn rect(&self) -> Rect { Rect::from_origin_size(self.pos, self.size()) }

    pub fn port_pos(&self, idx: usize, side: Side) -> Point {
        #[expect(clippy::cast_precision_loss, reason = "Necessary cast")]
        let mut pos = self.pos + Vec2::new(0.0, 20.0) + Vec2::new(0.0, 24.0) * idx as f64;

        if matches!(side, Side::Out) {
            pos.x += self.size().width;
        }

        pos
    }
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

impl GraphCore {
    pub fn new(graph: &crate::GraphView, driver: &GraphWidgetDriver<Self>) -> Self {
        let mut out_edges = graph.out_edge_map();

        let nodes: BTreeMap<_, _> = graph
            .nodes
            .iter()
            .map(|(&i, n)| {
                (i, Node {
                    pos: n.pos,
                    in_edges: n.in_edges.clone(),
                    out_edges: out_edges.remove(&i).unwrap_or_else(|| unreachable!()),
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

    pub fn cell_point(&self, row: Anchor<f64>, col: Anchor<f64>) -> Point {
        Point::new(
            col.1
                + col
                    .0
                    .and_then(|n| self.nodes.get(&n))
                    .map_or(0.0, |n| n.pos.x),
            row.1
                + row
                    .0
                    .and_then(|n| self.nodes.get(&n))
                    .map_or(0.0, |n| n.pos.y),
        )
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

// Coordinate and optional node anchor point
#[derive(Debug, Clone, Copy)]
pub struct Anchor<T>(Option<usize>, T);

impl GraphWidget for GraphCore {
    type Col = Anchor<f64>;
    type Context<'a> = EventCtx<'a>;
    type Node = usize;
    type Point = Point;
    type PortIdx = usize;
    type Row = Anchor<f64>;

    fn cursor_cell(&self, cursor: &Cursor<Self>) -> (Self::Row, Self::Col) {
        let (node, pos) = match *cursor {
            Cursor::Node(n) => {
                let node = &self.nodes[&n];
                (Some(n), node.rect().size().to_vec2() * 0.5)
            },
            Cursor::Port(wi_core::Port(s, n, p)) => {
                let node = &self.nodes[&n];
                (Some(n), node.port_pos(p, s) - node.pos)
            },
            Cursor::FixedPoint(p) => (None, p.to_vec2()),
        };
        (Anchor(node, pos.y), Anchor(node, pos.x))
    }

    fn nearest_port(
        &self,
        n: &Self::Node,
        side: Side,
        cell: (&Self::Row, &Self::Col),
    ) -> Option<(Self::PortIdx, Self::Col)> {
        let node = &self.nodes[n];
        let len = match side {
            Side::In => node.in_edges.len(),
            Side::Out => node.out_edges.len(),
        };
        let pos = self.cell_point(*cell.0, *cell.1);

        let (port, _) = (0..len)
            .map(|p| (p, node.port_pos(p, side).distance_squared(pos)))
            .min_by(|(_, d), (_, e)| d.total_cmp(e))?;

        Some((
            port,
            Anchor(Some(*n), node.port_pos(port, side).x - node.pos.x),
        ))
    }

    fn step_port_by(
        &self,
        port: &wi_core::Port<Self>,
        count: isize,
    ) -> (Option<(NonZeroIsize, Self::PortIdx)>, Self::Row) {
        let wi_core::Port(side, n, port) = *port;
        let node = &self.nodes[&n];

        let res = port.saturating_add_signed(count).min(
            match side {
                Side::In => node.in_edges.len(),
                Side::Out => node.out_edges.len(),
            }
            .saturating_sub(1),
        );

        #[expect(clippy::cast_possible_wrap, reason = "The wrap here is intended")]
        (
            NonZeroIsize::new(res.wrapping_sub(port) as isize).map(|d| (d, res)),
            Anchor(Some(n), node.port_pos(res, side).y - node.pos.y),
        )
    }

    fn port_connection(
        &self,
        port: &wi_core::Port<Self>,
    ) -> Option<(wi_core::Port<Self>, Self::Row, Self::Col)> {
        let wi_core::Port(side, node, port) = *port;
        let node = &self.nodes[&node];

        let conn = match side {
            Side::In => node.in_edges[port],
            Side::Out => node.out_edges[port].first().copied(),
        }?;

        let node = &self.nodes[&conn.node];
        let pos = node.port_pos(conn.port, side.flip()) - node.pos;
        Some((
            wi_core::Port(side.flip(), conn.node, conn.port),
            Anchor(Some(conn.node), pos.y),
            Anchor(Some(conn.node), pos.x),
        ))
    }

    fn update_cursor(&mut self, update: CursorUpdate, cursor: &Cursor<Self>, ctx: &mut EventCtx) {
        match update {
            CursorUpdate::Move => (),
            CursorUpdate::CenterInView => {
                self.pan.pan = match cursor {
                    Cursor::Node(n) => self.nodes[n].rect().center(),
                    &Cursor::Port(wi_core::Port(s, n, p)) => self.nodes[&n].port_pos(p, s),
                    Cursor::FixedPoint(p) => *p,
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
