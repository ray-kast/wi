use masonry::kurbo::Point;
use wi_xilem::{
    graph::{Checked, Edge, Graph, NodeStyle, Port},
    view::graph_editor,
};
use xilem::{
    view::{flex_col, FlexExt},
    winit::dpi::LogicalSize,
    EventLoop, WidgetView, WindowOptions, Xilem,
};

#[derive(Debug, Clone)]
struct Node {
    name: &'static str,
    style: NodeStyle,
    pos: Point,
    inputs: Vec<&'static str>,
    outputs: Vec<&'static str>,
}

impl wi_xilem::graph::Node for Node {
    type Icon = ();
    type PortShape = ();
    type Prototype = ();
    type Widget = ();

    const PROTOTYPE: Self::Prototype = ();

    fn create((): Self::Prototype, position: Point) -> Self {
        Self {
            name: "tmp",
            style: NodeStyle::Large,
            pos: position,
            inputs: vec!["in 1", "in 2"],
            outputs: vec!["out 1", "out 2"],
        }
    }

    #[inline]
    fn in_arity(&self) -> u16 {
        self.inputs
            .len()
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    #[inline]
    fn out_arity(&self) -> u16 {
        self.outputs
            .len()
            .try_into()
            .unwrap_or_else(|_| unreachable!())
    }

    #[inline]
    fn in_port(
        &self,
        index: u16,
    ) -> Port<'_, Self::PortShape, wi_xilem::graph::InputLabel<'_, Self::Widget>> {
        Port {
            name: self.inputs[usize::from(index)].into(),
            ..Port::default()
        }
    }

    #[inline]
    fn out_port(&self, index: u16) -> Port<'_, Self::PortShape, wi_xilem::graph::OutputLabel<'_>> {
        Port {
            name: self.outputs[usize::from(index)].into(),
            ..Port::default()
        }
    }

    #[inline]
    fn name(&self) -> std::borrow::Cow<'_, str> { self.name.into() }

    #[inline]
    fn position(&self) -> Point { self.pos }

    #[inline]
    fn position_mut(&mut self) -> Option<&mut Point> { Some(&mut self.pos) }

    #[inline]
    fn style(&self) -> NodeStyle { self.style }
}

struct State {
    graph: Checked<Graph<Node>>,
}

fn app_logic(state: &mut State) -> impl WidgetView<State> + use<> {
    flex_col((
        graph_editor(state.graph.clone(), |state: &mut State, graph, _| {
            state.graph = graph;
        })
        .flex(1.0),
    ))
}

fn main() {
    let mut graph = Graph::<Node>::new();

    let a = graph.add_node(
        Node {
            name: "thing",
            style: NodeStyle::Small,
            pos: Point::new(0.0, 0.0),
            inputs: vec![],
            outputs: vec!["value"],
        }
        .into(),
    );

    let b = graph.add_node(
        Node {
            name: "smal",
            style: NodeStyle::Medium,
            pos: Point::new(240.0, 0.0),
            inputs: vec!["input", "extra"],
            outputs: vec!["output"],
        }
        .into(),
    );

    let c = graph.add_node(
        Node {
            name: "large node",
            style: NodeStyle::Large,
            pos: Point::new(240.0, 72.0),
            inputs: vec!["first", "second"],
            outputs: vec!["output"],
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

    let app = Xilem::new_simple(
        State {
            graph: Checked::new(graph.into()),
        },
        app_logic,
        WindowOptions::new("wi")
            .with_min_inner_size(LogicalSize::new(525.0, 350.0))
            .with_initial_inner_size(LogicalSize::new(810.0, 540.0))
            .with_resizable(true),
    );
    app.run_in(EventLoop::with_user_event()).unwrap();
}
