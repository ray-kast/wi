use masonry::kurbo::Point;
use wi_xilem::graph;
use xilem::{
    view::{flex, FlexExt},
    winit::{dpi::LogicalSize, window::Window},
    EventLoop, WidgetView, Xilem,
};

#[derive(Default)]
struct State {}

fn app_logic(_data: &mut State) -> impl WidgetView<State> + use<> {
    flex((graph()
        .with(|g| {
            g.node(Point::new(0.0, 0.0), 3, 2)
                .node(Point::new(240.0, 0.0), 1, 1)
                .node(Point::new(240.0, 64.0), 2, 0)
                .edge((0, 0), (1, 0))
                .edge((0, 1), (2, 0))
                .edge((0, 1), (2, 1))
        })
        .flex(1.0),))
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
