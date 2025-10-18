use masonry::kurbo::Point;
use wi_xilem::{
    graph::{Edge, Graph, LargeNode, Node, NodeKind, PortInfo, SmallNode, WidgetNode},
    graph_editor, Checked,
};
use xilem::{
    view::{flex, FlexExt},
    winit::{dpi::LogicalSize, window::Window},
    EventLoop, WidgetView, Xilem,
};

#[derive(Default)]
struct State {}

fn app_logic(_data: &mut State) -> impl WidgetView<State> + use<> {
    let mut graph = Graph::<(), (), ()>::new();

    let a = graph.add_node(
        Node {
            name: "thing".into(),
            icon: (),
            position: Point::new(0.0, 0.0),
            width: 84.0,
            kind: NodeKind::Widget(WidgetNode {
                widget: None,
                input: None,
                output: Some(PortInfo {
                    name: "value".into(),
                    data: (),
                }),
            }),
        }
        .into(),
    );

    let b = graph.add_node(
        Node {
            name: "smal".into(),
            icon: (),
            position: Point::new(240.0, 0.0),
            width: 96.0,
            kind: NodeKind::Small(SmallNode {
                inputs: vec![
                    PortInfo {
                        name: "input".into(),
                        data: (),
                    },
                    PortInfo {
                        name: "extra".into(),
                        data: (),
                    },
                ],
                outputs: vec![PortInfo {
                    name: "output".into(),
                    data: (),
                }],
            }),
        }
        .into(),
    );

    let c = graph.add_node(
        Node {
            name: "large node".into(),
            icon: (),
            position: Point::new(240.0, 72.0),
            width: 128.0,
            kind: NodeKind::Large(LargeNode {
                inputs: vec![
                    (
                        PortInfo {
                            name: "first".into(),
                            data: (),
                        },
                        None,
                    ),
                    (
                        PortInfo {
                            name: "second".into(),
                            data: (),
                        },
                        Some(()),
                    ),
                ],
                outputs: vec![PortInfo {
                    name: "output".into(),
                    data: (),
                }],
            }),
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

    flex((graph_editor(Checked::new(graph.into())).flex(1.0),))
}

fn main() {
    let app = Xilem::new(State::default(), app_logic);
    let attrs = Window::default_attributes()
        .with_title("wi")
        .with_min_inner_size(LogicalSize::new(525.0, 350.0))
        .with_inner_size(LogicalSize::new(810.0, 540.0))
        .with_resizable(true);
    app.run_windowed_in(EventLoop::with_user_event(), attrs)
        .unwrap();
}
