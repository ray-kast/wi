use std::io;

use ratatui::{
    crossterm::event::{self, Event, KeyEvent, KeyEventKind},
    prelude::*,
};
use wi_ratatui::graph::{
    Checked, Edge, Graph, Label, LargeStyle, Port, Ports, SmallStyle, StyleKind, WidgetLabel,
    WidgetStyle,
};

struct Node {
    name: &'static str,
    pos: Position,
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
    type Position = Position;
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
            pos: Position::new(0, 0),
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
            pos: Position::new(30, 0),
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
            pos: Position::new(30, 6),
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
            pos: Position::new(60, 7),
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
}

fn render(state: &mut WidgetState) -> impl FnOnce(&mut Frame) {
    |frame| frame.render_stateful_widget(&state.graph, frame.area(), &mut state.graph_state)
}

fn render_loop<W: io::Write>(
    term: &mut Terminal<CrosstermBackend<W>>,
    state: &mut WidgetState,
) -> io::Result<bool> {
    term.draw(render(state))?;

    match event::read()? {
        Event::Key(KeyEvent {
            kind: KeyEventKind::Press,
            ..
        }) => Ok(false),
        _ => Ok(true),
    }
}

fn main() {
    let mut term = ratatui::init();
    let (graph, graph_state) = wi_ratatui::widget::GraphEditor::new(Checked::new(graph().into()));
    let mut state = WidgetState { graph, graph_state };

    let err = loop {
        match render_loop(&mut term, &mut state) {
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
