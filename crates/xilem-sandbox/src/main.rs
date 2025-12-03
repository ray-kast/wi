use masonry::kurbo::Point;
use wi_xilem::{
    graph::{
        Checked, Edge, Graph, Label, LargeStyle, Port, Ports, SmallStyle, StyleKind, WidgetLabel,
        WidgetStyle,
    },
    view::{graph_editor, graph_editor_modals},
};
use xilem::{
    view::{button, flex_row, label, zstack},
    winit::dpi::LogicalSize,
    EventLoop, WidgetView, WindowOptions, Xilem,
};

#[derive(Debug, Clone)]
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

impl wi_xilem::graph::Node for Node {
    type Icon = ();
    type PortShape = ();
    type Position = Point;
    type Prototype = ();
    type Widget = ();

    fn create((): Self::Prototype, position: Point) -> Self {
        Self {
            name: "tmp",
            style: LargeStyle {
                label: Label::default(),
                in_ports: ports(["in 1", "in 2"]),
                out_ports: ports(["out 1", "out 2"]),
            }
            .into(),
            pos: position,
        }
    }

    #[inline]
    fn name(&self) -> std::borrow::Cow<'_, str> { self.name.into() }

    #[inline]
    fn description(&self) -> std::borrow::Cow<'_, str> { self.name.into() }

    #[inline]
    fn position(&self) -> Point { self.pos }

    #[inline]
    fn position_mut(&mut self) -> Option<&mut Point> { Some(&mut self.pos) }

    #[inline]
    fn style(&self) -> StyleKind<'_, Self::Icon, Self::PortShape, Self::Widget> {
        self.style.clone()
    }
}

impl wi_xilem::graph::WidgetNode for Node {}

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
            pos: Point::new(240.0, 0.0),
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
            pos: Point::new(240.0, 72.0),
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
            pos: Point::new(480.0, 96.0),
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

struct State {
    graph: Checked<Graph<Node>>,
}

fn app_logic(_: &mut State) -> impl WidgetView<State> + use<> {
    graph_editor_modals(|state: &mut State, params, mut modal| {
        zstack((
            graph_editor(state.graph.clone(), params, |s: &mut State, g, _| {
                s.graph = g;
            }),
            modal.want_node_prototype().map(|m| {
                flex_row((
                    button(label("OK"), move |_| m.respond(Some(()))),
                    button(label("Cancel"), move |_| m.respond(None)),
                ))
            }),
        ))
    })
}

fn main() {
    let app = Xilem::new_simple(
        State {
            graph: Checked::new(graph().into()),
        },
        app_logic,
        WindowOptions::new("wi")
            .with_min_inner_size(LogicalSize::new(525.0, 350.0))
            .with_initial_inner_size(LogicalSize::new(810.0, 540.0))
            .with_resizable(true),
    );
    app.run_in(EventLoop::with_user_event()).unwrap();
}
