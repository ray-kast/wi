use std::{io, mem};

use ratatui::{
    crossterm::event::{self, Event, KeyEvent, KeyEventKind},
    prelude::*,
};
use wi_ratatui::{
    graph::{
        Checked, Edge, Graph, Label, LargeStyle, Port, Ports, SmallStyle, StyleKind, WidgetLabel,
        WidgetStyle,
    },
    vector::Point,
    widget::Cx,
};

struct Node {
    name: &'static str,
    pos: Point,
    style: StyleKind<'static, (), (), ()>,
}

fn port<L: Default>(s: &'static str) -> Port<'static, (), L> {
    Port {
        name: s.into(),
        description: "".into(),
        shape: (),
        label: L::default(),
    }
}

fn ports<I: IntoIterator<Item = &'static str>, L: Default>(
    it: I,
) -> Ports<'static, Port<'static, (), L>> {
    Ports::Vec(it.into_iter().map(port).collect())
}

impl wi_ratatui::graph::Node for Node {
    type Icon = ();
    type PortShape = ();
    type Position = Point;
    type Prototype = ();
    type Widget = ();

    fn create((): Self::Prototype, position: Self::Position) -> Self {
        Self {
            name: "tmp",
            pos: position,
            style: LargeStyle {
                label: Label::default(),
                in_ports: ports(["in 1", "in 2"]),
                out_ports: ports(["out 1", "out 2"]),
            }
            .into(),
        }
    }

    #[inline]
    fn name(&self) -> std::borrow::Cow<'_, str> { self.name.into() }

    #[inline]
    fn description(&self) -> std::borrow::Cow<'_, str> { self.name.into() }

    #[inline]
    fn position(&self) -> Self::Position { self.pos }

    #[inline]
    fn position_mut(&mut self) -> Option<&mut Self::Position> { Some(&mut self.pos) }

    #[inline]
    fn style(&self) -> StyleKind<'_, Self::Icon, Self::PortShape, Self::Widget> {
        self.style.clone()
    }
}

impl wi_ratatui::graph::TuiNode for Node {}

fn graph() -> Graph<Node> {
    let mut graph = Graph::new();

    let a = graph.add_node(
        Node {
            name: "thing",
            pos: Point::new(0.0, 0.0),
            style: WidgetStyle {
                label: WidgetLabel::default(),
                in_port: None,
                out_port: Some(port("value")),
            }
            .into(),
        }
        .into(),
    );

    let b = graph.add_node(
        Node {
            name: "smal",
            pos: Point::new(30.0, 0.0),
            style: SmallStyle {
                label: Label::default(),
                in_ports: ports(["input", "extra"]),
                out_ports: ports(["output"]),
            }
            .into(),
        }
        .into(),
    );

    let c = graph.add_node(
        Node {
            name: "large node",
            pos: Point::new(30.0, 6.0),
            style: LargeStyle {
                label: Label::default(),
                in_ports: ports(["first", "second"]),
                out_ports: ports(["output", "aux"]),
            }
            .into(),
        }
        .into(),
    );

    let d = graph.add_node(
        Node {
            name: "fold",
            pos: Point::new(60.0, 7.0),
            style: SmallStyle {
                label: Label::default(),
                in_ports: ports(["a", "b"]),
                out_ports: ports(["out"]),
            }
            .into(),
        }
        .into(),
    );

    graph.add_edge(a, b, Edge {
        from_port: 0,
        to_port: 0,
    });
    graph.add_edge(a, c, Edge {
        from_port: 0,
        to_port: 0,
    });
    graph.add_edge(a, c, Edge {
        from_port: 0,
        to_port: 1,
    });
    graph.add_edge(c, d, Edge {
        from_port: 0,
        to_port: 0,
    });
    graph.add_edge(c, d, Edge {
        from_port: 1,
        to_port: 1,
    });

    graph
}

struct WidgetState {
    graph: wi_ratatui::widget::GraphEditor<Node>,
    graph_state: wi_ratatui::widget::State,
    render_requested: bool,
}

fn render(state: &mut WidgetState) -> impl FnOnce(&mut Frame) {
    |frame| frame.render_stateful_widget(&state.graph, frame.area(), &mut state.graph_state)
}

fn event_loop<W: io::Write>(
    term: &mut Terminal<CrosstermBackend<W>>,
    state: &mut WidgetState,
) -> io::Result<bool> {
    if mem::take(&mut state.render_requested) {
        term.draw(render(state))?;
    }

    match event::read()? {
        Event::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: key_state,
        }) => {
            let mut cx = Cx::new();

            state
                .graph
                .handle_crossterm_key(code, modifiers, key_state, &mut cx);
            state.render_requested = cx.render_requested;

            Ok(!cx.quit_requested)
        },
        _ => Ok(true),
    }
}

fn main() {
    let mut term = ratatui::init();
    let (graph, graph_state) = wi_ratatui::widget::GraphEditor::new(Checked::new(graph().into()));
    let mut state = WidgetState {
        graph,
        graph_state,
        render_requested: true,
    };

    let err = loop {
        match event_loop(&mut term, &mut state) {
            Ok(true) => (),
            Ok(false) => break None,
            Err(err) => break Some(err),
        }
    };

    ratatui::restore();
    if let Some(err) = err {
        eprintln!("ERROR: {err}");
        std::process::exit(1);
    }
}
