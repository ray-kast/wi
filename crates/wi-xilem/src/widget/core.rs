use std::{collections::{HashMap}, mem};

use masonry::{
    core::{EventCtx, PointerInfo, PointerState, ScrollDelta},
    kurbo::{Point, Rect, Size},
};
use wi_core::GraphWidget;
use xilem::{dpi::PhysicalPosition, Affine, Vec2};

use crate::Port;

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
            64.0,
            16.0 * (self.in_edges.len().max(self.out_edges.len()) as f64),
        )
    }

    pub fn rect(&self) -> Rect { Rect::from_origin_size(self.pos, self.size()) }

    pub fn port_pos(&self, idx: usize, out: bool) -> Point {
        #[allow(clippy::cast_precision_loss)]
        let mut pos = self.pos + Vec2::new(0.0, 8.0) + Vec2::new(0.0, 16.0) * idx as f64;

        if out {
            pos.x += self.size().width;
        }

        pos
    }
}

#[derive(Debug)]
struct PanDrag {
    pointer: PointerInfo,
    start_pos: PhysicalPosition<f64>,
    start_pan: Vec2,
}

impl PanDrag {
    fn new(pointer: PointerInfo, state: &PointerState, start_pan: Vec2) -> Self {
        Self {
            pointer,
            start_pos: state.position,
            start_pan,
        }
    }
}

#[derive(Debug)]
pub struct Pan {
    pan: Vec2,
    drag: Option<PanDrag>,
}

impl Pan {
    fn translation(&self, size: Size) -> Vec2 { size.to_vec2() * 0.5 - self.pan }

    pub fn begin_drag(&mut self, pointer: PointerInfo, state: &PointerState) {
        self.drag = Some(PanDrag::new(pointer, state, self.pan));
    }

    pub fn update_drag(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        zoom: &Zoom,
        ctx: &mut EventCtx,
    ) -> bool {
        let Some(drag) = self.drag.as_mut() else {
            return false;
        };

        if *pointer != drag.pointer {
            return false;
        }

        let delta = ctx.local_position(drag.start_pos) - ctx.local_position(state.position);

        let prev = mem::replace(&mut self.pan, drag.start_pan + delta / zoom.scale());
        if prev != self.pan {
            ctx.request_render();
        }

        true
    }

    pub fn complete_drag(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        zoom: &Zoom,
        ctx: &mut EventCtx,
    ) {
        if self.update_drag(pointer, state, zoom, ctx) {
            self.drag = None;
        }
    }

    pub fn cancel_drag(&mut self, pointer: &PointerInfo, ctx: &mut EventCtx) {
        let Some(drag) = self.drag.as_mut() else {
            return;
        };

        if *pointer != drag.pointer {
            return;
        }

        let prev = mem::replace(&mut self.pan, drag.start_pan);
        if prev != self.pan {
            ctx.request_render();
        }

        self.drag = None;
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Zoom(f64);

impl Zoom {
    fn scale(&self) -> f64 { 1.5_f64.powf(self.0) }

    pub fn scroll(&mut self, delta: &ScrollDelta, ctx: &mut EventCtx) {
        let delta = match delta {
            &ScrollDelta::PageDelta(x, y) => Vec2::new(f64::from(x) * 10.0, f64::from(y) * 10.0),
            &ScrollDelta::LineDelta(x, y) => Vec2::new(x.into(), y.into()),
            ScrollDelta::PixelDelta(p) => Vec2::new(p.x / 16.0, p.y / 16.0),
        };

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
            pan: Pan { pan, drag: None },
            zoom: Zoom(0.0),
        }
    }

    #[inline]
    pub fn view_transform(&self, size: Size) -> Affine {
        Affine::scale_about(self.zoom.scale(), (size.to_vec2() * 0.5).to_point())
            * Affine::translate(self.pan.translation(size))
    }
}

impl GraphWidget<usize> for GraphCore {
    type EventCtx<'a> = EventCtx<'a>;

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

    fn view_node(&mut self, node: &usize, ctx: &mut EventCtx) {
        self.pan.pan = self.nodes[node].rect().center().to_vec2();
        ctx.request_render();
    }
}
