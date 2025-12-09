use std::{borrow::Cow, fmt::Write, sync::Arc};

use derive_where::derive_where;
use keyboard_types::{Modifiers, NamedKey};
use petgraph::{
    graph::NodeIndex,
    visit::{EdgeRef, IntoEdgeReferences, IntoNodeReferences},
};
use ratatui::{
    buffer::Buffer,
    layout::{HorizontalAlignment, Position, Rect, Size},
    style::{Color, Style},
    text::Span,
    widgets::{StatefulWidget, Widget},
};
use wi_core::{
    opinions::graph::{self, Checked, Graph, Label, WidgetLabel},
    ActionKind, CurrentOperatorStatus, Cursor, EdgeCursor, GraphWidgetDriver, LastChord, ModeKind,
    Port, Side, SidedPort, Status, WPort,
};

pub use self::core::Cx;
use self::core::EditorCore;
use super::vector::Point;
use crate::{
    graph::{NodeStyle as _, StyleKind as NodeStyle, TuiNode},
    widget::{
        edge::Edge,
        node::NodeExt,
        text::{render_span, Layout},
    },
};

mod cell;
mod core;
mod edge;
mod node;
mod text;

#[derive_where(Debug; N, N::Prototype)]
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
            State(()),
        )
    }

    pub fn graph(&self) -> Checked<Graph<N>> {
        // SAFETY: All operations on self.core.graph preserve the validity of
        //         the graph
        unsafe { Checked::new_unchecked(Arc::clone(&self.core.graph)) }
    }

    pub fn handle_char_input(&mut self, chars: &str, mods: Modifiers, cx: &mut Cx<'_>) -> bool {
        self.driver
            .handle_char_input(&mut self.core, chars, mods, cx)
    }

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

#[cfg(feature = "crossterm")]
mod crossterm {
    use keyboard_types::{Modifiers, NamedKey};
    use ratatui::crossterm::event::{
        KeyCode, KeyEventState, KeyModifiers, MediaKeyCode, ModifierKeyCode,
    };

    use super::{Cx, GraphEditor, TuiNode};

