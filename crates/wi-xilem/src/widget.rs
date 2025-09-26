use std::{collections::HashMap, mem};

use masonry::{
    core::{
        keyboard::{Key, KeyState},
        EventCtx, Ime, KeyboardEvent, Modifiers, PointerButton, PointerEvent, PointerInfo,
        PointerState, ScrollDelta, TextEvent, Widget,
    },
    kurbo::{Circle, PathEl, Point, Rect, Size, Stroke},
    peniko::{color::OpaqueColor, Fill},
};
use smallvec::smallvec;
use wi_core::{GraphWidget, GraphWidgetDriver};
use xilem::{dpi::PhysicalPosition, Affine, Vec2};

use crate::Port;

#[derive(Debug)]
struct Node {
    pos: Point,
    in_edges: Vec<Option<Port>>,
    out_edges: Vec<Vec<Port>>,
}

impl Node {
    fn size(&self) -> Size {
        Size::new(
            64.0,
            16.0 * (self.in_edges.len().max(self.out_edges.len()) as f64),
        )
    }

    fn rect(&self) -> Rect { Rect::from_origin_size(self.pos, self.size()) }

    fn port_pos(&self, idx: usize, out: bool) -> Point {
        let mut pos = self.pos + Vec2::new(0.0, 8.0) + Vec2::new(0.0, 16.0) * (idx as f64);

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

    fn update(
        &mut self,
        pointer: &PointerInfo,
        state: &PointerState,
        pan: &mut Vec2,
        zoom: &Zoom,
        ctx: &mut EventCtx,
    ) {
        if *pointer != self.pointer {
            return;
        }

        let delta = ctx.local_position(self.start_pos) - ctx.local_position(state.position);

        let prev = mem::replace(pan, self.start_pan + delta / zoom.scale());
        if prev != *pan {
            ctx.request_render();
        }
    }

    fn complete(
        this: &mut Option<Self>,
        pointer: &PointerInfo,
        state: &PointerState,
        pan: &mut Vec2,
        zoom: &Zoom,
        ctx: &mut EventCtx,
    ) {
        let Some(me) = this.as_mut() else { return };

        if *pointer != me.pointer {
            return;
        }

        me.update(pointer, state, pan, zoom, ctx);
        *this = None;
    }

    fn cancel(this: &mut Option<Self>, pointer: &PointerInfo, pan: &mut Vec2, ctx: &mut EventCtx) {
        let Some(me) = this.as_mut() else { return };

        if *pointer != me.pointer {
            return;
        }

        let prev = mem::replace(pan, me.start_pan);
        if prev != *pan {
            ctx.request_render();
        }

        *this = None;
    }
}

#[derive(Debug)]
#[repr(transparent)]
struct Zoom(f64);

impl Zoom {
    fn scale(&self) -> f64 { 1.5_f64.powf(self.0) }

    fn update(&mut self, delta: &ScrollDelta, ctx: &mut EventCtx) {
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

        if self.0 != prev {
            ctx.request_render();
        }
    }
}

#[derive(Debug)]
struct GraphCore {
    nodes: HashMap<usize, Node>,
}

impl GraphWidget<usize> for GraphCore {
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
}

#[derive(Debug)]
pub struct Graph {
    core: GraphCore,
    driver: GraphWidgetDriver<usize>,
    pan: Vec2,
    zoom: Zoom,
    pan_drag: Option<PanDrag>,
}

impl Graph {
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
            core: GraphCore { nodes },
            driver: GraphWidgetDriver::new(graph.nodes.keys().copied().min().unwrap_or(usize::MAX)),
            pan,
            zoom: Zoom(0.0),
            pan_drag: None,
        }
    }
}

impl Widget for Graph {
    fn accepts_focus(&self) -> bool { true }

    fn accepts_text_input(&self) -> bool { true }

    fn register_children(&mut self, ctx: &mut masonry::core::RegisterCtx) {}

    fn layout(
        &mut self,
        ctx: &mut masonry::core::LayoutCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        bc: &masonry::core::BoxConstraints,
    ) -> masonry::kurbo::Size {
        bc.max()
    }

