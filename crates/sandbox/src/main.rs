use masonry::kurbo::Point;
use wi_xilem::graph;
use winit::{dpi::LogicalSize, window::Window};
use xilem::{
    view::{flex, label},
    EventLoop, WidgetView, Xilem,
};

#[derive(Default)]
struct State {}

fn app_logic(data: &mut State) -> impl WidgetView<State> + use<> {
    flex((
        label("Graph"),
        graph().with(|g| {
            g.node(Point::new(0.0, 0.0), 0, 2)
                .node(Point::new(100.0, 0.0), 1, 1)
                .edge((0, 1), (1, 0))
        }),
    ))
}

fn main() {
    let app = Xilem::new(State::default(), app_logic);
    let attrs = Window::default_attributes()
        .with_title("wi")
        .with_min_inner_size(LogicalSize::new(525.0, 350.0))
        .with_resizable(true);
    app.run_windowed_in(EventLoop::with_user_event(), attrs)
        .unwrap();
}