    impl<N: TuiNode> GraphEditor<N> {
        #[expect(clippy::too_many_lines, reason = "Oops! All match block")]
        pub fn handle_crossterm_key(
            &mut self,
            code: KeyCode,
            mods: KeyModifiers,
            state: KeyEventState,
            cx: &mut Cx<'_>,
        ) -> bool {
            let _ = state;
            let mut mods_out = Modifiers::empty();
            for modifier in mods.iter() {
                mods_out.insert(match modifier {
                    KeyModifiers::ALT => Modifiers::ALT,
                    KeyModifiers::META | KeyModifiers::SUPER | KeyModifiers::HYPER => {
                        Modifiers::META
                    },
                    KeyModifiers::SHIFT => Modifiers::SHIFT,
                    KeyModifiers::CONTROL => Modifiers::CONTROL,
                    m => panic!("Unknown modifier(s): {m}"),
                });
            }

            let key = match code {
                KeyCode::Backspace => NamedKey::Backspace,
                KeyCode::Enter => NamedKey::Enter,
                KeyCode::Left => NamedKey::ArrowLeft,
                KeyCode::Right => NamedKey::ArrowRight,
                KeyCode::Up => NamedKey::ArrowUp,
                KeyCode::Down => NamedKey::ArrowDown,
                KeyCode::Home => NamedKey::Home,
                KeyCode::End => NamedKey::End,
                KeyCode::PageUp => NamedKey::PageUp,
                KeyCode::PageDown => NamedKey::PageDown,
                KeyCode::Tab => NamedKey::Tab,
                KeyCode::Delete => NamedKey::Delete,
                KeyCode::Insert => NamedKey::Insert,
                KeyCode::F(1) => NamedKey::F1,
                KeyCode::F(2) => NamedKey::F2,
                KeyCode::F(3) => NamedKey::F3,
                KeyCode::F(4) => NamedKey::F4,
                KeyCode::F(5) => NamedKey::F5,
                KeyCode::F(6) => NamedKey::F6,
                KeyCode::F(7) => NamedKey::F7,
                KeyCode::F(8) => NamedKey::F8,
                KeyCode::F(9) => NamedKey::F9,
                KeyCode::F(10) => NamedKey::F10,
                KeyCode::F(11) => NamedKey::F11,
                KeyCode::F(12) => NamedKey::F12,
                KeyCode::F(13) => NamedKey::F13,
                KeyCode::F(14) => NamedKey::F14,
                KeyCode::F(15) => NamedKey::F15,
                KeyCode::F(16) => NamedKey::F16,
                KeyCode::F(17) => NamedKey::F17,
                KeyCode::F(18) => NamedKey::F18,
                KeyCode::F(19) => NamedKey::F19,
                KeyCode::F(20) => NamedKey::F20,
                KeyCode::F(21) => NamedKey::F21,
                KeyCode::F(22) => NamedKey::F22,
                KeyCode::F(23) => NamedKey::F23,
                KeyCode::F(24) => NamedKey::F24,
                KeyCode::F(25) => NamedKey::F25,
                KeyCode::F(26) => NamedKey::F26,
                KeyCode::F(27) => NamedKey::F27,
                KeyCode::F(28) => NamedKey::F28,
                KeyCode::F(29) => NamedKey::F29,
                KeyCode::F(30) => NamedKey::F30,
                KeyCode::F(31) => NamedKey::F31,
                KeyCode::F(32) => NamedKey::F32,
                KeyCode::F(33) => NamedKey::F33,
                KeyCode::F(34) => NamedKey::F34,
                KeyCode::F(35) => NamedKey::F35,
                KeyCode::Char(c) => {
                    return self.driver.handle_char_input(
                        &mut self.core,
                        &c.to_string(),
                        mods_out,
                        cx,
                    )
                },
                KeyCode::Esc => NamedKey::Escape,
                KeyCode::CapsLock => NamedKey::CapsLock,
                KeyCode::ScrollLock => NamedKey::ScrollLock,
                KeyCode::NumLock => NamedKey::NumLock,
                KeyCode::PrintScreen => NamedKey::PrintScreen,
                KeyCode::Pause => NamedKey::Pause,
                KeyCode::Menu => NamedKey::ContextMenu,
                KeyCode::Media(MediaKeyCode::Play) => NamedKey::MediaPlay,
                KeyCode::Media(MediaKeyCode::Pause) => NamedKey::MediaPause,
                KeyCode::Media(MediaKeyCode::PlayPause) => NamedKey::MediaPlayPause,
                KeyCode::Media(MediaKeyCode::Stop) => NamedKey::MediaStop,
                KeyCode::Media(MediaKeyCode::FastForward) => NamedKey::MediaFastForward,
                KeyCode::Media(MediaKeyCode::Rewind) => NamedKey::MediaRewind,
                KeyCode::Media(MediaKeyCode::TrackNext) => NamedKey::MediaTrackNext,
                KeyCode::Media(MediaKeyCode::TrackPrevious) => NamedKey::MediaTrackPrevious,
                KeyCode::Media(MediaKeyCode::Record) => NamedKey::MediaRecord,
                KeyCode::Media(MediaKeyCode::LowerVolume) => NamedKey::AudioVolumeDown,
                KeyCode::Media(MediaKeyCode::RaiseVolume) => NamedKey::AudioVolumeUp,
                KeyCode::Media(MediaKeyCode::MuteVolume) => NamedKey::AudioVolumeMute,
                KeyCode::Modifier(ModifierKeyCode::LeftShift | ModifierKeyCode::RightShift) => {
                    NamedKey::Shift
                },
                KeyCode::Modifier(ModifierKeyCode::LeftControl | ModifierKeyCode::RightControl) => {
                    NamedKey::Control
                },
                KeyCode::Modifier(ModifierKeyCode::LeftAlt | ModifierKeyCode::RightAlt) => {
                    NamedKey::Alt
                },
                KeyCode::Modifier(
                    ModifierKeyCode::LeftSuper
                    | ModifierKeyCode::LeftHyper
                    | ModifierKeyCode::LeftMeta
                    | ModifierKeyCode::RightSuper
                    | ModifierKeyCode::RightHyper
                    | ModifierKeyCode::RightMeta,
                ) => NamedKey::Meta,
                KeyCode::BackTab
                | KeyCode::F(0 | 36..)
                | KeyCode::Null
                | KeyCode::KeypadBegin
                | KeyCode::Media(MediaKeyCode::Reverse)
                | KeyCode::Modifier(
                    ModifierKeyCode::IsoLevel3Shift | ModifierKeyCode::IsoLevel5Shift,
                ) => NamedKey::Unidentified,
            };

            self.driver
                .handle_named_keypress(&mut self.core, key, mods_out, cx)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct State(());

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
        focus_port: Option<(Side, &WPort<EditorCore<N>>)>,
    ) {
        let (idx, node) = node;
        let focused = focus_node == Some(idx);
        let style = node.style();

        let Some((head_rect, _)) = node.head_rect_offset() else {
            return;
        };
        let Some((body_rect, _)) = node.body_rect_offset() else {
            return;
        };

        for pos in area.intersection(head_rect).positions() {
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
            );
        }

