use std::{fmt::Write, sync::Arc};

use keyboard_types::{Modifiers, NamedKey};
use petgraph::{
    graph::NodeIndex,
    visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
};
use ratatui::{
    buffer::Buffer,
    layout::{HorizontalAlignment, Position, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{StatefulWidget, Widget},
};
use wi_core::{
    opinions::graph::{self, Checked, Graph, Label, WidgetLabel},
    ActionKind, CurrentOperatorStatus, Cursor, EdgeCursor, GraphWidgetDriver, LastChord, ModeKind,
    Port, Side, SidedPort, Status, WSidedPort,
};

pub use self::core::{Cx, PrototypeCallback};
use self::{core::EditorCore, edge::EdgeBuffer, node::NodeExt, text::Layout};
use crate::{
    graph::{NodeStyle as _, StyleKind as NodeStyle, TuiNode},
    vector::{Insets, Point, SignedRect},
};

mod cell;
mod core;
mod edge;
mod node;
mod text;

#[expect(missing_debug_implementations, reason = "Contains Box<dyn FnOnce>")]
pub struct GraphEditor<N: TuiNode> {
    core: EditorCore<N>,
    driver: GraphWidgetDriver<EditorCore<N>>,
}

impl<N: TuiNode> GraphEditor<N> {
    #[must_use]
    pub fn new(graph: Checked<Graph<N>>) -> (Self, State) {
        let core = EditorCore::new(graph);

        (
            Self {
                driver: GraphWidgetDriver::new(&core),
                core,
            },
            State {
                cursor_position: Position::ORIGIN,
            },
        )
    }

    pub fn graph(&self) -> Checked<Graph<N>> {
        // SAFETY: All operations on self.core.graph preserve the validity of
        //         the graph
        unsafe { Checked::new_unchecked(Arc::clone(&self.core.graph)) }
    }

    #[inline]
    pub const fn wants_node_prototype(&mut self) -> &mut Option<PrototypeCallback<N>> {
        &mut self.core.want_node_prototype
    }

    #[inline]
    pub fn handle_str_input(&mut self, s: &str, mods: Modifiers, cx: &mut Cx<'_>) -> bool {
        self.driver.handle_str_input(&mut self.core, s, mods, cx)
    }

    #[inline]
    pub fn handle_char_input(&mut self, chr: char, mods: Modifiers, cx: &mut Cx<'_>) -> bool {
        self.driver.handle_char_input(&mut self.core, chr, mods, cx)
    }

    #[inline]
    pub fn handle_named_keypress(
        &mut self,
        key: NamedKey,
        mods: Modifiers,
        cx: &mut Cx<'_>,
    ) -> bool {
        self.driver
            .handle_named_keypress(&mut self.core, key, mods, cx)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct State {
    cursor_position: Position,
}

impl State {
    #[inline]
    #[must_use]
    pub fn cursor_position(&self) -> Position { self.cursor_position }
}

impl<N: TuiNode> GraphEditor<N> {
    fn paint_port(area: Rect, buf: &mut Buffer, pos: Position, side: Side, focused: bool) {
        if !area.contains(pos) {
            return;
        }

        let style = Style::default().fg(Color::White);
        buf[pos]
            .set_style(if focused {
                style.bg(Color::LightBlue)
            } else {
                style
            })
            .set_char(match side {
                Side::In => '╴',
                Side::Out => '╶',
            });
    }

    #[expect(
        clippy::too_many_lines,
        reason = "The logic here would be difficult to refactor"
    )]
    fn paint_node(
        area: Rect,
        buf: &mut Buffer,
        node: (NodeIndex, &Arc<N>),
        focus_node: Option<NodeIndex>,
        focus_port: Option<WSidedPort<EditorCore<N>>>,
    ) {
        let (idx, node) = node;
        let focused = focus_node == Some(idx);
        let style = node.style();

        let Some(head_rect) = node.head_rect() else {
            return;
        };
        let Some(body_rect) = node.body_rect() else {
            return;
        };

        for pos in area.intersection(head_rect.as_bounding_rect()).positions() {
            let cell = &mut buf[pos];
            let style = cell.style();
            cell.set_style(
                style
                    .bg(if focused {
                        Color::LightBlue
                    } else {
                        Color::Green
                    })
                    .fg(Color::White),
            )
            .set_char(' ');
        }

        for pos in area.intersection(body_rect.as_bounding_rect()).positions() {
            let cell = &mut buf[pos];
            let cell_style = cell.style();
            cell.set_style(
                cell_style
                    .bg(match (focused, &style) {
                        (_, NodeStyle::Large(_)) | (false, _) => Color::DarkGray,
                        (true, _) => Color::LightBlue,
                    })
                    .fg(Color::Gray),
            )
            .set_char(' ');
        }

        let label = match &style {
            NodeStyle::Widget(w) => w.label.as_ref(),
            NodeStyle::Small(s) => WidgetLabel::Label(s.label.as_ref()),
            NodeStyle::Large(l) => WidgetLabel::Label(l.label.as_ref()),
        };
        let label = match label {
            WidgetLabel::Label(Label { content, icon }) => {
                Span::raw(content.unwrap_or_else(|| node.name()))
            },
            WidgetLabel::Widget(w) => todo!(),
        };

        let Some(label_rect) = node.label_rect() else {
            unreachable!()
        };

        Layout::prepare(
            label_rect,
            HorizontalAlignment::Center,
            &label,
            Style::new().fg(Color::White).bold(),
        )
        .render(area, buf);

        let Some(label_rect) = node.port_label_rect() else {
            unreachable!()
        };
        for (i, port) in style.in_ports() {
            let graph::Port {
                name,
                description,
                shape,
                label,
            } = port;

            let y = node.port_inner_row(i, Side::In);
            if let Some(pos) = body_rect.nudge_add(0, y.into()).as_top_left() {
                Self::paint_port(
                    area,
                    buf,
                    pos,
                    Side::In,
                    focus_port == Some(SidedPort(Side::In, Port(idx, i))),
                );
            }

            if let Some(label) = label {
                let label = match label.as_ref() {
                    WidgetLabel::Label(Label { content, icon }) => {
                        Span::raw(content.unwrap_or(name))
                    },
                    WidgetLabel::Widget(w) => todo!(),
                };
                let label_rect = label_rect.nudge_add(0, y.into()).height(1);

                Layout::prepare(
                    label_rect,
                    HorizontalAlignment::Left,
                    &label,
                    Style::new().fg(Color::White).not_bold(),
                )
                .render(area, buf);
            }
        }

        for (i, port) in style.out_ports() {
            let graph::Port {
                name,
                description,
                shape,
                label,
            } = port;

            let y = node.port_inner_row(i, Side::Out);
            if let Some(pos) = body_rect.nudge_add(0, y.into()).as_top_right(true) {
                Self::paint_port(
                    area,
                    buf,
                    pos,
                    Side::Out,
                    focus_port == Some(SidedPort(Side::Out, Port(idx, i))),
                );
            }

            if let Some(label) = label {
                let Label { content, icon } = label.as_ref();
                let label = Span::raw(content.unwrap_or(name));
                let label_rect = label_rect.nudge_add(0, y.into()).height(1);

                Layout::prepare(
                    label_rect,
                    HorizontalAlignment::Right,
                    &label,
                    Style::new().fg(Color::White).not_bold(),
                )
                .render(area, buf);
            }
        }
    }

    fn paint_status(area: Rect, buf: &mut Buffer, status: Status) {
        const MODE_WIDTH: u32 = 40;
        const CHORD_WIDTH: u32 = 10;
        const GAP: u32 = 1;

        let Status {
            count,
            mode,
            last_action,
            current_operator,
            pending_op,
            last_chord,
            debug: _,
        } = status;

        let mode_hl = mode != ModeKind::Normal || current_operator.is_some();
        let mut mode = format!("-- {} --", match mode {
            ModeKind::Normal => "NORMAL",
        });

        let mut chord = String::new();

        if let Some(count) = count {
            write!(chord, "{count}").unwrap();
        }

        chord.push_str(&pending_op);

        if let Some(CurrentOperatorStatus {
            operator_chord,
            operator,
            pending_op,
        }) = current_operator
        {
            write!(mode, " ({})", operator.name()).unwrap();

            chord.push_str(&operator_chord);
            chord.push_str(&pending_op);
        }

        let chord_hl = !chord.is_empty();
        if chord.is_empty() {
            let LastChord {
                count,
                operator_prefix,
                chord: prev_chord,
            } = last_chord;

            if let Some(count) = count {
                write!(chord, "{count}").unwrap();
            }

            if let Some(operator_prefix) = operator_prefix {
                chord.push_str(&operator_prefix);
            }

            chord.push_str(&prev_chord);
        }

        let last_action = last_action.map(ActionKind::name).unwrap_or_default();

        let rect = SignedRect::from(area);

        let mode = Span::raw(mode);
        let mode = Layout::prepare(
            rect.width(MODE_WIDTH),
            HorizontalAlignment::Left,
            &mode,
            if mode_hl {
                Style::new()
                    .fg(Color::White)
                    .bg(Color::Black)
                    .bold()
                    .reversed()
            } else {
                Style::new().fg(Color::White)
            },
        );
        let chord = Span::raw(chord);
        let chord = Layout::prepare(
            rect.inset(Insets::ZERO.left(MODE_WIDTH + GAP))
                .width_aligned(CHORD_WIDTH, HorizontalAlignment::Right),
            HorizontalAlignment::Left,
            &chord,
            Style::new().fg(if chord_hl {
                Color::White
            } else {
                Color::DarkGray
            }),
        );

        let last_action = Span::raw(last_action);
        let last_action = Layout::prepare(
            rect.inset(Insets::ZERO.left(MODE_WIDTH + GAP).right(CHORD_WIDTH + GAP)),
            HorizontalAlignment::Right,
            &last_action,
            Style::new().fg(Color::DarkGray),
        );

        mode.render(area, buf);
        last_action.render(area, buf);
        chord.render(area, buf);
    }
}

impl<N: TuiNode> StatefulWidget for &GraphEditor<N> {
    type State = State;

    fn render(self, mut area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if area.height == 0 {
            return;
        }
        area.height = area.height.saturating_sub(1);

        let status_row = Rect {
            y: area.y + area.height,
            height: 1,
            ..area
        };
        GraphEditor::<N>::paint_status(status_row, buf, self.driver.status());

        if area.height > 0 {
            let focus_node;
            let focus_port;
            let focus_edge;
            let focus_point;
            let focus_pos;

            match self.driver.cursor() {
                &Cursor::Node(n) => {
                    focus_node = Some(n);
                    focus_port = None;
                    focus_edge = None;
                    focus_point = None;
                    focus_pos = self.core.graph[n]
                        .outer_rect()
                        .and_then(SignedRect::as_center);
                },
                &Cursor::Port(p) => {
                    focus_node = None;
                    focus_port = Some(p);
                    focus_edge = None;
                    focus_point = None;
                    focus_pos = self.core.graph[p.1 .0]
                        .port_pos(p.1 .1, p.0, false)
                        .to_position();
                },
                Cursor::Edge(e) => {
                    focus_node = None;
                    focus_port = Some(e.anchor_port());
                    focus_edge = Some(e);
                    focus_point = None;
                    focus_pos = self.core.graph[e.from.0]
                        .edge_midpoint(e.from.1, &self.core.graph[e.to.0], e.to.1)
                        .to_position();
                },
                &Cursor::FixedPoint(p) => {
                    focus_node = None;
                    focus_port = None;
                    focus_edge = None;
                    focus_point = Some(p);
                    focus_pos = p.to_position();
                },
            }

            if let Some(focus_pos) = focus_pos {
                state.cursor_position = focus_pos;
            }

            let mut edges = EdgeBuffer::new(area.intersection(buf.area).as_size());

            for edge in self.core.graph.edge_references() {
                let from = edge.source();
                let to = edge.target();
                let &graph::Edge { from_port, to_port } = edge.weight();

                if focus_edge
                    .is_some_and(|e| (e.from, e.to) == (Port(from, from_port), Port(to, to_port)))
                {
                    continue;
                }

                let from = &self.core.graph[from];
                let to = &self.core.graph[to];

                let Some(from_pos) = from
                    .port_pos(from_port, Side::Out, true)
                    .to_position_signed()
                else {
                    continue;
                };
                let Some(to_pos) = to.port_pos(to_port, Side::In, true).to_position_signed() else {
                    continue;
                };

                edges.push(from_pos, to_pos, Color::White);
            }

            if let Some(&EdgeCursor {
                from: Port(from, from_port),
                to: Port(to, to_port),
                anchor: _,
            }) = focus_edge
            {
                let from = &self.core.graph[from];
                let to = &self.core.graph[to];

                if let Some(from_pos) = from
                    .port_pos(from_port, Side::Out, true)
                    .to_position_signed()
                    && let Some(to_pos) = to.port_pos(to_port, Side::In, true).to_position_signed()
                {
                    edges.push(from_pos, to_pos, Color::LightBlue);
                }
            }

            edges.render(area, buf);

            let area = area.intersection(buf.area);

            for node in self.core.graph.node_references() {
                GraphEditor::paint_node(area, buf, node, focus_node, focus_port);
            }

            if let Some(p) = focus_point.and_then(Point::to_position)
                && area.contains(p)
            {
                buf[p].set_style(Style::new().bg(Color::LightBlue));
            }
        }
    }
}
