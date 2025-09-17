use masonry::kurbo::Point;
use xilem::{
    view::{flex, label},
    EventLoop, WidgetView, Xilem,
};

use crate::graph::graph;

mod graph;

#[derive(Default)]
struct State {}

fn app_logic(data: &mut State) -> impl WidgetView<State> + use<> {
    flex((
        label("hello world!"),
        graph().with(|g| {
            g.node(Point::new(0.0, 0.0))
                .node(Point::new(100.0, 0.0))
                .edge((0, 1), (1, 0))
        }),
    ))
}

fn main() {
    let app = Xilem::new(State::default(), app_logic);
    app.run_windowed(EventLoop::with_user_event(), "wi".into())
        .unwrap();
}