        for pos in area.intersection(body_rect).positions() {
            let cell = &mut buf[pos];
            let cell_style = cell.style();
            cell.set_style(
                cell_style
                    .bg(match (focused, &style) {
                        (_, NodeStyle::Large(_)) | (false, _) => Color::DarkGray,
                        (true, _) => Color::LightBlue,
                    })
                    .fg(Color::Gray),
            );

            if focused {
                cell.set_char(' ');
            }
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

        let Some((label_rect, label_offs)) = node.label_rect() else {
            unreachable!()
        };

        render_span(
            label_rect,
            buf,
            label_offs,
            HorizontalAlignment::Center,
            &label,
            Style::new().fg(Color::White).bold(),
        );

        let Some((label_rect, label_offs)) = node.port_label_rect_offset() else {
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
            let pos = Position {
                x: body_rect.x,
                y: body_rect.y + y,
            };

            Self::paint_port(
                area,
                buf,
                pos,
                Side::In,
                focus_port == Some((Side::In, &Port(idx, i))),
            );

            if let Some(label) = label {
                let label = match label.as_ref() {
                    WidgetLabel::Label(Label { content, icon }) => {
                        Span::raw(content.unwrap_or(name))
                    },
                    WidgetLabel::Widget(w) => todo!(),
                };

                let label_rect = Rect {
                    x: label_rect.x,
                    y: label_rect.y + y,
                    width: label_rect.width,
                    height: 1,
                };
                let label_offs = Position {
                    x: label_offs.x,
                    y: label_offs.y.saturating_sub(y),
                };

                render_span(
                    label_rect,
                    buf,
                    label_offs,
                    HorizontalAlignment::Left,
                    &label,
                    Style::new().fg(Color::White).not_bold(),
                );
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
            let pos = Position {
                x: body_rect.x + body_rect.width - 1,
                y: body_rect.y + y,
            };

            Self::paint_port(
                area,
                buf,
                pos,
                Side::Out,
                focus_port == Some((Side::Out, &Port(idx, i))),
            );

            if let Some(label) = label {
                let Label { content, icon } = label.as_ref();
                let label = Span::raw(content.unwrap_or(name));

                let label_rect = Rect {
                    x: label_rect.x,
                    y: label_rect.y + y,
                    width: label_rect.width,
                    height: 1,
                };
                let label_offs = Position {
                    x: label_offs.x,
                    y: label_offs.y.saturating_sub(y),
                };

                render_span(
                    label_rect,
                    buf,
                    label_offs,
                    HorizontalAlignment::Right,
                    &label,
                    Style::new().fg(Color::White).not_bold(),
                );
            }
        }
    }

    #[expect(
        clippy::too_many_lines,
        reason = "The logic here would be difficult to refactor"
    )]
    fn paint_status(area: Rect, buf: &mut Buffer, status: Status) {
        const MODE_WIDTH: u16 = 40;
        const CHORD_WIDTH: u16 = 10;

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
        let mut mode = Cow::Borrowed(match mode {
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
            let mut mode_str = mode.into_owned();
            write!(mode_str, " ({})", operator.name()).unwrap();
            mode = mode_str.into();

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

        let mode = Span::raw(mode);
        let mode = Layout::prepare(
            Size {
                width: MODE_WIDTH,
                height: 1,
            },
            Position::ORIGIN,
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
            Size {
                width: CHORD_WIDTH,
                height: 1,
            },
            Position::ORIGIN,
            HorizontalAlignment::Left,
            &chord,
            Style::new().fg(if chord_hl {
                Color::White
            } else {
                Color::DarkGray
            }),
        );

        let last_action_offs = mode.width().saturating_add(1);
        let chord_offs = mode.width().max(area.width.saturating_sub(CHORD_WIDTH));

        let last_action = Span::raw(last_action);
        let last_action = Layout::prepare(
            Size {
                width: area
                    .width
                    .saturating_sub(
                        CHORD_WIDTH
                            .max(chord.width())
                            .saturating_add(last_action_offs),
                    )
                    .saturating_sub(1),
                height: 1,
            },
            Position::ORIGIN,
            HorizontalAlignment::Right,
            &last_action,
            Style::new().fg(Color::DarkGray),
        );

        mode.render(area, buf);

        last_action.render(
            Rect {
                x: area.x.saturating_add(last_action_offs),
                width: area
                    .width
                    .saturating_sub(last_action_offs)
                    .saturating_sub(1),
                ..area
            },
            buf,
        );

        chord.render(
            Rect {
                x: area.x.saturating_add(chord_offs),
                width: area.width.saturating_sub(chord_offs),
                ..area
            },
            buf,
        );
    }
}

impl<N: TuiNode> StatefulWidget for &GraphEditor<N> {
    type State = State;

    fn render(self, mut area: Rect, buf: &mut Buffer, State(()): &mut Self::State) {
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

            match self.driver.cursor() {
                &Cursor::Node(n) => {
                    focus_node = Some(n);
                    focus_port = None;
                    focus_edge = None;
                    focus_point = None;
                },
                &Cursor::Port(SidedPort(s, ref p)) => {
                    focus_node = None;
                    focus_port = Some((s, p));
                    focus_edge = None;
                    focus_point = None;
                },
                Cursor::Edge(e) => {
                    focus_node = None;
                    focus_port = Some(e.anchor_port());
                    focus_edge = Some(e);
                    focus_point = None;
                },
                &Cursor::FixedPoint(p) => {
                    focus_node = None;
                    focus_port = None;
                    focus_edge = None;
                    focus_point = Some(p);
                },
            }

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

                let Some(from_pos) = from.port_pos(from_port, Side::Out, true).to_position() else {
                    continue;
                };
                let Some(to_pos) = to.port_pos(to_port, Side::In, true).to_position() else {
                    continue;
                };

                Edge::new(from_pos, to_pos, Color::White).render(area, buf);
            }

            if let Some(&EdgeCursor {
                from: Port(from, from_port),
                to: Port(to, to_port),
                anchor: _,
            }) = focus_edge
            {
                let from = &self.core.graph[from];
                let to = &self.core.graph[to];

                if let Some(from_pos) = from.port_pos(from_port, Side::Out, true).to_position()
                    && let Some(to_pos) = to.port_pos(to_port, Side::In, true).to_position()
                {
                    Edge::new(from_pos, to_pos, Color::LightBlue).render(area, buf);
                }
            }

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
