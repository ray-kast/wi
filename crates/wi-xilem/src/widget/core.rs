use std::{collections::HashMap, mem};

use masonry::{
    core::{EventCtx, PointerInfo, PointerState, ScrollDelta},
    kurbo::{Point, Rect, Size},
};
use wi_core::{Cursor, GraphWidget};
use xilem::{dpi::PhysicalPosition, Affine, Vec2};

use crate::{widget::drag::DragHandler, Port};

#[derive(Debug)]
pub struct Node {
    pos: Point,
    pub in_edges: Vec<Option<Port>>,
    pub out_edges: Vec<Vec<Port>>,
}

impl Node {
    fn size(&self) -> Size {
        #[allow(clippy::cast_precision_loss)]
        Size::new(
            128.0,
            16.0 + 24.0 * (self.in_edges.len().max(self.out_edges.len()) as f64),
        )
    }

    pub fn rect(&self) -> Rect { Rect::from_origin_size(self.pos, self.size()) }

    pub fn port_pos(&self, idx: usize, out: bool) -> Point {
        #[allow(clippy::cast_precision_loss)]
        let mut pos = self.pos + Vec2::new(0.0, 20.0) + Vec2::new(0.0, 24.0) * idx as f64;

        if out {
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

    pub fn scroll(&mut self, delta: &ScrollDelta, transp: bool, ctx: &mut EventCtx) {
        let delta = scroll_pixels(delta);
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

        #[allow(clippy::float_cmp)]
        if self.0 != prev {
            ctx.request_render();
        }
    }
}

#[derive(Debug)]
pub struct GraphCore {
    pub nodes: HashMap<usize, Node>,
    node_drag: DragHandler<(usize, Point)>,
    pub pan: Pan,
    pub zoom: Zoom,
}

impl GraphCore {
    pub fn new(graph: &crate::GraphView) -> Self {
        let mut out_edges = graph.out_edge_map();

        let nodes: HashMap<_, _> = graph
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

        let Some(node) = self.nodes.iter().find_map(|(&k, v)| {
            v.rect().contains(point).then_some(k)
        }) else {
            return;
        };

        self.node_drag
            .begin_drag(pointer, state, (node, self.nodes[&node].pos));
    }

    #[inline]
    fn node_drag_delta(
        nodes: &mut HashMap<usize, Node>,
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
    type EventCtx<'a> = EventCtx<'a>;
    type Node = usize;
    type Point = Point;
    type PortIdx = usize;

    fn in_edges<'a>(&'a self, node: &usize) -> impl IntoIterator<Item = &'a usize>
    where usize: 'a {
        self.nodes
            .get(node)
            .into_iter()
            .flat_map(|n| n.in_edges.iter())
            .filter_map(|e| Some(&e.as_ref()?.node))
    }

    fn out_edges<'a>(&'a self, node: &usize) -> impl IntoIterator<Item = &'a usize>
    where usize: 'a {
        self.nodes
            .get(node)
            .into_iter()
            .flat_map(|n| n.out_edges.iter())
            .flatten()
            .map(|p| &p.node)
    }

    fn view_cursor(&mut self, cursor: &Cursor<Self>, ctx: &mut EventCtx) {
        self.pan.pan = match cursor {
            Cursor::Node(n) => self.nodes[n].rect().center(),
            &Cursor::InPort(ref n, p) => self.nodes[n].port_pos(p, false),
            &Cursor::OutPort(ref n, p) => self.nodes[n].port_pos(p, true),
            Cursor::FixedPoint(p) => *p,
        }
        .to_vec2();
        self.zoom.0 = 0.0;
        ctx.request_render();
    }
}
