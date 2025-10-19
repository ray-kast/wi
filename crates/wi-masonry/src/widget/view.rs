use std::mem;

use masonry::{
    core::{EventCtx, PointerInfo, PointerState, ScrollDelta},
    kurbo::{Point, Size, Vec2},
};
use masonry::dpi::PhysicalPosition;

use super::drag::DragHandler;

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
    pub fn new(pan: Vec2) -> Self {
        Self {
            pan,
            drag: DragHandler::new(),
        }
    }

    #[inline]
    pub fn translation(&self, size: Size) -> Vec2 { size.to_vec2() * 0.5 - self.pan }

    #[inline]
    pub fn in_drag(&self) -> bool { self.drag.in_drag() }

    #[inline]
    pub fn center_on(&mut self, point: Point) { self.pan = point.to_vec2(); }

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

impl Default for Zoom {
    #[inline]
    fn default() -> Self { Self(Self::DEFAULT) }
}

impl Zoom {
    const DEFAULT: f64 = 0.0;

    #[inline]
    pub fn scale(&self) -> f64 { 1.5_f64.powf(self.0) }

    #[inline]
    pub fn reset(&mut self) { self.0 = Self::DEFAULT; }

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
