use derive_where::derive_where;
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Style},
    text::Span,
    widgets::StatefulWidget,
};
use wi_core::{
    opinions::graph::{Checked, Graph, Label, Port, WidgetLabel},
    GraphWidgetDriver, Side,
};

use self::core::EditorCore;
use crate::{
    graph::{NodeStyle as _, StyleKind as NodeStyle, TuiNode},
    widget::node::NodeExt,
};

mod core;
mod node;
mod vector;

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
}

#[derive(Debug, Clone, Copy)]
pub struct State(());

impl<N: TuiNode> StatefulWidget for &GraphEditor<N> {
    type State = State;

    fn render(self, area: Rect, buf: &mut Buffer, State(()): &mut Self::State) {
        let area = area.intersection(buf.area);

        for node in self.core.graph.node_weights() {
            let style = node.style();

            let head_rect = node.head_rect();
            for pos in area.intersection(head_rect).positions() {
                let cell = &mut buf[pos];
                let style = cell.style();
                cell.set_style(style.bg(Color::Green).fg(Color::White));
            }

            let body_rect = node.body_rect();
            if !matches!(style, NodeStyle::Widget(_)) {
                for pos in area.intersection(body_rect).positions() {
                    let cell = &mut buf[pos];
                    let style = cell.style();
                    cell.set_style(style.bg(Color::DarkGray).fg(Color::Gray));
                }
            }

            let label = match &style {
                NodeStyle::Widget(w) => w.label.as_ref(),
                NodeStyle::Small(s) => WidgetLabel::Label(s.label.as_ref()),
                NodeStyle::Large(l) => WidgetLabel::Label(l.label.as_ref()),
            };
            let label = match label {
                WidgetLabel::Label(Label { content, icon }) => Span::raw(node.name()),
                WidgetLabel::Widget(w) => todo!(),
            };

            let label_rect = node.label_rect();
            let label_width = label.width().try_into().unwrap_or(u16::MAX);
            let label_rect = Rect {
                x: label_rect.x + label_rect.width.saturating_sub(label_width) / 2,
                y: label_rect.y,
                width: label_rect.width.min(label_width),
                height: label_rect.height,
            };

            for (pos, grapheme) in area.intersection(label_rect).positions().zip(
                label
                    .styled_graphemes(match node.style() {
                        NodeStyle::Widget(_) => Style::default().fg(Color::White),
                        NodeStyle::Small(_) | NodeStyle::Large(_) => {
                            Style::default().fg(Color::White).bold()
                        },
                    })
                    .skip(area.x.saturating_sub(label_rect.x).into()),
            ) {
                let cell = &mut buf[pos];
                cell.set_style(grapheme.style);
                cell.set_symbol(grapheme.symbol);
            }

            let label_rect = node.port_label_rect();
            for (i, port) in style.in_ports() {
                let Port {
                    name,
                    description,
                    shape,
                    label,
                } = port;

                let y = node.port_inner_y(i, Side::In);
                let pos = Position {
                    x: body_rect.x,
                    y: body_rect.y + y,
                };

                if area.contains(pos) {
                    let cell = &mut buf[pos];
                    cell.set_style(Style::default().fg(Color::White));
                    cell.set_char('╴');
                }

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
                    for (pos, grapheme) in area.intersection(label_rect).positions().zip(
                        label
                            .styled_graphemes(Style::default().fg(Color::White))
                            .skip(area.x.saturating_sub(label_rect.x).into()),
                    ) {
                        let cell = &mut buf[pos];
                        cell.set_style(grapheme.style);
                        cell.set_symbol(grapheme.symbol);
                    }
                }
            }

            for (i, port) in style.out_ports() {
                let Port {
                    name,
                    description,
                    shape,
                    label,
                } = port;

                let y = node.port_inner_y(i, Side::Out);
                let pos = Position {
                    x: body_rect.x + body_rect.width - 1,
                    y: body_rect.y + y,
                };

                if area.contains(pos) {
                    let cell = &mut buf[pos];
                    cell.set_style(Style::default().fg(Color::White));
                    cell.set_char('╶');
                }

                if let Some(label) = label {
                    let Label { content, icon } = label.as_ref();
                    let label = Span::raw(content.unwrap_or(name));

                    let label_width = label.width().try_into().unwrap_or(u16::MAX);
                    let label_rect = Rect {
                        x: label_rect.x + label_rect.width.saturating_sub(label_width),
                        y: label_rect.y + y,
                        width: label_rect.width.min(label_width),
                        height: 1,
                    };
                    for (pos, grapheme) in area.intersection(label_rect).positions().zip(
                        label
                            .styled_graphemes(Style::default().fg(Color::White))
                            .skip(area.x.saturating_sub(label_rect.x).into()),
                    ) {
                        let cell = &mut buf[pos];
                        cell.set_style(grapheme.style);
                        cell.set_symbol(grapheme.symbol);
                    }
                }
            }
        }
    }
}