    fn paint(
        &mut self,
        ctx: &mut masonry::core::PaintCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        scene: &mut masonry::vello::Scene,
    ) {
        let transform =
            Affine::scale_about(self.zoom.scale(), (ctx.size().to_vec2() * 0.5).to_point())
                * Affine::translate(ctx.size().to_vec2() * 0.5 - self.pan);

        for node in self.core.nodes.values() {
            for (i, port) in node.in_edges.iter().enumerate() {
                let Some(port) = port else { continue };
                let from = &self.core.nodes[&port.node];
                let from_pos = from.port_pos(port.port, true);
                let to_pos = node.port_pos(i, false);

                scene.stroke(
                    &Stroke::new(4.0),
                    transform,
                    OpaqueColor::from_rgb8(0x7f, 0x7f, 0x7f),
                    None,
                    &[
                        PathEl::MoveTo(from_pos),
                        PathEl::CurveTo(
                            from_pos + Vec2::new(16.0, 0.0),
                            to_pos + Vec2::new(-16.0, 0.0),
                            to_pos,
                        ),
                    ],
                );
            }
        }

        for (&i, node) in &self.core.nodes {
            let rect = node.rect();

            scene.stroke(
                &Stroke::new(4.0),
                transform,
                if *self.driver.focus_node() == i {
                    OpaqueColor::from_rgb8(0x90, 0x37, 0x22)
                } else {
                    OpaqueColor::from_rgb8(0x3a, 0x3a, 0x3a)
                },
                None,
                &rect,
            );

            for port in 0..node.in_edges.len() {
                scene.fill(
                    Fill::NonZero,
                    transform,
                    OpaqueColor::from_rgb8(0x9a, 0x9a, 0x9a),
                    None,
                    &Circle::new(node.port_pos(port, false), 4.0),
                );
            }

            for port in 0..node.out_edges.len() {
                scene.fill(
                    Fill::NonZero,
                    transform,
                    OpaqueColor::from_rgb8(0x9a, 0x9a, 0x9a),
                    None,
                    &Circle::new(node.port_pos(port, true), 4.0),
                );
            }
        }
    }

    // TODO: review
    fn accessibility_role(&self) -> accesskit::Role { accesskit::Role::ScrollView }

    fn accessibility(
        &mut self,
        ctx: &mut masonry::core::AccessCtx,
        _props: &masonry::core::PropertiesRef<'_>,
        node: &mut accesskit::Node,
    ) {
        for node in &self.core.nodes {
            // TODO
        }
    }

    fn children_ids(&self) -> smallvec::SmallVec<[masonry::core::WidgetId; 16]> { smallvec![] }

    fn on_text_event(
        &mut self,
        ctx: &mut EventCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        event: &TextEvent,
    ) {
        match event {
            TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Character(s),
                modifiers,
                is_composing: false,
                ..
            }) => self.driver.handle_char_input(&mut self.core, s, modifiers),
            TextEvent::Keyboard(KeyboardEvent {
                state: KeyState::Down,
                key: Key::Named(k),
                modifiers,
                is_composing: false,
                ..
            }) => self
                .driver
                .handle_named_keypress(&mut self.core, k, modifiers),
            TextEvent::Ime(Ime::Commit(s)) => {
                self.driver
                    .handle_char_input(&mut self.core, s, &Modifiers::empty());
            },
            _ => return,
        }
        ctx.set_handled();
    }

    fn on_pointer_event(
        &mut self,
        ctx: &mut EventCtx,
        _props: &mut masonry::core::PropertiesMut<'_>,
        event: &PointerEvent,
    ) {
        // TODO: how do i do this differently
        ctx.request_focus();

        match event {
            PointerEvent::Down {
                button: Some(PointerButton::Auxiliary),
                pointer,
                state,
            } => {
                if state.buttons == PointerButton::Auxiliary.into() {
                    self.pan_drag = Some(PanDrag::new(*pointer, state, self.pan));
                } else {
                    PanDrag::cancel(&mut self.pan_drag, pointer, &mut self.pan, ctx);
                }
            },
            PointerEvent::Move(u) => {
                if let Some(drag) = &mut self.pan_drag {
                    drag.update(&u.pointer, &u.current, &mut self.pan, &self.zoom, ctx);
                }
            },
            PointerEvent::Up {
                button: Some(PointerButton::Auxiliary),
                pointer,
                state,
            } => {
                PanDrag::complete(
                    &mut self.pan_drag,
                    pointer,
                    state,
                    &mut self.pan,
                    &self.zoom,
                    ctx,
                );
            },
            PointerEvent::Cancel(i) => {
                PanDrag::cancel(&mut self.pan_drag, i, &mut self.pan, ctx);
            },
            PointerEvent::Scroll {
                pointer: _,
                delta,
                state,
            } => {
                if state.modifiers == Modifiers::CONTROL {
                    self.zoom.update(delta, ctx);
                }
            },
            _ => (),
        }
    }
}
